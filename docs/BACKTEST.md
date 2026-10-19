# Asumsi Backtest SokanLab

Dokumen ini mendefinisikan **semua** asumsi mesin backtest supaya hasil dapat
diaudit dan direproduksi. Semua aritmetika metrik dan akuntansi memakai integer
(tanpa `f64`), sehingga determinisme tidak bergantung pada arsitektur FPU.

## 1. Model eksekusi

| Peristiwa | Aturan |
|---|---|
| Sinyal strategi di bar `t` | Dihitung dari data bar `t` (setelah close), lalu **disimpan pending** |
| Fill | Terjadi di **harga open bar `t+1`** — tidak pernah di bar sinyal (anti look-ahead) |
| Slippage | `slippage_ticks` (default 1) tick melawan arah: beli +1, jual −1 |
| Fee | `floor(notional_ticks × fee_bps / 10_000)` tick, dibebankan saat fill |
| Stop / target | Dicek intrabar memakai `low`/`high` bar; **stop diutamakan** bila keduanya tersentuh di bar yang sama |
| Akhir data | Posisi terbuka ditutup di harga **close bar terakhir** (`exit_reason: end_of_data`) |
| Equity | Mark-to-market setiap close bar: `cash + qty × close` |

Saat reverse posisi (sinyal berlawanan), engine menutup posisi lama lebih dulu
di fill yang sama (`exit_reason: fill`), lalu membuka posisi baru.

Strategi `sl_tp` memakai dua lapis: sinyal strategi (fill open berikutnya) **dan**
stop/target intrabar milik engine. Kolom `exit_reason` tiap trade menunjukkan
lapisan mana yang keluar: `fill`, `stop_loss`, `take_profit`, `end_of_data`.

## 2. Unit & skala integer

| Besaran | Unit | Catatan |
|---|---|---|
| Harga | tick (`*_ticks`, integer) | Input CSV langsung dipakai; tidak ada konversi tersembunyi |
| Kuantitas | milli-unit (`qty_milli`) | 1000 = 1 unit |
| Volume | milli (`volume_milli`) | dari CSV apa adanya |
| Return/metrik rasio | basis poin (`_bps`) | 10 000 = 100 % |
| Sharpe | mili (`sharpe_milli`) | 1000 = 1,0 |

Semua pembagian **truncate ke arah nol**. Konsekuensinya nilai persentase sangat
kecil dapat terbaca `0` (mis. drawdown < ~1 bps), dan fee kecil pada notional
kecil bisa `floor` menjadi 0 tick.

## 3. Definisi metrik (`metrics.csv`)

| Metrik | Definisi |
|---|---|
| `bars` | Jumlah bar yang diproses |
| `trades` | Jumlah trade bulat (pasangan buka–tutup) |
| `wins` / `losses` | `pnl_ticks > 0` / `< 0` (n=0 netral tidak dihitung keduanya) |
| `win_rate_bps` | `wins × 10 000 / trades` (0 bila tanpa trade) |
| `total_return_bps` | `(final_equity − initial_cash) × 10 000 / initial_cash` |
| `max_drawdown_bps` | Peak-to-trough equity berbasis close bar: `max((peak − equity) × 10 000 / peak)` |
| `exposure_bps` | `bars_in_market × 10 000 / bars` |
| `fees_total_ticks` | Akumulasi seluruh fee |
| `avg_pnl_ticks` | Rata-rata `pnl_ticks` per trade (truncate) |
| `sharpe_milli` | **Per-bar, belum annualized** — lihat bawah |
| `initial_cash_ticks` / `final_equity_ticks` | Uang awal & equity akhir |

**Sharpe per-bar**: dari kurva equity, return tiap bar dihitung dalam mili-bps
(`(cur − prev) × 10_000_000 / prev`), lalu `mean × 1000 / std` dengan **varians
populasi** dan akar via `u128::isqrt`. `sharpe_milli / 1000` = Sharpe per-bar
(1000 → 1,0). Tidak ada faktor annualisasi karena timeframe 1m/1h/1D membutuhkan
faktor berbeda; kalikan sendiri sesuai horizon bila perlu (mis. √(bar/tahun)).

## 4. Berkas laporan (`out/backtest/`)

| Berkas | Isi |
|---|---|
| `metrics.csv` | Blok `metric,value` + string `assumptions` yang sama dengan yang tersimpan di `report.json` |
| `ledger.csv` | `bar,ts_millis,kind,side,price_ticks,reference_ticks,qty_milli,fee_ticks,cash_after_ticks,qty_after_milli` |
| `equity.csv` | `bar,ts_millis,equity_ticks,cash_ticks,qty_milli,close_ticks` (satu baris per bar) |
| `trades.csv` | `entry_bar,exit_bar,entry_ticks,exit_ticks,qty_milli,pnl_ticks,exit_reason` |
| `report.json` | Metrik + asumsi + daftar trade (mesin-baca) |

`pnl_ticks` tiap trade sudah bersih dari fee masuk dan keluar
(`notional(exit − entry) − fee_entry − fee_exit`), belum termasuk slippage
karena slippage sudah terwujud di harga fill.

## 5. Kebijakan data & determinisme

- Bar hilang **tidak diinterpolasi**; keputusan berbasis indeks bar
  (uji `time_gaps_do_not_change_decisions`).
- Timestamp wajib naik ketat saat impor (duplikat/mundur ditolak).
- Tidak ada RNG, tidak ada jam dinding, tidak ada I/O jaringan di engine —
  dua run dengan input & parameter sama menghasilkan laporan byte-identik
  (`diff -r` diuji pada Fase 4).
- Dataset dibaca sekali ke memori (`Vec<Bar>`); target desain 10k–1M bar.

## 6. Yang **tidak** dimodelkan (batasan MVP)

- Funding rate, spread bid/ask terpisah, market impact, partial fill antrean.
- Slippage dinamis per ukuran order (hanya tick tetap).
- Biaya borrowing/short (posisi jual juga dihitung aritmetika simetris).
- Dividen, split, atau penyesuaian kontrak.
- Jam bursa/DST: waktu hanya label integer milidetik.

## 7. Reproduksi cepat

```bash
./target/release/sokanlab-desktop backtest \
  --parquet /tmp/opencode/SYNTHUSDT-1m.parquet \
  --strategy ma_cross --fast 9 --slow 21 \
  --cash 1000000 --qty 1000 --fee-bps 10 --slippage 1 \
  --out /tmp/opencode/run-a
./target/release/sokanlab-desktop backtest --parquet /tmp/opencode/SYNTHUSDT-1m.parquet \
  --strategy ma_cross --fast 9 --slow 21 --out /tmp/opencode/run-b
diff -r /tmp/opencode/run-a /tmp/opencode/run-b && echo "identik"
```
