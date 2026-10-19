// SokanLab app — state bersama GUI (chart + backtest).
use sokanlab_backtest::{BacktestResult, Config, CostModel};
use sokanlab_domain::Bar;
use sokanlab_ui::runner::{Runner, StrategyKind};
use sokanlab_ui::ChartState;

#[derive(Debug, Clone, Copy)]
pub struct BacktestSettings {
    pub strategy: StrategyKind,
    pub config: Config,
    pub cost: CostModel,
    /// Parameter berubah sejak run terakhir → tandai hasil basi.
    pub dirty: bool,
    /// Job sudah diminta, menunggu dikirim ke worker.
    pub pending: bool,
}

impl Default for BacktestSettings {
    fn default() -> Self {
        Self {
            strategy: StrategyKind::BuyAndHold,
            config: Config::default(),
            cost: CostModel::default(),
            dirty: false,
            pending: false,
        }
    }
}

pub struct AppState {
    /// Chart + dataset aktif (dipakai panel chart).
    pub chart: ChartState,
    /// Copy bar untuk worker backtest (tidak berbagi borrow dengan UI).
    pub bars: Vec<Bar>,
    pub runner: Runner,
    pub bt: BacktestSettings,
    pub result: Option<BacktestResult>,
    pub notice: Option<String>,
    pub export_dir: String,
    /// Lokasi berkas layout (restart persistence); env `SOKANLAB_LAYOUT` menimpanya.
    pub layout_path: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            chart: ChartState::new(),
            bars: Vec::new(),
            runner: Runner::new(),
            bt: BacktestSettings::default(),
            result: None,
            notice: None,
            export_dir: "out/backtest".to_string(),
            layout_path: std::env::var("SOKANLAB_LAYOUT")
                .unwrap_or_else(|_| "data/layout.json".to_string()),
        }
    }

    pub fn load(&mut self, bars: Vec<Bar>) {
        self.bars = bars.clone();
        self.chart.load(bars);
        self.bt.dirty = true;
        self.result = None;
    }

    pub fn has_data(&self) -> bool {
        !self.bars.is_empty()
    }

    /// Simpan layout chart (viewport, drawing, tema, watchlist) ke disk.
    pub fn save_layout(&self) -> std::io::Result<()> {
        self.chart
            .save_layout(std::path::Path::new(&self.layout_path))
    }

    /// Pulihkan layout dari disk, bila ada. Berkas tidak ada bukan error fatal.
    pub fn restore_layout(&mut self) -> Result<(), String> {
        self.chart
            .restore_layout(std::path::Path::new(&self.layout_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sokanlab_domain::Bar;

    fn bars(n: usize) -> Vec<Bar> {
        (0..n)
            .map(|i| Bar {
                ts_millis: i as i64 * 60_000,
                open_ticks: 100,
                high_ticks: 105,
                low_ticks: 95,
                close_ticks: 100 + (i % 4) as i64,
                volume_milli: 10,
            })
            .collect()
    }

    #[test]
    fn load_fills_chart_and_marks_dirty() {
        let mut s = AppState::new();
        s.load(bars(10));
        assert!(s.has_data());
        assert_eq!(s.chart.bars.len(), 10);
        assert!(s.bt.dirty);
        assert!(s.result.is_none());
    }

    #[test]
    fn default_settings_use_plan_costs() {
        let s = AppState::new();
        assert_eq!(s.bt.cost.fee_bps, 10);
        assert_eq!(s.bt.cost.slippage_ticks, 1);
        assert_eq!(s.bt.strategy, StrategyKind::BuyAndHold);
    }

    /// Jalur yang dipakai panel: submit ke worker → poll → hasil siap.
    #[test]
    fn runner_roundtrip_via_appstate() {
        use sokanlab_ui::runner::{RunOutcome, RunRequest};
        let mut s = AppState::new();
        s.load(bars(30));
        assert!(s.has_data());
        let req = RunRequest {
            strategy: s.bt.strategy,
            config: s.bt.config,
            cost: s.bt.cost,
        };
        assert!(s.runner.submit(s.bars.clone(), req));
        let mut out = None;
        for _ in 0..600 {
            if let Some(o) = s.runner.poll() {
                out = Some(o);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        match out.expect("hasil worker tidak tiba") {
            RunOutcome::Done(r) => {
                assert_eq!(r.strategy, "buy_and_hold");
                assert_eq!(r.metrics.bars, 30);
            }
            RunOutcome::Failed(e) => panic!("gagal: {e}"),
        }
        assert!(!s.runner.is_running());
    }

    /// Persistence: simpan layout → state baru memulihkannya (restart).
    #[test]
    fn layout_persists_across_appstate_restart() {
        let dir = std::env::temp_dir().join("sokanlab-appstate-layout");
        let _ = std::fs::remove_dir_all(&dir);
        let mut a = AppState::new();
        a.layout_path = dir.join("layout.json").to_string_lossy().into_owned();
        a.load(bars(10));
        a.chart.watchlist = vec!["SYNTH/USDT".into()];
        a.chart.add_drawing(sokanlab_ui::horizontal_line(123));
        a.save_layout().unwrap();

        let mut b = AppState::new();
        b.layout_path = a.layout_path.clone();
        b.load(bars(10));
        b.restore_layout().unwrap();
        assert_eq!(b.chart.watchlist, vec!["SYNTH/USDT".to_string()]);
        assert_eq!(b.chart.drawings.items, a.chart.drawings.items);

        // Berkas hilang → error jelas (bukan panic).
        let mut c = AppState::new();
        c.layout_path = dir.join("tidak-ada.json").to_string_lossy().into_owned();
        assert!(c.restore_layout().is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
