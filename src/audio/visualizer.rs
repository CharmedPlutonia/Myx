//! Real-time FFT frequency-band visualizer.
//!
//! Vendored and adapted from aome510/spotify-player (`ui/streaming.rs`, MIT,
//! © 2021 Thang Pham). Decoupled here from that app's global state so it writes
//! to a plain `Arc<Mutex<VisBands>>` that myx owns.
//!
//! The design is a **tee'd audio sink**: it forwards every packet it receives
//! unchanged to the real backend while computing a windowed FFT on a copy. The
//! equalizer sits immediately before this sink, so these bands describe the
//! sound after EQ. The hot path is allocation-free and the UI reads the bands
//! via `try_lock`, so the audio thread never stalls waiting on a render.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use librespot_playback::audio_backend::{Sink, SinkResult};
use librespot_playback::convert::Converter;
use librespot_playback::decoder::AudioPacket;
use rustfft::{num_complex::Complex, FftPlanner};

const FFT_SIZE: usize = 1024;
/// New samples consumed per FFT frame (overlap = FFT_SIZE - HOP_SIZE).
const HOP_SIZE: usize = 128;
pub const NUM_BANDS: usize = 128;

/// Cava's default band edges: 50 Hz – 10 kHz, log-spaced.
const LOWER_HZ: f32 = 50.0;
const UPPER_HZ: f32 = 10_000.0;
/// Higher than cava's 0.77 so the bars glide instead of stepping.
const NOISE_REDUCTION: f32 = 0.93;
/// Original peak-envelope decay, per hop. Quiet passages stay quiet.
const DECAY_FACTOR_PEAK: f32 = 0.9985;

/// Shared frequency-band state written by the audio sink, read by the renderer.
pub struct VisBands {
    pub values: [f32; NUM_BANDS],
    pub updated_at: Instant,
    pub peak_envelope: f32,
    pub is_active: bool,
}

impl VisBands {
    pub fn new() -> Self {
        Self {
            values: [0.0; NUM_BANDS],
            updated_at: Instant::now(),
            peak_envelope: 1e-6,
            is_active: false,
        }
    }

    pub fn shared() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::new()))
    }
}

impl Default for VisBands {
    fn default() -> Self {
        Self::new()
    }
}

/// A tee'd sink: forwards audio to `inner` and computes FFT bands on the side.
pub struct VisualizationSink {
    inner: Box<dyn Sink>,
    sample_buf: VecDeque<f32>,
    bands: Arc<Mutex<VisBands>>,
    fft: Arc<dyn rustfft::Fft<f32>>,
    hann_window: Vec<f32>,
    fft_buf: Vec<Complex<f32>>,
    magnitudes: Vec<f32>,
    sample_rate: f32,
    band_ranges: Vec<(usize, usize)>,
    new_bands: [f32; NUM_BANDS],
}

impl VisualizationSink {
    pub fn new(inner: Box<dyn Sink>, bands: Arc<Mutex<VisBands>>, sample_rate: f32) -> Self {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let hann_window: Vec<f32> = (0..FFT_SIZE)
            .map(|i| {
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE - 1) as f32).cos())
            })
            .collect();
        let band_ranges = precompute_band_ranges(FFT_SIZE / 2, NUM_BANDS, sample_rate);
        Self {
            inner,
            sample_buf: VecDeque::with_capacity(FFT_SIZE * 2),
            bands,
            fft,
            hann_window,
            fft_buf: vec![Complex::new(0.0, 0.0); FFT_SIZE],
            magnitudes: vec![0.0; FFT_SIZE / 2],
            sample_rate,
            band_ranges,
            new_bands: [0.0; NUM_BANDS],
        }
    }
}

impl Sink for VisualizationSink {
    fn start(&mut self) -> SinkResult<()> {
        self.inner.start()
    }

    fn stop(&mut self) -> SinkResult<()> {
        if let Ok(mut g) = self.bands.lock() {
            g.values.fill(0.0);
            g.peak_envelope = 1e-6;
            g.updated_at = Instant::now();
            g.is_active = false;
        }
        self.sample_buf.clear();
        self.inner.stop()
    }

