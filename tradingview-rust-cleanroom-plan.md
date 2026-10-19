# Rencana: Aplikasi Charting dan Backtesting Rust lintas-platform

**Status:** revisi disepakati 2026-10-16 — belum ada kode aplikasi yang dibuat
**Tujuan:** aplikasi desktop personal (rencana open-source nanti), local-first/offline-first, cepat, berjalan di Windows/macOS/Linux; fokus backtest + chart pasar tanpa akun aplikasi. Data disuplai pengguna (CSV/Parquet yang diperoleh pengguna secara sah); MVP tidak wajib memakai jaringan.

## 1. Tujuan dan batas proyek

Buat aplikasi baru yang menawarkan **kategori kemampuan serupa** aplikasi charting desktop: chart candlestick, watchlist, beberapa tab/panel, indikator, drawing sederhana, data lokal, serta backtest strategi. Implementasi, desain visual, nama, ikon, teks, dan aset harus dibuat mandiri.

“Tanpa login” berarti aplikasi ini tidak meminta akun TradingView maupun akun milik aplikasi baru. Ini **bukan** cara melewati login, langganan, pembatasan, atau autentikasi layanan TradingView. Mode chart/backtest MVP bekerja penuh offline dari file lokal yang disuplai pengguna (CSV/Parquet pair yang diperoleh secara sah dan diunduh pengguna di luar aplikasi). Adapter unduhan provider publik bersifat opsional pasca-MVP, opt-in eksplisit, dan hanya memakai endpoint publik terdokumentasi tanpa key bila didukung.

### Yang tidak ditiru atau disertakan

- Source code, source map, aset, ikon, bunyi, teks, branding, atau tampilan piksel-per-piksel TradingView.
- Endpoint/API internal, cookie/session, autentikasi, langganan, atau feed data TradingView.
- Klaim kompatibilitas Pine Script atau salinan sintaks/engine proprieternya.
- Pengiriman data/telemetri ke layanan tanpa opt-in yang jelas.

Gunakan REA sebagai bahan memahami **kelas fitur dan batas teknis**, bukan sebagai sumber materi untuk disalin. Targetnya adalah **kesetaraan kemampuan yang diprioritaskan untuk backtest offline**, bukan clone 100% atau pixel-perfect. Tata letak, warna, ikon, tipografi, label, shortcut, detail interaksi, dan aset harus dirancang mandiri. Jangan menggunakan screenshot/source map/source code TradingView sebagai blueprint visual maupun menyalin implementasinya. REA untuk OpenCode sudah teregistrasi 2026-10-16 (`configured_opencode`, skill v42; provider Ghidra/Hopper/JADX sehat); REA Claude Code terdeteksi stale.

### Batas kemiripan dan target kesetaraan fitur

Secara teknis kita dapat membuat charting app yang terasa familiar dan punya banyak alat sejenis—candlestick, crosshair, watchlist, indikator, drawing tools, multi-panel, serta backtest. Tetapi menjanjikan seluruh UI dan perilaku TradingView “100% sama” tidak realistis dan bukan target clean-room yang aman. Produk asal terus berubah, sebagian fiturnya bergantung pada akun, server, lisensi data, dan perilaku runtime yang tidak terungkap dari analisis statis. Kita juga sengaja tidak menyalin aset, merek, endpoint privat, atau detail desain yang khas.

Target penerimaan: **fitur inti yang disebut di bawah punya workflow lengkap dan diuji**, dengan tampilan orisinal. Tools grafik umum boleh menghasilkan konstruksi matematika yang sama sesuai definisi publik (mis. garis horizontal pada harga tertentu), tetapi ikon, panel, susunan toolbar, shortcut default, animasi, pilihan warna, dan micro-interaction dibuat sendiri. Untuk fungsi proprietary/berbasis server, cari alternatif mandiri atau tandai tidak didukung—jangan menyamar sebagai TradingView.

## 2. Sasaran pengguna dan hasil

Pengguna utama: satu orang (pemakaian pribadi; rencana open-source nanti) yang menganalisis data pasar yang telah mereka peroleh secara sah, menyusun chart, lalu menguji ide strategi secara offline.

Hasil yang diinginkan:

