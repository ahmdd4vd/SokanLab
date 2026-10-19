// SokanLab backtest — Apache-2.0.
// Engine event-loop deterministik (dipanggil di luar UI thread).
// Anti look-ahead: keputusan pada bar t dieksekusi paling cepat di open bar t+1.
// Seluruh uang dalam tick integer (tidak ada f64 dalam pembukuan).
// Gap bar tidak diisi: indikator & eksekusi memakai bar apa adanya (berbasis indeks
// bar, bukan jarak waktu), sehingga dataset dengan bar hilang tetap deterministik.
use sokanlab_domain::Bar;
use sokanlab_strategy_api::{BarCtx, Signal, Strategy};

/// 1000 milli-unit = 1 unit penuh.
pub const MILLI: i64 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Fill,
    StopLoss,
    TakeProfit,
    EndOfData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedgerEvent {
    pub bar: usize,
    pub ts_millis: i64,
    pub kind: EventKind,
    pub side: Side,
    /// Harga eksekusi setelah slippage (ticks).
    pub price_ticks: i64,
    /// Harga referensi sebelum slippage (ticks).
    pub reference_ticks: i64,
    /// Qty dalam milli-unit, negatif untuk sell.
    pub qty_milli: i64,
    pub fee_ticks: i64,
    /// Cash setelah event (ticks).
    pub cash_after_ticks: i64,
    /// Posisi setelah event (milli-unit).
    pub qty_after_milli: i64,
}

/// Model biaya. Default sesuai plan: 10 bps (0,10%) + 1 tick slippage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CostModel {
    /// Fee per fill dalam basis points notional (10 = 0,10%).
    pub fee_bps: i64,
    /// Slippage satu arah dalam ticks (buy: +slippage, sell: -slippage).
    pub slippage_ticks: i64,
}

