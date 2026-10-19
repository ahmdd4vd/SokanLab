// SokanLab ui — painter equity curve + drawdown (mini-chart hasil backtest).
use egui::{Color32, CornerRadius, FontId, Painter, Pos2, Rect, Stroke};
use sokanlab_backtest::BacktestResult;

use crate::theme::{bear_color, EMBER, GRID};

/// Area gambar equity; padding supaya label tidak terpotong.
pub fn plot_area(available: Rect) -> Rect {
    let pad = 12.0;
    Rect::from_min_max(
        Pos2::new(available.min.x + pad, available.min.y + pad),
        Pos2::new(available.max.x - pad, available.max.y - pad),
    )
}

/// Gambar kurva equity (ticks) + garis dasar modal awal.
pub fn draw_equity(painter: &Painter, area: Rect, r: &BacktestResult) {
    if r.equity.len() < 2 {
        painter.text(
            Pos2::new(area.center().x, area.center().y),
            egui::Align2::CENTER_CENTER,
            "kurva equity butuh >= 2 bar",
            FontId::monospace(12.0),
            GRID,
        );
        return;
    }
    let min = r
        .equity
        .iter()
        .map(|p| p.equity_ticks)
        .min()
        .unwrap_or(0)
        .min(r.initial_cash_ticks);
    let max = r
        .equity
        .iter()
        .map(|p| p.equity_ticks)
        .max()
        .unwrap_or(1)
        .max(r.initial_cash_ticks);
    let span = (max - min).max(1) as f64;
    let n = r.equity.len() as f64;
    let (ax, ay, aw, ah) = (
        f64::from(area.min.x),
        f64::from(area.min.y),
        f64::from(area.width()),
        f64::from(area.height()),
    );
    let to_pos = |i: usize, v: i64| -> Pos2 {
        Pos2::new(
            (ax + (i as f64 / (n - 1.0)) * aw) as f32,
            (ay + ah - ((v - min) as f64 / span) * ah) as f32,
        )
    };

    // Grid + garis modal awal.
    for k in 0..=4 {
        let y = (f64::from(area.min.y) + f64::from(area.height()) * (k as f64 / 4.0)) as f32;
        painter.line_segment(
            [Pos2::new(area.min.x, y), Pos2::new(area.max.x, y)],
            Stroke::new(1.0, GRID),
        );
    }
    let base_y = to_pos(0, r.initial_cash_ticks).y;
    painter.line_segment(
        [Pos2::new(area.min.x, base_y), Pos2::new(area.max.x, base_y)],
        Stroke::new(1.0, Color32::from_rgb(0x8a, 0x93, 0x9c)),
    );

    // Kurva.
    let pts: Vec<Pos2> = r
        .equity
        .iter()
        .enumerate()
        .map(|(i, p)| to_pos(i, p.equity_ticks))
        .collect();
    let up = r.final_equity_ticks >= r.initial_cash_ticks;
    let color = if up { EMBER } else { bear_color() };
    for w in pts.windows(2) {
        painter.line_segment([w[0], w[1]], Stroke::new(1.6, color));
    }

    painter.text(
        Pos2::new(area.min.x, area.min.y - 4.0),
        egui::Align2::LEFT_BOTTOM,
        format!(
            "equity {} → {} ({} bps)",
            r.initial_cash_ticks, r.final_equity_ticks, r.metrics.total_return_bps
        ),
        FontId::monospace(11.0),
        color,
    );
}

/// Bar drawdown (merah-ish) di bawah kurva.
pub fn draw_drawdown_bars(painter: &Painter, area: Rect, r: &BacktestResult) {
    if r.equity.len() < 2 {
        return;
    }
    let mut peak = i64::MIN;
    let mut max_dd = 1i64;
    let dds: Vec<i64> = r
        .equity
        .iter()
        .map(|p| {
            peak = peak.max(p.equity_ticks);
            if peak > 0 {
                ((peak - p.equity_ticks) * 10_000 / peak).max(0)
            } else {
                0
            }
        })
        .collect();
    for d in &dds {
        max_dd = max_dd.max(*d);
    }
    let base = Rect::from_min_max(
        Pos2::new(area.min.x, area.max.y),
        Pos2::new(
            area.max.x,
            area.max.y + (area.height() as f64 * 0.18) as f32,
        ),
    );
    let n = r.equity.len() as f64;
    let w = (f64::from(base.width()) / n).max(1.0);
    let (bminx, bmaxy, bh) = (
        f64::from(base.min.x),
        f64::from(base.max.y),
        f64::from(base.height()),
    );
    for (i, d) in dds.iter().enumerate() {
        let h = (*d as f64 / max_dd as f64) * bh;
        let x = bminx + i as f64 * w;
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(x as f32, bmaxy as f32),
                Pos2::new((x + w) as f32, (bmaxy - h) as f32),
            ),
            CornerRadius::ZERO,
            Color32::from_rgba_unmultiplied(0x8a, 0x93, 0x9c, 0x70),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sokanlab_backtest::{run, Config, CostModel};
    use sokanlab_domain::Bar;
    use sokanlab_strategy_api::BuyAndHold;

    fn result(n: usize) -> BacktestResult {
        let bars: Vec<Bar> = (0..n)
            .map(|i| Bar {
                ts_millis: i as i64 * 60_000,
                open_ticks: 100,
                high_ticks: 105,
                low_ticks: 95,
                close_ticks: 100 + (i as i64 % 7),
                volume_milli: 10,
            })
            .collect();
        let mut s = BuyAndHold::new();
        run(&bars, &mut s, Config::default(), CostModel::default(), None)
    }

    #[test]
    fn paints_headless_without_panic() {
        let ctx = egui::Context::default();
        let r = result(40);
        let mut full = ctx.run_pass(egui::RawInput::default(), |ctx| {
            ctx.root_ui(|ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let area = ui.available_rect_before_wrap();
                    let p = plot_area(area);
                    let painter = ui.painter_at(area);
                    draw_equity(&painter, p, &r);
                    draw_drawdown_bars(&painter, p, &r);
                });
            });
        });
        full.textures_delta.clear();
    }

    #[test]
    fn short_result_does_not_panic() {
        let ctx = egui::Context::default();
        let r = result(1);
        let mut full = ctx.run_pass(egui::RawInput::default(), |ctx| {
            ctx.root_ui(|ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let area = ui.available_rect_before_wrap();
                    let painter = ui.painter_at(area);
                    draw_equity(&painter, plot_area(area), &r);
                });
            });
        });
        full.textures_delta.clear();
    }
}