1. Membuka data CSV dan Parquet milik pengguna tanpa akun dan tanpa jaringan.
2. Menyimpan data impor secara lokal agar chart dan backtest berikutnya bisa berjalan offline.
3. Melihat candlestick dan volume dengan interaksi cepat pada dataset besar.
4. Membuat watchlist lokal, beberapa layout/tab, indikator umum, dan anotasi.
5. Menjalankan backtest yang deterministik, transparan soal biaya/asumsi, serta menghindari look-ahead bias.
6. Mengekspor hasil dan grafik ke format terbuka.
7. Membangun binary native untuk Windows, macOS, dan Linux dari satu workspace Rust.

## 3. Baseline REA yang memengaruhi rancangan

Observasi berikut berasal dari analisis statis paket, bukan eksekusi aplikasi:

- MSIX yang diperiksa berukuran 168,047,410 byte, SHA-256 `fb9ac708c7dd4e82e563a75f678b79ddba6baf6b92cda7552b375ddad50ef84e`. Manifest menyatakan `TradingView.Desktop` versi `3.4.1.8194`, x64. Evidence inventory: `ev_7fe156cbdd317afe7b3959267045d73606bb3fdf5619ff1845e76ee3307b0bef`.
- Paket berisi Electron/Chromium, ASAR, preload, beberapa native addon, resource UI dan banyak bundle/source map. ASAR SHA-256 `a70a18bfd8f1db8d113993709d8294344545a29b3ccf7f7c3436667907023906`; ringkasan inventory: `ev_536ef23b5884aeef01756cabca2974fd49166907c949d438b28c2c56d0bf88c6`.
- Preload statis menunjukkan API renderer–main yang luas; konfigurasi keamanan berbeda antar jenis jendela. Ini mendukung keputusan membuat boundary antarmodul yang eksplisit, bukan meniru implementasi tersebut. Evidence analisis preload terbatas: `ev_97bb9a0fbb721f7d4d9ca295ff1283ae50fd09aba560395ac3b998337250f058`.
- Fuse Electron yang teramati mencakup beberapa opsi runtime/Inspector aktif dan validasi ASAR tertanam tidak aktif. Ini bukan kesimpulan bahwa aplikasi rentan, tetapi menguatkan keputusan untuk membangun paket sendiri dengan hardening dan supply-chain hygiene. Evidence: `ev_57fef24dc30deb03b4762d51c84732d3d04c85576dd80fdf43cb5054481efc66`.
- Ghidra tidak menyelesaikan import `TradingView.exe` besar; sebagian analisis JavaScript juga parsial. Detail perilaku native maupun runtime produk asal tetap unknown. Karena itu, rancangan ini tidak mengasumsikan kesetaraan internal atau perilaku yang belum terbukti.

## 4. Arsitektur yang direkomendasikan

### Desktop/UI

- **Rust workspace**; UI native dengan `eframe`/`egui` dan renderer `wgpu`.
- Tidak memakai Electron atau frontend JavaScript. Semua UI, model data, chart, indikator, serta backtester ditulis dalam Rust.
- Implementasikan candlestick renderer khusus (GPU batching/instancing atau jalur CPU untuk mode sederhana); jangan mengandalkan plot generik sebelum benchmark.
- Struktur dock/panel menggunakan crate egui docking yang kompatibel; simpan layout sebagai schema JSON berversi.
- Dialog file dan lokasi data melalui crate Rust lintas-platform; abstraksikan perbedaan OS di satu modul platform.

### Core dan crate workspace

```text
apps/desktop/             entry point, lifecycle, menu/window
crates/domain/            Symbol, Bar, Timeframe, Strategy, Order, Position
crates/data-import/       CSV/Parquet parsing, validasi, normalisasi timezone
crates/data-store/        metadata SQLite, cache/data Parquet
crates/chart/             transform viewport, candle renderer, crosshair
crates/indicators/        indikator murni + state incremental
crates/backtest/          event loop, fills, portfolio, metrics
crates/strategy-api/      built-in strategy model dan kontrak plugin tahap lanjut
crates/ui/                view-model, panels, commands, user interaction
crates/export/            CSV/JSON/PNG/SVG export
```

