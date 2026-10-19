// SokanLab ui — painter candlestick/volume/crosshair via egui (jalur CPU).
// Renderer khusus: hanya bar yang terlihat digambar, tanpa plot generik.
// Matematika memakai f64 (chart crate); egui memakai f32 → konversi di batas.
use egui::{Align2, Color32, CornerRadius, FontId, Painter, Rect, Stroke, StrokeKind};
use sokanlab_chart::drawing::{
    bars_between, channel_segments, fib_ext_price, fib_level_price, fib_time_ts, gann_fan_lines,
    pitchfork_lines, position_levels, regression_fit, regression_segments, Anchor, DashStyle,
    Drawing, DrawingItem, Projector, StickerIcon, TextAlign, TextSize, EXTEND_PX, FIB_EXT_MILLI,
    FIB_LEVELS_MILLI,
};
use sokanlab_chart::{
    candle_geom, visible_indices, volume_geom, volume_scale, CandleGeom, Viewport,
};
use sokanlab_domain::Bar;

use crate::theme::{bear_color, bull_color, EMBER, GRID, SLATE};

fn f(v: f32) -> f64 {
    v as f64
}
fn pos(x: f64, y: f64) -> egui::Pos2 {
    egui::pos2(x as f32, y as f32)
}
fn v2(x: f64, y: f64) -> egui::Vec2 {
    egui::vec2(x as f32, y as f32)
}

/// Warna hover object (bukan terpilih) — lebih terang dari SLATE.
const HOVER: Color32 = Color32::from_rgb(0x9a, 0xa5, 0xb2);
/// Warna garis level fib selain 0/50/100%.
const FIB_LINE: Color32 = Color32::from_rgb(0x77, 0x82, 0x8d);
/// Latar/teks kotak label sumbu (tag harga & waktu).
const TAG_BG: Color32 = Color32::from_rgb(0x2b, 0x31, 0x3d);
const TAG_FG: Color32 = Color32::from_rgb(0xe8, 0xe4, 0xdc);

/// Palet warna objek (orisinal SokanLab) untuk panel properti.
pub const ITEM_PALETTE: [([u8; 3], &str); 8] = [
    ([0xe8, 0xa3, 0x3d], "Ember"),
    ([0x5b, 0x65, 0x70], "Slate"),
    ([0x7f, 0xb2, 0xd9], "Sky"),
    ([0x7f, 0xae, 0x6a], "Fern"),
    ([0xd9, 0x7f, 0x7f], "Rose"),
    ([0xa9, 0x8f, 0xd9], "Violet"),
    ([0xd9, 0xc9, 0xa3], "Sand"),
    ([0xe8, 0xe4, 0xdc], "Bone"),
];

/// Lebar strip sumbu harga di kanan plot (label harga & tag crosshair).
pub const AXIS_W: f32 = 64.0;
/// Tinggi strip sumbu waktu di bawah plot.
pub const TIME_H: f32 = 22.0;

pub struct PlotLayout {
    /// Area plotting harga (tanpa sumbu).
    pub price: Rect,
    /// Area pane volume di bawah pane harga.
    pub volume: Rect,
    /// Pane indikator (RSI/MACD) di bawah pane volume, seurutan `ChartState::panes`.
    pub panes: Vec<Rect>,
    /// Strip sumbu harga (kanan plot, setinggi area plot).
    pub axis_y: Rect,
    /// Strip sumbu waktu (bawah kiri plot, selebar area plot).
    pub axis_x: Rect,
    pub spacing_px: f64,
}

/// Lebar area plot di dalam `area` (di luar strip sumbu kanan).
pub fn plot_width(area: Rect) -> f32 {
    let axis_w = AXIS_W.min((area.width() - 80.0).max(0.0));
    (area.width() - axis_w).max(32.0)
}

/// Tinggi pane harga tanpa pane indikator. Dipakai viewport YM mapping dan
/// anchor zoom agar konsisten dengan `split_plot`.
pub fn price_pane_height(area: Rect) -> f32 {
    split_plot_panes(area, 8.0, 0).price.height()
}

/// Bagi area: pane harga + volume + `pane_count` pane indikator (maks 3), plus
/// strip sumbu kanan (harga) dan bawah (waktu). Tanpa pane indikator: 78% harga,
/// 22% volume. Dengan pane: pane harga menyusut, tiap pane ~20% tinggi efektif.
pub fn split_plot_panes(area: Rect, spacing_px: f64, pane_count: usize) -> PlotLayout {
    let gap = 6.0f32;
    let plot_w = plot_width(area);
    let plot_h = (area.height() - TIME_H).max(48.0);
    let plot = Rect::from_min_size(area.min, v2(f(plot_w), f(plot_h)));
    let axis_y = Rect::from_min_max(
        egui::pos2(area.min.x + plot_w, area.min.y),
        egui::pos2(area.max.x, area.min.y + plot_h),
    );
    let axis_x = Rect::from_min_max(
        egui::pos2(area.min.x, area.min.y + plot_h),
        egui::pos2(area.min.x + plot_w, area.max.y),
    );
    let n = pane_count.min(3);
    let usable = (plot_h - gap * (1.0 + n as f32)).max(8.0);
    let (price_h, vol_h, pane_h) = if n == 0 {
        let price_h = (usable * 0.78).floor();
        (price_h, (plot_h - price_h - gap).max(8.0), 0.0)
    } else {
        let price_frac = (1.0 - 0.14 - 0.20 * n as f32).max(0.34);
        (
            (usable * price_frac).floor(),
            (usable * 0.14).floor().max(8.0),
            (usable * 0.20).floor().max(8.0),
        )
    };
    let price = Rect::from_min_size(plot.min, v2(f(plot_w), f(price_h)));
    let mut y = area.min.y + price_h + gap;
    let volume = Rect::from_min_size(pos(f(area.min.x), f(y)), v2(f(plot_w), f(vol_h)));
    y += vol_h + gap;
    let mut panes = Vec::with_capacity(n);
    for k in 0..n {
        let h = if k + 1 == n {
            (area.min.y + plot_h - y).max(8.0)
        } else {
            pane_h
        };
        panes.push(Rect::from_min_size(
            pos(f(area.min.x), f(y)),
            v2(f(plot_w), f(h)),
        ));
        y += h + gap;
    }
    PlotLayout {
        price,
        volume,
        panes,
        axis_y,
        axis_x,
        spacing_px,
    }
}

/// Bagi area tanpa pane indikator (jalur lama, tetap kompatibel).
pub fn split_plot(area: Rect, spacing_px: f64) -> PlotLayout {
    split_plot_panes(area, spacing_px, 0)
}

/// Pilih spacing agar bar terakhir pas di lebar area.
/// Boleh sub-pixel: saat bar > piksel, `draw_step` yang men-sampel bar.
pub fn fit_spacing(area_width: f64, visible: usize) -> f64 {
    if visible == 0 {
        return 8.0;
    }
    (area_width / visible as f64).clamp(1e-9, 40.0)
}

/// Jarak sampel bar: gambar tiap ke-`step` bar agar jaraknya ≥ ~1 px.
/// `step = 1` berarti semua bar digambar (spacing ≥ 1 px).
pub fn draw_step(spacing_px: f64) -> usize {
    if spacing_px <= 0.0 {
        return 1;
    }
    (1.0 / spacing_px).ceil().max(1.0) as usize
}

/// Grid harga + waktu (tipis, orisinal).
pub fn draw_grid(painter: &Painter, layout: &PlotLayout, rows: usize) {
    let stroke = Stroke::new(1.0, GRID);
    for i in 0..=rows {
        let frac = i as f64 / rows.max(1) as f64;
        let y = f(layout.price.max.y) - frac * f(layout.price.height());
        painter.line_segment(
            [pos(f(layout.price.min.x), y), pos(f(layout.price.max.x), y)],
            stroke,
        );
    }
    let cols = 6;
    for i in 0..=cols {
        let frac = i as f64 / cols as f64;
        let x = f(layout.price.min.x) + frac * f(layout.price.width());
        painter.line_segment(
            [pos(x, f(layout.price.min.y)), pos(x, f(layout.price.max.y))],
            stroke,
        );
    }
}

/// Gambar candle yang terlihat. Offset kiri plotting = price.min.x.
/// Saat zoom-out ekstrem (spacing sub-pixel) bar disampel tiap `draw_step`.
pub fn draw_candles(
    painter: &Painter,
    layout: &PlotLayout,
    v: &Viewport,
    bars: &[Bar],
    visible_start: usize,
) -> usize {
    let range = visible_indices(bars, v.t_min, v.t_max);
    let step = draw_step(layout.spacing_px);
    let ox = f(layout.price.min.x);
    let oy = f(layout.price.min.y);
    let mut drawn = 0usize;
    for i in range.step_by(step) {
        let g: CandleGeom = candle_geom(v, layout.spacing_px, visible_start, i, &bars[i]);
        let color = if bars[i].close_ticks >= bars[i].open_ticks {
            bull_color()
        } else {
            bear_color()
        };
        let x = ox + g.x_center;
        painter.line_segment(
            [pos(x, oy + g.wick_top), pos(x, oy + g.wick_bottom)],
            Stroke::new(1.0, color),
        );
        let half = (layout.spacing_px * 0.35).max(0.5);
        let top = oy + g.body_top;
        let bot = (oy + g.body_bottom).max(top + 1.0);
        painter.rect_filled(
            Rect::from_two_pos(pos(x - half, top), pos(x + half, bot)),
            CornerRadius::ZERO,
            color,
        );
        drawn += 1;
    }
    drawn
}

pub fn draw_volume(
    painter: &Painter,
    layout: &PlotLayout,
    bars: &[Bar],
    t_min: i64,
    t_max: i64,
    visible_start: usize,
) {
    let range = visible_indices(bars, t_min, t_max);
    if range.is_empty() {
        return;
    }
    let max = volume_scale(bars, range.clone());
    let step = draw_step(layout.spacing_px);
    let width = (layout.spacing_px * 0.7 * step as f64).max(1.0);
    let ox = f(layout.volume.min.x);
    let oy = f(layout.volume.min.y);
    let h = f(layout.volume.height());
    painter.text(
        pos(ox + 5.0, oy + 1.0),
        Align2::LEFT_TOP,
        "Vol",
        FontId::monospace(10.0),
        SLATE,
    );
    for i in range.step_by(step) {
        let x = ox + (i - visible_start) as f64 * layout.spacing_px;
        let color = if bars[i].close_ticks >= bars[i].open_ticks {
            bull_color()
        } else {
            bear_color()
        };
        let g = volume_geom(x, width, bars[i].volume_milli, max, h);
        painter.rect_filled(
            Rect::from_min_max(
                pos(ox + g.x_left, oy + g.y_top),
                pos(ox + g.x_right, oy + g.y_bottom),
            ),
            CornerRadius::ZERO,
            color.gamma_multiply(0.55),
        );
    }
}

