// SokanLab desktop — Apache-2.0.
// Sub-perintah: (kosong) jalankan GUI eframe; `import` CLI CSV offline;
// `backtest` CLI offline (parquet/CSV → laporan); `--smoke`/`--render`/`--headless` uji cepat.
mod app;

use sokanlab_ui::runner::StrategyKind;
use std::path::{Path, PathBuf};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const CONFIG_SCHEMA: u32 = 1;

fn config_path() -> PathBuf {
    std::env::var_os("SOKANLAB_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("config/default.toml"))
}

/// Logger minimal dengan redaksi token/kunci agar tidak bocor ke log.
fn log_redacted(msg: &str) {
    let mut out = msg.to_string();
    for secret in ["token", "key", "secret"] {
        if out.to_lowercase().contains(secret) {
            out = format!("<redacted:{secret}>");
            break;
        }
    }
    eprintln!("[sokanlab] {out}");
}

fn check_config() -> Result<(), String> {
    let path = config_path();
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("config tidak terbaca {}: {e}", path.display()))?;
    if !text.contains("schema_version = 1") {
        return Err("config schema_version bukan 1".into());
    }
    Ok(())
}

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone())
}

/// Batas ukuran berkas teks impor (byte). Default 1 GiB ≈ 6–8 juta bar;
/// bisa ditimpa `SOKANLAB_MAX_IMPORT_MB` untuk dataset di luar target desain.
fn max_import_bytes() -> u64 {
    std::env::var("SOKANLAB_MAX_IMPORT_MB")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(1024)
        * 1024
        * 1024
}

/// Baca teks impor dengan batas ukuran; gagal cepat bila melewati batas.
fn read_text_limited_with(path: &str, limit: u64) -> Result<String, String> {
    if let Ok(m) = std::fs::metadata(path) {
        if m.len() > limit {
            return Err(format!(
                "{path} berukuran {} MB melebihi batas {} MB (set SOKANLAB_MAX_IMPORT_MB untuk menimpa)",
                m.len() / (1024 * 1024),
                limit / (1024 * 1024)
            ));
        }
    }
    std::fs::read_to_string(path).map_err(|e| format!("baca {path}: {e}"))
}

fn read_text_limited(path: &str) -> Result<String, String> {
    read_text_limited_with(path, max_import_bytes())
}

fn cmd_import(args: &[String]) -> i32 {
    let file = arg_value(args, "--file");
    let symbol = arg_value(args, "--symbol").unwrap_or_else(|| "BTC/USDT".into());
    let timeframe = arg_value(args, "--timeframe").unwrap_or_else(|| "1m".into());
    let db = arg_value(args, "--db").unwrap_or_else(|| "data/catalog.db".into());
    let out = arg_value(args, "--out").unwrap_or_else(|| "data/BTCUSDT-1m.parquet".into());
    let file = match file {
        Some(f) => f,
        None => {
            eprintln!(
                "pakai: import --file <csv> [--symbol BTC/USDT] [--timeframe 1m] [--db ...] [--out ...]"
            );
            return 2;
        }
    };
    let text = match read_text_limited(&file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    if !["1m", "1h", "1D"].contains(&timeframe.as_str()) {
        eprintln!("error: timeframe harus 1m/1h/1D, dapat {timeframe}");
        return 2;
    }
    let (base, quote) = match symbol.split_once('/') {
        Some((b, q)) if !b.is_empty() && !q.is_empty() => (b.to_string(), q.to_string()),
        _ => {
            eprintln!("error: symbol harus BASE/QUOTE, dapat {symbol}");
            return 2;
        }
    };
    let preview = sokanlab_data_import::preview_csv(&text, 3);
    let bars = match sokanlab_data_import::parse_csv_ohlcv(&text) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: CSV tidak valid: {e:?}");
            eprintln!("preview: {preview:?}");
            return 2;
        }
    };
    let ts_min = bars.first().map(|b| b.ts_millis).unwrap_or(0);
    let ts_max = bars.last().map(|b| b.ts_millis).unwrap_or(0);
    let gaps = 0; // deteksi gap antar-timeframe menyusul
    let duplicates = 0; // duplikat sudah ditolak saat parse
    if let Err(e) = sokanlab_data_store::write_bars_parquet(Path::new(&out), &bars) {
        eprintln!("error: tulis parquet: {e}");
        return 2;
    }
    let bytes = match std::fs::read(&out) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: baca ulang parquet: {e}");
            return 2;
        }
    };
    let sha = sokanlab_data_store::sha256_hex(&bytes);
    let id = format!("{base}/{quote}-{timeframe}");
    let rec = sokanlab_data_store::DatasetRecord {
        id: id.clone(),
        base,
        quote,
        timeframe: timeframe.clone(),
        ts_min,
        ts_max,
        bars: bars.len() as i64,
        sha256: sha,
        source_file: file.clone(),
    };
    let conn = match sokanlab_data_store::open_catalog(Path::new(&db)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: katalog: {e}");
            return 2;
        }
    };
    if let Err(e) = sokanlab_data_store::upsert_dataset(&conn, &rec) {
        eprintln!("error: upsert katalog: {e}");
        return 2;
    }
    println!(
        "{}",
        sokanlab_data_store::provenance_json(&rec, gaps, duplicates)
    );
    0
}