Prinsip: domain/core tidak mengetahui egui atau OS. UI mengirim command dan menerima state/event. Data provider hanya mengimplementasikan trait yang terdokumentasi; modul provider tidak dapat membaca keychain atau file lain tanpa izin eksplisit.

### Data dan storage

- Format internal OHLCV berkolom/SoA agar filtering dan chart sampling efisien; metadata dataset menyimpan provider, venue/bursa, simbol asli, timeframe, zona waktu, satuan harga, waktu pengambilan, lisensi/ketentuan yang ditampilkan provider, serta rentang tanggal.
- Timestamp disimpan UTC dengan presisi eksplisit; zona waktu bursa hanya untuk tampilan.
- Harga/biaya transaksi memakai representasi desimal/tick yang eksplisit, bukan `f64` untuk pembukuan. Konversi ke floating-point hanya di layer render.
- SQLite untuk katalog, watchlist, layout, konfigurasi kecil, riwayat impor dan status cakupan data; Parquet untuk dataset besar. CSV sebagai format impor/ekspor dasar.
- MVP offline: tidak ada adapter jaringan wajib. Impor berjalan lewat worker dengan progress, pembatalan, validasi urutan waktu, duplikasi, nilai kosong, batas memori, serta penulisan atomik (file sementara lalu commit katalog setelah sukses).
- Adapter provider publik (opsional pasca-MVP) wajib terpisah dan eksplisit bila kelak ditambahkan: memakai endpoint publik terdokumentasi tanpa key, menghormati rate limit/ketentuan/redistribusi, pagination sopan, `429`/`Retry-After` dipatuhi, checkpoint/resume aman. Bukan scraping halaman TradingView, endpoint internal, websocket privat, cookie, atau mekanisme anti-bot.
- Data tidak diubah diam-diam: simpan provenance/import report, parameter dan versi adapter, waktu pengambilan, checksum, opsi zona waktu/unit, status cakupan, gap, serta license note. Dataset lama dapat di-refresh secara eksplisit; jangan timpa tanpa konfirmasi.

### Backtest

- Engine deterministik, berjalan di luar UI thread. Model input immutable; event/ledger hasil dapat direproduksi.
- Strategi MVP berupa blok/parameter built-in: moving-average cross, RSI threshold, buy-and-hold, stop-loss/take-profit. Strategi mengonsumsi data sampai waktu `t`; keputusan pada penutupan bar `t` paling cepat diisi di bar `t+1` kecuali model eksekusi lain dipilih eksplisit.
- Order, fill, cash, posisi, fee dan slippage dicatat dalam event ledger. Tetapkan mata uang, lot size, tick size, fee model dan asumsi gap.
- Stop dan target yang sama-sama tersentuh dalam satu candle memiliki aturan konservatif yang didokumentasikan; jangan memilih urutan menguntungkan tanpa data intrabar.
- Metrik: total/annualized return bila periode memadai, max drawdown, volatilitas, Sharpe dengan asumsi yang ditampilkan, win rate, trade count, exposure, fee total, equity curve, dan daftar transaksi.
- Uji bebas look-ahead, batas tanggal, indikator warm-up, bar kosong/duplikat, strategi tanpa transaksi, dan konsistensi hasil serial vs paralel.

### Keamanan dan privasi

- Default pengolahan lokal dan offline; tidak ada akun aplikasi, telemetry, crash upload, atau auto-update pada MVP. MVP tidak membuka socket jaringan.
- Impor lokal hanya atas tindakan pengguna; validasi ukuran file, format, decompression limits, dan nama path. Jangan menjalankan script dari dataset.
- Error/log tidak boleh mencatat token, credential, atau seluruh isi file.
- Bila kelak ada adapter provider opsional: hanya mengakses host provider yang dinyatakan, menjelaskan host/data yang dikirim, dan mematuhi rate limit, ketentuan layanan, serta hak penggunaan/redistribusi; jangan scraping TradingView, memakai endpoint internalnya, meniru klien web, mengirim cookie/session, atau menghindari autentikasi/anti-bot.
- Jika nanti dipilih provider yang butuh API key, konektor harus opt-in, menjelaskan host/data yang dikirim, menyimpan key di OS keychain, dan tidak memakai kredensial untuk mengakses TradingView tanpa izin.
- Strategi kustom dinamis bukan MVP. Jika kelak ditambahkan, gunakan plugin WebAssembly dari source milik pengguna, capabilities minimal, batas waktu/memori/fuel, API versioned, dan tanpa akses filesystem/network default.