/// Crosshair + label harga/waktu; memakai koordinat yang sama dengan viewport.
/// Garis indikator overlay (SMA/EMA/VWAP) di atas candle; hanya bar terlihat.
/// Segmen terputus saat nilai belum ada (warm-up) atau di luar jangkauan.
pub fn draw_overlays(
    painter: &Painter,
    layout: &PlotLayout,
    v: &Viewport,
    bars: &[Bar],
    visible_start: usize,
    overlays: &[crate::OverlayLine],
) {
    if overlays.is_empty() || bars.is_empty() || layout.spacing_px <= 0.0 {
        return;
    }
    let range = visible_indices(bars, v.t_min, v.t_max);
    if range.is_empty() {
        return;
    }
    let step = draw_step(layout.spacing_px);
    let ox = f(layout.price.min.x);
    let oy = f(layout.price.min.y);
    for line in overlays {
        let color = Color32::from_rgb(line.color[0], line.color[1], line.color[2]);
        let stroke = Stroke::new(1.4, color);
        let end = range.end.min(line.values.len());
        let mut pts: Vec<egui::Pos2> = Vec::new();
        let mut i = range.start;
        while i < end {
            match line.values[i] {
                Some(price) => {
                    let x = ox
                        + (i - visible_start) as f64 * layout.spacing_px
                        + layout.spacing_px / 2.0;
                    pts.push(pos(x, oy + v.y_for_price(price)));
                }
                None => {
                    if pts.len() > 1 {
                        painter.add(egui::Shape::line(std::mem::take(&mut pts), stroke));
                    } else {
                        pts.clear();
                    }
                }
            }
            i += step;
        }
        if pts.len() > 1 {
            painter.add(egui::Shape::line(pts, stroke));
        }
    }
}

pub fn draw_crosshair(
    painter: &Painter,
    layout: &PlotLayout,
    v: &Viewport,
    mouse: Option<egui::Pos2>,
) -> Option<(i64, i64)> {
    let p = mouse?;
    let in_plot = layout.price.contains(p)
        || layout.volume.contains(p)
        || layout.panes.iter().any(|r| r.contains(p));
    if !in_plot {
        return None;
    }
    let stroke = Stroke::new(1.0, SLATE);
    if layout.price.contains(p) {
        painter.line_segment(
            [
                pos(f(layout.price.min.x), f(p.y)),
                pos(f(layout.price.max.x), f(p.y)),
            ],
            stroke,
        );
    }
    painter.line_segment(
        [
            pos(f(p.x), f(layout.price.min.y)),
            pos(f(p.x), f(layout.axis_x.min.y)),
        ],
        stroke,
    );
    let price = v.price_at_y(f(p.y) - f(layout.price.min.y));
    let time = v.time_at_x(f(p.x) - f(layout.price.min.x));
    if layout.price.contains(p) {
        tag_v(painter, &layout.axis_y, p.y, &format!("{price}"));
    }
    tag_h(
        painter,
        &layout.axis_x,
        p.x,
        &fmt_time(time, v.t_max - v.t_min),
    );
    Some((price, time))
}

/// Kotak label pada strip sumbu kanan, terpusat di `y`.
fn tag_v(painter: &Painter, area: &Rect, y: f32, text: &str) {
    let y = y.clamp(area.min.y + 8.0, area.max.y - 8.0);
    let rect = Rect::from_min_max(
        egui::pos2(area.min.x, y - 8.0),
        egui::pos2(area.max.x, y + 8.0),
    );
    painter.rect_filled(rect, CornerRadius::ZERO, TAG_BG);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::monospace(10.0),
        TAG_FG,
    );
}

/// Kotak label pada strip sumbu bawah, terpusat di `x`.
fn tag_h(painter: &Painter, area: &Rect, x: f32, text: &str) {
    let galley = painter.layout_no_wrap(text.to_string(), FontId::monospace(10.0), TAG_FG);
    let w = (galley.size().x + 10.0).min(area.width());
    let cx = x.clamp(area.min.x + w / 2.0, area.max.x - w / 2.0);
    let rect = Rect::from_center_size(
        egui::pos2(cx, area.center().y),
        egui::vec2(w, (area.height() - 4.0).max(10.0)),
    );
    painter.rect_filled(rect, CornerRadius::ZERO, TAG_BG);
    painter.galley(rect.center() - galley.size() / 2.0, galley, TAG_FG);
}

/// Sumbu harga (kanan) & waktu (bawah): latar strip, garis batas, dan label.
/// Label harga sejajar baris grid pane harga, label waktu sejajar kolom grid.
pub fn draw_axes(painter: &Painter, layout: &PlotLayout, v: &Viewport) {
    let bg = bg_color();
    painter.rect_filled(layout.axis_y, CornerRadius::ZERO, bg);
    painter.rect_filled(layout.axis_x, CornerRadius::ZERO, bg);
    let border = Stroke::new(1.0, GRID);
    painter.line_segment(
        [
            pos(f(layout.axis_y.min.x), f(layout.axis_y.min.y)),
            pos(f(layout.axis_y.min.x), f(layout.axis_y.max.y)),
        ],
        border,
    );
    painter.line_segment(
        [
            pos(f(layout.axis_x.min.x), f(layout.axis_x.min.y)),
            pos(f(layout.axis_x.max.x), f(layout.axis_x.min.y)),
        ],
        border,
    );
    let font = FontId::monospace(10.0);
    for i in 0..=5 {
        let frac = i as f64 / 5.0;
        let y = f(layout.price.max.y) - frac * f(layout.price.height());
        let price = v.price_at_y(y - f(layout.price.min.y));
        painter.text(
            pos(f(layout.axis_y.min.x) + 5.0, y),
            Align2::LEFT_CENTER,
            format!("{price}"),
            font.clone(),
            SLATE,
        );
    }
    let span = v.t_max - v.t_min;
    for i in 0..=6 {
        let frac = i as f64 / 6.0;
        let x = f(layout.price.min.x) + frac * f(layout.price.width());
        let ts = v.time_at_x(x - f(layout.price.min.x));
        painter.text(
            pos(x, f(layout.axis_x.min.y) + 11.0),
            Align2::CENTER_CENTER,
            fmt_time(ts, span),
            font.clone(),
            SLATE,
        );
    }
}

/// Format waktu UTC ringkas untuk label sumbu: <= 3 jam → "HH:MM",
/// <= 120 hari → "DD/MM HH:MM", selain itu → "YYYY-MM-DD".
fn fmt_time(ts_millis: i64, span_ms: i64) -> String {
    let (y, mo, d, hh, mm) = civil_from_millis(ts_millis);
    if span_ms <= 3 * 3_600_000 {
        format!("{hh:02}:{mm:02}")
    } else if span_ms <= 120 * 86_400_000 {
        format!("{d:02}/{mo:02} {hh:02}:{mm:02}")
    } else {
        format!("{y:04}-{mo:02}-{d:02}")
    }
}

/// Epoch millis (UTC) → (tahun, bulan, hari, jam, menit). Algoritma sipil
/// Hinnant terbalik: aritmetika integer murni (deterministik).
fn civil_from_millis(ts_millis: i64) -> (i64, u32, u32, u32, u32) {
    let days = ts_millis.div_euclid(86_400_000);
    let ms = ts_millis.rem_euclid(86_400_000);
    let hh = (ms / 3_600_000) as u32;
    let mm = ((ms / 60_000) % 60) as u32;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d, hh, mm)
}

/// Label harga terakhir pada strip sumbu kanan (kotak Ember, teks gelap).
pub fn draw_last_price(painter: &Painter, layout: &PlotLayout, v: &Viewport, last: Option<i64>) {
    let Some(last) = last else { return };
    let y = (f(layout.price.min.y) + v.y_for_price(last)) as f32;
    let y = y.clamp(layout.axis_y.min.y + 8.0, layout.axis_y.max.y - 8.0);
    let rect = Rect::from_min_max(
        egui::pos2(layout.axis_y.min.x, y - 8.0),
        egui::pos2(layout.axis_y.max.x, y + 8.0),
    );
    painter.rect_filled(rect, CornerRadius::ZERO, EMBER);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        format!("{last}"),
        FontId::monospace(10.0),
        Color32::from_rgb(0x14, 0x18, 0x1d),
    );
}

