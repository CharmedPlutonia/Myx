//! Spectrum bars under the album art.
//!
//! Drawn to cava's documented defaults: auto bar count, width 2, spacing 1,
//! bottom-aligned, linear scale, 50 Hz–10 kHz (applied in the sink), noise
//! reduction 0.77 with monstercat/waves off, and a single foreground color
//! (`gradient = 0`, `foreground = default`).

use crate::*;

/// Cava `bar_width` / `bar_spacing`.
const BAR_W: usize = 2;
const BAR_GAP: usize = 1;

pub(crate) fn render_visualizer(f: &mut Frame, app: &App, theme: Theme, area: Rect) {
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
    // Already linear 0–1 after the sink's autosens + gravity filter.
    let values: [f32; NUM_BANDS] = guard.values;
    drop(guard);

    let h = area.height as usize;
    let unit = BAR_W + BAR_GAP;
    let n = area.width as usize / unit;
    if n == 0 || h == 0 {
        return;
    }
    // `bars = 0` fills the width; `center_align = 1` centers a shortfall.
    let used = n * unit - BAR_GAP;
    let x0 = area.x + ((area.width as usize - used) / 2) as u16;

    let mut cols = vec![0.0f32; n];
    for (i, c) in cols.iter_mut().enumerate() {
        let lo = i * NUM_BANDS / n;
        let hi = ((i + 1) * NUM_BANDS / n).max(lo + 1).min(NUM_BANDS);
        let sum: f32 = values[lo..hi].iter().copied().sum();
        *c = (sum / (hi - lo) as f32).clamp(0.0, 1.0);
    }

    const LEVELS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    // `foreground = default`: the UI's text role, not a cover-derived gradient.
    let color: ratatui::style::Color = theme.text.into();
    let style = Style::default().fg(color);

    let mut lines: Vec<Line> = Vec::with_capacity(h);
    for row in 0..h {
        let from_bottom = (h - 1 - row) as f32;
        let mut spans: Vec<Span> = Vec::with_capacity(used);
        for (i, &v) in cols.iter().enumerate() {
            let filled = v * h as f32 - from_bottom;
            let ch = if filled >= 1.0 {
                '█'
            } else if filled <= 0.0 {
                ' '
            } else {
                LEVELS[((filled * 8.0) as usize).clamp(1, 8) - 1]
            };
            let cell = if ch == ' ' {
                Span::raw(" ".repeat(BAR_W))
            } else {
                Span::styled(ch.to_string().repeat(BAR_W), style)
            };
            spans.push(cell);
            if i + 1 < n {
                spans.push(Span::raw(" ".repeat(BAR_GAP)));
            }
        }
        lines.push(Line::from(spans));
    }
    let vrect = Rect {
        x: x0,
        y: area.y,
        width: used as u16,
        height: area.height,
    };
    f.render_widget(Paragraph::new(lines), vrect);
}
