// SokanLab export — Apache-2.0. CSV/JSON laporan hasil backtest (format terbuka).
use sokanlab_backtest::{BacktestResult, EventKind};

fn kind_name(k: EventKind) -> &'static str {
    match k {
        EventKind::Fill => "fill",
        EventKind::StopLoss => "stop_loss",
        EventKind::TakeProfit => "take_profit",
        EventKind::EndOfData => "end_of_data",
    }
}

fn side_name(s: sokanlab_backtest::Side) -> &'static str {
    match s {
        sokanlab_backtest::Side::Buy => "buy",
        sokanlab_backtest::Side::Sell => "sell",
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Laporan metrik ringkas (CSV satu blok).
pub fn metrics_csv(r: &BacktestResult) -> String {
    let m = &r.metrics;
    let mut s = String::from("metric,value\n");
    s.push_str(&format!("strategy,{}\n", json_escape(&r.strategy)));
    s.push_str(&format!("bars,{}\n", m.bars));
    s.push_str(&format!("trades,{}\n", m.trades));
    s.push_str(&format!("wins,{}\n", m.wins));
    s.push_str(&format!("losses,{}\n", m.losses));
    s.push_str(&format!("win_rate_bps,{}\n", m.win_rate_bps()));
    s.push_str(&format!("total_return_bps,{}\n", m.total_return_bps));
    s.push_str(&format!("max_drawdown_bps,{}\n", m.max_drawdown_bps));
    s.push_str(&format!("exposure_bps,{}\n", m.exposure_bps));
    s.push_str(&format!("fees_total_ticks,{}\n", m.fees_total_ticks));
    s.push_str(&format!("avg_pnl_ticks,{}\n", m.avg_pnl_ticks));
    s.push_str(&format!("sharpe_milli,{}\n", m.sharpe_milli));
    s.push_str(&format!("initial_cash_ticks,{}\n", r.initial_cash_ticks));
    s.push_str(&format!("final_equity_ticks,{}\n", r.final_equity_ticks));
    s.push_str(&format!("assumptions,{}\n", json_escape(&r.assumptions)));
    s
}

/// Ledger order/fill lengkap.
pub fn ledger_csv(r: &BacktestResult) -> String {
    let mut s =
        String::from("bar,ts_millis,kind,side,price_ticks,reference_ticks,qty_milli,fee_ticks,cash_after_ticks,qty_after_milli\n");
    for e in &r.ledger {
        s.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            e.bar,
            e.ts_millis,
            kind_name(e.kind),
            side_name(e.side),
            e.price_ticks,
            e.reference_ticks,
            e.qty_milli,
            e.fee_ticks,
            e.cash_after_ticks,
            e.qty_after_milli
        ));
    }
    s
}

/// Kurva equity per bar.
pub fn equity_csv(r: &BacktestResult) -> String {
    let mut s = String::from("bar,ts_millis,equity_ticks,cash_ticks,qty_milli,close_ticks\n");
    for p in &r.equity {
        s.push_str(&format!(
            "{},{},{},{},{},{}\n",
            p.bar, p.ts_millis, p.equity_ticks, p.cash_ticks, p.qty_milli, p.close_ticks
        ));
    }
    s
}

/// Daftar transaksi bulat.
pub fn trades_csv(r: &BacktestResult) -> String {
    let mut s =
        String::from("entry_bar,exit_bar,entry_ticks,exit_ticks,qty_milli,pnl_ticks,exit_reason\n");
    for t in &r.trades {
        s.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            t.entry_bar,
            t.exit_bar,
            t.entry_ticks,
            t.exit_ticks,
            t.qty_milli,
            t.pnl_ticks,
            kind_name(t.exit_reason)
        ));
    }
    s
}