/// Muat dataset pertama dari katalog ke AppState (data/catalog.db → data/BASEQUOTE-tf.parquet).
fn load_dataset_into(st: &mut app::AppState) -> Option<String> {
    let conn = sokanlab_data_store::open_catalog(Path::new("data/catalog.db")).ok()?;
    let mut stmt = conn
        .prepare("SELECT id, base, quote, timeframe FROM datasets ORDER BY id LIMIT 1")
        .ok()?;
    let row = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .ok()
        .and_then(|mut it| it.next().and_then(|v| v.ok()));
    let (id, base, quote, timeframe) = row?;
    // Nama file mengikuti default CLI: BASE+QUOTE-timeframe (tanpa slash).
    let parquet = format!("data/{base}{quote}-{timeframe}.parquet");
    match sokanlab_data_store::read_bars_parquet(Path::new(&parquet)) {
        Ok(bars) => {
            let n = bars.len();
            st.chart.watchlist.push(id.clone());
            st.load(bars);
            eprintln!("[sokanlab] muat {n} bar dari {parquet}");
            Some(id)
        }
        Err(e) => {
            eprintln!("[sokanlab] gagal muat {parquet}: {e}");
            None
        }
    }
}

/// Strategi dari argumen CLI (nama sama dengan StrategyKind::name()).
fn parse_strategy(args: &[String]) -> Result<StrategyKind, String> {
    let name = arg_value(args, "--strategy").unwrap_or_else(|| "buy_and_hold".into());
    let num = |flag: &str, default: i64| -> i64 {
        arg_value(args, flag)
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(default)
    };
    match name.as_str() {
        "buy_and_hold" => Ok(StrategyKind::BuyAndHold),
        "ma_cross" => {
            let fast = num("--fast", 9);
            let slow = num("--slow", 21);
            if fast < 1 || slow <= fast {
                return Err(format!(
                    "parameter ma_cross tidak valid: fast={fast} slow={slow}"
                ));
            }
            Ok(StrategyKind::MaCross {
                fast: fast as usize,
                slow: slow as usize,
            })
        }
        "rsi_threshold" => {
            let period = num("--rsi-period", 14);
            let oversold = num("--oversold", 30);
            let overbought = num("--overbought", 70);
            if period < 2 || oversold < 1 || overbought > 99 || oversold >= overbought {
                return Err(format!(
                    "parameter rsi tidak valid: period={period} oversold={oversold} overbought={overbought}"
                ));
            }
            Ok(StrategyKind::RsiThreshold {
                period: period as usize,
                oversold,
                overbought,
            })
        }
        "sl_tp" => {
            let stop_ticks = num("--stop", 500);
            let target_ticks = num("--target", 1_000);
            if stop_ticks < 1 || target_ticks < 1 {
                return Err(format!(
                    "parameter sl_tp tidak valid: stop={stop_ticks} target={target_ticks}"
                ));
            }
            Ok(StrategyKind::StopLossTakeProfit {
                stop_ticks,
                target_ticks,
            })
        }
        other => Err(format!(
            "strategi tidak dikenal: {other} (buy_and_hold|ma_cross|rsi_threshold|sl_tp)"
        )),
    }
}