/// Pane indikator (RSI/MACD): label, garis panduan, histogram, dan garis nilai.
/// Skala RSI tetap 0–100; MACD auto-simetris dari nilai bar terlihat.
pub fn draw_indicator_panes(
    painter: &Painter,
    layout: &PlotLayout,
    v: &Viewport,
    bars: &[Bar],
    visible_start: usize,
    panes: &[crate::PaneSeries],
) {
    if bars.is_empty() {
        return;
    }
    let range = visible_indices(bars, v.t_min, v.t_max);
    if range.is_empty() {
        return;
    }
    let step = draw_step(layout.spacing_px);
    let font = FontId::monospace(10.0);
    for (rect, series) in layout.panes.iter().zip(panes.iter()) {
        let ox = f(rect.min.x);
        let oy = f(rect.min.y);
        let h = f(rect.height());
        let w = f(rect.width());
        painter.text(
            pos(ox + 5.0, oy + 1.0),
            Align2::LEFT_TOP,
            &series.label,
            font.clone(),
            SLATE,
        );
        let max_abs = if series.fixed_rsi {
            100i64
        } else {
            let mut m = 1i64;
            for i in range.clone().step_by(step) {
                for (vals, _) in &series.lines {
                    if let Some(Some(val)) = vals.get(i) {
                        m = m.max(val.abs());
                    }
                }
                if let Some(Some(val)) = series.hist.get(i) {
                    m = m.max(val.abs());
                }
            }
            m
        };
        let scale_max = if series.fixed_rsi {
            100.0
        } else {
            (max_abs as f64) * 1.1
        };
        let y_of = |val: f64| {
            if series.fixed_rsi {
                // Skala tetap 0..=100: 0 di dasar pane.
                oy + h - (val / scale_max) * h
            } else {
                // Skala simetris (MACD): 0 di tengah pane, ±maks mengisi pane.
                oy + h / 2.0 - (val / scale_max) * (h / 2.0)
            }
        };
        if series.fixed_rsi {
            for level in [30.0f64, 50.0, 70.0] {
                let style = DrawStyle {
                    stroke: Stroke::new(1.0, GRID),
                    dash: DashStyle::Dotted,
                };
                line(
                    painter,
                    &style,
                    pos(ox, y_of(level)),
                    pos(ox + w, y_of(level)),
                );
            }
        } else {
            painter.line_segment(
                [pos(ox, y_of(0.0)), pos(ox + w, y_of(0.0))],
                Stroke::new(1.0, GRID),
            );
        }
        if !series.hist.is_empty() {
            let y0 = y_of(0.0);
            let bw = (layout.spacing_px * 0.7 * step as f64).max(1.0);
            for i in range.clone().step_by(step) {
                if let Some(Some(val)) = series.hist.get(i) {
                    let x = ox + (i - visible_start) as f64 * layout.spacing_px;
                    let y = y_of(*val as f64);
                    let y1 = y0.min(y);
                    let bh = (y0.max(y) - y1).max(1.0);
                    let color = if *val >= 0 {
                        bull_color()
                    } else {
                        bear_color()
                    };
                    painter.rect_filled(
                        Rect::from_min_size(pos(x, y1), v2(bw, bh)),
                        CornerRadius::ZERO,
                        color.gamma_multiply(0.5),
                    );
                }
            }
        }
        for (vals, rgb) in &series.lines {
            let stroke = Stroke::new(1.2, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
            let end = range.end.min(vals.len());
            let mut pts: Vec<egui::Pos2> = Vec::new();
            let mut i = range.start;
            while i < end {
                match vals[i] {
                    Some(val) => pts.push(pos(
                        ox + (i - visible_start) as f64 * layout.spacing_px
                            + layout.spacing_px / 2.0,
                        y_of(val as f64),
                    )),
                    None => {
                        if pts.len() > 1 {
                            painter.add(egui::Shape::line(std::mem::take(&mut pts), stroke));
                        } else {
                            pts.clear();
                        }
                    }
                }
                i += step;
            }
            if pts.len() > 1 {
                painter.add(egui::Shape::line(pts, stroke));
            }
        }
    }
}

/// Gaya gambar objek: stroke + pola garis (solid/putus-putus/titik).
#[derive(Clone, Copy)]
pub struct DrawStyle {
    pub stroke: Stroke,
    pub dash: DashStyle,
}

/// Panjang (dash, gap) per pola garis; `None` = solid.
fn dash_params(dash: DashStyle) -> Option<(f32, f32)> {
    match dash {
        DashStyle::Solid => None,
        DashStyle::Dashed => Some((6.0, 4.0)),
        DashStyle::Dotted => Some((1.5, 3.5)),
    }
}

impl DrawStyle {
    pub fn normal() -> Self {
        Self {
            stroke: Stroke::new(1.0, SLATE),
            dash: DashStyle::Solid,
        }
    }
    pub fn preview() -> Self {
        Self {
            stroke: Stroke::new(1.0, EMBER.gamma_multiply(0.85)),
            dash: DashStyle::Dashed,
        }
    }
}

fn line(painter: &Painter, style: &DrawStyle, a: egui::Pos2, b: egui::Pos2) {
    match dash_params(style.dash) {
        Some((dash, gap)) => {
            let shapes = egui::Shape::dashed_line(&[a, b], style.stroke, dash, gap);
            painter.extend(shapes);
        }
        None => {
            painter.line_segment([a, b], style.stroke);
        }
    }
}

fn polyline(painter: &Painter, style: &DrawStyle, pts: &[egui::Pos2], close: bool) {
    match dash_params(style.dash) {
        Some((dash, gap)) => {
            let shapes = egui::Shape::dashed_line(pts, style.stroke, dash, gap);
            painter.extend(shapes);
        }
        None if close => {
            painter.add(egui::Shape::closed_line(pts.to_vec(), style.stroke));
        }
        None => {
            painter.add(egui::Shape::line(pts.to_vec(), style.stroke));
        }
    }
}

fn arrow_head(painter: &Painter, style: &DrawStyle, a: egui::Pos2, b: egui::Pos2) {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1.0 {
        return;
    }
    let (ux, uy) = (dx / len, dy / len);
    let head = 9.0f32.min(len * 0.4);
    let half = 4.0f32;
    let base = egui::pos2(b.x - ux * head, b.y - uy * head);
    let p1 = egui::pos2(base.x - uy * half, base.y + ux * half);
    let p2 = egui::pos2(base.x + uy * half, base.y - ux * half);
    painter.add(egui::Shape::convex_polygon(
        vec![b, p1, p2],
        style.stroke.color,
        Stroke::NONE,
    ));
}

/// Kotak label kecil dengan latar gelap (untuk fib/measure/position).
fn label_box(
    painter: &Painter,
    at: egui::Pos2,
    align: Align2,
    lines: &[(String, Color32)],
    bg: Color32,
) {
    if lines.is_empty() {
        return;
    }
    let font = FontId::monospace(10.0);
    let mut galleys = Vec::new();
    let mut w = 0.0f32;
    let mut h = 0.0f32;
    for (text, color) in lines {
        let g = painter.layout_no_wrap(text.clone(), font.clone(), *color);
        w = w.max(g.size().x);
        h += g.size().y + 2.0;
        galleys.push(g);
    }
    let pad = 3.0;
    let size = egui::vec2(w + pad * 2.0, h + pad * 2.0);
    let rect = align.anchor_size(at, size);
    painter.rect_filled(rect, CornerRadius::same(2), bg);
    let mut y = rect.min.y + pad;
    for g in galleys {
        let x = rect.min.x + pad;
        let gs = g.size();
        painter.galley(egui::pos2(x, y), g, Color32::PLACEHOLDER);
        y += gs.y + 2.0;
    }
}

fn bg_color() -> Color32 {
    Color32::from_rgba_unmultiplied(0x14, 0x18, 0x1d, 230)
}

/// Gambar objek drawing memakai proyeksi viewport yang sama dengan candle
/// (agar anchor harga/waktu konsisten setelah zoom/resize).
#[allow(clippy::too_many_arguments)]
pub fn draw_drawings(
    painter: &Painter,
    layout: &PlotLayout,
    v: &Viewport,
    bars: &[Bar],
    items: &[DrawingItem],
    selected: &[usize],
    hover: Option<usize>,
    hidden: bool,
) {
    if hidden {
        return;
    }
    // Objek gambar di-clip ke pane harga (padanan TV: tidak menembus
    // volume/pane indikator walau anchor di luar rentang terlihat).
    let painter = &painter.with_clip_rect(layout.price);
    for (i, it) in items.iter().enumerate() {
        if !it.visible {
            continue; // sembunyi per objek (padanan eye toggle TV)
        }
        // Semua objek terpilih (multi-seleksi) ditandai ember.
        let focused = selected.contains(&i);
        let color = match it.style {
            Some(st) => Color32::from_rgb(st.color[0], st.color[1], st.color[2]),
            None if focused => EMBER,
            None if hover == Some(i) => HOVER,
            None => SLATE,
        };
        let width = match it.style {
            Some(st) => st.width.max(1) as f32,
            None => {
                if focused {
                    2.0
                } else {
                    1.0
                }
            }
        };
        // Opasitas per objek (0–100%) mengalikan warna efektif objek.
        let color = color.gamma_multiply(it.opacity_factor());
        let style = DrawStyle {
            stroke: Stroke::new(width, color),
            dash: it.style.map(|st| st.dash).unwrap_or(DashStyle::Solid),
        };
        draw_one(painter, layout, v, bars, &it.drawing, &style, Some(it));
    }
}

/// Pratinjau objek yang sedang ditempatkan (putus-putus, tidak disimpan).
pub fn draw_preview(painter: &Painter, layout: &PlotLayout, v: &Viewport, d: &Drawing) {
    let painter = &painter.with_clip_rect(layout.price);
    draw_one(painter, layout, v, &[], d, &DrawStyle::preview(), None);
}

/// Kotak seleksi (Ctrl+seret): isi tembus pandang + tepi putus-putus ember.
/// Tidak di-clip ke pane harga agar terlihat utuh saat melintasi sumbu.
pub fn draw_rubber_band(painter: &Painter, rect: Rect, accent: Color32) {
    if rect.width() < 0.5 && rect.height() < 0.5 {
        return; // klik tanpa seret: tidak menggambar kotak
    }
    painter.rect_filled(rect, CornerRadius::ZERO, accent.gamma_multiply(0.10));
    let stroke = Stroke::new(1.0, accent.gamma_multiply(0.75));
    let p = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
        rect.left_top(),
    ];
    let _ = painter.add(egui::Shape::dashed_line(&p, stroke, 4.0, 4.0));
}

