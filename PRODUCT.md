# PRODUCT.md — SokanLab (working title)

**Status:** draft identitas 2026-10-16
**Jenis:** aplikasi desktop personal, offline-first, open-source rencana Apache-2.0
**Fokus:** chart OHLCV lokal + backtest deterministik. Bukan feed live, bukan tiruan TradingView.

## 1. Nama

- **Working title:** `SokanLab` (dipilih 2026-10-16; alternatif yang ditolak: `RustCandle` karena risiko merek kata “Rust”)
- **Tagline:** Backtest offline milikmu sendiri.
- **Catatan merek:** `SokanLab` lebih aman daripada nama berunsur “Rust”. Sebelum publish tetap:
  1. Cek tabrakan App Store/crates.io/GitHub untuk “SokanLab”.
  2. Tambahkan disclaimer “Built with Rust. Not affiliated with the Rust Project.” bila menyebut Rust di deskripsi.
- Dilarang memakai kata TradingView, TV, Pine, atau meniru brand lain (termasuk Muse/Meta) pada nama, ikon, dan copywriting.

## 2. Masalah dan pengguna

Pengguna: satu orang yang sudah punya file data sah (CSV/Parquet pair) dan ingin lihat chart + uji strategi offline tanpa akun, tanpa jaringan wajib, tanpa telemetri.

Janji produk:
1. Impor file lokal → langsung bisa chart.
2. Backtest deterministik dengan asumsi fee/slippage yang terlihat.
3. Tetap offline penuh pada MVP.

Non-tujuan MVP: live feed, multi-aset portfolio, Pine Script, sinkron cloud, mobile.

## 3. Ciri khas (bukan AI-slop)

Prinsip kualitas setara ikon iOS premium (contoh acuan mutu: sederhana, solid, terbaca kecil — tanpa meniru bentuk/warna/logo Muse atau TradingView):

- **Satu mark abstrak:** dua persegi membulat yang digabung menjadi bentuk candle abstrak (body + wick sebagai satu siluet), digambar vektor dari nol.
- **Anti AI-slop:** tanpa mesh-gradient berlebihan, tanpa glow/stock-chart clipart, tanpa garis tipis yang hilang di 16px, tanpa teks di ikon, tanpa tiruan swirl/“Muse-like”.
- **Keterbacaan kecil:** siluet tetap jelas di 16–32px; uji di light/dark dan grayscale.
- **Bentuk app:** squircle iOS, safe-area 10%, varian light/dark + monochrome.
- **Palet orisinal (jangan pakai biru TradingView):**
  - Ink `#14181D` (background dark / teks light)
  - Paper `#F6F3EC` (background light)
  - Ember `#E8A33D` (aksen candle-bull, dipakai hemat)
  - Slate `#5B6570` (sekunder/volume)
- **Tipografi UI:** sans sistem + angka tabular/mono untuk harga; bukan font proprietary produk lain.
- **Nada copy:** Indonesia/Inggris sederhana, tanpa klaim “100% sama dengan TradingView”.

Deliverable ikon: SVG sumber + PNG 16/32/64/128/256/512/1024, light/dark/mono, file di `assets/brand/`.

Larangan keras: tidak menyalin aset/source-map/screenshot TradingView; tidak menyalin bentuk/warna/gradasi logo Muse/Meta; semua aset digambar mandiri dan dicatat pembuatnya.

## 4. Lisensi: Apache-2.0 (bukan MIT)

Keputusan user 2026-10-16: pakai **Apache-2.0**.

Konsekuensi:
- Boleh pakai/fork komersial, modifikasi, distribusi dengan syarat: sertakan `LICENSE` + `NOTICE`, nyatakan perubahan file yang diubah, pertahankan atribusi.
- Ada pemberian paten eksplisit + terminasi bila gugat paten; ada klausul tanpa garansi.
- Kompatibel dengan banyak crate Rust, tapi tiap dependensi tetap dicek (hindari GPL/copyleft kuat di binary closed bila kelak didistribusikan tertutup; untuk full open-source ini lebih longgar).
- Perlu `LICENSE` (teks Apache-2.0 penuh) + `NOTICE` (holder + tahun) sebelum publish. Holder: `ahmdd4vd (2026)`. Ganti bila pakai nama legal lengkap.

## 5. Scope MVP terkait produk

- Impor CSV/Parquet offline, katalog lokal SQLite + data Parquet.
- Chart 1 panel: candlestick, volume, crosshair, zoom/pan/fit.
- Indikator: SMA, EMA, RSI, volume (formula publik, terdokumentasi).
- Drawing dasar: horizontal/vertical line, trendline, ray, rectangle; undo/redo.
- Backtest 1 instrumen/run: MA-cross, RSI-threshold, buy-and-hold, SL/TP; ledger order/fill; equity/drawdown; ekspor JSON/CSV.
- Tema terang/gelap orisinal; shortcut dan label orisinal.

## 6. Langkah berikut

1. Isi holder + tahun Apache-2.0, tambah `LICENSE`/`NOTICE`.
2. Finalisasi Tahap 0 di plan: format CSV, timeframe, ukuran data tipikal, model fee/slippage.
3. Buat sketsa ikon vektor orisinal (3 varian) lalu uji 16px.
4. Scaffold workspace Cargo sesuai `tradingview-rust-cleanroom-plan.md`.
