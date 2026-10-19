// SokanLab indicators — Apache-2.0.
// Formula publik (SMA/EMA/RSI Wilder). Versi inkremental untuk event loop backtest:
// state diperbarui satu bar pada satu waktu, jadi tidak mungkin melihat bar mendatang.
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndicatorError {
    ZeroPeriod,
}

impl std::fmt::Display for IndicatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroPeriod => write!(f, "periode harus > 0"),
        }
    }
}

impl std::error::Error for IndicatorError {}

// SMA inkremental. Warm-up: `None` sampai `period` bar terkumpul.
#[derive(Debug, Clone)]
pub struct Sma {
    period: usize,
    window: VecDeque<i64>,
    sum: i64,
}

impl Sma {
    pub fn new(period: usize) -> Result<Self, IndicatorError> {
        if period == 0 {
            return Err(IndicatorError::ZeroPeriod);
        }
        Ok(Self {
            period,
            window: VecDeque::with_capacity(period),
            sum: 0,
        })
    }

    /// Masukkan satu nilai; `None` selama warm-up.
    pub fn update(&mut self, value: i64) -> Option<i64> {
        self.window.push_back(value);
        self.sum += value;
        if self.window.len() > self.period {
            self.sum -= self.window.pop_front().unwrap_or(0);
        }
        if self.window.len() == self.period {
            Some(self.sum / self.period as i64)
        } else {
            None
        }
    }

    pub fn value(&self) -> Option<i64> {
        (self.window.len() == self.period).then(|| self.sum / self.period as i64)
    }

    pub fn ready(&self) -> bool {
        self.window.len() == self.period
    }
}

// EMA inkremental, di-seed dengan SMA `period` pertama (konvensi umum).
#[derive(Debug, Clone)]
pub struct Ema {
    k_num: i64, // k = k_num / k_den
    k_den: i64,
    warm: Sma,
    prev: Option<i64>,
}

impl Ema {
    pub fn new(period: usize) -> Result<Self, IndicatorError> {
        if period == 0 {
            return Err(IndicatorError::ZeroPeriod);
        }
        Ok(Self {
            // k = 2/(period+1), disimpan sebagai pecahan agar deterministik.
            k_num: 2,
            k_den: period as i64 + 1,
            warm: Sma::new(period)?,
            prev: None,
        })
    }

    pub fn update(&mut self, value: i64) -> Option<i64> {
        match self.warm.update(value) {
            None => None,
            Some(seed) => {
                match self.prev {
                    // Bar seed: EMA = SMA (konvensi umum), belum di-blend.
                    None => {
                        self.prev = Some(seed);
                        Some(seed)
                    }
                    Some(prev) => {
                        // ema = prev + k*(value - prev), k = k_num/k_den:
                        // bobot k pada bar terbaru, (1-k) pada EMA sebelumnya.
                        let ema = prev + (value - prev) * self.k_num / self.k_den;
                        self.prev = Some(ema);
                        Some(ema)
                    }
                }
            }
        }
    }

    pub fn value(&self) -> Option<i64> {
        self.prev
    }
}

/// RSI Wilder inkremental: rata-rata gain/loss dengan smoothing 1/period.
/// Warm-up: butuh `period` perubahan harga, jadi `None` sampai saat itu.
/// Rata-rata disimpan dalam milli (×1000) supaya truncation integer tidak
/// membekukan RSI (pada data berlangkah 0/±1 tick, `avg_loss=(0*13+1)/14` = 0
/// selamanya tanpa skala ini, sehingga RSI terjebak di 100).
#[derive(Debug, Clone)]
pub struct Rsi {
    period: usize,
    prev_close: Option<i64>,
    diffs: usize,
    sum_gain_milli: i64,
    sum_loss_milli: i64,
    avg_gain_milli: i64,
    avg_loss_milli: i64,
    ready: bool,
}

/// Skala rata-rata RSI: 1000 = 1 tick; presisi 1/1000 tick.
const RSI_SCALE: i64 = 1000;

impl Rsi {
    pub fn new(period: usize) -> Result<Self, IndicatorError> {
        if period == 0 {
            return Err(IndicatorError::ZeroPeriod);
        }
        Ok(Self {
            period,
            prev_close: None,
            diffs: 0,
            sum_gain_milli: 0,
            sum_loss_milli: 0,
            avg_gain_milli: 0,
            avg_loss_milli: 0,
            ready: false,
        })
    }