/// Satu objek, gaya apa pun (dipakai normal + pratinjau).
fn draw_one(
    painter: &Painter,
    layout: &PlotLayout,
    v: &Viewport,
    bars: &[Bar],
    d: &Drawing,
    style: &DrawStyle,
    item: Option<&DrawingItem>,
) {
    let ox = f(layout.price.min.x);
    let oy = f(layout.price.min.y);
    let at = |a: &Anchor| -> egui::Pos2 {
        pos(
            ox + v.x_for_time(a.ts_millis),
            oy + v.y_for_price(a.price_ticks),
        )
    };
    let left = f(layout.price.min.x);
    let right = f(layout.price.max.x);
    let top = f(layout.price.min.y);
    let bottom = f(layout.price.max.y);
    let wide = EXTEND_PX;
    // Opasitas objek (pratinjau tanpa item = penuh). Warna yang diambil dari
    // konstanta/palet objek juga dikalikan supaya seluruh objek konsisten.
    let op = item.map(|it| it.opacity_factor()).unwrap_or(1.0);

    match d {
        Drawing::HorizontalLine { price_ticks } => {
            let y = oy + v.y_for_price(*price_ticks);
            line(painter, style, pos(left, y), pos(right, y));
        }
        Drawing::HorizontalRay { origin } => {
            let x0 = ox + v.x_for_time(origin.ts_millis);
            let y = oy + v.y_for_price(origin.price_ticks);
            line(painter, style, pos(x0, y), pos(right, y));
        }
        Drawing::VerticalLine { ts_millis } => {
            let x = ox + v.x_for_time(*ts_millis);
            line(painter, style, pos(x, top), pos(x, bottom));
        }
        Drawing::CrossLine { anchor } => {
            let p0 = at(anchor);
            line(painter, style, pos(left, f(p0.y)), pos(right, f(p0.y)));
            line(painter, style, pos(f(p0.x), top), pos(f(p0.x), bottom));
        }
        Drawing::Trendline { a, b } => {
            line(painter, style, at(a), at(b));
        }
        Drawing::Arrow { a, b } => {
            let (pa, pb) = (at(a), at(b));
            line(painter, style, pa, pb);
            arrow_head(painter, style, pa, pb);
        }
        Drawing::Ray { origin, through } => {
            let (p0, p1) = (at(origin), at(through));
            let (dx, dy) = (p1.x - p0.x, p1.y - p0.y);
            let len = (dx * dx + dy * dy).sqrt().max(1e-6);
            let far = egui::pos2(p0.x + dx / len * wide as f32, p0.y + dy / len * wide as f32);
            line(painter, style, p0, far);
        }
        Drawing::ExtendedLine { a, b } => {
            let (pa, pb) = (at(a), at(b));
            let (dx, dy) = (pb.x - pa.x, pb.y - pa.y);
            let len = (dx * dx + dy * dy).sqrt().max(1e-6);
            let ux = dx / len * wide as f32;
            let uy = dy / len * wide as f32;
            line(
                painter,
                style,
                egui::pos2(pa.x - ux, pa.y - uy),
                egui::pos2(pb.x + ux, pb.y + uy),
            );
        }
        Drawing::Rectangle { a, b } => {
            let rect = Rect::from_two_pos(at(a), at(b));
            painter.rect_filled(
                rect,
                CornerRadius::ZERO,
                style.stroke.color.gamma_multiply(0.08),
            );
            painter.rect_stroke(rect, CornerRadius::ZERO, style.stroke, StrokeKind::Middle);
        }
        Drawing::Ellipse { a, b } => {
            let (pa, pb) = (at(a), at(b));
            let center = egui::pos2((pa.x + pb.x) / 2.0, (pa.y + pb.y) / 2.0);
            let radius = egui::vec2(
                ((pb.x - pa.x) / 2.0).abs().max(0.5),
                ((pb.y - pa.y) / 2.0).abs().max(0.5),
            );
            painter.add(egui::Shape::ellipse_stroke(center, radius, style.stroke));
        }
        Drawing::Triangle { a, b, c } => {
            let pts = vec![at(a), at(b), at(c)];
            painter.add(egui::Shape::convex_polygon(
                pts.clone(),
                style.stroke.color.gamma_multiply(0.08),
                Stroke::NONE,
            ));
            polyline(painter, style, &pts, true);
        }
        Drawing::FibRetracement { a, b } => {
            let (pa, pb) = (at(a), at(b));
            line(painter, style, pa, pb);
            // Level kustom objek bila ada; jika tidak, level bawaan.
            let custom = item.map(|it| it.fib_levels.as_slice()).unwrap_or(&[]);
            let levels: Vec<(i32, Option<[u8; 3]>)> = if custom.is_empty() {
                FIB_LEVELS_MILLI.iter().map(|m| (*m, None)).collect()
            } else {
                custom.iter().map(|l| (l.milli, l.color)).collect()
            };
            for (milli, tint) in levels {
                let price = fib_level_price(a.price_ticks, b.price_ticks, milli);
                let y = oy + v.y_for_price(price);
                let color = match tint {
                    Some(rgb) => Color32::from_rgb(rgb[0], rgb[1], rgb[2]).gamma_multiply(op),
                    None if milli == 500 => EMBER.gamma_multiply(op),
                    None if milli == 0 || milli == 1000 => style.stroke.color,
                    None => FIB_LINE.gamma_multiply(op),
                };
                let st = DrawStyle {
                    stroke: Stroke::new(1.0, color),
                    dash: style.dash,
                };
                line(painter, &st, pos(left, y), pos(right, y));
                let text = format!("{:.1}% {}", milli as f64 / 10.0, price);
                painter.text(
                    egui::pos2(layout.price.max.x - 4.0, y as f32),
                    Align2::RIGHT_CENTER,
                    text,
                    FontId::monospace(10.0),
                    color,
                );
            }
        }
        Drawing::RangeMeasure { a, b } => {
            let (pa, pb) = (at(a), at(b));
            let rect = Rect::from_two_pos(pa, pb);
            painter.rect_filled(
                rect,
                CornerRadius::ZERO,
                style.stroke.color.gamma_multiply(0.06),
            );
            painter.rect_stroke(rect, CornerRadius::ZERO, style.stroke, StrokeKind::Middle);
            line(painter, style, pa, pb);
            let d_price = b.price_ticks - a.price_ticks;
            let pct = if a.price_ticks != 0 {
                d_price as f64 / a.price_ticks.unsigned_abs() as f64 * 100.0
            } else {
                0.0
            };
            let n_bars = bars_between(bars, a.ts_millis, b.ts_millis);
            let mut lines = vec![(
                format!("Δ {} ({:+.2}%)", d_price, pct),
                if d_price >= 0 {
                    EMBER.gamma_multiply(op)
                } else {
                    bear_color().gamma_multiply(op)
                },
            )];
            if n_bars > 0 {
                lines.push((format!("{n_bars} bar"), SLATE.gamma_multiply(op)));
            }
            label_box(
                painter,
                egui::pos2(pb.x + 8.0, pb.y - 8.0),
                Align2::LEFT_BOTTOM,
                &lines,
                bg_color(),
            );
        }
        Drawing::Position {
            long,
            entry,
            target,
        } => {
            let pe = at(entry);
            let pt = at(target);
            let (_, stop) = position_levels(entry.price_ticks, target.price_ticks);
            let ps = egui::pos2(pt.x, (oy + v.y_for_price(stop)) as f32);
            let profit = if *long {
                EMBER.gamma_multiply(op)
            } else {
                bear_color().gamma_multiply(op)
            };
            let loss = if *long {
                bear_color().gamma_multiply(op)
            } else {
                EMBER.gamma_multiply(op)
            };
            // kotak target (profit) dan stop (loss)
            let r_profit = Rect::from_two_pos(pe, pt);
            let r_loss = Rect::from_two_pos(pe, ps);
            painter.rect_filled(r_profit, CornerRadius::ZERO, profit.gamma_multiply(0.16));
            painter.rect_filled(r_loss, CornerRadius::ZERO, loss.gamma_multiply(0.16));
            painter.rect_stroke(
                r_profit,
                CornerRadius::ZERO,
                Stroke::new(1.0, profit),
                StrokeKind::Middle,
            );
            painter.rect_stroke(
                r_loss,
                CornerRadius::ZERO,
                Stroke::new(1.0, loss),
                StrokeKind::Middle,
            );
            line(painter, style, pe, pt);
            line(painter, style, pe, ps);
            let pct = |p: i64| -> f64 {
                if entry.price_ticks != 0 {
                    (p - entry.price_ticks) as f64 / entry.price_ticks.unsigned_abs() as f64 * 100.0
                } else {
                    0.0
                }
            };
            let label_t = format!(
                "Target {} ({:+.2}%)",
                target.price_ticks,
                pct(target.price_ticks)
            );
            let label_s = format!("Stop {stop} ({:+.2}%)", pct(stop));
            label_box(
                painter,
                egui::pos2(pt.x + 6.0, pt.y),
                Align2::LEFT_CENTER,
                &[(label_t, profit)],
                bg_color(),
            );
            label_box(
                painter,
                egui::pos2(ps.x + 6.0, ps.y),
                Align2::LEFT_CENTER,
                &[(label_s, loss)],
                bg_color(),
            );
        }
        Drawing::ParallelChannel { a, b, c, kind } => {
            let (pa, pb, pc) = (at(a), at(b), at(c));
            // Varian + perpanjangan objek; geometri dipakai bersama hit-test
            // (chart crate) supaya yang digambar = yang bisa diklik.
            let ext = item.map(|it| it.extend).unwrap_or_default();
            for (s0, s1) in channel_segments(
                *kind,
                ext,
                (f(pa.x), f(pa.y)),
                (f(pb.x), f(pb.y)),
                (f(pc.x), f(pc.y)),
            ) {
                line(painter, style, pos(s0.0, s0.1), pos(s1.0, s1.1));
            }
        }
        Drawing::Pitchfork { a, b, c } => {
            let (pa, pb, pc) = (at(a), at(b), at(c));
            // Varian (Andrew/Schiff/Modified Schiff/Inside) menentukan origin
            // median; geometri dipakai bersama hit-test (chart crate).
            let kind = item.map(|it| it.pitchfork).unwrap_or_default();
            let fk = pitchfork_lines(
                kind,
                (f(pa.x), f(pa.y)),
                (f(pb.x), f(pb.y)),
                (f(pc.x), f(pc.y)),
            );
            let len = (fk.dir.0 * fk.dir.0 + fk.dir.1 * fk.dir.1).sqrt().max(1e-6);
            let ext = |o: (f64, f64)| {
                let ux = fk.dir.0 / len * wide;
                let uy = fk.dir.1 / len * wide;
                (pos(o.0 - ux, o.1 - uy), pos(o.0 + ux, o.1 + uy))
            };
            let (m0, m1) = ext(fk.median_from);
            line(painter, style, m0, m1); // median
            let (t0, t1) = ext(fk.tine1_from);
            line(painter, style, t0, t1); // tine 1
            let (t2, t3) = ext(fk.tine2_from);
            line(painter, style, t2, t3); // tine 2
            let faint = DrawStyle {
                stroke: Stroke::new(1.0, style.stroke.color.gamma_multiply(0.55)),
                dash: style.dash,
            };
            line(
                painter,
                &faint,
                pos(fk.helper.0 .0, fk.helper.0 .1),
                pos(fk.helper.1 .0, fk.helper.1 .1),
            ); // garis bantu (b–c atau a–b)
        }
        Drawing::FibExtension { a, b, c } => {
            let (pa, pb, pc) = (at(a), at(b), at(c));
            line(painter, style, pa, pb);
            line(painter, style, pb, pc);
            for m in FIB_EXT_MILLI {
                let price = fib_ext_price(a.price_ticks, b.price_ticks, c.price_ticks, m);
                let y = oy + v.y_for_price(price);
                let color = if m == 1000 {
                    EMBER.gamma_multiply(op)
                } else {
                    style.stroke.color
                };
                let st = DrawStyle {
                    stroke: Stroke::new(1.0, color),
                    dash: style.dash,
                };
                line(painter, &st, pos(left, y), pos(right, y));
                painter.text(
                    egui::pos2(layout.price.max.x - 4.0, y as f32),
                    Align2::RIGHT_CENTER,
                    format!("{:.1}% {}", m as f64 / 10.0, price),
                    FontId::monospace(10.0),
                    color,
                );
            }
        }
        Drawing::FibTime { a } => {
            for (k, ts) in fib_time_ts(a.ts_millis, bars) {
                let x = ox + v.x_for_time(ts);
                line(painter, style, pos(x, top), pos(x, bottom));
                painter.text(
                    pos(x + 2.0, top + 2.0),
                    Align2::LEFT_TOP,
                    format!("{k}"),
                    FontId::monospace(10.0),
                    style.stroke.color,
                );
            }
        }
        Drawing::RegressionChannel { a, b } => {
            // Geometri regresi dipakai bersama hit-test (chart crate).
            let ext = item.map(|it| it.extend).unwrap_or_default();
            if let Some(fit) = regression_fit(bars, a.ts_millis, b.ts_millis) {
                let proj = PlotProj { ox, oy, v };
                for (s0, s1) in regression_segments(&fit, &proj, ext) {
                    line(painter, style, pos(s0.0, s0.1), pos(s1.0, s1.1));
                }
            }
        }
        Drawing::FibTrendExtension { a, b, c } => {
            let (pa, pb, pc) = (at(a), at(b), at(c));
            line(painter, style, pa, pb);
            line(painter, style, pb, pc);
            // Level horizontal dari x(c) ke kanan (opsi extend kiri ke tepi plot).
            let x0 = if item.map(|it| it.extend.left()).unwrap_or(false) {
                left
            } else {
                f(pc.x)
            };
            for m in FIB_EXT_MILLI {
                let price = fib_ext_price(a.price_ticks, b.price_ticks, c.price_ticks, m);
                let y = oy + v.y_for_price(price);
                let color = if m == 1000 {
                    EMBER.gamma_multiply(op)
                } else {
                    style.stroke.color
                };
                let st = DrawStyle {
                    stroke: Stroke::new(1.0, color),
                    dash: style.dash,
                };
                line(painter, &st, pos(x0, y), pos(right, y));
                painter.text(
                    egui::pos2(layout.price.max.x - 4.0, y as f32),
                    Align2::RIGHT_CENTER,
                    format!("{:.1}% {}", m as f64 / 10.0, price),
                    FontId::monospace(10.0),
                    color,
                );
            }
        }
        Drawing::ElliottImpulse { points }
        | Drawing::ElliottCorrection { points }
        | Drawing::Xabcd { points }
        | Drawing::Abcd { points } => {
            let pts: Vec<egui::Pos2> = points.iter().map(&at).collect();
            if pts.len() >= 2 {
                painter.add(egui::Shape::line(pts.clone(), style.stroke));
            }
            if let Some(labels) = d.pattern_labels() {
                for (i, q) in pts.iter().enumerate() {
                    if let Some(lb) = labels.get(i) {
                        painter.text(
                            egui::pos2(q.x + 6.0, q.y - 4.0),
                            Align2::LEFT_BOTTOM,
                            *lb,
                            FontId::monospace(11.0),
                            style.stroke.color.gamma_multiply(op),
                        );
                    }
                }
            }
        }
        Drawing::GannFan { a, b } => {
            let (pa, pb) = (at(a), at(b));
            for (label, o, far) in gann_fan_lines((f(pa.x), f(pa.y)), (f(pb.x), f(pb.y))) {
                line(painter, style, pos(o.0, o.1), pos(far.0, far.1));
                // Label sudut di tepi kanan plot bila garis memotongnya.
                let (dx, dy) = (far.0 - o.0, far.1 - o.1);
                let lx = right - 4.0;
                if dx.abs() > 1e-6 {
                    let t = (lx - o.0) / dx;
                    if (0.0..=1.0).contains(&t) {
                        let y = o.1 + dy * t;
                        if y >= top + 8.0 && y <= bottom - 4.0 {
                            painter.text(
                                pos(lx, y),
                                Align2::RIGHT_CENTER,
                                label,
                                FontId::monospace(10.0),
                                style.stroke.color.gamma_multiply(op * 0.9),
                            );
                        }
                    }
                }
            }
        }
        Drawing::Brush { points } | Drawing::Highlighter { points } => {
            let pts: Vec<egui::Pos2> = points.iter().map(&at).collect();
            if pts.len() >= 2 {
                let (w, alpha) = if matches!(d, Drawing::Highlighter { .. }) {
                    ((style.stroke.width * 3.0).max(6.0), 0.35)
                } else {
                    (style.stroke.width, 1.0)
                };
                painter.add(egui::Shape::line(
                    pts,
                    Stroke::new(w, style.stroke.color.gamma_multiply(alpha * op)),
                ));
            }
        }
        Drawing::Sticker { at: p, icon } => {
            draw_sticker(painter, at(p), *icon, style.stroke.color.gamma_multiply(op));
        }
        Drawing::Text { at: a, text } => {
            let (size, align) = item
                .map(|it| (it.text_size, it.text_align))
                .unwrap_or((TextSize::M, TextAlign::Left));
            draw_note(painter, at(a), text, style, false, size, align);
        }
        Drawing::Callout { at: a, text } => {
            let (size, align) = item
                .map(|it| (it.text_size, it.text_align))
                .unwrap_or((TextSize::M, TextAlign::Left));
            draw_note(painter, at(a), text, style, true, size, align);
        }
    }
}