## 5. Ruang lingkup fitur

### Matriks kemampuan charting

Matriks ini menetapkan kesetaraan **workflow**, bukan salinan UI/implementasi TradingView.

| Area | MVP | Tahap lanjutan | Kriteria |
|---|---|---|---|
| Chart dasar | Candlestick, OHLC/last value, volume, crosshair, zoom/pan, fit, rentang waktu | multi-chart, sinkronisasi simbol/timeframe/crosshair | koordinat harga/waktu tetap akurat setelah zoom, resize, dan ganti dataset |
| Drawing | horizontal/vertical line, trendline, ray, rectangle; pilih/pindah/ubah/hapus; undo/redo | parallel channel, Fibonacci retracement, text/note, magnet/snap, grup/lock | objek tersimpan sebagai data milik app; hit-test dan anchor harga/waktu konsisten |
| Indikator | SMA, EMA, RSI, volume | MACD, Bollinger Bands, ATR, custom parameter combinations | definisi/formula didokumentasikan; warm-up dan missing bars diuji |
| Watchlist/layout | watchlist lokal, panel chart + data summary, tab/layout tersimpan | multi-panel grid, linked groups, templates | perubahan layout ter-persist dan pulih setelah restart |
| Backtest | strategi built-in, satu instrumen/run, order/fill ledger, fee/slippage, equity/drawdown | multi-asset portfolio, parameter sweep, walk-forward | hasil deterministik dan asumsi eksekusi terlihat di laporan |
| Interaksi | shortcut orisinal, context menu, search lokal, tool properties | konfigurasi shortcut, dockable panels, scripting sandbox | setiap aksi penting dapat dilakukan via mouse dan keyboard, UI tidak macet saat kerja berat |
| Presentasi | tema terang/gelap, format angka, locale/timezone | custom theme, lebih banyak format export | legibility dan accessibility diuji; visual identity tetap orisinal |

**Batas tegas:** bukan target MVP/roadmap untuk menyalin toolbar, ikon, nama tool, shortcut, pixel geometry, warna persis, panel/window composition, teks, animasi, format proprietary atau detail interaksi unik TradingView. Gunakan label dan shortcut sendiri; nama generik untuk konsep matematika umum boleh bila perlu. Tidak menjanjikan seluruh fitur produk asal atau kompatibilitas Pine Script.

### MVP (offline, data dari pengguna)

- Katalog simbol berasal dari file simbol lokal; pencarian lokal. Jangan menyajikan katalog TradingView seolah-olah kita memiliki feed atau hak datanya.
- Impor OHLCV CSV/Parquet milik pengguna dengan mapping kolom, preview, validasi dan laporan impor (jumlah bar, gap/duplikat, checksum, rentang UTC).
- Dataset lokal + katalog simbol; chart dan backtest memakai salinan lokal. Seluruh alur MVP tetap offline; tidak ada refresh jaringan wajib.
- Chart 1 panel: candlestick, volume, crosshair, zoom/pan, fit-to-data, timeframe, rentang tanggal.
- Tema terang/gelap yang dirancang baru; layout/tab sederhana; watchlist lokal.
- Indikator SMA, EMA, RSI dan volume; parameter dapat diedit.
- Backtest satu instrumen, satu posisi/strategi per run, fee/slippage, log transaksi, equity curve, hasil ringkas.
- Ekspor data hasil dan laporan JSON/CSV.
- Tanpa login dan tanpa jaringan wajib pada MVP.

### Tahap berikutnya (opsional, tetap opt-in)

- Parquet lanjutan, multi-panel, multi-simbol, sinkronisasi crosshair, drawing trendline/measurement.
- Strategy builder lebih kaya dan parameter sweep dengan seed/reproducibility.
- Plugin strategi Rust→WASM tersandbox, setelah threat model dan test harness siap.
- Adapter provider publik opsional hanya setelah endpoint terdokumentasi, ketentuan penggunaan, lisensi, rate limit, dan cakupan historis dinilai; konektor tetap terpisah dan opsional. Contoh yang mungkin dievaluasi: bursa kripto untuk spot OHLCV; saham/forex umumnya perlu data vendor berlisensi atau sumber yang hak pakainya jelas.
- Corporate action, futures/contract rollover, FX conversion, intrabar/tick data, portfolio multi-aset, walk-forward, Monte Carlo sebagai modul lanjutan karena asumsi dan kompleksitasnya besar.

