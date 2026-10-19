// SokanLab ui — Apache-2.0. Tema orisinal SokanLab (bukan turunan produk lain).
use egui::{Color32, Stroke, Visuals};

pub const INK: Color32 = Color32::from_rgb(0x14, 0x18, 0x1d);
pub const PAPER: Color32 = Color32::from_rgb(0xf6, 0xf3, 0xec);
pub const EMBER: Color32 = Color32::from_rgb(0xe8, 0xa3, 0x3d);
pub const SLATE: Color32 = Color32::from_rgb(0x5b, 0x65, 0x70);
pub const GRID: Color32 = Color32::from_rgb(0x2a, 0x30, 0x38);

/// Warna bullish/bearish orisinal: Ember (naik) dan Slate-abu (turun).
pub fn bull_color() -> Color32 {
    EMBER
}

pub fn bear_color() -> Color32 {
    Color32::from_rgb(0x8a, 0x93, 0x9c)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

pub fn apply(ctx: &egui::Context, theme: Theme) {
    let mut v = match theme {
        Theme::Dark => Visuals::dark(),
        Theme::Light => Visuals::light(),
    };
    v.panel_fill = match theme {
        Theme::Dark => INK,
        Theme::Light => PAPER,
    };
    v.window_fill = v.panel_fill;
    v.extreme_bg_color = v.panel_fill;
    v.faint_bg_color = match theme {
        Theme::Dark => Color32::from_rgb(0x1c, 0x21, 0x27),
        Theme::Light => Color32::from_rgb(0xee, 0xea, 0xe1),
    };
    v.widgets.noninteractive.bg_stroke = Stroke::new(
        1.0,
        match theme {
            Theme::Dark => GRID,
            Theme::Light => Color32::from_rgb(0xd8, 0xd2, 0xc6),
        },
    );
    v.widgets.active.bg_fill = EMBER;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, EMBER);
    ctx.set_visuals(v);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn themes_differ() {
        assert_ne!(bull_color(), bear_color());
    }

    #[test]
    fn apply_does_not_panic() {
        let ctx = egui::Context::default();
        apply(&ctx, Theme::Dark);
        apply(&ctx, Theme::Light);
    }
}
