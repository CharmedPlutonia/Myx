//! Embedded cava, fed only Myx's playback.
//!
//! A private FIFO carries signed 16-bit stereo from the audio sink. Cava never
//! opens the system monitor. Knobs come from `[cava]` in the Myx config; the
//! input method is fixed so that cannot be overridden back to system-wide.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use myx::audio::{install_cava_fifo, CavaFifo};
use myx::config::CavaConfig;

pub(crate) struct Cava {
    child: Child,
    bars: Arc<Mutex<Vec<f32>>>,
}

impl Cava {
    pub(crate) fn spawn(cfg: &CavaConfig) -> Option<Self> {
        let dir = std::env::temp_dir().join(format!("myx-cava-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let fifo_path = dir.join("audio.fifo");
        let conf_path = dir.join("cava.conf");
        let _ = std::fs::remove_file(&fifo_path);
        if nix_mkfifo(&fifo_path).is_err() {
            return None;
        }
        let fifo = CavaFifo::open(&fifo_path).ok()?;
        install_cava_fifo(fifo);

        let channels = if cfg.channels.eq_ignore_ascii_case("mono") {
            "mono"
        } else {
            "stereo"
        };
        let autosens = if cfg.autosens { 1 } else { 0 };
        let mut config = format!(
            "\
[general]
framerate = {framerate}
sensitivity = {sensitivity}
autosens = {autosens}
bars = {bars}
lower_cutoff_freq = {lower}
higher_cutoff_freq = {higher}
channels = {channels}

[input]
method = fifo
source = {source}
sample_rate = 44100
sample_bits = 16

[output]
method = raw
raw_target = /dev/stdout
data_format = ascii
ascii_max_range = 1000
bar_delimiter = 59
frame_delimiter = 10

[smoothing]
noise_reduction = {noise}
",
            framerate = cfg.framerate.max(1),
            sensitivity = cfg.sensitivity.max(0),
            bars = cfg.bars.clamp(2, 200),
            lower = cfg.lower_cutoff.max(1),
            higher = cfg.higher_cutoff.max(cfg.lower_cutoff + 1),
            source = fifo_path.display(),
            noise = cfg.noise_reduction.clamp(0, 100),
        );
        if let Some(extra) = cfg.extra.as_deref().filter(|s| !s.trim().is_empty()) {
            config.push('\n');
            config.push_str(extra);
            config.push('\n');
        }
        if std::fs::write(&conf_path, config).is_err() {
            return None;
        }
        let mut child = Command::new("cava")
            .arg("-p")
            .arg(&conf_path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let stdout = child.stdout.take()?;
        let bars = Arc::new(Mutex::new(Vec::new()));
        let shared = Arc::clone(&bars);
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                let frame: Vec<f32> = line
                    .split(';')
                    .filter_map(|s| s.trim().parse::<f32>().ok())
                    .map(|v| (v / 1000.0).clamp(0.0, 1.0))
                    .collect();
                if frame.is_empty() {
                    continue;
                }
                if let Ok(mut g) = shared.lock() {
                    *g = frame;
                }
            }
        });
        Some(Self { child, bars })
    }

    pub(crate) fn bars(&self) -> Option<Vec<f32>> {
        self.bars.lock().ok().and_then(|g| {
            if g.is_empty() {
                None
            } else {
                Some(g.clone())
            }
        })
    }
}

fn nix_mkfifo(path: &std::path::Path) -> std::io::Result<()> {
    let status = Command::new("mkfifo").arg(path).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other("mkfifo failed"))
    }
}

impl Drop for Cava {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