## 6. Tahapan kerja dan deliverable

### Tahap 0 — Spesifikasi dan desain mandiri (default disepakati 2026-10-16)

- Platform awal: Linux x86_64 prioritas; Windows x86_64 + macOS arm64 target build berikutnya.
- Format CSV MVP: `timestamp,open,high,low,close,volume` (timestamp UTC ISO-8601 atau epoch detik; dipetakan saat impor).
- Timeframe MVP: `1m, 1h, 1D`; granularitas lain via resample terdokumentasi.
- Ukuran data tipikal: 10k–1M bar per dataset untuk benchmark awal.
- Model biaya/slippage default: fee 0,1% per fill + slippage 1 tick; dapat diubah per run dan tercatat di laporan.
- Fitur chart wajib MVP: candlestick, volume, crosshair, zoom/pan/fit, rentang tanggal.

- Tetapkan platform awal, format CSV, granularitas candle, unit harga, tujuan backtest dan batas penggunaan personal.
- Buat nama/identitas visual baru dan wireframe konseptual non-pixel-copy.
- Susun schema OHLCV, konfigurasi, watchlist, backtest result, dan threat model.

**Selesai bila:** seluruh perilaku MVP dapat diuji tanpa akun, layanan TradingView, data proprietary, atau aset TradingView.

### Tahap 1 — Fondasi Rust dan build lintas-platform

- Workspace Cargo, lint/format, CI test dan build matrix Windows/macOS/Linux.
- Logging lokal teredaksi, konfigurasi berversi, command/event UI–core.
- Benchmark renderer kosong dan memory profile dataset representatif.

**Selesai bila:** aplikasi kosong dapat dibangun, dibuka, dan ditutup di tiga OS tanpa mengakses jaringan.

### Tahap 2 — Data lokal (impor pengguna, offline)

- Fokus CSV/Parquet import wizard, normalisasi timestamp, validasi candle, metadata dan cache.
- Fixture sintetis + fixture format publik/offline; jangan menyimpan fixture market data yang melanggar syarat provider.
- Bila adapter provider opsional kelak ditambahkan: definisikan trait provider, daftar host diizinkan, pagination sopan, retry terbatas, checkpoint/resume, progress dan pembatalan.

**Selesai bila:** import valid menghasilkan jumlah bar tepat dan input rusak memberi error yang jelas; pembatalan tidak meninggalkan dataset parsial yang kelihatan lengkap; dataset terimpor dapat dipakai ulang offline.

## 7. Alur pemakaian data (offline MVP)

1. Pengguna menyiapkan file OHLCV sendiri di luar aplikasi dari sumber sah (mis. ekspor CSV/Parquet pair).
2. Pengguna memilih file lokal di aplikasi; aplikasi memvalidasi timestamp, duplikat, rentang, dan gap secara lokal.
3. Hasil ditulis dulu ke staging; UI menyatakan status lengkap atau parsial secara eksplisit.
4. Setelah valid, data dicatat ke katalog lokal dan disimpan Parquet. Metadata mencakup sumber file, pair, timeframe, UTC range, imported-at, checksum, dan coverage report.
5. Chart/backtest membaca salinan lokal dan tetap tersedia offline.

MVP ditujukan untuk **bar OHLCV historis**. Dataset lebih halus seperti trades/ticks dan cakupan multi-aset ditunda. Adapter unduhan jaringan bukan bagian MVP.

### Tahap 3 — Chart dan navigasi

- Candlestick/volume, interaksi viewport, watchlist, tab/layout, tema.
- Pisahkan rendering dari data model; optimalkan hanya berdasarkan profiling.

**Selesai bila:** navigasi chart tidak memblokir UI dan viewport tetap responsif pada dataset target yang ditentukan di Tahap 0.

