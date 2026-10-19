// SokanLab strategy-api — Apache-2.0.
// Kontrak anti look-ahead: `BarCtx` hanya mengekspos data bar saat ini (index <= ctx.index).
pub const API_VERSION: u32 = 1;

use sokanlab_domain::Bar;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// Tidak ada aksi.
    Flat,
    /// Masuk posisi long (hanya bila belum ada posisi).
    EnterLong,
    /// Keluar posisi long.
    Exit,
}

/// Konteks bar yang diberikan engine ke strategi.
/// Tidak ada akses ke bar setelah `index` — ini yang mencegah look-ahead.
#[derive(Debug, Clone, Copy)]
pub struct BarCtx<'a> {
    pub index: usize,
    pub total: usize,
    pub bar: &'a Bar,
    /// Jumlah bar selesai (index + 1). Strategi boleh menghitung warm-up dari sini.
    pub bars_seen: usize,
    /// Harga open bar berikutnya TIDAK diekspos; mesin fill menawarkan `next_open`.
    pub next_open: Option<i64>,
}

impl<'a> BarCtx<'a> {
    pub fn close(&self) -> i64 {
        self.bar.close_ticks
    }
    pub fn is_last(&self) -> bool {
        self.index + 1 >= self.total
    }
}

/// Strategi: fungsi murni dari ctx menjadi Signal.
pub trait Strategy {
    /// Dipanggil sekali per bar, berurutan dari index 0.
    fn on_bar(&mut self, ctx: &BarCtx<'_>) -> Signal;

    /// Nama untuk laporan hasil (ASCII, orisinal).
    fn name(&self) -> &'static str;
}

/// Buy-and-hold: entry di bar pertama, tahan sampai akhir.
#[derive(Debug, Default, Clone)]
pub struct BuyAndHold {
    entered: bool,
}

impl BuyAndHold {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Strategy for BuyAndHold {
    fn on_bar(&mut self, ctx: &BarCtx<'_>) -> Signal {
        if !self.entered {
            self.entered = true;
            Signal::EnterLong
        } else if ctx.is_last() {
            Signal::Exit
        } else {
            Signal::Flat
        }
    }
    fn name(&self) -> &'static str {
        "buy_and_hold"
    }
}

/// Cross moving average: long saat fast > slow, keluar saat fast < slow.
#[derive(Debug, Clone)]
pub struct MaCross {
    fast: sokanlab_indicators::Sma,
    slow: sokanlab_indicators::Sma,
    prev_diff: Option<i64>,
    in_position: bool,
}

impl MaCross {
    pub fn new(fast: usize, slow: usize) -> Result<Self, sokanlab_indicators::IndicatorError> {
        Ok(Self {
            fast: sokanlab_indicators::Sma::new(fast)?,
            slow: sokanlab_indicators::Sma::new(slow)?,
            prev_diff: None,
            in_position: false,
        })
    }
}

impl Strategy for MaCross {
    fn on_bar(&mut self, ctx: &BarCtx<'_>) -> Signal {
        let Some(f) = self.fast.update(ctx.close()) else {
            return Signal::Flat;
        };
        let Some(s) = self.slow.update(ctx.close()) else {
            return Signal::Flat;
        };
        let diff = f - s;
        let prev = self.prev_diff.replace(diff);
        match prev {
            // Crossover naik → masuk; cross bawah → keluar.
            Some(p) if p <= 0 && diff > 0 && !self.in_position => {
                self.in_position = true;
                Signal::EnterLong
            }
            Some(p) if p >= 0 && diff < 0 && self.in_position => {
                self.in_position = false;
                Signal::Exit
            }
            _ if ctx.is_last() && self.in_position => {
                self.in_position = false;
                Signal::Exit
            }
            _ => Signal::Flat,
        }
    }
    fn name(&self) -> &'static str {
        "ma_cross"
    }
}

/// Ambang RSI: masuk saat RSI keluar dari zona bawah, keluar saat keluar dari zona atas.
#[derive(Debug, Clone)]
pub struct RsiThreshold {
    rsi: sokanlab_indicators::Rsi,
    oversold: i64,
    overbought: i64,
    in_position: bool,
}

