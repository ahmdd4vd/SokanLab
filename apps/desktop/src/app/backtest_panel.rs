// SokanLab app — panel backtest (form + hasil), worker thread terpisah dari UI.
use egui::{ComboBox, RichText, Ui};
use sokanlab_backtest::{BacktestResult, Config, CostModel};
use sokanlab_ui::equity::{draw_drawdown_bars, draw_equity, plot_area};
use sokanlab_ui::runner::{RunOutcome, RunRequest, StrategyKind};

use super::state::AppState;

const STRATEGIES: [&str; 4] = [
    "Buy & Hold",
    "MA Cross",
    "RSI Threshold",
    "Stop Loss / Take Profit",
];

fn kind_to_label(k: &StrategyKind) -> usize {
    match k {
        StrategyKind::BuyAndHold => 0,
        StrategyKind::MaCross { .. } => 1,
        StrategyKind::RsiThreshold { .. } => 2,
        StrategyKind::StopLossTakeProfit { .. } => 3,
    }
}

/// Panel parameter backtest. Mengembalikan `true` bila pengguna menekan Jalankan.
pub fn backtest_form(ui: &mut Ui, s: &mut AppState) -> bool {
    ui.separator();
    ui.heading("Backtest");
    let mut selected = kind_to_label(&s.bt.strategy);
    ComboBox::from_id_salt("bt-strategy")
        .selected_text(STRATEGIES[selected])
        .show_ui(ui, |ui| {
            for (i, name) in STRATEGIES.iter().enumerate() {
                ui.selectable_value(&mut selected, i, *name);
            }
        });
    let mut kind = s.bt.strategy;
    match kind {
        StrategyKind::MaCross { fast, slow } => {
            let (mut fast, mut slow) = (fast, slow);
            ui.horizontal(|ui| {
                ui.label("fast");
                ui.add(egui::Slider::new(&mut fast, 2usize..=50).text("bar"));
                ui.label("slow");
                ui.add(egui::Slider::new(&mut slow, 3usize..=200).text("bar"));
            });
            if slow <= fast {
                ui.colored_label(
                    sokanlab_ui::theme::EMBER,
                    "slow harus > fast (perbaiki bila sama)",
                );
            }
            kind = StrategyKind::MaCross { fast, slow };
        }
        StrategyKind::RsiThreshold {
            period,
            oversold,
            overbought,
        } => {
            let (mut period, mut oversold, mut overbought) = (period, oversold, overbought);
            ui.horizontal(|ui| {
                ui.label("periode");
                ui.add(egui::Slider::new(&mut period, 2usize..=50).text("bar"));
            });
            ui.horizontal(|ui| {
                ui.label("bawah");
                ui.add(egui::Slider::new(&mut oversold, 1i64..=49));
                ui.label("atas");
                ui.add(egui::Slider::new(&mut overbought, 51i64..=99));
            });
            kind = StrategyKind::RsiThreshold {
                period,
                oversold,
                overbought,
            };
        }
        StrategyKind::StopLossTakeProfit {
            stop_ticks,
            target_ticks,
        } => {
            let (mut stop_ticks, mut target_ticks) = (stop_ticks, target_ticks);
            ui.horizontal(|ui| {
                ui.label("stop (tick)");
                ui.add(egui::Slider::new(&mut stop_ticks, 1i64..=5_000));
            });
            ui.horizontal(|ui| {
                ui.label("target (tick)");
                ui.add(egui::Slider::new(&mut target_ticks, 1i64..=10_000));
            });
            kind = StrategyKind::StopLossTakeProfit {
                stop_ticks,
                target_ticks,
            };
        }
        StrategyKind::BuyAndHold => {
            ui.small("masuk di bar pertama, tahan sampai akhir data");
        }
    }
    s.bt.strategy = kind;

    ui.separator();
    ui.label(format!(
        "modal {} tick · fee {} bps · slippage {} tick",
        s.bt.config.initial_cash_ticks, s.bt.cost.fee_bps, s.bt.cost.slippage_ticks
    ));
    ui.horizontal(|ui| {
        ui.label("modal");
        if ui
            .add(egui::Slider::new(
                &mut s.bt.config.initial_cash_ticks,
                100..=100_000_000,
            ))
            .changed()
        {
            s.bt.dirty = true;
        }
    });
    ui.horizontal(|ui| {
        ui.label("fee bps");
        if ui
            .add(egui::Slider::new(&mut s.bt.cost.fee_bps, 0..=100))
            .changed()
        {
            s.bt.dirty = true;
        }
    });
    ui.horizontal(|ui| {
        ui.label("slippage");
        if ui
            .add(egui::Slider::new(&mut s.bt.cost.slippage_ticks, 0..=50))
            .changed()
        {
            s.bt.dirty = true;
        }
    });

    let disabled = s.runner.is_running() || !s.has_data();
    if disabled && !s.has_data() {
        ui.small("impor dataset dulu untuk menjalankan backtest");
    }
    let clicked = ui
        .add_enabled(!disabled, egui::Button::new("Jalankan backtest"))
        .clicked();
    if clicked {
        s.bt.pending = true;
    }
    s.bt.pending
}

