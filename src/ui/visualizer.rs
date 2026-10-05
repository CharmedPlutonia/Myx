//! Spectrum bars under the album art.
//!
//! Prefer a real cava process (default settings, raw output). If `cava` is not
//! installed, fall back to the built-in FFT. Either way the strip keeps the
//! compact footprint and a one-cell gap between 2-wide bars.

use crate::*;

const BAR_W: usize = 2;
const BAR_GAP: usize = 1;

pub(crate) fn render_visualizer(f: &mut Frame, app: &App, theme: Theme, area: Rect) {
    if let Some(bars) = app.cava.as_ref().and_then(|c| c.bars()) {
        // Cava's stereo default is already low-in-the-center. Draw it as-is.
        draw_bars(f, theme, area, &bars, false);
        return;
    }
    let active = app
        .svc
        .engine
        .bands
        .try_lock()
        .map(|g| g.is_active)
        .unwrap_or(false);
    if !active {
        return;
    }
    let Ok(guard) = app.svc.engine.bands.try_lock() else {
        return;
    };
    let values = guard.values;
    drop(guard);
    draw_bars(f, theme, area, &values, true);
}

fn draw_bars(f: &mut Frame, theme: Theme, area: Rect, values: &[f32], mirror: bool) {
    if values.is_empty() {
        return;
    }
    // The layout already reserved [cava] height rows. Fill that strip.
    // [cava] bars is the drawn count, not just cava's internal resolution.
    let vh = area.height;
    let unit = BAR_W + BAR_GAP;
    let want = myx::config::get().cava.bars.clamp(2, 200) as usize;
    let fit = (area.width as usize / unit).max(1);
    let n = want.min(fit);
    let used = n * unit - BAR_GAP;
    let vw = (used as u16).min(area.width);
    let vrect = Rect {
        x: area.x + area.width.saturating_sub(used as u16) / 2,
        y: area.y + area.height.saturating_sub(vh) / 2,
        width: used as u16,
        height: vh,
    };
    let h = vrect.height as usize;
    if h == 0 {
        return;
    }

    let mut cols = vec![0.0f32; n];
    if mirror {
        let half = n / 2;
        for i in 0..n {
            let from_center = if i < half { half - 1 - i } else { i - half };
            let side = if i < half { half } else { n - half };
            let t = if side <= 1 {
                0.0
            } else {
                from_center as f32 / (side - 1) as f32 * (values.len() - 1) as f32
            };
            cols[i] = sample(values, t);
        }
    } else {
        for (i, c) in cols.iter_mut().enumerate() {
            let t = if n <= 1 {
                0.0
            } else {
                i as f32 / (n - 1) as f32 * (values.len() - 1) as f32
            };
            *c = sample(values, t);
        }
    }

    const LEVELS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let color: ratatui::style::Color = theme.text.into();
    let style = Style::default().fg(color);

    let mut lines: Vec<Line> = Vec::with_capacity(h);
    for row in 0..h {
        let from_bottom = (h - 1 - row) as f32;
        let mut spans: Vec<Span> = Vec::with_capacity(n * 2);
        for (i, &v) in cols.iter().enumerate() {
            let filled = v * h as f32 - from_bottom;
            let ch = if filled >= 1.0 {
                '█'
            } else if filled <= 0.0 {
                ' '
            } else {
                LEVELS[((filled * 8.0) as usize).clamp(1, 8) - 1]
            };
            if ch == ' ' {
                spans.push(Span::raw(" ".repeat(BAR_W)));
            } else {
                spans.push(Span::styled(ch.to_string().repeat(BAR_W), style));
            }
            if i + 1 < n {
                spans.push(Span::raw(" ".repeat(BAR_GAP)));
            }
        }
        lines.push(Line::from(spans));
    }
    f.render_widget(Paragraph::new(lines), vrect);
}

fn sample(values: &[f32], t: f32) -> f32 {
    if values.len() == 1 {
        return values[0].clamp(0.0, 1.0);
    }
    let x = t.clamp(0.0, (values.len() - 1) as f32);
    let i = x.floor() as usize;
    let frac = x - i as f32;
    let a = values[i];
    let b = values[(i + 1).min(values.len() - 1)];
    (a + (b - a) * frac).clamp(0.0, 1.0)
}