/// Data untuk backtest CLI: --csv, --parquet, atau dataset pertama katalog.
fn load_bars_for_backtest(args: &[String]) -> Result<(String, Vec<sokanlab_domain::Bar>), String> {
    if let Some(f) = arg_value(args, "--csv") {
        let text = read_text_limited(&f)?;
        let bars = sokanlab_data_import::parse_csv_ohlcv(&text)
            .map_err(|e| format!("CSV tidak valid {f}: {e:?}"))?;
        return Ok((f, bars));
    }
    let parquet = match arg_value(args, "--parquet") {
        Some(f) => f,
        None => {
            let symbol = arg_value(args, "--symbol").unwrap_or_else(|| "BTC/USDT".into());
            let timeframe = arg_value(args, "--timeframe").unwrap_or_else(|| "1m".into());
            let (base, quote) = symbol
                .split_once('/')
                .ok_or_else(|| format!("symbol harus BASE/QUOTE, dapat {symbol}"))?;
            format!("data/{base}{quote}-{timeframe}.parquet")
        }
    };
    let bars = sokanlab_data_store::read_bars_parquet(Path::new(&parquet))
        .map_err(|e| format!("baca {parquet}: {e}"))?;
    Ok((parquet, bars))
}

/// Backtest offline dari CLI: jalankan engine, tulis laporan, cetak ringkasan.
fn cmd_backtest(args: &[String]) -> i32 {
    let (src, bars) = match load_bars_for_backtest(args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    if bars.is_empty() {
        eprintln!("error: dataset kosong: {src}");
        return 2;
    }
    let kind = match parse_strategy(args) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    let cash = arg_value(args, "--cash")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(1_000_000);
    let qty = arg_value(args, "--qty")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(1_000);
    let fee_bps = arg_value(args, "--fee-bps")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(10);
    let slippage = arg_value(args, "--slippage")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(1);
    if cash <= 0 || qty <= 0 || fee_bps < 0 || slippage < 0 {
        eprintln!(
            "error: parameter biaya tidak valid: cash={cash} qty={qty} fee_bps={fee_bps} slippage={slippage}"
        );
        return 2;
    }
    let config = sokanlab_backtest::Config {
        initial_cash_ticks: cash,
        qty_milli: qty,
    };
    let cost = sokanlab_backtest::CostModel {
        fee_bps,
        slippage_ticks: slippage,
    };
    let stops = kind.engine_stops();
    let Some(mut strategy) = kind.build() else {
        eprintln!("error: parameter strategi tidak valid: {kind:?}");
        return 2;
    };
    let result = sokanlab_backtest::run(&bars, strategy.as_mut(), config, cost, stops);
    let out_dir = arg_value(args, "--out").unwrap_or_else(|| "out/backtest".into());
    match sokanlab_export::write_all(Path::new(&out_dir), &result) {
        Ok(files) => {
            eprintln!(
                "backtest-ok bars={} source={src} files={} dir={out_dir}",
                bars.len(),
                files.len()
            );
        }
        Err(e) => {
            eprintln!("error: ekspor laporan: {e}");
            return 2;
        }
    }
    println!("{}", app::backtest_panel::summarize(&result, config, cost));
    0
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version") {
        println!("sokanlab-desktop {APP_VERSION} (schema {CONFIG_SCHEMA})");
        return;
    }
    if args.iter().any(|a| a == "import") {
        std::process::exit(cmd_import(&args));
    }
    if args.iter().any(|a| a == "backtest") {
        std::process::exit(cmd_backtest(&args));
    }
    if args.iter().any(|a| a == "--render") {
        let out = arg_value(&args, "--out").unwrap_or_else(|| "docs/shot.png".into());
        let bars = arg_value(&args, "--bars")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(500);
        let (w, h) = arg_value(&args, "--size")
            .and_then(|v| {
                let (a, b) = v.split_once('x')?;
                Some((a.parse::<usize>().ok()?, b.parse::<usize>().ok()?))
            })
            .unwrap_or((1200, 800));
        let demo = arg_value(&args, "--demo");
        match app::headless::render(std::path::Path::new(&out), bars, w, h, demo.as_deref()) {
            Ok(tris) => {
                println!("render-ok triangles={tris} file={out} bars={bars} size={w}x{h}");
            }
            Err(e) => {
                eprintln!("error: render: {e}");
                std::process::exit(2);
            }
        }
        return;
    }
    if args.iter().any(|a| a == "--save-layout") {
        // Simpan layout (viewport/drawing/tema/watchlist) ke berkas — jalur yang sama
        // dengan on_exit GUI, supaya bisa diuji end-to-end tanpa window.
        let mut st = app::AppState::new();
        if let Some(p) = arg_value(&args, "--layout") {
            st.layout_path = p;
        }
        load_dataset_into(&mut st);
        if let Err(e) = st.restore_layout() {
            eprintln!("[sokanlab] layout tidak dipulihkan: {e}");
        }
        let out = arg_value(&args, "--save-layout").unwrap_or_else(|| "data/layout.json".into());
        st.layout_path = out.clone();
        match st.save_layout() {
            Ok(()) => println!("layout-tersimpan {out}"),
            Err(e) => {
                eprintln!("error: simpan layout: {e}");
                std::process::exit(2);
            }
        }
        return;
    }
    if args.iter().any(|a| a == "--headless") {
        // Rerender chart tanpa window: verifikasi lapar data + layout, lalu keluar.
        let mut st = app::AppState::new();
        if let Some(p) = arg_value(&args, "--layout") {
            st.layout_path = p;
        }
        load_dataset_into(&mut st);
        if let Err(e) = st.restore_layout() {
            eprintln!("[sokanlab] layout tidak dipulihkan: {e}");
        }
        println!("{}", st.chart.layout_json());
        println!("headless-ok bars={}", st.chart.bars.len());
        return;
    }
    log_redacted("open SokanLab desktop (offline, tanpa jaringan)");
    if let Err(e) = check_config() {
        log_redacted("config gagal");
        eprintln!("error: {e}");
        std::process::exit(2);
    }

    let mut pre = app::AppState::new();
    if let Some(p) = arg_value(&args, "--layout") {
        pre.layout_path = p;
    }
    if let Some(id) = load_dataset_into(&mut pre) {
        log_redacted(&format!("dataset aktif: {id}"));
    }
    if let Err(e) = pre.restore_layout() {
        log_redacted(&format!("layout tidak dipulihkan: {e}"));
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("SokanLab")
            .with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };
    let bench_left = arg_value(&args, "--bench-frames").and_then(|v| v.parse::<u64>().ok());
    let res = eframe::run_native(
        "SokanLab",
        native_options,
        Box::new(move |cc| {
            let mut sapp = app::SokanApp::new_with(cc, pre);
            if let Some(n) = bench_left {
                sapp.bench = Some(app::FrameBench::new(n));
            }
            Ok(Box::new(sapp) as Box<dyn eframe::App>)
        }),
    );
    if let Err(e) = res {
        log_redacted("GUI gagal dijalankan");
        eprintln!("error: gui: {e}");
        std::process::exit(3);
    }
    log_redacted("close normal");
    if args.iter().any(|a| a == "--smoke") {
        println!("smoke-ok version={APP_VERSION}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_text_limit_enforced() {
        let dir = std::env::temp_dir().join("sokanlab-import-limit-test");
        let _ = std::fs::create_dir_all(&dir);
        let p = dir.join("x.csv");
        std::fs::write(&p, "hello").unwrap();
        let ps = p.to_string_lossy().into_owned();
        assert!(read_text_limited_with(&ps, 4).is_err());
        assert_eq!(read_text_limited_with(&ps, 5).unwrap(), "hello");
        assert!(read_text_limited_with(&ps, 1024 * 1024).is_ok());
        // Berkas tidak ada → error yang jelas.
        assert!(read_text_limited_with(&format!("{ps}.hilang"), 1024).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn max_import_bytes_is_sane() {
        assert!(max_import_bytes() >= 1024 * 1024);
    }
}
