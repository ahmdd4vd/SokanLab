// SokanLab chart — Apache-2.0. Geometri + navigasi murni, tanpa GUI.
// Prinsip: koordinat harga/waktu harus akurat setelah zoom, resize, ganti dataset.

pub mod drawing;

use sokanlab_domain::Bar;
use std::ops::Range;

/// Mode skala sumbu harga (padanan TV: linear/log/persen).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScaleMode {
    #[default]
    Linear,
    Log,
    Percent,
}

impl ScaleMode {
    pub fn label(self) -> &'static str {
        match self {
            ScaleMode::Linear => "Linear",
            ScaleMode::Log => "Log",
            ScaleMode::Percent => "Persen",
        }
    }

    /// Kunci JSON stabil untuk persistensi layout.
    pub fn key(self) -> &'static str {
        match self {
            ScaleMode::Linear => "linear",
            ScaleMode::Log => "log",
            ScaleMode::Percent => "percent",
        }
    }

    pub fn from_key(k: &str) -> Option<Self> {
        match k {
            "linear" => Some(ScaleMode::Linear),
            "log" => Some(ScaleMode::Log),
            "percent" => Some(ScaleMode::Percent),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub width_px: f64,
    pub height_px: f64,
    pub t_min: i64,
    pub t_max: i64,
    pub p_min: i64,
    pub p_max: i64,
    /// Mode skala harga; `Percent` memakai `pct_base` sebagai titik 0%.
    pub scale: ScaleMode,
    /// Balik arah sumbu harga (TV: invert scale).
    pub invert: bool,
    /// Basis skala persen (harga, ticks). 0 = belum diisi → dianggap 1.
    pub pct_base: i64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            width_px: 800.0,
            height_px: 400.0,
            t_min: 0,
            t_max: 1,
            p_min: 0,
            p_max: 1,
            scale: ScaleMode::Linear,
            invert: false,
            pct_base: 0,
        }
    }
}

impl Viewport {
    pub fn x_for_time(self, ts_millis: i64) -> f64 {
        let span = (self.t_max - self.t_min).max(1) as f64;
        ((ts_millis - self.t_min) as f64 / span) * self.width_px
    }

    /// Nilai ternormalisasi dari harga menurut mode skala.
    fn v_for(self, price_ticks: i64) -> f64 {
        match self.scale {
            ScaleMode::Linear => price_ticks as f64,
            ScaleMode::Log => (price_ticks.max(1) as f64).ln(),
            ScaleMode::Percent => {
                let base = if self.pct_base != 0 { self.pct_base } else { 1 };
                (price_ticks as f64 - base as f64) / base as f64 * 100.0
            }
        }
    }

    fn v_range(self) -> (f64, f64) {
        (self.v_for(self.p_min), self.v_for(self.p_max))
    }

    fn v_span(self) -> f64 {
        let (vmin, vmax) = self.v_range();
        let span = vmax - vmin;
        if self.scale == ScaleMode::Linear {
            span.max(1.0)
        } else {
            span.abs().max(1e-9)
        }
    }

    fn price_from_v(self, v: f64) -> i64 {
        let out = match self.scale {
            ScaleMode::Linear => v.round(),
            ScaleMode::Log => v.exp().round(),
            ScaleMode::Percent => {
                let base = if self.pct_base != 0 { self.pct_base } else { 1 };
                (base as f64 * (1.0 + v / 100.0)).round()
            }
        };
        out.clamp(i64::MIN as f64, i64::MAX as f64) as i64
    }

    /// Fraksi vertikal efektif (0 = bawah, 1 = atas) setelah memperhitungkan invert.
    fn y_eff(self, y: f64) -> f64 {
        if self.invert {
            self.height_px - y
        } else {
            y
        }
    }

    pub fn y_for_price(self, price_ticks: i64) -> f64 {
        let (vmin, _) = self.v_range();
        let frac = (self.v_for(price_ticks) - vmin) / self.v_span();
        let y = self.height_px - frac * self.height_px;
        if self.invert {
            self.height_px - y
        } else {
            y
        }
    }

    pub fn time_at_x(self, x: f64) -> i64 {
        if self.width_px <= 0.0 {
            return self.t_min;
        }
        self.t_min + ((x / self.width_px) * (self.t_max - self.t_min) as f64).round() as i64
    }