### Tahap 4 — Indikator dan backtest MVP

- Indikator incremental dan strategy builder built-in.
- Event ledger, model fee/slippage, hasil, equity curve, export.

**Selesai bila:** hasil test kecil dapat dihitung manual; tidak ada fill pada bar sebelum sinyal; serial/parallel memberi hasil identik.

### Tahap 5 — Hardening, packaging, dan beta personal

- Uji crash/import fuzzing, persistence migration, resource limits, offline behavior.
- Paket installer masing-masing OS, hash/checksum release, signing bila tersedia.
- Profil startup, penggunaan memori, frame time dan backtest throughput; tetapkan angka target setelah baseline diukur.

**Selesai bila:** smoke test pada Windows/macOS/Linux lulus, data lokal bertahan setelah restart, dan dokumentasi menjelaskan semua asumsi backtest.

## 7. Pengujian dan kriteria mutu

- Unit tests untuk timestamp, tick rounding, indikator, order state machine dan metrics.
- Property tests untuk invariants ledger: cash/position reconcile, tidak ada fill tanpa order, equity konsisten dengan posisi terbuka.
- Regression fixtures untuk timezone, DST, missing bars, split/gap, malformed CSV dan nilai ekstrem.
- Determinisme: input + konfigurasi + seed sama menghasilkan bytes/metrics hasil yang sama.
- Performance: benchmark import, redraw chart, pan/zoom, indikator dan backtest pada beberapa ukuran dataset; laporkan p50/p95 serta RAM, bukan klaim “sangat cepat” tanpa ukuran.
- Cross-platform CI: Linux x86_64; Windows x86_64; macOS arm64 dan x86_64 bila runner tersedia.
- Audit privasi: MVP tidak membuka socket; test juga memverifikasi tidak ada analytics/crash reporting.

## 8. Risiko dan keputusan yang perlu diambil sebelum implementasi

1. **Feed dan lisensi data:** MVP offline memakai file pengguna; tidak ada ketergantungan provider. Adapter jaringan apa pun tetap tunduk pada ketentuan penggunaan, rate limit, batas cakupan, dan bisa berubah. Data saham/forex tidak otomatis tersedia gratis.
2. **Kesetaraan chart:** UI/UX yang terasa familiar bisa dicapai lewat pola umum, tetapi tidak menjanjikan salinan pixel-perfect, seluruh fitur produk asal, atau kompatibilitas data proprietary.
3. **Semantik backtest:** candle OHLC saja tidak tahu urutan intrabar; hasil harus menampilkan asumsi dan batasannya.
4. **Cross-platform GPU:** backend WGPU/driver berbeda-beda; sediakan fallback CPU dan test pada GPU/OS beragam.
5. **Plugin:** arbitrary Rust native plugin berisiko crash/akses luas; tunda atau sandbox melalui WASM dengan batas capability.
6. **Kinerja:** benchmark lebih dulu; jangan mengorbankan akurasi harga atau determinisme demi optimasi prematur.
7. **Batas analisis produk asal:** sebagian reverse engineering sebelumnya parsial dan EXE utama belum terdecompile; semua fitur dalam rencana ini adalah pilihan desain baru, bukan klaim bahwa produk asal berperilaku sama.

## 9. Keputusan awal yang direkomendasikan

- Bahasa implementasi: Rust untuk UI, domain, data, chart dan backtest.
- UI: `eframe`/`egui` + `wgpu`; chart candlestick khusus.
- Pengalaman: local-first, tanpa akun, offline sebagai default.
- Data MVP: impor CSV/Parquet offline milik pengguna, simpan lokal sebagai Parquet; adapter unduhan publik tanpa key hanya opsional pasca-MVP.
- Strategi MVP: blok built-in; WASM Rust strategi pada tahap lanjutan.
- Storage: SQLite metadata + Parquet data.
- Identitas produk: nama, visual, ikon, shortcut, komposisi layout dan copywriting orisinal.

**Langkah pertama setelah rencana disetujui:** finalisasi spesifikasi MVP—OS/arsitektur prioritas, format CSV yang tersedia, ukuran data tipikal, model biaya/slippage, dan fitur chart wajib—lalu buat skeleton workspace Cargo dan test fixtures sintetis.