//! Spectrum bars under the album art.
//!
//! Cava's default `channels = stereo`: both sides are mirrored, low frequencies
//! in the center and highs toward the edges (`example_files/config`). Bar width
//! stays at cava's 2. Spacing is 0 (cava's default is 1; this fork drops the gap).
//! The drawing rect is the original compact size.

use crate::*;

/// Cava `bar_width`. Spacing is intentionally 0.
const BAR_W: usize = 2;

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
    let values: [f32; NUM_BANDS] = guard.values;
    drop(guard);

    // Original footprint: a centered band, not the full pane.
    let vh = ((area.height as u32 * 3 / 5) as u16)
        .clamp(6, 14)
        .min(area.height);
    let vw = ((area.width as u32 * 9 / 10) as u16)
        .clamp(24, 80)
        .min(area.width);
    let n = (vw as usize / BAR_W).max(2);
    let used = n * BAR_W;
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

    // Stereo mirror, sampled between bands so a bar does not jump to the next
    // bin. Lows stay in the center.
    let half = n / 2;
    let mut cols = vec![0.0f32; n];
    for i in 0..n {
        let from_center = if i < half { half - 1 - i } else { i - half };
        let side = if i < half { half } else { n - half };
        let t = if side <= 1 {
            0.0
        } else {
            from_center as f32 / (side - 1) as f32 * (NUM_BANDS - 1) as f32
        };
        cols[i] = sample_band(&values, t);
    }
    for _ in 0..3 {
        let src = cols.clone();
        for i in 0..n {
            let l = src[i.saturating_sub(1)];
            let r = src[(i + 1).min(n - 1)];
            cols[i] = l * 0.25 + src[i] * 0.5 + r * 0.25;
        }
    }

    const LEVELS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let color: ratatui::style::Color = theme.text.into();
    let style = Style::default().fg(color);

    let mut lines: Vec<Line> = Vec::with_capacity(h);
    for row in 0..h {
        let from_bottom = (h - 1 - row) as f32;
        let mut spans: Vec<Span> = Vec::with_capacity(n);
        for &v in &cols {
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
        }
        lines.push(Line::from(spans));
    }
    f.render_widget(Paragraph::new(lines), vrect);
}

fn sample_band(values: &[f32; NUM_BANDS], t: f32) -> f32 {
    let x = t.clamp(0.0, (NUM_BANDS - 1) as f32);
    let i = x.floor() as usize;
    let frac = x - i as f32;
    let a = values[i];
    let b = values[(i + 1).min(NUM_BANDS - 1)];
    (a + (b - a) * frac).clamp(0.0, 1.0)
}
