//! Embedded default cava.
//!
//! Cava owns the spectrum: sensitivity 100, autosens, stereo (lows in the
//! center), 50 Hz–10 kHz, noise reduction 77. The only overrides are the ones
//! required to draw it inside the TUI — raw ASCII on stdout, and a fixed bar
//! count the strip can sample. Input is cava's own default capture.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

pub(crate) struct Cava {
    child: Child,
    bars: Arc<Mutex<Vec<f32>>>,
}

impl Cava {
    pub(crate) fn spawn() -> Option<Self> {
        let path = std::env::temp_dir().join(format!("myx-cava-{}.conf", std::process::id()));
        let config = "\
[general]
# defaults: framerate 60, sensitivity 100, autosens 1, 50–10000 Hz
bars = 64

[output]
method = raw
raw_target = /dev/stdout
data_format = ascii
ascii_max_range = 1000
bar_delimiter = 59
frame_delimiter = 10

[smoothing]
noise_reduction = 77
";
        if std::fs::write(&path, config).is_err() {
            return None;
        }
        let mut child = Command::new("cava")
            .arg("-p")
            .arg(&path)
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

impl Drop for Cava {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