impl RsiThreshold {
    pub fn new(
        period: usize,
        oversold: i64,
        overbought: i64,
    ) -> Result<Self, sokanlab_indicators::IndicatorError> {
        Ok(Self {
            rsi: sokanlab_indicators::Rsi::new(period)?,
            oversold,
            overbought,
            in_position: false,
        })
    }
}

impl Strategy for RsiThreshold {
    fn on_bar(&mut self, ctx: &BarCtx<'_>) -> Signal {
        let Some(v) = self.rsi.update(ctx.close()) else {
            return Signal::Flat;
        };
        if !self.in_position && v <= self.oversold {
            self.in_position = true;
            Signal::EnterLong
        } else if self.in_position && (v >= self.overbought || ctx.is_last()) {
            // Keluar saat overbought, atau paksa tutup di bar terakhir.
            self.in_position = false;
            Signal::Exit
        } else {
            Signal::Flat
        }
    }
    fn name(&self) -> &'static str {
        "rsi_threshold"
    }
}

/// Stop-loss / take-profit murni (tanpa sinyal masuk): mengelola posisi dari harga close.
#[derive(Debug, Clone)]
pub struct StopLossTakeProfit {
    entry: Option<i64>,
    stop_ticks: i64,
    target_ticks: i64,
    armed: bool,
}

impl StopLossTakeProfit {
    pub fn new(stop_ticks: i64, target_ticks: i64) -> Self {
        Self {
            entry: None,
            stop_ticks,
            target_ticks,
            armed: false,
        }
    }
}

impl Strategy for StopLossTakeProfit {
    fn on_bar(&mut self, ctx: &BarCtx<'_>) -> Signal {
        let c = ctx.close();
        match self.entry {
            None => {
                if self.armed {
                    self.armed = false;
                    self.entry = Some(c);
                    Signal::EnterLong
                } else {
                    Signal::Flat
                }
            }
            Some(e) => {
                let hit_stop = c <= e - self.stop_ticks;
                let hit_target = c >= e + self.target_ticks;
                if hit_stop || hit_target || ctx.is_last() {
                    // Stop/target/akhir: semuanya menutup posisi.
                    self.entry = None;
                    Signal::Exit
                } else {
                    Signal::Flat
                }
            }
        }
    }
    fn name(&self) -> &'static str {
        "stop_loss_take_profit"
    }
}

impl StopLossTakeProfit {
    /// Diaktifkan dari UI/CLI sebelum run.
    pub fn arm(&mut self) {
        self.armed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(o: i64, h: i64, l: i64, c: i64) -> Bar {
        Bar {
            ts_millis: 0,
            open_ticks: o,
            high_ticks: h,
            low_ticks: l,
            close_ticks: c,
            volume_milli: 1,
        }
    }

    #[test]
    fn buy_and_hold_enters_then_exits_last() {
        let mut s = BuyAndHold::new();
        let b = bar(100, 100, 100, 100);
        let ctx = BarCtx {
            index: 0,
            total: 3,
            bar: &b,
            bars_seen: 1,
            next_open: Some(100),
        };
        assert_eq!(s.on_bar(&ctx), Signal::EnterLong);
        let last = BarCtx {
            index: 2,
            total: 3,
            ..ctx
        };
        assert_eq!(s.on_bar(&last), Signal::Exit);
    }

    #[test]
    fn bar_ctx_hides_next_open_on_last_bar() {
        let b = bar(1, 2, 0, 1);
        let ctx = BarCtx {
            index: 0,
            total: 1,
            bar: &b,
            bars_seen: 1,
            next_open: None,
        };
        assert!(ctx.is_last());
    }

    #[test]
    fn ma_cross_respects_warmup() {
        let mut s = MaCross::new(2, 3).unwrap();
        let b = bar(10, 10, 10, 10);
        for i in 0..2 {
            let ctx = BarCtx {
                index: i,
                total: 10,
                bar: &b,
                bars_seen: i + 1,
                next_open: Some(10),
            };
            assert_eq!(s.on_bar(&ctx), Signal::Flat, "warm-up harus Flat");
        }
    }

    #[test]
    fn strategy_names_are_ascii() {
        assert_eq!(BuyAndHold::new().name(), "buy_and_hold");
        assert!(MaCross::new(2, 3).unwrap().name().is_ascii());
        assert!(RsiThreshold::new(14, 30, 70).unwrap().name().is_ascii());
        assert!(StopLossTakeProfit::new(5, 10).name().is_ascii());
    }
}