    pub fn price_at_y(self, y: f64) -> i64 {
        if self.height_px <= 0.0 {
            return self.p_min;
        }
        let frac = 1.0 - (self.y_eff(y) / self.height_px);
        let (vmin, vmax) = self.v_range();
        self.price_from_v(vmin + frac * (vmax - vmin))
    }

    /// Zoom waktu terhadap anchor; timestamp anchor tetap di pixel yang sama.
    pub fn zoom_time(mut self, factor: f64, anchor_x: f64) -> Self {
        let factor = factor.clamp(0.05, 20.0);
        if self.width_px <= 0.0 {
            return self;
        }
        let anchor_t = self.time_at_x(anchor_x);
        let span = ((self.t_max - self.t_min) as f64 * factor).max(1.0) as i64;
        let frac = (anchor_x / self.width_px).clamp(0.0, 1.0);
        self.t_min = anchor_t - (frac * span as f64) as i64;
        self.t_max = self.t_min + span;
        self
    }

    /// Zoom harga terhadap anchor; harga anchor tetap di pixel yang sama.
    pub fn zoom_price(mut self, factor: f64, anchor_y: f64) -> Self {
        let factor = factor.clamp(0.05, 20.0);
        if self.height_px <= 0.0 {
            return self;
        }
        let y_eff = self.y_eff(anchor_y);
        let frac = (1.0 - y_eff / self.height_px).clamp(0.0, 1.0);
        if self.scale == ScaleMode::Linear {
            let anchor_p = self.price_at_y(anchor_y);
            let span = (((self.p_max - self.p_min) as f64) * factor).max(1.0) as i64;
            self.p_min = anchor_p - (frac * span as f64) as i64;
            self.p_max = self.p_min + span;
            return self;
        }
        // Log/persen: zoom di ruang ternormalisasi agar jarak visual seragam.
        let (vmin, vmax) = self.v_range();
        let av = vmin + frac * (vmax - vmin);
        let span_v = (vmax - vmin).abs() * factor;
        let vmax_new = av + frac * span_v;
        let vmin_new = vmax_new - span_v;
        self.p_min = self.price_from_v(vmin_new);
        self.p_max = self.price_from_v(vmax_new);
        if self.p_max <= self.p_min {
            self.p_max = self.p_min + 1;
        }
        self
    }

    /// Pan pixel: geser ke kanan = lihat waktu lebih lama.
    pub fn pan_px(mut self, dx: f64, dy: f64) -> Self {
        if self.width_px > 0.0 {
            let dt = (dx / self.width_px) * (self.t_max - self.t_min) as f64;
            self.t_min -= dt.round() as i64;
            self.t_max -= dt.round() as i64;
        }
        if self.height_px > 0.0 {
            let sign = if self.invert { -1.0 } else { 1.0 };
            if self.scale == ScaleMode::Linear {
                let dp = sign * (dy / self.height_px) * (self.p_max - self.p_min) as f64;
                self.p_min += dp.round() as i64;
                self.p_max += dp.round() as i64;
            } else {
                let (vmin, vmax) = self.v_range();
                let dv = sign * (dy / self.height_px) * (vmax - vmin);
                self.p_min = self.price_from_v(vmin + dv);
                self.p_max = self.price_from_v(vmax + dv);
                if self.p_max <= self.p_min {
                    self.p_max = self.p_min + 1;
                }
            }
        }
        self
    }

    /// Fit rentang harga dari bar terlihat, dengan padding persen.
    pub fn fit_price(mut self, bars: &[Bar]) -> Self {
        let vis = visible_indices(bars, self.t_min, self.t_max);
        let mut lo = i64::MAX;
        let mut hi = i64::MIN;
        for i in vis {
            lo = lo.min(bars[i].low_ticks);
            hi = hi.max(bars[i].high_ticks);
        }
        if lo == i64::MAX {
            return self;
        }
        if lo == hi {
            lo -= 1;
            hi += 1;
        }
        let pad = ((hi - lo) as f64 * 0.05).ceil() as i64;
        self.p_min = lo - pad;
        self.p_max = hi + pad;
        self
    }