/// Kotak teks pada satu titik (dipakai Text & Callout). Callout menambah ekor
/// garis dari anchor ke sudut kotak. Ukuran font & perataan mengikuti meta item.
fn draw_note(
    painter: &Painter,
    p: egui::Pos2,
    text: &str,
    style: &DrawStyle,
    callout: bool,
    size: TextSize,
    align: TextAlign,
) {
    let fg = style.stroke.color;
    let shown = if text.is_empty() { "..." } else { text };
    let galley = painter.layout_no_wrap(shown.to_string(), FontId::proportional(size.px()), fg);
    let pad = egui::vec2(10.0, 6.0);
    let (dx, dy) = if callout { (16.0, 14.0) } else { (8.0, 6.0) };
    let h = galley.size().y + pad.y;
    let box_pos = match align {
        TextAlign::Left => egui::pos2(p.x + dx, p.y - dy - h),
        TextAlign::Center => egui::pos2(p.x - galley.size().x / 2.0, p.y - dy - h),
        TextAlign::Right => egui::pos2(p.x - galley.size().x - dx, p.y - dy - h),
    };
    let rect = Rect::from_min_size(box_pos, galley.size() + pad);
    if callout {
        let tail = match align {
            TextAlign::Left => egui::pos2(rect.left() + 8.0, rect.bottom()),
            TextAlign::Center => rect.center_bottom(),
            TextAlign::Right => rect.right_bottom(),
        };
        painter.line_segment([p, tail], Stroke::new(1.0, fg));
    }
    painter.rect_filled(
        rect,
        CornerRadius::same(3),
        Color32::from_rgba_premultiplied(18, 22, 32, 235),
    );
    painter.rect_stroke(
        rect,
        CornerRadius::same(3),
        Stroke::new(1.0, fg.gamma_multiply(0.65)),
        StrokeKind::Middle,
    );
    painter.galley(rect.min + egui::vec2(5.0, 3.0), galley, fg);
    painter.circle_filled(p, 2.5, fg);
}

/// Proyektor layar untuk geometri yang dipakai bersama hit-test (chart crate).
struct PlotProj<'a> {
    ox: f64,
    oy: f64,
    v: &'a Viewport,
}
impl Projector for PlotProj<'_> {
    fn x(&self, ts_millis: i64) -> f64 {
        self.ox + self.v.x_for_time(ts_millis)
    }
    fn y(&self, price_ticks: i64) -> f64 {
        self.oy + self.v.y_for_price(price_ticks)
    }
}