    pub fn update(&mut self, close: i64) -> Option<i64> {
        let Some(prev) = self.prev_close else {
            self.prev_close = Some(close);
            return None;
        };
        self.prev_close = Some(close);
        let diff = close - prev;
        let gain = diff.max(0) * RSI_SCALE;
        let loss = (-diff).max(0) * RSI_SCALE;

        if !self.ready {
            self.sum_gain_milli += gain;
            self.sum_loss_milli += loss;
            self.diffs += 1;
            if self.diffs < self.period {
                return None;
            }
            let n = self.period as i64;
            self.avg_gain_milli = self.sum_gain_milli / n;
            self.avg_loss_milli = self.sum_loss_milli / n;
            self.ready = true;
            return Some(rsi_from(self.avg_gain_milli, self.avg_loss_milli));
        }

        let n = self.period as i64;
        self.avg_gain_milli = (self.avg_gain_milli * (n - 1) + gain) / n;
        self.avg_loss_milli = (self.avg_loss_milli * (n - 1) + loss) / n;
        Some(rsi_from(self.avg_gain_milli, self.avg_loss_milli))
    }

    pub fn value(&self) -> Option<i64> {
        self.ready
            .then(|| rsi_from(self.avg_gain_milli, self.avg_loss_milli))
    }
}

/// RSI dalam skala 0..=100 dari rata-rata gain/loss (integer, deterministik).
fn rsi_from(avg_gain: i64, avg_loss: i64) -> i64 {
    if avg_loss == 0 {
        return 100;
    }
    // rs = avg_gain/avg_loss; rsi = 100 - 100/(1+rs), ditulis agar bebas pembulatan ganda.
    let rs = avg_gain * 100 / avg_loss;
    (100 - 10_000 / (100 + rs)).clamp(0, 100)
}

/// Batch: jalankan indikator pada seluruh seri (dipakai chart & ekspor).
pub fn sma_series(values: &[i64], period: usize) -> Vec<Option<i64>> {
    let mut s = Sma::new(period).expect("period > 0");
    values.iter().map(|v| s.update(*v)).collect()
}

pub fn ema_series(values: &[i64], period: usize) -> Vec<Option<i64>> {
    let mut e = Ema::new(period).expect("period > 0");
    values.iter().map(|v| e.update(*v)).collect()
}

/// MACD: EMA(fast) − EMA(slow) atas harga dalam skala milli (×1000) plus garis
/// sinyal (EMA sederhana dari MACD, di-seed nilai MACD pertama). Semua nilai
/// dalam milli supaya presisi lintas tick terjaga (pane MACD rentangnya kecil).
/// `slow` sebaiknya > `fast` (dijamin pemanggil).
pub fn macd_series(
    closes: &[i64],
    fast: usize,
    slow: usize,
    signal: usize,
) -> (Vec<Option<i64>>, Vec<Option<i64>>) {
    assert!(fast > 0 && slow > 0 && signal > 0, "period MACD harus > 0");
    let milli: Vec<i64> = closes.iter().map(|c| c.saturating_mul(1000)).collect();
    let ef = ema_series(&milli, fast);
    let es = ema_series(&milli, slow);
    let macd: Vec<Option<i64>> = ef
        .iter()
        .zip(es.iter())
        .map(|(a, b)| match (a, b) {
            (Some(x), Some(y)) => Some(x - y),
            _ => None,
        })
        .collect();
    let k_num: i64 = 2;
    let k_den: i64 = signal as i64 + 1;
    let mut sig: Vec<Option<i64>> = Vec::with_capacity(macd.len());
    let mut prev: Option<i64> = None;
    for m in &macd {
        match (m, prev) {
            (None, _) => sig.push(None),
            (Some(v), None) => {
                prev = Some(*v);
                sig.push(Some(*v));
            }
            (Some(v), Some(p)) => {
                let next = v + (p - v) * k_num / k_den;
                prev = Some(next);
                sig.push(Some(next));
            }
        }
    }
    (macd, sig)
}

/// VWAP jangkar (anchored): Sigma(harga*volume)/Sigma(volume) dari bar pertama;
/// `None` selama volume kumulatif masih 0. Dipakai overlay chart.
pub fn vwap_series(closes: &[i64], volumes_milli: &[i64]) -> Vec<Option<i64>> {
    let mut out = Vec::with_capacity(closes.len());
    let mut pv: i128 = 0;
    let mut vol: i128 = 0;
    for (i, c) in closes.iter().enumerate() {
        let v = volumes_milli.get(i).copied().unwrap_or(0).max(0) as i128;
        pv += *c as i128 * v;
        vol += v;
        if vol > 0 {
            let q = (pv / vol).clamp(i64::MIN as i128, i64::MAX as i128);
            out.push(Some(q as i64));
        } else {
            out.push(None);
        }
    }
    out
}

