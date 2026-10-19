# SokanLab

Chart OHLCV lokal + backtest deterministik. Desktop Rust, **offline penuh** — tanpa
jaringan, tanpa akun, tanpa endpoint pihak ketiga. Data sepenuhnya milik pengguna
(CSV/Parquet), hasil backtest bisa direproduksi bit-per-bit.

> Status: MVP fungsional (Fase 0–5 selesai; Fase 6 batch 1–8 — parity alat gambar
> (33 alat: +regression/Gann/Elliott/pola/goresan/sticker), properti objek,
> skala/overlay/snapshot, multi-seleksi, varian channel, katalog template gaya,
> ukuran & perataan teks, klon `Ctrl+seret` — selesai 2026-10-19). Linux terverifikasi di environment
> pengembangan. Preview Windows x86_64 (paket portable) sudah di-cross-compile + di-smoke-test
> via Wine — lihat `docs/WINDOWS-PREVIEW.md` — tetapi **belum dijalankan di Windows asli**;
> macOS belum diuji. Lihat `ROADMAP.md` untuk status jujur per fase.

Unduh preview: [Releases](https://github.com/ahmdd4vd/SokanLab/releases) — `v0.1.0-preview`.

Bukan tiruan TradingView: SokanLab hanya mengambil **kebutuhan** (charting + backtest),
bukan kode, aset, brand, atau format skripnya. Semua visual, nama, dan bagian dalam
adalah karya orisinal (lihat `PRODUCT.md` dan `NOTICE`).

## Fitur

- Impor CSV → validasi → Parquet + katalog SQLite + provenance SHA-256 (`import`)
- Chart candle + volume + crosshair, zoom/pan/fit, **22 alat gambar** (termasuk tiga
  channel — paralel/disjoint/flat dengan opsi **Extend**, pitchfork varian, fib
  extension/time, teks/callout) + magnet 3 tingkat + edit anchor/badan + properti objek
  (warna/tebal/pola garis/opasitas/nama/kunci/sembunyi) + **level fib kustom**
  (tambah/hapus/persen/warna per level) + **katalog template gaya bernama** +
  object tree dengan z-order/duplikat/grup, undo/redo, **skala log/persen/invert + drag
  sumbu**, **strip sumbu harga/waktu berlabel**, **snapshot PNG (`Alt+S`, resolusi
  1×/2×/4×)**, pintasan keyboard ala TradingView (GUI eframe)
- Indikator inkremental: SMA, EMA, RSI (Wilder, presisi milli), MACD (presisi milli); **overlay SMA/EMA/VWAP jangkar** di atas candle + **pane terpisah RSI/MACD** (maks 3 pane)
- Backtest deterministik: sinyal bar `t` → fill di open bar `t+1`; fee + slippage;
  stop/target intrabar; ledger order/fill lengkap
- Metrik: return, drawdown, Sharpe per-bar, win rate, exposure, equity curve
- Ekspor laporan 5 berkas (`metrics.csv`, `ledger.csv`, `equity.csv`, `trades.csv`, `report.json`)
- Persistence layout (viewport, drawing, template gaya, tema, watchlist) lewat JSON berversi
- Audit offline otomatis: `scripts/check-offline.sh`

## Mulai cepat

Prasyarat: Rust stabil (edisi 2021). Tidak ada prasyarat jaringan saat build
(crate diambil dari registry sekali saat build pertama; runtime aplikasi tidak
melakukan koneksi apa pun).

```bash
cargo build --release

# 1) Impor CSV ke Parquet + katalog
./target/release/sokanlab-desktop import \
  --file fixtures/valid-3bars.csv \
  --symbol BTC/USDT --timeframe 1m \
  --db data/catalog.db --out data/BTCUSDT-1m.parquet

# 2) Backtest CLI (deterministik, offline)
./target/release/sokanlab-desktop backtest \
  --parquet data/BTCUSDT-1m.parquet \
  --strategy ma_cross --fast 9 --slow 21 \
  --out out/backtest

# 3) GUI
./target/release/sokanlab-desktop
```

Format CSV MVP (header wajib, timestamp epoch detik atau milidetik):

```csv
timestamp,open,high,low,close,volume
1700000000,100,110,90,105,1000
1700000060,105,115,95,110,1200
```

Aturan validasi: nilai integer, OHLC konsisten, volume ≥ 0, waktu naik ketat
(duplikat & mundur ditolak), kolom kosong ditolak dengan posisi baris/kolom.

## Perintah CLI

| Perintah | Keterangan |
|---|---|
| `import --file <csv> [--symbol BTC/USDT] [--timeframe 1m] [--db ...] [--out ...]` | Impor CSV → Parquet + katalog + provenance |
| `backtest [--csv <f> \| --parquet <f>] --strategy <k> --out <dir>` | Backtest offline + ekspor 5 berkas laporan |
| `--render --out <png> --bars N --size WxH` | Render chart headless ke PNG (tanpa display) |
| `--headless [--layout <f>]` | Muat dataset + layout, cetak layout JSON (diagnostik) |
| `--save-layout <f> [--layout <f>]` | Simpan layout chart ke berkas (jalur yang sama dengan GUI) |
| `--bench-frames N` | Ukur frame-time GUI: repaint kontinu N frame, cetak avg/p50/p95 ms |
| `--smoke` | Jalankan GUI lalu cetak `smoke-ok` |
| `--version` | Versi + schema config |

Strategi: `buy_and_hold`, `ma_cross` (`--fast`, `--slow`), `rsi_threshold`
(`--rsi-period`, `--oversold`, `--overbought`), `sl_tp` (`--stop`, `--target`).
Biaya: `--fee-bps` (default 10) dan `--slippage` tick (default 1).

## Alat gambar & pintasan keyboard

33 alat: Trend Line, Ray, Extended Line, Horizontal Line, Horizontal Ray, Vertical
Line, Crossline, Arrow, Rectangle, Ellipse, Triangle, Fib Retracement (level kustom
opsional), Date & Price Range, Long/Short Position, Parallel Channel, **Disjoint
Channel** (batas kedua boleh bergeser di sumbu waktu), **Flat Top/Bottom Channel**
(batas kedua horizontal), **Regression Channel** (regresi kuadrat-terkecil atas close
+ batas ±σ), Pitchfork (varian Andrew/Schiff/Modified Schiff/Inside), Fib Extension,
**Fib Trend Extension** (level dari titik ketiga ke kanan), Fib Time, **Elliott
Impulse (label 1–5) & Elliott Correction (A–C)**, **XABCD**, **ABCD**, **Gann Fan**
(9 rasio, 1×1 = a→b), Text, Callout, **Brush**, **Highlighter**, **Sticker** (14 ikon
vektor) (+ Measure kilat). Penempatan lewat klik-klik atau seret-satu-kali (pratinjau
putus-putus); **Brush & Highlighter digambar dengan seret bebas** (titik disampel
saat pointer bergerak, tanpa magnet; highlighter tebal dan tembus pandang), **Sticker**
cukup sekali klik; magnet Off/Weak/Strong; `Shift` mengunci horizontal/vertikal/45°;
klik kanan membuka menu; handle & badan objek bisa digeser; **`Ctrl`/`⌘`+seret pada
objek = klon lalu geser** (konvensi TV).

Setiap objek punya panel properti di panel kiri: **nama kustom, 8 warna, tebal 1–4,
pola garis (solid/putus/titik), opasitas 0–100%, varian pitchfork, editor level fib
(tambah/hapus, geser % dengan satuan 0,1%, warna per level atau otomatis), isi teks
(Text/Callout), toggle tampil & kunci, z-order (ke depan/belakang), duplikat, grup**
(drag badan objek bergrup menggeser semua anggotanya). Untuk channel ada **kombo
varian** (Parallel/Disjoint/Flat Top-Bottom) dan **dua centang Extend (kiri/kanan)**
— default tidak diperpanjang, sama seperti TV; garis channel digambar sebagai segmen
yang benar-benar terlihat (satu-satunya bagian yang bisa diklik). Dua tombol template:
**"Jadikan default"** (objek baru memakai gaya itu) dan **"Terapkan ke sejenis"**
(samakan gaya ke semua objek jenis sama).

**Katalog template gaya** (batch 7): simpan warna/tebal/pola/opasitas objek primer
sebagai nama (maks 32 karakter; 16 entri; nama sama = diperbarui) lewat kolom nama +
tombol **Simpan**, lalu **Terapkan** ke seluruh objek terpilih atau **Hapus** dari
katalog. Template ikut tersimpan di layout. Daftar objek (object tree) menampilkan
semuanya dan ikut tersimpan di layout (`schema:7`; berkas v1–v6 tetap termuat).

**Teks lanjutan** (batch 8): Text/Callout punya pilihan **ukuran S/M/L** (12/16/22 px)
dan **perataan kiri/tengah/kanan** per objek — keduanya tersimpan di layout dan
menyesuaikan kotak label maupun label titik pola lainnya.

**Seleksi banyak** (batch 6, konvensi mouse TradingView): `Ctrl`/`⌘`+klik objek
menambah/melepas dari seleksi, `Ctrl`/`⌘`+seret di area kosong menggambar **kotak
seleksi** (objek yang tersentuh kotak ikut terpilih; objek tersembunyi/terkunci
dilewati). Semua objek terpilih ditandai ember dan bisa digeser bersama lewat drag
badan, dihapus sekaligus (`Del`, satu langkah undo), diubah z-order-nya (urutan
relatif tetap), diduplikat, digrup dengan satu id, serta diubah warna/tebal/pola/
opasitasnya sekaligus dari panel properti — panel menampilkan objek **primer**
(indeks teratas) yang memegang handle edit. Seleksi **tidak** ikut disimpan ke
layout (hanya berlaku selama sesi).

Skala harga: **Linear / Log / Persen** (basis = close bar terlihat pertama), **Invert**,
dan **drag sumbu** langsung (sumbu harga vertikal, sumbu waktu horizontal; dobel-klik =
reset). Snapshot PNG: `Alt+S` atau tombol di panel — disimpan ke
`data/snapshot-<epoch>.png` (area chart, tanpa panel samping) dengan pilihan resolusi
1×/2×/4× (render ulang pada skala piksel, dibatasi ~8K). Sumbu harga (kanan) dan sumbu
waktu (bawah) digambar berlabel; crosshair dan harga terakhir punya tag kotak seperti
di TradingView. Indikator: overlay SMA/EMA (periode 2–400) + VWAP jangkar di atas
candle, plus **pane RSI (0–100, panduan 30/50/70)** dan **pane MACD** (garis + sinyal +
histogram, skala simetris otomatis) di bawah volume — periode diatur dari panel.
Seri dihitung sekali saat data/setelan berubah.

| Pintasan | Aksi |
|---|---|
| `Alt+T` `Alt+H` `Alt+V` `Alt+C` `Alt+F` | Trend / Horizontal / Vertical / Crossline / Fib |
| `1`–`9` | Pilih alat (khusus SokanLab; TV memakai angka untuk timeframe) |
| `Ctrl+Z` / `Ctrl+Y` / `Ctrl+Shift+Z` | Undo / Redo |
| `Ctrl+Alt+H` | Sembunyikan/tampilkan semua gambar |
| `Alt+R` | Reset tampilan (fit seluruh data) |
| `Alt+G` | Pergi ke tanggal (dialog: `YYYY-MM-DD[ HH:MM]` atau epoch) |
| `Alt+S` | Snapshot chart → `data/snapshot-<epoch>.png` |
| Drag sumbu harga / waktu | Skala vertikal / horizontal (dobel-klik = reset) |
| Dobel-klik sumbu harga / waktu | Fit ulang skala harga / rentang waktu penuh |
| `Ctrl+S` | Simpan layout |
| `←` `→` / `Ctrl+←` `Ctrl+→` | Geser 1 / 10 bar |
| `Ctrl+↑` `Ctrl+↓` / scroll | Zoom in/out (scroll = zoom di kursor) |
| `Shift` + scroll | Geser horizontal |
| `Esc` / `Del` | Batal berjenjang / hapus semua objek terpilih |
| `Ctrl`+klik / `Ctrl`+seret | Multi-seleksi objek / kotak seleksi (konvensi mouse TV) |
| `F1` | Dialog pintasan |

Perbandingan jujur dengan TradingView (apa yang sama, apa yang belum, rencana
batch berikutnya) ada di [`docs/TV-PARITY.md`](docs/TV-PARITY.md).

## Konfigurasi & variabel lingkungan

| Variabel | Default | Fungsi |
|---|---|---|
| `SOKANLAB_LAYOUT` | `data/layout.json` | Lokasi layout chart (restart persistence) |
| `SOKANLAB_MAX_IMPORT_MB` | `1024` | Batas ukuran berkas teks impor (MB) |
| `SOKANLAB_CONFIG` | `config/default.toml` | Berkas config (`schema_version = 1`) |

Batas aman tambahan: satu impor maksimum 20 juta baris (`MAX_ROWS` di `data-import`),
supaya input raksasa gagal cepat alih-alih kehabisan memori.

## Persistence & determinisme

- **Layout** disimpan saat aplikasi ditutup normal (`on_exit`) dan dapat dimuat ulang
  dari disk; format JSON berversi (`schema: 6` — skala/invert, overlay, pane
  indikator, gaya/pola garis, grup, teks, opasitas, level fib kustom, varian
  pitchfork, gaya default, varian channel + extend, katalog template), kompatibel
  mundur: berkas `schema: 1`–`5` termuat dan dinaikkan otomatis (metadata baru
  berdefault aman).
- **Katalog** SQLite berversi; DB dari versi SokanLab yang lebih baru ditolak dengan
  pesan jelas (mencegah korupsi), dan katalog lama dimigrasi otomatis ke versi 1.
- **Backtest** tidak memakai RNG dan hanya memakai aritmetika integer ± presisi milli
  untuk indikator; dua run dengan input sama menghasilkan laporan identik
  (`diff -r` teruji). Asumsi lengkap: [`docs/BACKTEST.md`](docs/BACKTEST.md).

## Workspace

| Crate | Isi |
|---|---|
| `apps/desktop` | Biner: GUI eframe + CLI |
| `crates/domain` | Bar + validasi |
| `crates/data-import` | Parser CSV + fuzz-lite |
| `crates/data-store` | Parquet, katalog SQLite berversi, sha256, provenance |
| `crates/chart` | Viewport, candle, drawing + hit-test |
| `crates/indicators` | SMA/EMA/RSI/MACD/VWAP inkremental |
| `crates/backtest` | Engine ledger/equity/metrik |
| `crates/strategy-api` | Kontrak strategi + 4 strategi bawaan |
| `crates/ui` | Theme, painter, runner worker, layout JSON |
| `crates/export` | 5 berkas laporan |

## Pengembangan

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace            # termasuk fuzz-lite deterministik
scripts/check-offline.sh          # audit 0 socket + smoke impor/backtest/render
scripts/package.sh                # dist tar.gz + sha256
```

Benchmark (import/backtest/render, 1M bar) dan profil memori dicatat di
[`docs/BENCH.md`](docs/BENCH.md). Pihak ketiga & lisensinya: [`THIRD-PARTY.md`](THIRD-PARTY.md).

## Batasan jujur (MVP)

- Sharpe dihitung **per-bar** dan belum annualized (timeframe 1m/1h/1D butuh faktor berbeda).
- Persentase metrik dalam basis poin integer (truncate); nilai sangat kecil bisa terbaca 0.
- Deteksi `gaps` antar-timeframe saat impor masih statis 0 (angka coverage; eksekusi tetap bar-apa-adanya).
- Renderer memakai jalur CPU egui; jalur GPU ditunda sampai profiling menuntut.
- Parity TradingView baru sebagian: 22 alat inti (channel lengkap: paralel/disjoint/
  flat + Extend) + 10 alat lanjutan (regression, fib trend extension, Elliott,
  XABCD/ABCD, Gann fan, brush, highlighter, sticker) + properti objek lanjutan
  (opasitas, level fib kustom, varian pitchfork, gaya default, katalog template
  bernama, ukuran & perataan teks) + object tree + grup/z-order + skala/sumbu
  berlabel + pane RSI/MACD + multi-seleksi (Ctrl+klik & kotak seleksi) + klon
  `Ctrl+seret` + hotkey TV-inti (~95% alur kerja inti, ~50% katalog alat TV); sisa
  (projection/forecast, Gann box, pola harmonic lain, template per jenis alat,
  editor teks lebih dalam) menyusul — detail + roadmap di
  [`docs/TV-PARITY.md`](docs/TV-PARITY.md).
- Multi-seleksi: kotak seleksi memilih objek yang **tersentuh** kotak (anchor di dalam
  atau segmen memotong); untuk garis tak-hingga (trendline yang diperpanjang) hanya
  bagian segmen antar-anchor yang dinilai. `Ctrl`+seret **pada objek** = klon lalu
  geser (batch 8); klon tidak mewarisi grup, objek terkunci dilewati.
- Regression channel menghitung regresi **per frame** atas seluruh bar di rentangnya
  (O(rentang), terukur ~1,85 ms per 600k bar) — rentang raksasa terasa di benda ini;
  rencana prefix-sum ada di batch 9.
- Snapshot headless menggambar ulang area chart dari state (bukan tangkapan GPU/window);
  panel samping tidak ikut, dan teks (termasuk label sumbu) dirender sebagai kotak pada
  raster headless — jalur font bertekstur belum diimplementasi (GUI normal menampilkan
  teks).
- Overlay hanya SMA/EMA/VWAP jangkar; VWAP dihitung dari bar pertama dataset (bukan
  sesi harian). Label sumbu waktu memakai UTC (bukan zona waktu lokal).
- Pane indikator: RSI & MACD saja (maks 3 pane), periode dari panel; skala MACD
  otomatis/simetris dan belum bisa dikunci manual; warna indikator tetap (belum bisa
  diubah pengguna).
- Varian pitchfork memakai definisi publik (median Andrew A→titik tengah B–C; Schiff
  origin = waktu A & harga rata-rata A–B; Modified Schiff = titik tengah A–B; Inside =
  titik tengah A–B → C); garis tine selalu sejajar median. Level fib kustom disimpan
  per objek; "Kembali bawaan" mengembalikan 7 level standar (0–100%).
- Template gaya bernama sudah ada (batch 7) tetapi **generik** (gaya + opasitas, bisa
  ke jenis alat apa pun) dan tanpa dialog per-tab seperti TV yang menempatkan template
  di dalam pengaturan alat; opsi **Extend** hanya untuk channel (TV juga punya Extend
  di alat garis lain; SokanLab memakai alat Ray/Extended Line untuk kebutuhan itu).
- Seleksi banyak tidak ikut disimpan ke layout (seperti TV) dan handle edit hanya
  tampil pada objek primer (indeks teratas) — anggota lain ditandai warna ember.
- Dataset 1M bar ≈ 668 MB RSS (seluruh dataset di memori); target desain 10k–1M bar.
- Layout disimpan saat penutupan normal; kill paksa tidak menyimpan.
- Windows/macOS: paket preview Windows x86_64 sudah di-cross-compile (mingw-w64) dan
  di-smoke-test via Wine (`docs/WINDOWS-PREVIEW.md`), tetapi belum dijalankan di Windows
  asli; macOS belum diuji; CI matrix 3-OS belum dieksekusi di GitHub.

## Lisensi

Apache-2.0 — lihat [`LICENSE`](LICENSE) dan [`NOTICE`](NOTICE).
Holder copyright: `ahmdd4vd (2026)`. Aset brand di `assets/brand/` adalah karya
orisinal SokanLab.