/// Laporan JSON lengkap (metrik + asumsi + transaksi; ledger lewat file terpisah).
pub fn report_json(r: &BacktestResult) -> String {
    let m = &r.metrics;
    let trades: Vec<String> = r
        .trades
        .iter()
        .map(|t| {
            format!(
                "{{\"entry_bar\":{},\"exit_bar\":{},\"entry_ticks\":{},\"exit_ticks\":{},\"qty_milli\":{},\"pnl_ticks\":{},\"exit_reason\":\"{}\"}}",
                t.entry_bar,
                t.exit_bar,
                t.entry_ticks,
                t.exit_ticks,
                t.qty_milli,
                t.pnl_ticks,
                kind_name(t.exit_reason)
            )
        })
        .collect();
    format!(
        "{{\"strategy\":\"{}\",\"assumptions\":\"{}\",\"initial_cash_ticks\":{},\"final_equity_ticks\":{},\"metrics\":{{\"bars\":{},\"trades\":{},\"wins\":{},\"losses\":{},\"win_rate_bps\":{},\"total_return_bps\":{},\"max_drawdown_bps\":{},\"exposure_bps\":{},\"fees_total_ticks\":{},\"avg_pnl_ticks\":{},\"sharpe_milli\":{}}},\"trades\":[{}]}}",
        json_escape(&r.strategy),
        json_escape(&r.assumptions),
        r.initial_cash_ticks,
        r.final_equity_ticks,
        m.bars,
        m.trades,
        m.wins,
        m.losses,
        m.win_rate_bps(),
        m.total_return_bps,
        m.max_drawdown_bps,
        m.exposure_bps,
        m.fees_total_ticks,
        m.avg_pnl_ticks,
        m.sharpe_milli,
        trades.join(",")
    )
}

/// Tulis semua laporan ke direktori: metrics.csv, ledger.csv, equity.csv, trades.csv, report.json.
pub fn write_all(
    dir: &std::path::Path,
    r: &BacktestResult,
) -> std::io::Result<Vec<std::path::PathBuf>> {
    std::fs::create_dir_all(dir)?;
    let mut written = Vec::new();
    for (name, content) in [
        ("metrics.csv", metrics_csv(r)),
        ("ledger.csv", ledger_csv(r)),
        ("equity.csv", equity_csv(r)),
        ("trades.csv", trades_csv(r)),
        ("report.json", report_json(r)),
    ] {
        let path = dir.join(name);
        std::fs::write(&path, content)?;
        written.push(path);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sokanlab_backtest::{run, Config, CostModel};
    use sokanlab_domain::Bar;
    use sokanlab_strategy_api::BuyAndHold;

    fn bars() -> Vec<Bar> {
        (0..8)
            .map(|i| Bar {
                ts_millis: i as i64 * 60_000,
                open_ticks: 100 + i as i64,
                high_ticks: 105 + i as i64,
                low_ticks: 95 + i as i64,
                close_ticks: 100 + i as i64,
                volume_milli: 10,
            })
            .collect()
    }

    fn sample() -> BacktestResult {
        let mut s = BuyAndHold::new();
        run(
            &bars(),
            &mut s,
            Config {
                initial_cash_ticks: 10_000,
                qty_milli: 1_000,
            },
            CostModel::default(),
            None,
        )
    }

    #[test]
    fn metrics_csv_has_key_rows() {
        let c = metrics_csv(&sample());
        assert!(c.contains("metric,value"));
        assert!(c.contains("total_return_bps,"));
        assert!(c.contains("sharpe_milli,"));
        assert!(c.contains("assumptions,"));
    }

    #[test]
    fn ledger_csv_has_header_and_rows() {
        let c = ledger_csv(&sample());
        let lines: Vec<&str> = c.lines().collect();
        assert!(lines[0].starts_with("bar,ts_millis,kind,side"));
        assert_eq!(lines.len(), 1 + sample().ledger.len());
    }

    #[test]
    fn equity_csv_rows_match_equity_points() {
        let r = sample();
        let lines = equity_csv(&r).lines().count();
        assert_eq!(lines, 1 + r.equity.len());
    }

    #[test]
    fn trades_csv_rows_match_trades() {
        let r = sample();
        assert_eq!(trades_csv(&r).lines().count(), 1 + r.trades.len());
    }

    #[test]
    fn report_json_is_braced_and_has_assumptions() {
        let j = report_json(&sample());
        assert!(j.starts_with('{') && j.ends_with('}'));
        assert!(j.contains("\"assumptions\""));
        assert!(j.contains("\"max_drawdown_bps\""));
        assert!(j.contains("\"sharpe_milli\""));
    }

    #[test]
    fn write_all_creates_five_files() {
        let dir = std::env::temp_dir().join("sokanlab-export-test");
        let _ = std::fs::remove_dir_all(&dir);
        let files = write_all(&dir, &sample()).unwrap();
        assert_eq!(files.len(), 5);
        for f in &files {
            assert!(f.exists());
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn json_escape_handles_quotes() {
        assert_eq!(json_escape("a\"b"), "a\\\"b");
    }
}