impl Default for CostModel {
    fn default() -> Self {
        Self {
            fee_bps: 10,
            slippage_ticks: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopLoss {
    /// Jarak stop dari harga entry (ticks, positif).
    pub stop_ticks: i64,
    /// Jarak target dari harga entry (ticks, positif).
    pub take_profit_ticks: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquityPoint {
    pub bar: usize,
    pub ts_millis: i64,
    /// Equity = cash + nilai posisi di harga close (ticks).
    pub equity_ticks: i64,
    pub cash_ticks: i64,
    pub qty_milli: i64,
    pub close_ticks: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Trade {
    pub entry_bar: usize,
    pub exit_bar: usize,
    pub entry_ticks: i64,
    pub exit_ticks: i64,
    pub qty_milli: i64,
    /// PnL bersih (ticks), setelah fee entry dan exit.
    pub pnl_ticks: i64,
    pub exit_reason: EventKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Metrics {
    pub bars: usize,
    pub trades: usize,
    pub wins: usize,
    pub losses: i64,
    /// Total return, basis points (10000 = +100%).
    pub total_return_bps: i64,
    /// Drawdown maksimum, basis points (non-negatif).
    pub max_drawdown_bps: i64,
    pub fees_total_ticks: i64,
    /// Porsi bar yang punya posisi, basis points.
    pub exposure_bps: i64,
    pub bars_in_market: usize,
    /// PnL rata-rata per trade (ticks).
    pub avg_pnl_ticks: i64,
    /// Sharpe per-bar (mean/std return per bar); 1000 = 1,0; belum annualized.
    pub sharpe_milli: i64,
}

impl Metrics {
    /// Win rate basis points (10000 = 100%).
    pub fn win_rate_bps(&self) -> i64 {
        if self.trades == 0 {
            return 0;
        }
        self.wins as i64 * 10_000 / self.trades as i64
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacktestResult {
    pub strategy: String,
    pub initial_cash_ticks: i64,
    pub final_equity_ticks: i64,
    pub ledger: Vec<LedgerEvent>,
    pub equity: Vec<EquityPoint>,
    pub trades: Vec<Trade>,
    pub metrics: Metrics,
    /// Asumsi eksekusi (ditampilkan di laporan, bukan disembunyikan).
    pub assumptions: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub initial_cash_ticks: i64,
    /// Qty per entry, milli-unit.
    pub qty_milli: i64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            initial_cash_ticks: 1_000_000,
            qty_milli: 1_000,
        }
    }
}

/// Nilai posisi dalam ticks (floor, deterministik).
fn notional_ticks(price_ticks: i64, qty_milli: i64) -> i64 {
    price_ticks * qty_milli / MILLI
}

fn fee_ticks(price_ticks: i64, qty_milli: i64, cost: CostModel) -> i64 {
    notional_ticks(price_ticks, qty_milli) * cost.fee_bps / 10_000
}

fn exec_price(reference_ticks: i64, side: Side, cost: CostModel) -> i64 {
    match side {
        Side::Buy => reference_ticks + cost.slippage_ticks,
        Side::Sell => reference_ticks - cost.slippage_ticks,
    }
}

/// Jalankan backtest deterministik atas `bars`.
pub fn run(
    bars: &[Bar],
    strategy: &mut dyn Strategy,
    config: Config,
    cost: CostModel,
    stops: Option<StopLoss>,
) -> BacktestResult {
    assert!(!bars.is_empty(), "butuh minimal satu bar");
    assert!(config.qty_milli > 0, "qty harus positif");
    let name = strategy.name();

    let mut ledger: Vec<LedgerEvent> = Vec::new();
    let mut equity: Vec<EquityPoint> = Vec::with_capacity(bars.len());
    let mut trades: Vec<Trade> = Vec::new();

    let mut st = EngineState {
        cash: config.initial_cash_ticks,
        qty_milli: 0,
        fees_total: 0,
        entry_price: 0,
        entry_bar: 0,
        entry_fee: 0,
        open: false,
    };
    let mut bars_in_market = 0usize;
    // Sinyal dari bar t hanya dieksekusi di open bar t+1 (anti look-ahead).
    let mut pending: Option<Signal> = None;

    for idx in 0..bars.len() {
        let bar = bars[idx];

        // (1) Eksekusi sinyal bar sebelumnya di OPEN bar ini.
        if let Some(sig) = pending.take() {
            match sig {
                Signal::EnterLong if !st.open => {
                    st.enter(
                        idx,
                        bar.ts_millis,
                        bar.open_ticks,
                        config,
                        cost,
                        &mut ledger,
                    );
                }
                Signal::Exit if st.open => {
                    st.exit(
                        bars,
                        idx,
                        EventKind::Fill,
                        bar.open_ticks,
                        config,
                        cost,
                        &mut ledger,
                        &mut trades,
                    );
                }
                _ => {}
            }
        }

        // (2) Stop/target intrabar, aturan konservatif: bila low dan high
        //     sama-sama menyentuh batas, diasumsikan stop lebih dulu.
        //     (Posisi yang baru terisi di open bar ini ikut diperiksa.)
        if st.open {
            if let Some(s) = stops {
                let stop = st.entry_price - s.stop_ticks;
                let target = st.entry_price + s.take_profit_ticks;
                if bar.low_ticks <= stop {
                    st.exit(
                        bars,
                        idx,
                        EventKind::StopLoss,
                        stop,
                        config,
                        cost,
                        &mut ledger,
                        &mut trades,
                    );
                } else if bar.high_ticks >= target {
                    st.exit(
                        bars,
                        idx,
                        EventKind::TakeProfit,
                        target,
                        config,
                        cost,
                        &mut ledger,
                        &mut trades,
                    );
                }
            }
        }

        // (3) Keputusan bar ini disimpan; hanya berguna bila masih ada bar berikutnya.
        let next = bars.get(idx + 1);
        let ctx = BarCtx {
            index: idx,
            total: bars.len(),
            bar: &bar,
            bars_seen: idx + 1,
            next_open: next.map(|b| b.open_ticks),
        };
        if next.is_some() {
            pending = Some(strategy.on_bar(&ctx));
        }

        // (4) Mark-to-market di close bar (posisi sesuai isi bar ini).
        if st.open {
            bars_in_market += 1;
        }
        equity.push(EquityPoint {
            bar: idx,
            ts_millis: bar.ts_millis,
            equity_ticks: st.cash + notional_ticks(bar.close_ticks, st.qty_milli),
            cash_ticks: st.cash,
            qty_milli: st.qty_milli,
            close_ticks: bar.close_ticks,
        });
    }

    // Posisi tersisa ditutup di close bar terakhir agar PnL selalu terukur.
    if st.open {
        let last = bars.len() - 1;
        st.exit(
            bars,
            last,
            EventKind::EndOfData,
            bars[last].close_ticks,
            config,
            cost,
            &mut ledger,
            &mut trades,
        );
        if let Some(p) = equity.last_mut() {
            p.cash_ticks = st.cash;
            p.qty_milli = st.qty_milli;
            p.equity_ticks = st.cash + notional_ticks(p.close_ticks, st.qty_milli);
        }
    }

    let final_equity_ticks = equity.last().map(|p| p.equity_ticks).unwrap_or(st.cash);
    let total_return_bps = if config.initial_cash_ticks == 0 {
        0
    } else {
        (final_equity_ticks - config.initial_cash_ticks) * 10_000 / config.initial_cash_ticks
    };
    let peak_dd = max_drawdown_bps(&equity);
    let wins = trades.iter().filter(|t| t.pnl_ticks > 0).count();
    let losses = trades.iter().filter(|t| t.pnl_ticks < 0).count() as i64;
    let avg_pnl = if trades.is_empty() {
        0
    } else {
        trades.iter().map(|t| t.pnl_ticks).sum::<i64>() / trades.len() as i64
    };
    let exposure_bps = if equity.is_empty() {
        0
    } else {
        bars_in_market as i64 * 10_000 / equity.len() as i64
    };
    let sharpe = sharpe_milli(&equity);

    let assumptions = format!(
        "isi pada open bar setelah sinyal; fee {} bps; slippage {} tick; equity mark-to-market di close; stop diutamakan bila stop & target tersentuh bar sama; qty {} milli-unit; fee dihitung floor (ticks)",
        cost.fee_bps, cost.slippage_ticks, config.qty_milli
    );

    let bars_count = equity.len();
    let trades_count = trades.len();

    BacktestResult {
        strategy: name.to_string(),
        initial_cash_ticks: config.initial_cash_ticks,
        final_equity_ticks,
        ledger,
        equity,
        trades,
        metrics: Metrics {
            bars: bars_count,
            trades: trades_count,
            wins,
            losses,
            total_return_bps,
            max_drawdown_bps: peak_dd,
            fees_total_ticks: st.fees_total,
            exposure_bps,
            bars_in_market,
            avg_pnl_ticks: avg_pnl,
            sharpe_milli: sharpe,
        },
        assumptions,
    }
}

/// State posisiEngine; dipisah agar borrow eksplisit dan mudah diuji.
struct EngineState {
    cash: i64,
    qty_milli: i64,
    fees_total: i64,
    entry_price: i64,
    entry_bar: usize,
    entry_fee: i64,
    open: bool,
}

impl EngineState {
    #[allow(clippy::too_many_arguments)]
    fn enter(
        &mut self,
        fill_idx: usize,
        ts_millis: i64,
        reference_ticks: i64,
        config: Config,
        cost: CostModel,
        ledger: &mut Vec<LedgerEvent>,
    ) {
        let px = exec_price(reference_ticks, Side::Buy, cost);
        let fee = fee_ticks(px, config.qty_milli, cost);
        self.cash -= notional_ticks(px, config.qty_milli) + fee;
        self.qty_milli += config.qty_milli;
        self.fees_total += fee;
        self.entry_price = px;
        self.entry_bar = fill_idx;
        self.entry_fee = fee;
        self.open = true;
        ledger.push(LedgerEvent {
            bar: fill_idx,
            ts_millis,
            kind: EventKind::Fill,
            side: Side::Buy,
            price_ticks: px,
            reference_ticks,
            qty_milli: config.qty_milli,
            fee_ticks: fee,
            cash_after_ticks: self.cash,
            qty_after_milli: self.qty_milli,
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn exit(
        &mut self,
        bars: &[Bar],
        fill_idx: usize,
        kind: EventKind,
        reference: i64,
        config: Config,
        cost: CostModel,
        ledger: &mut Vec<LedgerEvent>,
        trades: &mut Vec<Trade>,
    ) {
        let px = exec_price(reference, Side::Sell, cost);
        let fee = fee_ticks(px, config.qty_milli, cost);
        self.cash += notional_ticks(px, config.qty_milli) - fee;
        self.qty_milli -= config.qty_milli;
        self.fees_total += fee;
        ledger.push(LedgerEvent {
            bar: fill_idx,
            ts_millis: bars[fill_idx].ts_millis,
            kind,
            side: Side::Sell,
            price_ticks: px,
            reference_ticks: reference,
            qty_milli: -config.qty_milli,
            fee_ticks: fee,
            cash_after_ticks: self.cash,
            qty_after_milli: self.qty_milli,
        });
        trades.push(Trade {
            entry_bar: self.entry_bar,
            exit_bar: fill_idx,
            entry_ticks: self.entry_price,
            exit_ticks: px,
            qty_milli: config.qty_milli,
            pnl_ticks: notional_ticks(px - self.entry_price, config.qty_milli)
                - self.entry_fee
                - fee,
            exit_reason: kind,
        });
        self.open = false;
    }
}

fn max_drawdown_bps(equity: &[EquityPoint]) -> i64 {
    let mut peak = i64::MIN;
    let mut dd = 0i64;
    for p in equity {
        peak = peak.max(p.equity_ticks);
        if peak > 0 && p.equity_ticks < peak {
            dd = dd.max((peak - p.equity_ticks) * 10_000 / peak);
        }
    }
    dd
}

/// Return per bar dalam mili-bps (1000 = 0,1 bps); 0 bila equity referensi non-positif.
fn ret_milli_bps(prev: i64, cur: i64) -> i64 {
    if prev <= 0 {
        return 0;
    }
    (cur - prev) * 10_000_000 / prev
}

/// Sharpe per-bar dari kurva equity: mean/std return per bar (×1000, belum annualized).
/// Deterministik: semua aritmetika integer, akar via isqrt.
fn sharpe_milli(equity: &[EquityPoint]) -> i64 {
    if equity.len() < 2 {
        return 0;
    }
    let n = equity.len() as i128 - 1;
    let mut sum: i128 = 0;
    for w in equity.windows(2) {
        sum += ret_milli_bps(w[0].equity_ticks, w[1].equity_ticks) as i128;
    }
    let mean = sum / n;
    let mut sq: i128 = 0;
    for w in equity.windows(2) {
        let d = ret_milli_bps(w[0].equity_ticks, w[1].equity_ticks) as i128 - mean;
        sq += d * d;
    }
    let var = sq / n; // varians populasi
    if var <= 0 {
        return 0;
    }
    let sd = (var as u128).isqrt() as i128;
    if sd == 0 {
        return 0;
    }
    (mean * 1000 / sd) as i64
}

impl BacktestResult {
    /// Total return basis points (10000 = +100%).
    pub fn total_return_ticks_bps(&self) -> i64 {
        self.metrics.total_return_bps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sokanlab_strategy_api::{BuyAndHold, MaCross, StopLossTakeProfit};

    fn b(ts: i64, o: i64, h: i64, l: i64, c: i64) -> Bar {
        Bar {
            ts_millis: ts,
            open_ticks: o,
            high_ticks: h,
            low_ticks: l,
            close_ticks: c,
            volume_milli: 10,
        }
    }

    fn flat(n: usize, px: i64) -> Vec<Bar> {
        (0..n).map(|i| b(i as i64, px, px, px, px)).collect()
    }

    fn zero_cost() -> CostModel {
        CostModel {
            fee_bps: 0,
            slippage_ticks: 0,
        }
    }

    #[test]
    fn single_bar_produces_no_fill() {
        let bars = flat(1, 100);
        let mut s = BuyAndHold::new();
        let r = run(&bars, &mut s, Config::default(), zero_cost(), None);
        assert!(r.ledger.is_empty());
        assert_eq!(r.metrics.trades, 0);
        assert_eq!(r.final_equity_ticks, r.initial_cash_ticks);
    }

    #[test]
    fn fill_lands_on_next_bar_open() {
        let bars = vec![b(0, 100, 101, 99, 100), b(1, 110, 111, 109, 110)];
        let mut s = BuyAndHold::new();
        let r = run(&bars, &mut s, Config::default(), zero_cost(), None);
        let entry = r.ledger.first().expect("ada entry");
        assert_eq!(entry.bar, 1);
        assert_eq!(entry.price_ticks, 110);
    }

    #[test]
    fn never_fills_on_signal_bar() {
        let bars = vec![b(0, 100, 101, 99, 100), b(1, 90, 91, 89, 90)];
        let mut s = BuyAndHold::new();
        let r = run(&bars, &mut s, Config::default(), zero_cost(), None);
        assert!(r.ledger.iter().all(|e| e.bar >= 1));
        // Sinyal di bar 0 → fill di open bar 1 = 90, bukan harga bar 0.
        assert_eq!(r.ledger[0].price_ticks, 90);
    }

    #[test]
    fn slippage_is_adverse_both_ways() {
        let bars = vec![b(0, 100, 101, 99, 100), b(1, 100, 101, 99, 100)];
        let mut s = BuyAndHold::new();
        let r = run(
            &bars,
            &mut s,
            Config::default(),
            CostModel {
                fee_bps: 0,
                slippage_ticks: 1,
            },
            None,
        );
        let buy = r.ledger.iter().find(|e| e.side == Side::Buy).unwrap();
        let sell = r.ledger.iter().find(|e| e.side == Side::Sell).unwrap();
        assert_eq!(buy.price_ticks, 101);
        assert_eq!(sell.price_ticks, 99);
        assert_eq!(sell.reference_ticks, 100);
    }

    #[test]
    fn fee_is_charged_and_reduces_equity() {
        let bars = flat(4, 100);
        let mut s = BuyAndHold::new();
        // Cash kecil supaya selisih fee tidak hilang saat pembulatan bps.
        let cfg = Config {
            initial_cash_ticks: 10_000,
            qty_milli: 1_000,
        };
        let r = run(
            &bars,
            &mut s,
            cfg,
            CostModel {
                fee_bps: 100,
                slippage_ticks: 0,
            },
            None,
        );
        assert!(r.metrics.fees_total_ticks > 0);
        assert!(r.final_equity_ticks < r.initial_cash_ticks);
        assert!(r.total_return_ticks_bps() < 0);
    }

    #[test]
    fn zero_fee_flat_market_is_break_even() {
        let bars = flat(6, 100);
        let mut s = BuyAndHold::new();
        let r = run(&bars, &mut s, Config::default(), zero_cost(), None);
        assert_eq!(r.total_return_ticks_bps(), 0);
    }

    #[test]
    fn stop_and_target_same_bar_prefers_stop() {
        let bars = vec![
            b(0, 100, 101, 99, 100),
            b(1, 100, 101, 99, 100),
            b(2, 100, 120, 80, 100),
        ];
        let mut s = BuyAndHold::new();
        let r = run(
            &bars,
            &mut s,
            Config::default(),
            zero_cost(),
            Some(StopLoss {
                stop_ticks: 10,
                take_profit_ticks: 10,
            }),
        );
        assert!(r.ledger.iter().any(|e| e.kind == EventKind::StopLoss));
        assert!(!r.ledger.iter().any(|e| e.kind == EventKind::TakeProfit));
    }

    #[test]
    fn take_profit_fires_when_only_high_touches() {
        let bars = vec![
            b(0, 100, 101, 99, 100),
            b(1, 100, 101, 99, 100),
            b(2, 100, 130, 99, 120),
        ];
        let mut s = BuyAndHold::new();
        let r = run(
            &bars,
            &mut s,
            Config::default(),
            zero_cost(),
            Some(StopLoss {
                stop_ticks: 10,
                take_profit_ticks: 10,
            }),
        );
        assert!(r.ledger.iter().any(|e| e.kind == EventKind::TakeProfit));
    }

    #[test]
    fn equity_reconciles_cash_plus_position() {
        let bars: Vec<Bar> = (0..10)
            .map(|i| b(i, 100 + i, 105 + i, 95 + i, 100 + i))
            .collect();
        let mut s = BuyAndHold::new();
        let r = run(&bars, &mut s, Config::default(), CostModel::default(), None);
        for p in &r.equity {
            assert_eq!(
                p.equity_ticks,
                p.cash_ticks + p.qty_milli * p.close_ticks / MILLI
            );
            assert!(p.qty_milli >= 0);
        }
    }

    #[test]
    fn sum_of_trade_pnl_equals_equity_change() {
        let bars: Vec<Bar> = (0..20)
            .map(|i| b(i as i64, 100, 110, 90, 100 + (i % 5) as i64))
            .collect();
        let mut s = BuyAndHold::new();
        let r = run(&bars, &mut s, Config::default(), CostModel::default(), None);
        let pnl: i64 = r.trades.iter().map(|t| t.pnl_ticks).sum();
        assert_eq!(pnl, r.final_equity_ticks - r.initial_cash_ticks);
    }

    #[test]
    fn every_fill_has_matching_exit() {
        let closes = [
            90, 96, 102, 108, 114, 108, 102, 96, 90, 84, 90, 96, 102, 108, 114,
        ];
        let bars: Vec<Bar> = closes
            .iter()
            .enumerate()
            .map(|(i, c)| b(i as i64, *c, c + 1, c - 1, *c))
            .collect();
        let mut s = MaCross::new(2, 3).unwrap();
        let r = run(&bars, &mut s, Config::default(), CostModel::default(), None);
        let buys = r.ledger.iter().filter(|e| e.side == Side::Buy).count();
        let sells = r.ledger.iter().filter(|e| e.side == Side::Sell).count();
        assert!(buys > 0, "pola naik-turun harus memicu entry");
        // Setiap entry harus punya exit (posisi ditutup di akhir data).
        assert_eq!(buys, sells);
        assert!(r.final_equity_ticks > 0);
    }

    #[test]
    fn two_runs_identical() {
        let bars: Vec<Bar> = (0..50)
            .map(|i| b(i as i64, 100, 110, 90, 100 + (i % 7) as i64))
            .collect();
        let cfg = Config::default();
        let cost = CostModel::default();
        let mut s1 = BuyAndHold::new();
        let mut s2 = BuyAndHold::new();
        assert_eq!(
            run(&bars, &mut s1, cfg, cost, None),
            run(&bars, &mut s2, cfg, cost, None)
        );
    }

    #[test]
    fn empty_bars_panics() {
        let mut s = BuyAndHold::new();
        let got = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run(&[], &mut s, Config::default(), CostModel::default(), None)
        }));
        assert!(got.is_err());
    }

    #[test]
    fn drawdown_and_exposure_within_bounds() {
        let bars: Vec<Bar> = (0..30)
            .map(|i| {
                let c = if i < 15 { 120 } else { 80 };
                b(i as i64, 100, c + 2, c - 2, c)
            })
            .collect();
        let mut s = BuyAndHold::new();
        // Cash kecil (1 unit = 100 tick) supaya drawdown terlihat dalam bps.
        let cfg = Config {
            initial_cash_ticks: 200,
            qty_milli: 1_000,
        };
        let r = run(&bars, &mut s, cfg, zero_cost(), None);
        assert!(
            r.metrics.max_drawdown_bps > 0,
            "turun 120→80 harus drawdown"
        );
        assert!(r.metrics.max_drawdown_bps <= 10_000);
        assert!(r.metrics.exposure_bps <= 10_000);
        assert_eq!(r.metrics.bars, 30);
    }

    #[test]
    fn sltp_strategy_uses_stops_through_engine() {
        let bars: Vec<Bar> = (0..30)
            .map(|i| b(i as i64, 100, 112, 88, 100 + (i % 6) as i64 - 3))
            .collect();
        let mut s = StopLossTakeProfit::new(4, 6);
        s.arm();
        let r = run(&bars, &mut s, Config::default(), zero_cost(), None);
        assert!(!r.trades.is_empty());
        assert!(r.trades.iter().all(|t| matches!(
            t.exit_reason,
            EventKind::StopLoss | EventKind::TakeProfit | EventKind::EndOfData
        )));
    }

    #[test]
    fn assumptions_string_is_populated() {
        let bars = flat(3, 100);
        let mut s = BuyAndHold::new();
        let r = run(&bars, &mut s, Config::default(), CostModel::default(), None);
        assert!(r.assumptions.contains("fee 10 bps"));
        assert!(r.assumptions.contains("slippage 1"));
    }

    /// Semua angka dihitung tangan: qty 1 unit, fee 10 bps, slippage 1 tick.
    #[test]
    fn hand_calculated_four_bars() {
        let bars = vec![
            b(0, 10_000, 10_100, 9_900, 10_000),
            b(60_000, 11_000, 11_600, 10_900, 11_500),
            b(120_000, 11_500, 12_100, 11_400, 12_000),
            b(180_000, 12_000, 12_500, 11_900, 12_400),
        ];
        let cfg = Config {
            initial_cash_ticks: 1_000_000,
            qty_milli: 1_000,
        };
        let cost = CostModel {
            fee_bps: 10,
            slippage_ticks: 1,
        };
        let mut s = BuyAndHold::new();
        let r = run(&bars, &mut s, cfg, cost, None);

        // Entry: open bar 1 (11_000) + slippage = 11_001; fee floor = 11.
        assert_eq!(r.ledger.len(), 2);
        let e0 = r.ledger[0];
        assert_eq!((e0.bar, e0.kind, e0.side), (1, EventKind::Fill, Side::Buy));
        assert_eq!((e0.price_ticks, e0.reference_ticks), (11_001, 11_000));
        assert_eq!((e0.fee_ticks, e0.cash_after_ticks), (11, 988_988));
        assert_eq!((e0.qty_after_milli, e0.ts_millis), (1_000, 60_000));

        // Exit: close bar terakhir (12_400) − slippage = 12_399; fee floor = 12.
        let e1 = r.ledger[1];
        assert_eq!(
            (e1.bar, e1.kind, e1.side),
            (3, EventKind::EndOfData, Side::Sell)
        );
        assert_eq!((e1.price_ticks, e1.reference_ticks), (12_399, 12_400));
        assert_eq!((e1.fee_ticks, e1.cash_after_ticks), (12, 1_001_375));
        assert_eq!(e1.qty_after_milli, 0);

        // Trade: (12_399 − 11_001) − 11 − 12 = 1_375.
        assert_eq!(r.trades.len(), 1);
        let t = r.trades[0];
        assert_eq!(
            (
                t.entry_bar,
                t.exit_bar,
                t.entry_ticks,
                t.exit_ticks,
                t.pnl_ticks
            ),
            (1, 3, 11_001, 12_399, 1_375)
        );
        assert_eq!(t.exit_reason, EventKind::EndOfData);

        // Equity mark-to-market per bar.
        let eq: Vec<i64> = r.equity.iter().map(|p| p.equity_ticks).collect();
        assert_eq!(eq, vec![1_000_000, 1_000_488, 1_000_988, 1_001_375]);
        assert_eq!(r.final_equity_ticks, 1_001_375);
        assert_eq!(r.metrics.total_return_bps, 13);
        assert_eq!(r.metrics.fees_total_ticks, 23);
        assert_eq!(r.metrics.bars_in_market, 3);
        assert_eq!(r.metrics.exposure_bps, 7_500);
        assert_eq!(r.metrics.wins, 1);
        assert_eq!(r.metrics.losses, 0);
        assert_eq!(r.metrics.avg_pnl_ticks, 1_375);
        assert_eq!(r.metrics.max_drawdown_bps, 0);
        // Sharpe: return mili-bps [4880, 4997, 3866] → mean 4581, sd 507 → 9035.
        assert_eq!(r.metrics.sharpe_milli, 9_035);
    }

    #[test]
    fn sharpe_zero_when_flat_or_steady() {
        let point = |i: usize, v: i64| EquityPoint {
            bar: i,
            ts_millis: i as i64 * 60_000,
            equity_ticks: v,
            cash_ticks: v,
            qty_milli: 0,
            close_ticks: 0,
        };
        // Kurang dari 2 titik → 0.
        assert_eq!(sharpe_milli(&[point(0, 1_000)]), 0);
        // Equity datar → varians 0 → 0.
        let flat: Vec<EquityPoint> = (0..5).map(|i| point(i, 1_000)).collect();
        assert_eq!(sharpe_milli(&flat), 0);
        // Return konstan (x2 tiap bar) → varians 0 → 0.
        let steady = vec![point(0, 10_000), point(1, 20_000), point(2, 40_000)];
        assert_eq!(sharpe_milli(&steady), 0);
    }

    /// Determinisme lintas-thread: hasil paralel identik dengan serial.
    #[test]
    fn parallel_runs_identical_to_serial() {
        let make = || {
            let closes = [
                90, 96, 102, 108, 114, 108, 102, 96, 90, 84, 90, 96, 102, 108, 114,
            ];
            let bars: Vec<Bar> = closes
                .iter()
                .enumerate()
                .map(|(i, c)| b(i as i64, *c, c + 1, c - 1, *c))
                .collect();
            let mut s = MaCross::new(2, 3).unwrap();
            run(&bars, &mut s, Config::default(), CostModel::default(), None)
        };
        let serial = make();
        let handles: Vec<_> = (0..4).map(|_| std::thread::spawn(make)).collect();
        for h in handles {
            assert_eq!(h.join().unwrap(), serial);
        }
        assert_eq!(make(), serial);
    }

    /// Gap waktu antar bar tidak mengubah keputusan (bar hilang tidak diinterpolasi).
    #[test]
    fn time_gaps_do_not_change_decisions() {
        let closes = [
            90, 96, 102, 108, 114, 108, 102, 96, 90, 84, 90, 96, 102, 108, 114,
        ];
        let dense: Vec<Bar> = closes
            .iter()
            .enumerate()
            .map(|(i, c)| b(i as i64 * 60_000, *c, c + 1, c - 1, *c))
            .collect();
        let gapped: Vec<Bar> = closes
            .iter()
            .enumerate()
            .map(|(i, c)| b(i as i64 * 86_400_000, *c, c + 1, c - 1, *c))
            .collect();
        let mut s1 = MaCross::new(2, 3).unwrap();
        let mut s2 = MaCross::new(2, 3).unwrap();
        let r1 = run(
            &dense,
            &mut s1,
            Config::default(),
            CostModel::default(),
            None,
        );
        let r2 = run(
            &gapped,
            &mut s2,
            Config::default(),
            CostModel::default(),
            None,
        );
        assert_eq!(r1.metrics, r2.metrics);
        assert_eq!(r1.trades, r2.trades);
        let e1: Vec<_> = r1
            .equity
            .iter()
            .map(|p| (p.bar, p.equity_ticks, p.qty_milli))
            .collect();
        let e2: Vec<_> = r2
            .equity
            .iter()
            .map(|p| (p.bar, p.equity_ticks, p.qty_milli))
            .collect();
        assert_eq!(e1, e2);
        assert!(!r1.trades.is_empty(), "pola harus menghasilkan trade");
    }
}