/// Ikon "sticker": ikon vektor ukuran tetap (ala stiker TV), digambar sendiri
/// (bukan aset pihak lain).
fn draw_sticker(painter: &Painter, c: egui::Pos2, icon: StickerIcon, color: Color32) {
    let p = |x: f32, y: f32| egui::pos2(c.x + x, c.y + y);
    let fill = |pts: Vec<egui::Pos2>| egui::Shape::convex_polygon(pts, color, Stroke::NONE);
    let seg =
        |a: egui::Pos2, b: egui::Pos2, w: f32| painter.line_segment([a, b], Stroke::new(w, color));
    match icon {
        StickerIcon::ArrowUp => {
            seg(p(0.0, 11.0), p(0.0, -2.0), 3.0);
            painter.add(fill(vec![p(-7.0, -2.0), p(7.0, -2.0), p(0.0, -11.0)]));
        }
        StickerIcon::ArrowDown => {
            seg(p(0.0, -11.0), p(0.0, 2.0), 3.0);
            painter.add(fill(vec![p(-7.0, 2.0), p(7.0, 2.0), p(0.0, 11.0)]));
        }
        StickerIcon::ArrowLeft => {
            seg(p(11.0, 0.0), p(-2.0, 0.0), 3.0);
            painter.add(fill(vec![p(2.0, -7.0), p(2.0, 7.0), p(-11.0, 0.0)]));
        }
        StickerIcon::ArrowRight => {
            seg(p(-11.0, 0.0), p(2.0, 0.0), 3.0);
            painter.add(fill(vec![p(-2.0, -7.0), p(-2.0, 7.0), p(11.0, 0.0)]));
        }
        StickerIcon::TriangleUp => {
            painter.add(fill(vec![p(-9.0, 9.0), p(9.0, 9.0), p(0.0, -9.0)]));
        }
        StickerIcon::TriangleDown => {
            painter.add(fill(vec![p(-9.0, -9.0), p(9.0, -9.0), p(0.0, 9.0)]));
        }
        StickerIcon::Diamond => {
            painter.add(fill(vec![
                p(0.0, -11.0),
                p(9.0, 0.0),
                p(0.0, 11.0),
                p(-9.0, 0.0),
            ]));
        }
        StickerIcon::Circle => {
            painter.circle_filled(c, 9.0, color);
        }
        StickerIcon::Square => {
            painter.rect_filled(
                Rect::from_center_size(c, egui::vec2(17.0, 17.0)),
                CornerRadius::same(2),
                color,
            );
        }
        StickerIcon::Star => {
            let mut pts = Vec::with_capacity(10);
            for i in 0..10 {
                let ang = -std::f64::consts::FRAC_PI_2 + i as f64 * std::f64::consts::PI / 5.0;
                let r = if i % 2 == 0 { 11.0 } else { 4.5 };
                pts.push(p((ang.cos() * r) as f32, (ang.sin() * r) as f32));
            }
            painter.add(fill(pts));
        }
        StickerIcon::Check => {
            painter.add(egui::Shape::line(
                vec![p(-8.0, 0.0), p(-2.0, 7.0), p(9.0, -8.0)],
                Stroke::new(3.5, color),
            ));
        }
        StickerIcon::Cross => {
            seg(p(-7.0, -7.0), p(7.0, 7.0), 3.5);
            seg(p(-7.0, 7.0), p(7.0, -7.0), 3.5);
        }
        StickerIcon::Plus => {
            seg(p(0.0, -9.0), p(0.0, 9.0), 3.5);
            seg(p(-9.0, 0.0), p(9.0, 0.0), 3.5);
        }
        StickerIcon::Flag => {
            seg(p(0.0, -12.0), p(0.0, 12.0), 2.0);
            painter.add(fill(vec![p(0.0, -12.0), p(10.0, -6.5), p(0.0, -1.0)]));
        }
    }
}

/// Titik handle (indeks anchor + posisi layar absolut) untuk anchor drawing.
pub fn handle_points(layout: &PlotLayout, v: &Viewport, d: &Drawing) -> Vec<(usize, egui::Pos2)> {
    let ox = f(layout.price.min.x);
    let oy = f(layout.price.min.y);
    d.anchors()
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let p = match d {
                Drawing::HorizontalLine { .. } => pos(
                    f(layout.price.max.x) - 8.0,
                    oy + v.y_for_price(a.price_ticks),
                ),
                Drawing::VerticalLine { .. } => {
                    pos(ox + v.x_for_time(a.ts_millis), f(layout.price.max.y) - 8.0)
                }
                _ => pos(
                    ox + v.x_for_time(a.ts_millis),
                    oy + v.y_for_price(a.price_ticks),
                ),
            };
            (i, p)
        })
        .collect()
}