    pub fn resize(mut self, width_px: f64, height_px: f64) -> Self {
        self.width_px = width_px;
        self.height_px = height_px;
        self
    }
}

/// Rentang indeks bar yang terlihat (binary search, asumsi bars urut menaik).
pub fn visible_indices(bars: &[Bar], t_min: i64, t_max: i64) -> Range<usize> {
    let start = bars.partition_point(|b| b.ts_millis < t_min);
    let end = bars.partition_point(|b| b.ts_millis <= t_max);
    start..end.max(start)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CandleGeom {
    pub x_center: f64,
    pub body_left: f64,
    pub body_right: f64,
    pub body_top: f64,
    pub body_bottom: f64,
    pub wick_top: f64,
    pub wick_bottom: f64,
}

/// Geometri satu candle pada plotting area yang dimulai di x=0.
pub fn candle_geom(
    v: &Viewport,
    spacing_px: f64,
    visible_start: usize,
    index: usize,
    bar: &Bar,
) -> CandleGeom {
    let x_center = (index - visible_start) as f64 * spacing_px + spacing_px / 2.0;
    let body_w = (spacing_px * 0.7).max(1.0);
    let y_high = v.y_for_price(bar.high_ticks);
    let y_low = v.y_for_price(bar.low_ticks);
    let y_open = v.y_for_price(bar.open_ticks);
    let y_close = v.y_for_price(bar.close_ticks);
    CandleGeom {
        x_center,
        body_left: x_center - body_w / 2.0,
        body_right: x_center + body_w / 2.0,
        body_top: y_open.min(y_close),
        body_bottom: y_open.max(y_close),
        wick_top: y_high.min(y_low),
        wick_bottom: y_high.max(y_low),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolumeGeom {
    pub x_left: f64,
    pub x_right: f64,
    pub y_top: f64,
    pub y_bottom: f64,
}

/// Skala volume dari bar terlihat; height_px = tinggi pane volume.
pub fn volume_scale(bars: &[Bar], range: Range<usize>) -> i64 {
    let mut max = 0i64;
    for i in range {
        max = max.max(bars[i].volume_milli.abs());
    }
    max.max(1)
}

pub fn volume_geom(x_left: f64, width: f64, value: i64, max: i64, height_px: f64) -> VolumeGeom {
    let frac = (value.abs() as f64 / max as f64).clamp(0.0, 1.0);
    let h = frac * height_px;
    VolumeGeom {
        x_left,
        x_right: x_left + width,
        y_top: height_px - h,
        y_bottom: height_px,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_bars(n: usize) -> Vec<Bar> {
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

    #[test]
    fn origin_maps_to_corners() {
        let v = Viewport {
            width_px: 800.0,
            height_px: 600.0,
            t_min: 0,
            t_max: 1000,
            p_min: 0,
            p_max: 100,
            ..Default::default()
        };
        assert!((v.x_for_time(0)).abs() < 1e-9);
        assert!((v.x_for_time(1000) - 800.0).abs() < 1e-9);
        assert!((v.y_for_price(100)).abs() < 1e-9);
        assert!((v.y_for_price(0) - 600.0).abs() < 1e-9);
    }

    #[test]
    fn round_trip_price_accurate_after_zoom() {
        let v0 = Viewport {
            width_px: 800.0,
            height_px: 600.0,
            t_min: 0,
            t_max: 1000,
            p_min: 0,
            p_max: 100,
            ..Default::default()
        }
        .zoom_price(0.25, 300.0)
        .pan_px(0.0, 40.0)
        .resize(1024.0, 768.0);
        for p in [1, 7, 42, 99, 250] {
            let y = v0.y_for_price(p);
            let back = v0.price_at_y(y);
            assert!((back - p).abs() <= 1, "price {p} -> {back}");
        }
    }

    #[test]
    fn zoom_time_keeps_anchor_timestamp() {
        let v0 = Viewport {
            width_px: 800.0,
            height_px: 600.0,
            t_min: 0,
            t_max: 1000,
            p_min: 0,
            p_max: 100,
            ..Default::default()
        };
        let anchor_x = 640.0;
        let before = v0.time_at_x(anchor_x);
        let v1 = v0.zoom_time(0.5, anchor_x);
        assert_eq!(v1.time_at_x(anchor_x), before);
        assert!(v1.t_max - v1.t_min < v0.t_max - v0.t_min);
    }

    #[test]
    fn zoom_price_keeps_anchor_price() {
        let v0 = Viewport {
            width_px: 800.0,
            height_px: 600.0,
            t_min: 0,
            t_max: 1000,
            p_min: 0,
            p_max: 100,
            ..Default::default()
        };
        let anchor_y = 150.0;
        let before = v0.price_at_y(anchor_y);
        let v1 = v0.zoom_price(0.5, anchor_y);
        // toleransi 1 tick karena pembulatan ke i64.
        assert!((v1.price_at_y(anchor_y) - before).abs() <= 1);
    }

    #[test]
    fn skala_log_persen_invert_konsisten() {
        let mut v = Viewport {
            p_min: 100,
            p_max: 10_000,
            ..Default::default()
        };
        v.scale = ScaleMode::Log;
        for p in [100i64, 500, 1_000, 10_000] {
            let y = v.y_for_price(p);
            assert!((-1.0..=401.0).contains(&y), "y di luar pane: {y}");
            let back = v.price_at_y(y);
            let rel = (back - p).abs() as f64 / p as f64;
            assert!(rel < 0.01, "log roundtrip p={p} back={back}");
        }
        assert!(
            v.y_for_price(100) > v.y_for_price(1_000),
            "harga naik -> ke atas"
        );
        // persen: basis 1000
        v.scale = ScaleMode::Percent;
        v.pct_base = 1_000;
        let y0 = v.y_for_price(1_000);
        assert!(v.y_for_price(1_100) < y0, "+10% -> lebih tinggi");
        // invert membalik arah
        v.invert = true;
        assert!(v.y_for_price(1_100) > v.y_for_price(1_000));
        let back = v.price_at_y(v.y_for_price(1_100));
        assert!((back - 1_100).abs() <= 2, "roundtrip invert: {back}");
        // pan & zoom aman di mode non-linear + invert
        let w = v
            .zoom_price(1.2, 100.0)
            .pan_px(10.0, -5.0)
            .zoom_price(0.8, 200.0);
        assert!(w.p_max > w.p_min);
    }

    #[test]
    fn visible_indices_matches_window() {
        let bars = mk_bars(10);
        let r = visible_indices(&bars, bars[2].ts_millis, bars[5].ts_millis);
        assert_eq!(r, 2..6);
    }

    #[test]
    fn visible_indices_empty_when_out_of_range() {
        let bars = mk_bars(10);
        let r = visible_indices(&bars, 0, 10);
        assert!(r.is_empty());
    }

    #[test]
    fn candle_geom_orders_y_and_centers() {
        let bars = mk_bars(3);
        let v = Viewport {
            width_px: 600.0,
            height_px: 400.0,
            t_min: bars[0].ts_millis,
            t_max: bars[2].ts_millis,
            p_min: 0,
            p_max: 200,
            ..Default::default()
        };
        let g = candle_geom(&v, 10.0, 0, 1, &bars[1]);
        assert!((g.x_center - 15.0).abs() < 1e-9);
        assert!(g.body_top <= g.body_bottom);
        assert!(g.wick_top <= g.body_top);
        assert!(g.body_bottom <= g.wick_bottom);
        assert!(g.body_right > g.body_left);
    }

    #[test]
    fn fit_price_covers_visible_range() {
        let bars = mk_bars(20);
        let v = Viewport {
            width_px: 600.0,
            height_px: 400.0,
            t_min: bars[0].ts_millis,
            t_max: bars[19].ts_millis,
            p_min: -1000,
            p_max: 1000,
            ..Default::default()
        }
        .fit_price(&bars);
        assert!(v.p_min <= bars[0].low_ticks);
        assert!(v.p_max >= bars[19].high_ticks);
    }

    #[test]
    fn volume_scale_and_geom() {
        let bars = mk_bars(5);
        let max = volume_scale(&bars, 0..5);
        assert_eq!(max, 50);
        let g = volume_geom(0.0, 8.0, 25, max, 100.0);
        assert!((g.y_top - 50.0).abs() < 1e-9);
        assert!((g.y_bottom - 100.0).abs() < 1e-9);
        assert_eq!(g.x_right, 8.0);
    }
}
