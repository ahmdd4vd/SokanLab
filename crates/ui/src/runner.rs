// SokanLab backtest runner — Apache-2.0.
// Engine dijalankan di luar UI thread; hasil dikirim balik lewat channel.
use sokanlab_backtest::{run, BacktestResult, Config, CostModel, StopLoss};
use sokanlab_domain::Bar;
use sokanlab_strategy_api::{BuyAndHold, MaCross, RsiThreshold, StopLossTakeProfit, Strategy};
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyKind {
    BuyAndHold,
    MaCross {
        fast: usize,
        slow: usize,
    },
    RsiThreshold {
        period: usize,
        oversold: i64,
        overbought: i64,
    },
    StopLossTakeProfit {
        stop_ticks: i64,
        target_ticks: i64,
    },
}

impl StrategyKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::BuyAndHold => "buy_and_hold",
            Self::MaCross { .. } => "ma_cross",
            Self::RsiThreshold { .. } => "rsi_threshold",
            Self::StopLossTakeProfit { .. } => "sl_tp",
        }
    }

    pub fn build(&self) -> Option<Box<dyn Strategy>> {
        match self {
            Self::BuyAndHold => Some(Box::new(BuyAndHold::new())),
            Self::MaCross { fast, slow } => MaCross::new(*fast, *slow)
                .ok()
                .map(|s| Box::new(s) as Box<dyn Strategy>),
            Self::RsiThreshold {
                period,
                oversold,
                overbought,
            } => RsiThreshold::new(*period, *oversold, *overbought)
                .ok()
                .map(|s| Box::new(s) as Box<dyn Strategy>),
            Self::StopLossTakeProfit {
                stop_ticks,
                target_ticks,
            } => {
                let mut s = StopLossTakeProfit::new(*stop_ticks, *target_ticks);
                s.arm();
                Some(Box::new(s))
            }
        }
    }

    /// Stop/target dari strategi SL/TP, kalau memakai strategy itu.
    pub fn engine_stops(&self) -> Option<StopLoss> {
        match self {
            Self::StopLossTakeProfit {
                stop_ticks,
                target_ticks,
            } => Some(StopLoss {
                stop_ticks: *stop_ticks,
                take_profit_ticks: *target_ticks,
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RunRequest {
    pub strategy: StrategyKind,
    pub config: Config,
    pub cost: CostModel,
}

#[derive(Debug)]
pub enum RunOutcome {
    Done(Box<BacktestResult>),
    Failed(String),
}

pub struct Runner {
    tx: Sender<(Vec<Bar>, RunRequest)>,
    rx: Receiver<RunOutcome>,
    running: bool,
}

impl Default for Runner {
    fn default() -> Self {
        Self::new()
    }
}

impl Runner {
    pub fn new() -> Self {
        let (tx, job) = channel::<(Vec<Bar>, RunRequest)>();
        let (out_tx, rx) = channel::<RunOutcome>();
        std::thread::Builder::new()
            .name("sokanlab-backtest".into())
            .spawn(move || {
                // Worker tunggal: antrean berurutan, hasil deterministik.
                while let Ok((bars, req)) = job.recv() {
                    let stops = req.strategy.engine_stops();
                    let Some(mut strategy) = req.strategy.build() else {
                        let _ = out_tx
                            .send(RunOutcome::Failed("parameter strategi tidak valid".into()));
                        continue;
                    };
                    let result = run(&bars, strategy.as_mut(), req.config, req.cost, stops);
                    if out_tx.send(RunOutcome::Done(Box::new(result))).is_err() {
                        break;
                    }
                }
            })
            .expect("gagal membuat worker backtest");
        Self {
            tx,
            rx,
            running: false,
        }
    }

    /// Kirim pekerjaan; tidak memblokir UI.
    pub fn submit(&mut self, bars: Vec<Bar>, req: RunRequest) -> bool {
        if self.running {
            return false;
        }
        self.running = true;
        self.tx.send((bars, req)).is_ok()
    }

    /// Ambil hasil kalau sudah selesai.
    pub fn poll(&mut self) -> Option<RunOutcome> {
        match self.rx.try_recv() {
            Ok(o) => {
                self.running = false;
                Some(o)
            }
            Err(_) => None,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn req() -> RunRequest {
        RunRequest {
            strategy: StrategyKind::BuyAndHold,
            config: Config::default(),
            cost: CostModel::default(),
        }
    }

    /// Tunggu hasil worker (dengan batas waktu) agar test tidak flaky.
    fn wait_done(r: &mut Runner) -> RunOutcome {
        for _ in 0..600 {
            if let Some(o) = r.poll() {
                return o;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("hasil worker tidak tiba dalam batas waktu");
    }

    #[test]
    fn run_completes_and_reports() {
        let mut r = Runner::new();
        assert!(r.submit(bars(10), req()));
        assert!(r.is_running(), "flag running aktif sampai hasil diambil");
        match wait_done(&mut r) {
            RunOutcome::Done(res) => {
                assert_eq!(res.strategy, "buy_and_hold");
                assert!(!res.equity.is_empty());
            }
            RunOutcome::Failed(e) => panic!("gagal: {e}"),
        }
        assert!(!r.is_running());
    }

    #[test]
    fn second_submit_rejected_while_running() {
        let mut r = Runner::new();
        assert!(r.submit(bars(5), req()));
        // Sinkron: flag running aktif sampai hasil diambil → submit kedua ditolak.
        assert!(!r.submit(bars(5), req()));
        assert!(matches!(wait_done(&mut r), RunOutcome::Done(_)));
        // Setelah selesai, worker bisa menerima job baru.
        assert!(r.submit(bars(5), req()));
        assert!(matches!(wait_done(&mut r), RunOutcome::Done(_)));
    }

    #[test]
    fn invalid_params_report_failure() {
        let mut r = Runner::new();
        assert!(r.submit(
            bars(5),
            RunRequest {
                strategy: StrategyKind::MaCross { fast: 0, slow: 3 },
                config: Config::default(),
                cost: CostModel::default(),
            }
        ));
        assert!(matches!(wait_done(&mut r), RunOutcome::Failed(_)));
        assert!(!r.is_running());
    }

    #[test]
    fn repeated_runs_same_input_same_output() {
        let b = bars(30);
        let run_once = || {
            let mut s = BuyAndHold::new();
            run(&b, &mut s, Config::default(), CostModel::default(), None)
        };
        assert_eq!(run_once(), run_once());
    }

    /// Worker terpisah (dua Runner berbeda) memberi hasil identik — deterministik.
    #[test]
    fn two_runner_instances_identical() {
        let mut r1 = Runner::new();
        let mut r2 = Runner::new();
        assert!(r1.submit(bars(40), req()));
        assert!(r2.submit(bars(40), req()));
        let (o1, o2) = (wait_done(&mut r1), wait_done(&mut r2));
        match (o1, o2) {
            (RunOutcome::Done(a), RunOutcome::Done(b)) => assert_eq!(*a, *b),
            _ => panic!("kedua runner harus sukses"),
        }
    }
}