    fn write(&mut self, packet: AudioPacket, converter: &mut Converter) -> SinkResult<()> {
        if let AudioPacket::Samples(ref samples) = packet {
            // Interleaved stereo -> mono.
            self.sample_buf.extend(samples.chunks(2).map(|c| {
                if c.len() == 2 {
                    f64::midpoint(c[0], c[1]) as f32
                } else {
                    c[0] as f32
                }
            }));

            while self.sample_buf.len() >= FFT_SIZE {
                {
                    let (front, back) = self.sample_buf.as_slices();
                    if front.len() >= FFT_SIZE {
                        for (dst, (&s, &w)) in self
                            .fft_buf
                            .iter_mut()
                            .zip(front.iter().zip(self.hann_window.iter()))
                        {
                            *dst = Complex::new(s * w, 0.0);
                        }
                    } else {
                        let split = front.len();
                        for (dst, (&s, &w)) in self.fft_buf[..split]
                            .iter_mut()
                            .zip(front.iter().zip(self.hann_window[..split].iter()))
                        {
                            *dst = Complex::new(s * w, 0.0);
                        }
                        let remaining = FFT_SIZE - split;
                        for (dst, (&s, &w)) in self.fft_buf[split..].iter_mut().zip(
                            back[..remaining]
                                .iter()
                                .zip(self.hann_window[split..].iter()),
                        ) {
                            *dst = Complex::new(s * w, 0.0);
                        }
                    }
                }

                self.fft.process(&mut self.fft_buf);

                for (mag, c) in self.magnitudes.iter_mut().zip(self.fft_buf.iter()) {
                    *mag = c.norm();
                }

                fill_log_bands(&self.magnitudes, &self.band_ranges, &mut self.new_bands);
                smooth_bands(&mut self.new_bands);

                if let Ok(mut g) = self.bands.lock() {
                    let dt = (HOP_SIZE as f32 / self.sample_rate).clamp(0.001, 0.05);
                    let frame_peak = self.new_bands.iter().copied().fold(0.0_f32, f32::max);
                    // Slow release so a quiet bar can grow, without a loud hit
                    // pinning the scale so hard the rest disappears.
                    // Same envelope as the pre-cava visualizer: raw magnitudes,
                    // peak held with a slow decay. The sqrt scale is applied at draw.
                    let peak_decay = DECAY_FACTOR_PEAK.powf(self.sample_rate * dt / HOP_SIZE as f32);
                    g.peak_envelope = (g.peak_envelope * peak_decay).max(frame_peak).max(1e-6);
                    let tau = 0.12 + NOISE_REDUCTION * 0.32;
                    let integral = 1.0 - (-dt / tau).exp();
                    let gravity = (1.0 - NOISE_REDUCTION) * 6.0 * dt * g.peak_envelope;
                    for (stored, fresh) in g.values.iter_mut().zip(self.new_bands.iter()) {
                        if *stored < *fresh {
                            *stored += (*fresh - *stored) * integral;
                        } else {
                            *stored = (*stored - gravity).max(*fresh);
                        }
                    }
                    g.updated_at = Instant::now();
                }

                self.sample_buf.drain(..HOP_SIZE);
            }
        }

        self.inner.write(packet, converter)
    }
}

fn precompute_band_ranges(num_bins: usize, num_bands: usize, sample_rate: f32) -> Vec<(usize, usize)> {
    let nyquist = (sample_rate / 2.0).max(1.0);
    let hz_per_bin = nyquist / num_bins as f32;
    let f_lo = LOWER_HZ;
    let f_hi = UPPER_HZ.min(nyquist * 0.98).max(f_lo + 1.0);
    let mut used_up_to = ((f_lo / hz_per_bin) as usize).clamp(1, num_bins - 1);
    let mut ranges = Vec::with_capacity(num_bands);
    for band in 0..num_bands {
        if used_up_to >= num_bins {
            ranges.push((num_bins - 1, num_bins));
            continue;
        }
        let t_start = band as f32 / num_bands as f32;
        let t_end = (band + 1) as f32 / num_bands as f32;
        let natural_start = (f_lo * (f_hi / f_lo).powf(t_start) / hz_per_bin) as usize;
        let natural_end = (f_lo * (f_hi / f_lo).powf(t_end) / hz_per_bin) as usize;
        let start = natural_start.max(used_up_to).min(num_bins - 1);
        let end = natural_end.max(start + 1).min(num_bins);
        used_up_to = end;
        ranges.push((start, end));
    }
    ranges
}

fn fill_log_bands(magnitudes: &[f32], band_ranges: &[(usize, usize)], out: &mut [f32]) {
    for (band_val, &(start, end)) in out.iter_mut().zip(band_ranges.iter()) {
        let len = (end - start) as f32;
        let sum_sq: f32 = magnitudes[start..end].iter().map(|&v| v * v).sum();
        *band_val = (sum_sq / len).sqrt();
    }
}


/// Two neighbor passes so a single bin cannot spike one bar.
fn smooth_bands(bands: &mut [f32; NUM_BANDS]) {
    let mut scratch = [0.0f32; NUM_BANDS];
    for _ in 0..2 {
        scratch.copy_from_slice(bands);
        for i in 0..NUM_BANDS {
            let prev = scratch[i.saturating_sub(1)];
            let next = scratch[(i + 1).min(NUM_BANDS - 1)];
            bands[i] = prev * 0.25 + scratch[i] * 0.5 + next * 0.25;
        }
    }
}