/// Handle (kotak kecil) untuk anchor drawing terpilih — dipakai untuk drag-edit.
pub fn draw_anchor_handles(painter: &Painter, layout: &PlotLayout, v: &Viewport, d: &Drawing) {
    let painter = &painter.with_clip_rect(layout.price);
    for (_, p) in handle_points(layout, v, d) {
        let r = Rect::from_center_size(p, egui::vec2(7.0, 7.0));
        painter.rect_filled(r, CornerRadius::same(1), EMBER);
        painter.rect_stroke(
            r,
            CornerRadius::same(1),
            Stroke::new(1.0, Color32::from_rgb(0x14, 0x18, 0x1d)),
            StrokeKind::Middle,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sokanlab_chart::drawing::{ChannelKind, ExtendMode, FibLevel, PitchforkKind};

    fn bars(n: usize) -> Vec<Bar> {
        (0..n)
            .map(|i| Bar {
                ts_millis: 1_000 + i as i64 * 60_000,
                open_ticks: 100 + i as i64,
                high_ticks: 110 + i as i64,
                low_ticks: 90 + i as i64,
                close_ticks: 105 + i as i64,
                volume_milli: (i as i64 + 1) * 10,
            })
            .collect()
    }

    /// Kumpulkan bentuk dari satu pass gambar (dipakai tes geometri objek).
    fn paint_shapes(mut f: impl FnMut(&egui::Painter)) -> Vec<egui::Shape> {
        let ctx = egui::Context::default();
        let area = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        let mut full = ctx.run_pass(egui::RawInput::default(), |ctx| {
            ctx.root_ui(|ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    f(&ui.painter_at(area));
                });
            });
        });
        // Tanpa backend, delta tekstur harus dibuang sendiri (debug_asserts).
        full.textures_delta.clear();
        full.shapes.into_iter().map(|cs| cs.shape).collect()
    }

    /// Semua segmen garis lurus dari bentuk (termasuk di dalam Shape::Vec).
    fn line_segments(shapes: &[egui::Shape], out: &mut Vec<([egui::Pos2; 2], egui::Stroke)>) {
        for s in shapes {
            match s {
                egui::Shape::LineSegment { points, stroke } => out.push((*points, *stroke)),
                egui::Shape::Vec(v) => line_segments(v, out),
                _ => {}
            }
        }
    }

    /// Jarak titik ke garis tak-hingga lewat l0–l1 (piksel).
    fn dist_line(p: egui::Pos2, l0: egui::Pos2, l1: egui::Pos2) -> f32 {
        let (dx, dy) = (l1.x - l0.x, l1.y - l0.y);
        let n = (dx * dx + dy * dy).sqrt().max(1e-6);
        ((p.x - l0.x) * dy - (p.y - l0.y) * dx).abs() / n
    }

    /// Hitung bentuk di panel headless: tanpa window, tanpa display.
    fn paint_pass<R: Default>(f: impl FnMut(&mut egui::Ui) -> R) -> R {
        let ctx = egui::Context::default();
        let mut f = f;
        let mut out = R::default();
        let mut full = ctx.run_pass(egui::RawInput::default(), |ctx| {
            ctx.root_ui(|ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    out = f(ui);
                });
            });
        });
        // Tanpa backend, delta tekstur harus dibuang sendiri (debug_assert di Drop).
        full.textures_delta.clear();
        out
    }

    #[test]
    fn split_gives_volume_bottom() {
        let area = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        let l = split_plot(area, 8.0);
        assert!(l.price.height() > l.volume.height());
        assert!(l.volume.max.y <= area.max.y);
        assert!(l.volume.min.y > l.price.max.y);
        // Kontrak penting: peta Y viewport memakai tinggi pane harga ini.
        assert_eq!(l.price.height(), price_pane_height(area));
    }

    #[test]
    fn split_plot_panes_susun_dan_sumbu() {
        let area = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        let l = split_plot_panes(area, 8.0, 2);
        assert_eq!(l.panes.len(), 2);
        assert!(l.price.max.y < l.volume.min.y);
        assert!(l.volume.max.y < l.panes[0].min.y);
        assert!(l.panes[0].max.y < l.panes[1].min.y);
        assert!(l.panes[1].max.y <= area.max.y - TIME_H + 0.01);
        // Pane & volume selebar area plot; strip sumbu di kanan, waktu di bawah.
        assert_eq!(l.volume.width(), l.price.width());
        assert_eq!(l.panes[0].width(), l.price.width());
        assert_eq!(l.axis_y.min.x, l.price.max.x);
        assert!(l.axis_y.max.x <= area.max.x);
        assert_eq!(l.axis_x.min.y, l.axis_y.max.y);
        assert!(l.axis_x.width() <= l.price.width());
        // Tanpa pane: tetap kompatibel dengan split_plot + tinggi lama.
        let l0 = split_plot(area, 8.0);
        assert!(l0.panes.is_empty());
        assert_eq!(l0.price.height(), price_pane_height(area));
        assert!(
            l0.price.height() > l.price.height(),
            "pane menyusutkan harga"
        );
        // Batas 3 pane walau diminta lebih.
        assert_eq!(split_plot_panes(area, 8.0, 9).panes.len(), 3);
    }

    #[test]
    fn fmt_waktu_utc() {
        assert_eq!(fmt_time(0, 60_000), "00:00");
        assert_eq!(fmt_time(3_600_000, 60_000), "01:00");
        assert_eq!(fmt_time(0, 10 * 86_400_000), "01/01 00:00");
        assert_eq!(fmt_time(0, 400 * 86_400_000), "1970-01-01");
        // Nilai negatif (sebelum epoch) tidak panic.
        assert_eq!(fmt_time(-86_400_000, 400 * 86_400_000), "1969-12-31");
    }

    #[test]
    fn spacing_fits_width() {
        assert!((fit_spacing(800.0, 100) - 8.0).abs() < 1e-9);
        assert!((fit_spacing(800.0, 10_000) - 0.08).abs() < 1e-9);
        assert!((fit_spacing(800.0, 1_000_000) - 0.0008).abs() < 1e-12);
    }

    #[test]
    fn draw_step_decimates_subpixel() {
        assert_eq!(draw_step(8.0), 1);
        assert_eq!(draw_step(1.0), 1);
        assert_eq!(draw_step(0.5), 2);
        assert_eq!(draw_step(0.08), 13); // 12,5 → 13
        assert_eq!(draw_step(0.0008), 1250);
        assert_eq!(draw_step(0.0), 1);
    }

    #[test]
    fn candles_decimated_saat_subpixel() {
        let b = bars(10_000);
        let v = Viewport {
            width_px: 800.0,
            height_px: 600.0,
            t_min: b[0].ts_millis,
            t_max: b[9_999].ts_millis,
            p_min: 0,
            p_max: 20_000,
            ..Default::default()
        };
        // 10k bar ÷ 800 px = spacing 0,08 → step 13 → ~770 candle digambar, bukan 10k.
        let area = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        let layout = split_plot(area, fit_spacing(800.0, 10_000));
        let drawn = paint_pass(|ui| {
            let painter = ui.painter_at(area);
            draw_candles(&painter, &layout, &v, &b, 0)
        });
        assert!(drawn > 700 && drawn < 800, "drawn={drawn}");
    }

    #[test]
    fn fib_level_kustom_dan_opasitas() {
        let b = bars(30);
        let v = Viewport {
            width_px: 800.0,
            height_px: 600.0,
            t_min: b[0].ts_millis,
            t_max: b[29].ts_millis,
            p_min: 0,
            p_max: 20_000,
            ..Default::default()
        };
        let area = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        let layout = split_plot(area, 8.0);
        let mut it = DrawingItem::new(Drawing::FibRetracement {
            a: Anchor {
                ts_millis: b[2].ts_millis,
                price_ticks: 105,
            },
            b: Anchor {
                ts_millis: b[20].ts_millis,
                price_ticks: 135,
            },
        });
        let count_h = |it: &DrawingItem| -> usize {
            let shapes = paint_shapes(|p| {
                draw_drawings(
                    p,
                    &layout,
                    &v,
                    &b,
                    std::slice::from_ref(it),
                    &[],
                    None,
                    false,
                )
            });
            let mut segs = Vec::new();
            line_segments(&shapes, &mut segs);
            segs.iter()
                .filter(|(pts, _)| (pts[0].y - pts[1].y).abs() < 0.5)
                .count()
        };
        assert_eq!(count_h(&it), 7, "tujuh level bawaan");
        // Opasitas 40% → semua garis objek jadi semi-transparan (alpha ≈ 102).
        it.opacity = 40;
        let shapes = paint_shapes(|p| {
            draw_drawings(
                p,
                &layout,
                &v,
                &b,
                std::slice::from_ref(&it),
                &[],
                None,
                false,
            )
        });
        let mut segs = Vec::new();
        line_segments(&shapes, &mut segs);
        assert!(!segs.is_empty());
        for (_, st) in &segs {
            assert!(
                (90..=115).contains(&st.color.a()),
                "alpha={} harus ≈ 40%",
                st.color.a()
            );
        }
        it.opacity = 100;
        // Dua level kustom → dua garis horizontal; warna kustom dipakai apa adanya.
        it.fib_levels = vec![
            FibLevel {
                milli: 500,
                color: Some([0xf2, 0xb0, 0x4d]),
            },
            FibLevel {
                milli: 425,
                color: None,
            },
        ];
        assert_eq!(count_h(&it), 2, "dua level kustom");
        let shapes = paint_shapes(|p| {
            draw_drawings(
                p,
                &layout,
                &v,
                &b,
                std::slice::from_ref(&it),
                &[],
                None,
                false,
            )
        });
        let mut segs = Vec::new();
        line_segments(&shapes, &mut segs);
        assert!(
            segs.iter()
                .any(|(_, st)| st.color == Color32::from_rgb(0xf2, 0xb0, 0x4d)),
            "warna level kustom dipakai"
        );
    }

    #[test]
    fn varian_pitchfork_mengubah_geometri() {
        let b = bars(30);
        let v = Viewport {
            width_px: 800.0,
            height_px: 600.0,
            t_min: b[0].ts_millis,
            t_max: b[29].ts_millis,
            p_min: 90,
            p_max: 140,
            ..Default::default()
        };
        let area = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        let layout = split_plot(area, 8.0);
        let sx = |ts: i64| layout.price.min.x as f64 + v.x_for_time(ts);
        let sy = |pr: i64| layout.price.min.y as f64 + v.y_for_price(pr);
        let a = Anchor {
            ts_millis: b[2].ts_millis,
            price_ticks: 100,
        };
        let bb = Anchor {
            ts_millis: b[10].ts_millis,
            price_ticks: 120,
        };
        let c = Anchor {
            ts_millis: b[20].ts_millis,
            price_ticks: 110,
        };
        let mut it = DrawingItem::new(Drawing::Pitchfork { a, b: bb, c });
        let segs_of = |it: &DrawingItem| -> Vec<([egui::Pos2; 2], egui::Stroke)> {
            let shapes = paint_shapes(|p| {
                draw_drawings(
                    p,
                    &layout,
                    &v,
                    &b,
                    std::slice::from_ref(it),
                    &[],
                    None,
                    false,
                )
            });
            let mut segs = Vec::new();
            line_segments(&shapes, &mut segs);
            segs
        };
        let sa = egui::pos2(sx(a.ts_millis) as f32, sy(a.price_ticks) as f32);
        let sb = egui::pos2(sx(bb.ts_millis) as f32, sy(bb.price_ticks) as f32);
        let sc = egui::pos2(sx(c.ts_millis) as f32, sy(c.price_ticks) as f32);
        // Andrew: 4 garis (median, 2 tine, bantu) dan median lewat titik tengah b–c.
        let segs = segs_of(&it);
        assert_eq!(segs.len(), 4, "median + 2 tine + garis bantu");
        let mid_bc = egui::pos2((sb.x + sc.x) / 2.0, (sb.y + sc.y) / 2.0);
        assert!(
            dist_line(mid_bc, segs[0].0[0], segs[0].0[1]) < 1.0,
            "median Andrew lewat titik tengah b–c"
        );
        assert!(dist_line(sc, segs[0].0[0], segs[0].0[1]) > 5.0);
        // Garis bantu Andrew = b–c.
        assert!((segs[3].0[0].x - sb.x).abs() < 1.0 && (segs[3].0[0].y - sb.y).abs() < 1.0);
        assert!((segs[3].0[1].x - sc.x).abs() < 1.0 && (segs[3].0[1].y - sc.y).abs() < 1.0);
        // Inside: median titik tengah a–b → c (c tepat di median), bantu = a–b.
        it.pitchfork = PitchforkKind::Inside;
        let segs = segs_of(&it);
        assert_eq!(segs.len(), 4);
        assert!(
            dist_line(sc, segs[0].0[0], segs[0].0[1]) < 1.0,
            "median Inside lewat c"
        );
        let mid_ab = egui::pos2((sa.x + sb.x) / 2.0, (sa.y + sb.y) / 2.0);
        assert!(dist_line(mid_ab, segs[0].0[0], segs[0].0[1]) < 1.0);
        assert!((segs[3].0[0].x - sa.x).abs() < 1.0 && (segs[3].0[1].x - sb.x).abs() < 1.0);
        // Schiff: origin di waktu a, harga rata-rata a–b.
        it.pitchfork = PitchforkKind::Schiff;
        let segs = segs_of(&it);
        let o = egui::pos2(sa.x, (sa.y + sb.y) / 2.0);
        assert!(dist_line(o, segs[0].0[0], segs[0].0[1]) < 1.0);
    }

    fn sample_drawings() -> Vec<DrawingItem> {
        let raw = vec![
            Drawing::Trendline {
                a: Anchor {
                    ts_millis: 1_000,
                    price_ticks: 100,
                },
                b: Anchor {
                    ts_millis: 5_000,
                    price_ticks: 150,
                },
            },
            Drawing::HorizontalLine { price_ticks: 120 },
            Drawing::VerticalLine { ts_millis: 3_000 },
            Drawing::Ray {
                origin: Anchor {
                    ts_millis: 1_000,
                    price_ticks: 100,
                },
                through: Anchor {
                    ts_millis: 4_000,
                    price_ticks: 130,
                },
            },
            Drawing::Rectangle {
                a: Anchor {
                    ts_millis: 1_000,
                    price_ticks: 90,
                },
                b: Anchor {
                    ts_millis: 6_000,
                    price_ticks: 140,
                },
            },
            Drawing::HorizontalRay {
                origin: Anchor {
                    ts_millis: 2_000,
                    price_ticks: 95,
                },
            },
            Drawing::CrossLine {
                anchor: Anchor {
                    ts_millis: 3_500,
                    price_ticks: 130,
                },
            },
            Drawing::ExtendedLine {
                a: Anchor {
                    ts_millis: 1_000,
                    price_ticks: 110,
                },
                b: Anchor {
                    ts_millis: 4_000,
                    price_ticks: 100,
                },
            },
            Drawing::Arrow {
                a: Anchor {
                    ts_millis: 2_000,
                    price_ticks: 145,
                },
                b: Anchor {
                    ts_millis: 5_500,
                    price_ticks: 155,
                },
            },
            Drawing::Ellipse {
                a: Anchor {
                    ts_millis: 1_500,
                    price_ticks: 100,
                },
                b: Anchor {
                    ts_millis: 4_500,
                    price_ticks: 130,
                },
            },
            Drawing::Triangle {
                a: Anchor {
                    ts_millis: 1_000,
                    price_ticks: 95,
                },
                b: Anchor {
                    ts_millis: 5_000,
                    price_ticks: 95,
                },
                c: Anchor {
                    ts_millis: 3_000,
                    price_ticks: 125,
                },
            },
            Drawing::FibRetracement {
                a: Anchor {
                    ts_millis: 2_000,
                    price_ticks: 160,
                },
                b: Anchor {
                    ts_millis: 6_000,
                    price_ticks: 105,
                },
            },
            Drawing::RangeMeasure {
                a: Anchor {
                    ts_millis: 1_000,
                    price_ticks: 100,
                },
                b: Anchor {
                    ts_millis: 3_000,
                    price_ticks: 120,
                },
            },
            Drawing::Position {
                long: true,
                entry: Anchor {
                    ts_millis: 2_500,
                    price_ticks: 115,
                },
                target: Anchor {
                    ts_millis: 5_000,
                    price_ticks: 135,
                },
            },
            Drawing::ParallelChannel {
                a: Anchor {
                    ts_millis: 1_000,
                    price_ticks: 95,
                },
                b: Anchor {
                    ts_millis: 6_000,
                    price_ticks: 110,
                },
                c: Anchor {
                    ts_millis: 1_000,
                    price_ticks: 105,
                },
                kind: ChannelKind::Parallel,
            },
            Drawing::Pitchfork {
                a: Anchor {
                    ts_millis: 1_000,
                    price_ticks: 140,
                },
                b: Anchor {
                    ts_millis: 4_000,
                    price_ticks: 95,
                },
                c: Anchor {
                    ts_millis: 5_500,
                    price_ticks: 100,
                },
            },
            Drawing::FibExtension {
                a: Anchor {
                    ts_millis: 2_000,
                    price_ticks: 155,
                },
                b: Anchor {
                    ts_millis: 4_000,
                    price_ticks: 120,
                },
                c: Anchor {
                    ts_millis: 3_000,
                    price_ticks: 135,
                },
            },
            Drawing::FibTime {
                a: Anchor {
                    ts_millis: 1_000,
                    price_ticks: 100,
                },
            },
            Drawing::ParallelChannel {
                a: Anchor {
                    ts_millis: 2_000,
                    price_ticks: 100,
                },
                b: Anchor {
                    ts_millis: 5_000,
                    price_ticks: 120,
                },
                c: Anchor {
                    ts_millis: 3_000,
                    price_ticks: 90,
                },
                kind: ChannelKind::Disjoint,
            },
            Drawing::ParallelChannel {
                a: Anchor {
                    ts_millis: 2_500,
                    price_ticks: 90,
                },
                b: Anchor {
                    ts_millis: 6_000,
                    price_ticks: 130,
                },
                c: Anchor {
                    ts_millis: 2_500,
                    price_ticks: 160,
                },
                kind: ChannelKind::Flat,
            },
        ];
        let mut items: Vec<DrawingItem> = raw.into_iter().map(DrawingItem::new).collect();
        // variasi metadata: gaya, sembunyi, kunci, nama (untuk uji painter)
        items[0].style = Some(sokanlab_chart::drawing::ItemStyle {
            color: [0x7f, 0xb2, 0xd9],
            width: 3,
            dash: DashStyle::Dashed,
        });
        items[3].visible = false;
        items[4].locked = true;
        items[5].name = Some("channel uji".into());
        // channel varian: satu dengan perpanjangan dua arah, satu tanpa
        let n = items.len();
        items[n - 2].extend = ExtendMode::Both;
        items
    }

    #[test]
    fn draws_headless_without_panic() {
        let b = bars(50);
        let v = Viewport {
            width_px: 800.0,
            height_px: 400.0,
            t_min: b[0].ts_millis,
            t_max: b[20].ts_millis,
            p_min: 0,
            p_max: 200,
            ..Default::default()
        };
        paint_pass(|ui| {
            let area = ui.available_rect_before_wrap();
            let layout = split_plot(area, 8.0);
            let painter = ui.painter_at(area);
            draw_grid(&painter, &layout, 5);
            let start = visible_indices(&b, v.t_min, v.t_max).start;
            let drawn = draw_candles(&painter, &layout, &v, &b, start);
            draw_volume(&painter, &layout, &b, v.t_min, v.t_max, start);
            draw_crosshair(
                &painter,
                &layout,
                &v,
                Some(egui::pos2(area.min.x + 100.0, area.min.y + 100.0)),
            );
            draw_last_price(&painter, &layout, &v, Some(105));
            let all = sample_drawings();
            draw_drawings(&painter, &layout, &v, &b, &all, &[0], Some(1), false);
            draw_anchor_handles(&painter, &layout, &v, &all[0].drawing);
            draw_preview(&painter, &layout, &v, &all[11].drawing);
            drawn
        });
    }

    #[test]
    fn multi_seleksi_menandai_semua_dan_kotak_seleksi_tergambar() {
        let b = bars(10);
        let v = Viewport {
            width_px: 800.0,
            height_px: 600.0,
            t_min: b[0].ts_millis,
            t_max: b[9].ts_millis,
            p_min: 80,
            p_max: 130,
            ..Default::default()
        };
        let area = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        let layout = split_plot(area, 8.0);
        let mk = |i: usize| {
            DrawingItem::new(Drawing::Trendline {
                a: Anchor {
                    ts_millis: b[i].ts_millis,
                    price_ticks: 95,
                },
                b: Anchor {
                    ts_millis: b[i + 1].ts_millis,
                    price_ticks: 125,
                },
            })
        };
        let items = vec![mk(1), mk(4), mk(7)];
        let shapes = paint_shapes(|p| {
            draw_drawings(p, &layout, &v, &b, &items, &[0, 1], None, false);
            draw_rubber_band(
                p,
                Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(120.0, 80.0)),
                EMBER,
            );
        });
        let mut segs = Vec::new();
        line_segments(&shapes, &mut segs);
        let ember = segs.iter().filter(|(_, st)| st.color == EMBER).count();
        let slate = segs.iter().filter(|(_, st)| st.color == SLATE).count();
        assert!(ember >= 2, "dua objek terpilih ditandai ember ({ember})");
        assert!(slate >= 1, "objek tak terpilih tetap slate ({slate})");
        assert!(
            shapes.iter().any(|s| matches!(s, egui::Shape::Rect(_))),
            "kotak seleksi punya isi tembus pandang"
        );
        assert!(segs.len() >= 3, "tepi kotak seleksi putus-putus tergambar");
        // Kotak nol (klik tanpa seret) tidak menggambar isi/tepi apa pun.
        let zero = paint_shapes(|p| {
            draw_rubber_band(
                p,
                Rect::from_min_size(egui::pos2(50.0, 50.0), egui::vec2(0.2, 0.2)),
                EMBER,
            )
        });
        let mut zsegs = Vec::new();
        line_segments(&zero, &mut zsegs);
        assert!(
            zsegs.iter().all(|(_, st)| st.color != EMBER),
            "kotak nol tanpa tepi"
        );
        let tint = EMBER.gamma_multiply(0.10);
        assert!(
            !zero
                .iter()
                .any(|s| matches!(s, egui::Shape::Rect(r) if r.fill == tint)),
            "kotak nol tanpa isi"
        );
    }

    #[test]
    fn hidden_menyembunyikan_semua_drawing() {
        let b = bars(10);
        let v = Viewport {
            width_px: 800.0,
            height_px: 400.0,
            t_min: b[0].ts_millis,
            t_max: b[9].ts_millis,
            p_min: 0,
            p_max: 200,
            ..Default::default()
        };
        let area = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        let count_shapes = |hidden: bool| {
            paint_pass(|ui| {
                let layout = split_plot(area, 8.0);
                let painter = ui.painter_at(area);
                let all = sample_drawings();
                draw_drawings(&painter, &layout, &v, &b, &all, &[], None, hidden)
            })
        };
        // paint_pass tidak mengembalikan jumlah shape; cukup jamin tidak panic di dua mode.
        count_shapes(false);
        count_shapes(true);
    }

    #[test]
    fn only_visible_bars_drawn() {
        let b = bars(100);
        let v = Viewport {
            width_px: 800.0,
            height_px: 400.0,
            t_min: b[0].ts_millis,
            t_max: b[9].ts_millis,
            p_min: 0,
            p_max: 200,
            ..Default::default()
        };
        let drawn = paint_pass(|ui| {
            let area = ui.available_rect_before_wrap();
            let layout = split_plot(area, 8.0);
            let painter = ui.painter_at(area);
            draw_candles(&painter, &layout, &v, &b, 0)
        });
        assert_eq!(drawn, 10);
    }

    #[test]
    fn crosshair_returns_price_time() {
        let b = bars(10);
        let v = Viewport {
            width_px: 800.0,
            height_px: 400.0,
            t_min: b[0].ts_millis,
            t_max: b[9].ts_millis,
            p_min: 0,
            p_max: 200,
            ..Default::default()
        };
        let got = paint_pass(|ui| {
            let area = ui.available_rect_before_wrap();
            let layout = split_plot(area, 8.0);
            let painter = ui.painter_at(area);
            let inside = egui::pos2(area.min.x + 40.0, area.min.y + 40.0);
            draw_crosshair(&painter, &layout, &v, Some(inside))
        });
        assert!(got.is_some());
    }
    #[test]
    fn batch8_alat_baru_menggambar_bentuk() {
        let b = bars(30);
        let v = Viewport {
            width_px: 800.0,
            height_px: 600.0,
            t_min: b[0].ts_millis,
            t_max: b[29].ts_millis,
            p_min: 0,
            p_max: 20_000,
            ..Default::default()
        };
        let area = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        let layout = split_plot(area, 8.0);
        let an = |i: usize, p: i64| Anchor {
            ts_millis: b[i].ts_millis,
            price_ticks: p,
        };
        let shapes_for = |d: Drawing| -> Vec<egui::Shape> {
            let it = DrawingItem::new(d);
            paint_shapes(|p| {
                draw_drawings(
                    p,
                    &layout,
                    &v,
                    &b,
                    std::slice::from_ref(&it),
                    &[],
                    None,
                    false,
                )
            })
        };
        // Fib trend: 2 segmen trendline + 6 level horizontal + label teks.
        let sh = shapes_for(Drawing::FibTrendExtension {
            a: an(2, 100),
            b: an(12, 160),
            c: an(18, 140),
        });
        let mut segs = Vec::new();
        line_segments(&sh, &mut segs);
        let horiz = segs
            .iter()
            .filter(|(pts, _)| (pts[0].y - pts[1].y).abs() < 0.5)
            .count();
        assert_eq!(horiz, 6, "enam level fib extension");
        assert!(
            sh.iter().any(|s| matches!(s, egui::Shape::Text(_))),
            "label level"
        );
        // Gann fan: sembilan garis dari satu titik asal.
        let sh = shapes_for(Drawing::GannFan {
            a: an(3, 100),
            b: an(9, 130),
        });
        let mut segs = Vec::new();
        line_segments(&sh, &mut segs);
        assert_eq!(segs.len(), 9, "sembilan sudut Gann");
        // Regression: tiga garis (sigma 0 → berimpit, tetap tiga segmen).
        let sh = shapes_for(Drawing::RegressionChannel {
            a: an(5, 100),
            b: an(25, 100),
        });
        let mut segs = Vec::new();
        line_segments(&sh, &mut segs);
        assert_eq!(segs.len(), 3);
        // Brush/highlighter: satu polyline per objek.
        for d in [
            Drawing::Brush {
                points: vec![an(1, 100), an(5, 110), an(9, 104)],
            },
            Drawing::Highlighter {
                points: vec![an(1, 100), an(5, 110)],
            },
        ] {
            let sh = shapes_for(d);
            assert!(
                sh.iter()
                    .any(|s| matches!(s, egui::Shape::Path(_) | egui::Shape::LineSegment { .. })),
                "polyline garis bebas"
            );
        }
        // Sticker: ikon digambar (bentuk terisi).
        let sh = shapes_for(Drawing::Sticker {
            at: an(4, 120),
            icon: StickerIcon::Star,
        });
        assert!(!sh.is_empty(), "ikon sticker digambar");
        // Pola: label titik X-A-B-C-D di atas polyline.
        let sh = shapes_for(Drawing::Xabcd {
            points: vec![an(1, 90), an(4, 130), an(7, 100), an(10, 140), an(13, 110)],
        });
        let texts = sh
            .iter()
            .filter(|s| matches!(s, egui::Shape::Text(_)))
            .count();
        assert_eq!(texts, 5, "label X-A-B-C-D");
    }
}