/// Panel hasil: metrik, equity curve, daftar transaksi terakhir.
pub fn backtest_result(ui: &mut Ui, s: &mut AppState) {
    let Some(r) = s.result.as_ref() else {
        return;
    };
    ui.separator();
    ui.heading("Hasil");
    ui.label(RichText::new(format!("strategi: {}", r.strategy)).strong());
    ui.small(r.assumptions.clone());

    ui.add_space(4.0);
    let m = &r.metrics;
    egui::Grid::new("bt-metrics")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            ui.label("total return");
            ui.label(format!("{} bps", m.total_return_bps));
            ui.end_row();
            ui.label("max drawdown");
            ui.label(format!("{} bps", m.max_drawdown_bps));
            ui.end_row();
            ui.label("win rate");
            ui.label(format!(
                "{} bps ({}W/{}L dari {})",
                m.win_rate_bps(),
                m.wins,
                m.losses,
                m.trades
            ));
            ui.end_row();
            ui.label("Sharpe (per-bar)");
            ui.label(format!(
                "{:.3} · belum annualized",
                m.sharpe_milli as f64 / 1_000.0
            ));
            ui.end_row();
            ui.label("exposure");
            ui.label(format!("{} bps", m.exposure_bps));
            ui.end_row();
            ui.label("total fee");
            ui.label(format!("{} tick", m.fees_total_ticks));
            ui.end_row();
            ui.label("PnL rata-rata");
            ui.label(format!("{} tick", m.avg_pnl_ticks));
            ui.end_row();
            ui.label("equity akhir");
            ui.label(format!(
                "{} (modal {})",
                r.final_equity_ticks, r.initial_cash_ticks
            ));
            ui.end_row();
        });

    ui.add_space(6.0);
    let avail = ui.available_rect_before_wrap();
    let height = (avail.height() * 0.32).clamp(120.0, 320.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(avail.width(), height), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    draw_equity(&painter, plot_area(rect), r);
    draw_drawdown_bars(&painter, plot_area(rect), r);

    ui.add_space(6.0);
    egui::CollapsingHeader::new("Transaksi")
        .default_open(false)
        .show(ui, |ui| {
            let n = r.trades.len().min(200);
            egui::ScrollArea::vertical()
                .max_height(220.0)
                .show(ui, |ui| {
                    egui::Grid::new("bt-trades")
                        .num_columns(5)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.small("entry");
                            ui.small("exit");
                            ui.small("harga masuk");
                            ui.small("harga keluar");
                            ui.small("PnL");
                            ui.end_row();
                            for t in &r.trades[r.trades.len() - n..] {
                                ui.label(t.entry_bar.to_string());
                                ui.label(t.exit_bar.to_string());
                                ui.label(t.entry_ticks.to_string());
                                ui.label(t.exit_ticks.to_string());
                                let color = if t.pnl_ticks >= 0 {
                                    sokanlab_ui::theme::EMBER
                                } else {
                                    sokanlab_ui::theme::bear_color()
                                };
                                ui.colored_label(color, t.pnl_ticks.to_string());
                                ui.end_row();
                            }
                        });
                    if r.trades.len() > 200 {
                        ui.small(format!("… {} transaksi lain", r.trades.len() - 200));
                    }
                });
        });

    if ui.button("Ekspor hasil (CSV/JSON)").clicked() {
        match sokanlab_export::write_all(std::path::Path::new(&s.export_dir), r) {
            Ok(files) => {
                s.notice = Some(format!(
                    "tersimpan {} file di {}",
                    files.len(),
                    s.export_dir
                ));
            }
            Err(e) => s.notice = Some(format!("gagal ekspor: {e}")),
        }
    }
}

/// Terapkan job yang sudah antre, lalu poll respons worker.
pub fn pump(ui: &mut Ui, s: &mut AppState) {
    if s.bt.pending {
        s.bt.pending = false;
        let bars = s.bars.clone();
        let req = RunRequest {
            strategy: s.bt.strategy,
            config: s.bt.config,
            cost: s.bt.cost,
        };
        s.notice = None;
        if s.runner.submit(bars, req) {
            s.notice = Some("backtest berjalan di worker…".into());
        }
    }
    if let Some(outcome) = s.runner.poll() {
        match outcome {
            RunOutcome::Done(r) => {
                s.result = Some(*r);
                s.notice = Some("backtest selesai".into());
            }
            RunOutcome::Failed(e) => s.notice = Some(format!("backtest gagal: {e}")),
        }
    }
    if let Some(note) = s.notice.clone() {
        ui.small(note);
    }
}

/// Ringkas hasil untuk report CLI.
pub fn summarize(r: &BacktestResult, config: Config, cost: CostModel) -> String {
    format!(
        "{} | equity {} → {} | return {} bps | maxdd {} bps | winrate {} bps | sharpe {} mili | trades {} | fee {} tick | modal {} | fee_bps {} | slippage {}",
        r.strategy,
        r.initial_cash_ticks,
        r.final_equity_ticks,
        r.metrics.total_return_bps,
        r.metrics.max_drawdown_bps,
        r.metrics.win_rate_bps(),
        r.metrics.sharpe_milli,
        r.metrics.trades,
        r.metrics.fees_total_ticks,
        config.initial_cash_ticks,
        cost.fee_bps,
        cost.slippage_ticks
    )
}