pub fn rsi_series(closes: &[i64], period: usize) -> Vec<Option<i64>> {
    let mut r = Rsi::new(period).expect("period > 0");
    closes.iter().map(|v| r.update(*v)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macd_siap_setelah_ema_slow_dan_negatif_saat_turun() {
        // Harga konstan → MACD = 0 tepat setelah EMA slow (26) siap.
        let flat = vec![100i64; 40];
        let (m, s) = macd_series(&flat, 12, 26, 9);
        assert!(m[24].is_none(), "EMA slow (26) belum siap di index 24");
        assert_eq!(m[25], Some(0));
        assert_eq!(s[25], Some(0));
        // Titik acuan eksak: ramp naik, fast=3, slow=6 (milli ×1000).
        // Lag EMA pada ramp = (n-1)/2 step: fast 1 step, slow 2,5 step → 1500.
        let up: Vec<i64> = (0..30).map(|i| 100 + i).collect();
        let (m2, s2) = macd_series(&up, 3, 6, 3);
        assert_eq!(m2[5], Some(1500), "ef=104000, es=102500 → 1500");
        assert_eq!(s2[5], Some(1500), "sinyal di-seed nilai MACD pertama");
        assert_eq!(m2.last().unwrap(), &Some(1500), "MACD stabil di 1,5 step");
        assert_eq!(s2.last().unwrap(), &Some(1500), "sinyal konvergen ke MACD");
        // Ramp turun → MACD negatif (simetris: -1500).
        let down: Vec<i64> = (0..30).map(|i| 100 - i).collect();
        let (m3, _) = macd_series(&down, 3, 6, 3);
        assert_eq!(m3.last().unwrap(), &Some(-1500));
    }

    #[test]
    fn sma_matches_manual() {
        let v = [1, 2, 3, 4];
        assert_eq!(sma_series(&v, 2), vec![None, Some(1), Some(2), Some(3)]);
    }

    #[test]
    fn sma_zero_period_rejected() {
        assert!(Sma::new(0).is_err());
    }

    #[test]
    fn sma_warmup_then_ready() {
        let mut s = Sma::new(3).unwrap();
        assert_eq!(s.update(10), None);
        assert_eq!(s.update(20), None);
        assert!(!s.ready());
        assert_eq!(s.update(30), Some(20));
        assert!(s.ready());
    }

    #[test]
    fn sma_rolling_drops_oldest() {
        let mut s = Sma::new(2).unwrap();
        s.update(1);
        s.update(2);
        assert_eq!(s.update(3), Some(2)); // (2+3)/2
    }

    #[test]
    fn ema_matches_naive_definition() {
        let v = [10, 12, 14];
        let out = ema_series(&v, 2);
        assert_eq!(out[0], None);
        assert_eq!(out[1], Some(11));
        // ema_2 = 11 + 2/3 * (14 - 11) = 13: bobot α=2/3 pada nilai terbaru.
        assert_eq!(out[2], Some(13));
    }

    #[test]
    fn ema_zero_period_rejected() {
        assert!(Ema::new(0).is_err());
    }

    #[test]
    fn rsi_all_gains_is_100() {
        let v: Vec<i64> = (1..=20).collect();
        let out = rsi_series(&v, 14);
        assert_eq!(out[13], None);
        assert_eq!(out[14], Some(100));
    }

    #[test]
    fn rsi_zero_period_rejected() {
        assert!(Rsi::new(0).is_err());
    }

    /// Regresi: data berlangkah 0/±1 tick tidak boleh membekukan RSI di 100.
    #[test]
    fn rsi_not_stuck_on_one_tick_steps() {
        let mut v = 10_000i64;
        let mut closes = vec![];
        // Fase turun: perubahan 0/−1 tick per bar.
        for i in 0..40 {
            if i % 2 == 1 {
                v -= 1;
            }
            closes.push(v);
        }
        let rsi = rsi_series(&closes, 14).last().unwrap().unwrap();
        assert!(rsi < 40, "RSI fase turun harus rendah, dapat {rsi}");

        // Fase naik: perubahan 0/+1 tick per bar.
        for i in 0..40 {
            if i % 2 == 1 {
                v += 1;
            }
            closes.push(v);
        }
        let rsi = rsi_series(&closes, 14).last().unwrap().unwrap();
        assert!(rsi > 60, "RSI fase naik harus tinggi, dapat {rsi}");
        assert!(rsi <= 100);
    }
}
