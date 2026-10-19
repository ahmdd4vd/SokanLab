# ROADMAP SokanLab — fase terstruktur sampai goal

**Ruang kerja:** root repositori ini
**Sumber acuan:** `PRODUCT.md`, `tradingview-rust-cleanroom-plan.md`
**Goal akhir:** app desktop Rust offline (Linux/Windows/macOS) untuk chart OHLCV lokal + backtest deterministik, brand orisinal SokanLab, rilis open-source Apache-2.0.

## Aturan manage (biar tidak ngawur)

1. Satu fase selesai bila semua exit criteria centang + bukti test/log ada.
2. Dilarang loncat fase tanpa menandai status di tabel bawah.
3. MVP offline penuh: tidak ada socket jaringan, tidak ada endpoint TradingView, tidak ada aset/logo/teks tiruan.
4. Setiap perubahan spec tercatat di file ini (tanggal + alasan).
5. Setiap klaim “selesai” wajib ada artefak: file, test output, atau checksum.

## Status fase

| Fase | Nama | Status | Artefak |
|---|---|---|---|
| 0 | Produk & spesifikasi | berjalan | `PRODUCT.md`, plan revisi 2026-10-16 |
| 1 | Fondasi Rust + CI | selesai 2026-10-16 | workspace std-only, fmt/clippy/test/build/smoke OK, `docs/BENCH.md` |
| 2 | Data lokal offline | selesai 2026-10-16 | import CLI + SQLite + Parquet, 22 test lolos, e2e 1 valid + 4 rusak |
| 3 | Chart + navigasi | selesai 2026-10-17 | renderer candle/volume/crosshair, zoom/pan/fit, drawing+undo/redo, 51 test, PNG render |
| 4 | Indikator + backtest | selesai 2026-10-17 | Sma/Ema/Rsi incremental, engine ledger+equity+metrik (termasuk Sharpe), runner worker GUI + CLI `backtest`, ekspor 5 file; test hitung-manual & paralel identik; e2e laporan 2× run identik |
| 5 | Hardening + rilis Apache-2.0 | berjalan sejak 2026-10-18 | fuzz-lite + fix panic `i64::MIN`, migrasi katalog, resource limits, layout persist ke disk, decimation (GUI 1M: 4,9 ms/frame), `README`/`docs/BACKTEST.md`/`THIRD-PARTY.md`, `scripts/package.sh` + SHA-256 |
| 6 | Parity TradingView: alat gambar + pintasan | batch 1–7 selesai 2026-10-19 | 22 alat gambar (termasuk 3 channel: paralel/disjoint/flat + Extend) + magnet + edit + properti objek (opasitas, level fib kustom, varian pitchfork, gaya default, katalog template bernama) + object tree + multi-seleksi + grup/z + skala log/persen/invert + snapshot (1×–4×) + overlay SMA/EMA/VWAP + pane RSI/MACD + hotkey TV-inti, layout `schema:6` (v1–v5 kompatibel), 210 test, `docs/TV-PARITY.md` |

## Fase 0 — Produk & spesifikasi [berjalan]

Tujuan: kunci nama, identitas, lisensi, dan default teknis.
Tugas:
- [x] Pilih working title `SokanLab`, tagline, palet orisinal
- [x] Tetapkan Apache-2.0, holder `ahmdd4vd (2026)`; `LICENSE` + `NOTICE` ditulis
- [x] Tetapkan default CSV `timestamp,open,high,low,close,volume`, timeframe 1m/1h/1D, fee 0,1% + slippage 1 tick
- [x] Cek tabrakan nama SokanLab: websearch + crates.io tidak menemukan kolisi langsung (2026-10-16); tetap cek ulang App Store/GitHub sebelum rilis
- [ ] Isi holder resmi (masih sementara) sebelum publish
- [x] Buat 4 file ikon vektor orisinal + `ICON-TEST.md`; hasil jujur: A/C untuk ≥32px, `icon-small-mono.svg` wajib untuk 16–32px, B ditolak sebagai primer
Exit: holder resmi terisi sebelum publish; Fase 1 boleh mulai paralel karena sisa Fase 0 non-blokir.

## Fase 1 — Fondasi Rust + CI [selesai 2026-10-16]

Tujuan: workspace bisa build dan jalan tanpa jaringan (validasi Linux; Windows/macOS via CI).
Tugas:
- [x] Install toolchain Rust stabil di dev (cargo/rustc 1.99.1)
- [x] Scaffold `apps/desktop`, `crates/domain,data-import,data-store,chart,indicators,backtest,strategy-api,ui,export` (std-only, 0 deps eksternal)
- [x] Lint/format (`cargo fmt --check` OK), clippy `-D warnings` OK, logging lokal teredaksi, config `schema_version = 1`
- [x] CI matrix Linux/Windows/macOS ditulis (`.github/workflows/ci.yml`); runner 3 OS belum dieksekusi di env ini
- [x] Benchmark awal + memory profile dicatat di `docs/BENCH.md` (biner 466K, RSS 2,1 MB, viewport proksi 9,84 M/s)
Exit: tercapai di Linux (build+test+smoke exit 0, tanpa socket). Windows/macOS menunggu run CI.

## Fase 2 — Data lokal offline [selesai 2026-10-16]

Tujuan: CSV pengguna bisa diimpor dan dipakai ulang offline.
Tugas:
- [x] Import CLI: mapping kolom default + `preview_csv`, validasi waktu/duplikat/nol/bar rusak (`EmptyValue` berposisi, `BadRow`, `DuplicateTime`, `NonMonotonicTime`, `Empty`)
- [x] Staging → commit atomik → SQLite katalog (`schema_version=1`, tabel `datasets`) + Parquet Int64 6 kolom; roundtrip teruji
- [x] Provenance JSON + sha256 + coverage (`ts_min/max`, `bars`, `source_file`); reimport idempotent
- [x] Fixture `fixtures/`: valid-3bars, malformed-badrow, duplicate-time, empty-value; e2e valid exit 0, 4 kasus rusak exit 2 dengan preview
Exit: tercapai. Catatan jujur: deteksi `gaps` antar-timeframe masih 0 statis; DST tidak relevan untuk timestamp epoch integer (zona hanya untuk tampilan, Fase 3).

## Fase 3 — Chart + navigasi [selesai 2026-10-17]

Tujuan: chart cepat dan tidak blokir UI.
Tugas:
- [x] Renderer candle khusus (egui painter, hanya bar terlihat): wick + body, grid, pane volume, crosshair + label harga/waktu, label harga terakhir
- [x] Navigasi: zoom roda mouse dengan anchor, pan drag, fit-price, `visible_indices` binary search; uji round-trip koordinat setelah zoom/resize/pan
- [x] Drawing orisinal: horizontal/vertical line, trendline, ray, rectangle; anchor harga/waktu; hit-test + pick teratas; undo/redo
- [x] Shell eframe (Panel::top/left/Central), tema gelap/terang orisinal, watchlist, layout JSON berversi
- [x] Verifikasi: 51 test lolos, `clippy -D warnings` bersih, GUI native jalan di Xvfb (window 1200x800), render headless → PNG (`docs/shot.png`, `docs/shot-wide.png`)
Exit: tercapai. Bukti render: 15k–36k triangle untuk 120–100k bar (hanya bar terlihat), 1M bar dalam 0,62 s.

Catatan jujur:
- Renderer memakai jalur CPU egui (`Painter`), bukan instancing GPU mentah. Alasan: bisa diverifikasi headless dan stabil lintas driver; jalur GPU jadi kandidat Fase 5 bila profiling perlu.
- RSS 1M bar = 668 MB (dataset sintetis penuh di memory). Ini batas MVP, perlu SoA/streaming bila dataset > 1M bar.
- Layout persist baru JSON di memory + `on_exit` log; simpan ke disk penuh menyusul (butuh crate config). → **Diperbaiki Fase 5**: disimpan ke `data/layout.json` (env `SOKANLAB_LAYOUT`), berisi viewport + drawing + tema + watchlist, teruji roundtrip byte-identik.
- `docs/shot-wide.png` dirender dari 100k bar untuk uji Only-visible-bars-draw.

## Fase 4 — Indikator + backtest [selesai 2026-10-17]

Tujuan: strategi built-in deterministik dan teraudit.
Tugas:
- [x] SMA/EMA/RSI incremental + warm-up (`Option` sebelum cukup bar) + kebijakan missing-bar: bar hilang tidak diinterpolasi, keputusan berbasis indeks bar (teruji `time_gaps_do_not_change_decisions`)
- [x] Engine event-loop deterministik; sinyal bar `t` disimpan sebagai pending dan baru dieksekusi di open bar `t+1` (teruji `never_fills_on_signal_bar`, `fill_lands_on_next_bar_open`); UI menjalankannya di worker thread (`crates/ui/src/runner.rs`), GUI tidak blocking
- [x] Ledger order/fill/cash/posisi/fee/slippage + aturan stop-vs-target konservatif (stop dulu bila tersentuh bar sama: `stop_and_target_same_bar_prefers_stop`)
- [x] Metrik: return, max drawdown, Sharpe per-bar, win rate, exposure, equity curve, + string asumsi tersimpan di laporan
- [x] Uji look-ahead, hitung-manual (`hand_calculated_four_bars`), serial vs paralel identik (`parallel_runs_identical_to_serial`, `two_runner_instances_identical`), seed reproducibility (tidak ada RNG; hasil dua run `diff -r` identik)
- [x] Ekspor JSON/CSV lengkap: `metrics.csv`, `ledger.csv`, `equity.csv`, `trades.csv`, `report.json` (5 file per run)
- [x] Integrasi: panel backtest GUI (form strategi + metrik + kurva equity/drawdown + ekspor) dan CLI `backtest` (parquet/CSV, 4 strategi, exit 2 untuk input salah)
Exit: tercapai. Bukti: 95 test lolos, clippy `-D warnings` bersih; e2e 200 bar 4 strategi exit 0; dua run `ma_cross` menghasilkan direktori laporan identik (`diff -r`).

Catatan jujur:
- Sharpe dihitung per-bar (mean/std return per bar × 1000, varians populasi, integer `isqrt`); belum annualized karena timeframe 1m/1h/1D butuh faktor berbeda.
- Strategi SL/TP memakai dua lapis: exit strategi (sinyal, fill open berikutnya) dan stop/target intrabar engine (harga stop/target, `EventKind::StopLoss/TakeProfit`); alasan exit terlihat di `exit_reason` tiap trade.
- Deteksi `gaps` saat impor masih statis 0 (angka coverage; kebijakan eksekusi tetap bar-apa-adanya).

## Fase 5 — Hardening + rilis [berjalan sejak 2026-10-18]

Tujuan: beta personal + rilis open-source.
Tugas:
- [x] Fuzzing impor (fuzz-lite deterministik 2.301 kasus/run dengan seed tetap — menemukan panic nyata `i64::MIN` dan memperbaikinya), migration persistence (katalog v0→v1 otomatis, versi lebih baru ditolak dengan pesan jelas), resource limits (`MAX_ROWS` 20 juta di parser + batas berkas impor 1 GiB via `SOKANLAB_MAX_IMPORT_MB`, exit 2 dengan pesan MB), offline behavior (`scripts/check-offline.sh`: 0 pola jaringan di kode + 0 klien di graph build + smoke offline)
- [~] Paket per OS + checksum: `scripts/package.sh` (tar.gz + `.sha256`) teruji di Linux; paket Windows/macOS menunggu dijalankan di OS terkait/CI; code signing belum (tidak ada kunci)
- [x] Profil startup/memori/frame-time/throughput dicatat di `docs/BENCH.md` (GUI 1M bar: frame pertama 9 ms, 4,9 ms/frame, RSS 251 MB; render 1M 0,15 s; `--bench-frames` untuk ukur ulang)
- [x] Dokumentasi rilis: `README.md` (mulai cepat, CLI, env, batasan), `docs/BACKTEST.md` (asumsi lengkap mesin + definisi metrik + berkas laporan), `THIRD-PARTY.md`, `NOTICE` diperbarui
- [x] Persistence restart: layout chart → disk + dimuat saat start (`--layout` untuk uji e2e); katalog SQLite berversi
- [ ] Smoke test 3 OS: matrix CI diperbarui (impor/backtest/layout roundtrip/render/headless/audit offline) — belum dieksekusi di GitHub
- [ ] Holder resmi di `LICENSE`/`NOTICE` (sisa Fase 0; menunggu nama resmi)
Exit: smoke lulus (Linux ✓, 3 OS menunggu CI), data bertahan setelah restart (✓ layout + katalog), repo siap publish Apache-2.0 (dokumen ✓; menunggu holder + run CI).

Catatan jujur:
- Perbaikan performa Fase 5 berangkat dari temuan profiling: 1M bar dulu dilukis
  3 shape/bar (166 ms/frame) dan pemetaan viewport (x/y) tidak mengikuti ukuran
  pane area (candle bisa jatuh ke pane volume; crosshair/hit-test bisa meleset
  saat lebar pane ≠ lebar default). Sekarang ada decimation `draw_step` +
  `price_pane_height` + resize viewport (lebar & tinggi pane) tiap frame;
  angka sebelum/sesudah ada di `docs/BENCH.md`.
- Fuzz-lite bukan `cargo-fuzz`: liputan terbatas 2.301 kasus/run, tapi ikut
  `cargo test` sehingga regresi langsung ketahuan tanpa tooling tambahan.
- Paket rilis masih `tar.gz` manual (bukan installer MSI/dmg/deb); signing dan
  auto-update di luar scope personal MVP.
- GUI frame-time diukur dengan software GL (Xvfb), jadi angka nyata di GPU
  terdedikasi bisa lebih baik.

## Fase 6 — Parity TradingView: alat gambar + pintasan [batch 1–8 selesai 2026-10-19]

Tujuan: pengalaman menggambar + keyboard selengkap mungkin dengan TradingView
(perilaku/konvensi saja — bukan kode, aset, atau brand), tanpa keluar dari batasan
offline & bukan-klon.

Metode riset: artifact TradingView Desktop (MSIX → `resources/app.asar`) dianalisis
dengan REA `analyze_javascript_application` (sha256 `a70a18bf…`, 166 file relevan,
908 modul, 2.839 findings, 3 kanal IPC) — temuan: **mesin chart tidak dibundel**,
hanya shell window/tab + jembatan native. Karena itu daftar otoritatif diambil dari
dokumentasi publik TV (Shortcuts, Drawings List, Desktop shortcuts) dan seluruh
implementasi ditulis ulang dari nol. Detail + bukti: `docs/TV-PARITY.md`.

Batch 1 (selesai):
- [x] Model drawing 5 → 14 jenis + Measure kilat: hline, hray, vline, crossline, trend, ray, extended, arrow, rect, ellipse, triangle, fib (7 level), date & price range, long/short position
- [x] Penempatan: klik-klik & seret-satu-kali, pratinjau putus-putus, stay-in-mode, `Esc` berjenjang, `Del`, undo anchor
- [x] Magnet Off/Weak/Strong (OHLC + anchor objek), `Shift` kunci H/V/45°
- [x] Edit: drag handle, drag badan, klik kanan (hapus/sembunyikan/magnet/reset), daftar objek, sembunyikan semua
- [x] Hotkey TV-inti: `Alt+T/H/V/C/F`, `Alt+R`, `Ctrl+Alt+H`, `Ctrl+Z/Y/Shift+Z`, `Ctrl+S`, panah/`Ctrl+`panah, `Ctrl+↑↓`, `Shift`+scroll, `Esc`, `Del`; plus `1`–`9` alat & `F1` dialog (divergensi terdokumentasi karena TV memakai angka untuk timeframe — resampling belum ada di MVP)
- [x] Persistensi layout `schema:1` diperluas (`tools`, `hidden`, 14 kind) — roundtrip byte-identik; render headless ikut menggambar objek

Batch 2 (selesai):
- [x] Properti objek: 8 warna + tebal 1–4 + default per alat (`ItemStyle`), panel properti di sisi kiri
- [x] Object tree: toggle tampil & kunci per objek, nama kustom (escape JSON teruji), hapus per baris; magnet/pick melewati objek sembunyi/terkunci
- [x] Alat baru: Parallel Channel (3 titik), Pitchfork Andrew (3 titik), Fib Extension (0/61,8/100/127,2/161,8/261,8%), Fib Time (bar +1,+2,+3,+5,…+144) — model 14 → 18 jenis
- [x] Dobel-klik sumbu: harga = fit skala ulang, waktu = rentang penuh (event sintetis teruji)
- [x] `Alt+G` "pergi ke tanggal": parser `YYYY-MM-DD[ HH:MM[:SS]]` UTC + epoch detik/milidetik (algoritma days-from-civil), dialog GUI + `ChartState::center_on_time`
- [x] Persistensi `schema:2`: gaya/visible/locked/nama per objek; berkas `schema:1` tetap termuat (default) — roundtrip byte-identik teruji e2e

Batch 3 (selesai):
- [x] Skala harga `Linear/Log/Persen` (basis = close bar terlihat pertama) + `Invert`; transformasi v-space (log/persen) tanpa mengubah jalur linear — tes lama tetap identik
- [x] Drag sumbu langsung: sumbu harga = skala vertikal, sumbu waktu = skala horizontal (event sintetis teruji; dobel-klik tetap reset)
- [x] Snapshot GUI `Alt+S`: `render_chart_png` (headless rasterizer dipakai ulang) → `data/snapshot-<epoch>.png`, ukuran dari area chart terakhir
- [x] Gaya garis per objek: solid/putus-putus/titik (`DashStyle`), ikut di panel properti & layout
- [x] Teks & Callout: `Drawing::Text/Callout` (String di varian; `Drawing` tidak lagi `Copy`), editor isi teks di panel properti, ekor penunjuk untuk callout
- [x] Z-order (`move_item`), duplikat (offset 2 bar), grup-lite (id = maks+1; drag badan menggeser semua anggota; duplikat tidak mewarisi grup)
- [x] Indikator overlay SMA/EMA/VWAP (indikator `vwap_series` baru, `OverlaySettings`/`OverlayLine` di `ChartState`, digambar painter hanya untuk bar terlihat)
- [x] Persistensi `schema:3`: `scale/invert`, `overlays`, `dash`, `group`, `text/callout`; berkas v1/v2 tetap termuat — roundtrip byte-identik teruji e2e

Batch 4 (selesai):
- [x] Pane indikator terpisah di bawah volume: **RSI** (skala tetap 0–100, panduan 30/50/70) & **MACD** (garis + sinyal + histogram, skala simetris auto); maks 3 pane; panel samping (checkbox + periode); `indicators::macd_series` (milli, EMA fast−slow atas harga ×1000, sinyal di-seed nilai MACD pertama)
- [x] Layout multi-pane + **strip sumbu harga (kanan) & waktu (bawah) berlabel**: `split_plot_panes`/`plot_width`/`draw_axes` (format waktu UTC, sipil Hinnant), tag crosshair ber-kotak (harga kanan, waktu bawah), tag harga terakhir kotak Ember, label "Vol"
- [x] Objek gambar **di-clip ke pane harga** (tidak menembus volume/pane indikator); rasterizer headless kini menghormati `clip_rect` seperti backend GPU
- [x] **Snapshot dengan opsi resolusi** 1×/2×/4× (raster ulang pada skala piksel, dibatasi ~8K) + tombol di panel samping
- [x] **Perbaikan bug EMA**: bobot α kini pada bar terbaru (rumus lama terbalik — overlay EMA dan MACD ikut salah); acuan eksak `ema_series([10,12,14], 2) = [None, 11, 13]`
- [x] Persistensi `schema:4` (`panes`); berkas v1–v3 tetap termuat (entri pane tak dikenal dilewati)

Batch 5 (selesai):
- [x] **Opasitas per objek** 0–100%: `DrawingItem.opacity` + `opacity_factor()`, painter mengalikan warna efektif objek (termasuk level fib berwarna, info box range/posisi, label) — slider di panel properti, `"opacity"` di layout hanya bila ≠ 100
- [x] **Varian pitchfork** Andrew/Schiff/Modified Schiff/Inside: `PitchforkKind` + geometri bersama `pitchfork_lines` (median & tine sejajar + segmen bantu) dipakai painter *dan* `hit_test_item` agar tampilan & klik selalu konsisten; definisi publik (Schiff = waktu A & harga rata-rata A–B; Modified Schiff = titik tengah A–B; Inside = titik tengah A–B → C, tine lewat A & B)
- [x] **Level fib kustom + warna per level**: `FibLevel { milli, color }` per objek; `hit_test_item` menghormati level kustom; editor panel (tambah/hapus, DragValue persen 0,1%, swatch siklus auto↔8 warna palet, "Kembali bawaan")
- [x] **Template gaya**: `ChartState::set_default_style_from_selected` (objek baru mewarisi gaya) + `apply_style_to_same_kind` (samakan gaya+opasitas ke semua objek `degree` sama); tombol di panel properti + notifikasi hasil
- [x] Persistensi `schema:5` (`opacity`, `pitchfork`, `fib`, `default_style`); reader v1–v5, metadata baru berdefault aman untuk berkas lama; nilai liar dijepit (opacity 0–100, pitchfork tak dikenal → Andrew)

Batch 6 (selesai):
- [x] **Multi-seleksi ala TV**: `Ctrl+klik` (⌘ di macOS) menambah/melepas objek (`ChartState::toggle_in_selection`) + **kotak seleksi `Ctrl+seret`** di area kosong → `Gesture::Rubber` → `chart::drawing::rubber_select` (anchor di dalam kotak atau segmen memotong kotak; garis horizontal/vertikal dinilai dari posisinya; objek sembunyi/terkunci dilewati); kotak digambar putus-putus Ember (`painter::draw_rubber_band`), Ctrl+klik/Ctrl+seret tercatat di dialog F1 (tanpa hotkey keyboard baru)
- [x] **Operasi kolektif**: drag badan = geser semua terpilih (tanpa mengkolaps seleksi), `Del`/tombol hapus = hapus semua (satu langkah undo, `History::remove_many`), z-order naik/turun satu langkah dengan urutan relatif tetap (`History::move_items_z`), duplikat sekaligus (`History::duplicate_many`), grup satu id (`assign_group_many`/`clear_group_many`), edit warna/tebal/pola/opasitas diterapkan ke semua terpilih (`sync_style_from_primary`)
- [x] **Panel & daftar objek**: baris terpilih ditandai, Ctrl+klik di daftar, catatan jumlah terpilih, tombol hapus berlabel "Hapus N objek", properti menampilkan "(primer dari N terpilih)"; hapus per baris menyesuaikan indeks seleksi (`selection_after_removal`); menu klik kanan kunci/sembunyi berlaku ke semua terpilih; `Esc` membersihkan seluruh seleksi
- [x] Satu objek **primer** (indeks teratas) memegang handle edit & panel properti — sinkron dengan `pick` yang mengembalikan objek teratas
- [x] Persistensi **tetap `schema:5`** (seleksi tidak ikut disimpan); roundtrip berkas v5 tetap identik byte-for-byte; demo render headless menampilkan multi-seleksi + kotak seleksi sebagai bukti visual

Batch 7 (selesai):
- [x] **Varian channel**: `Drawing::ParallelChannel` bertambah `kind: ChannelKind` (Parallel/Disjoint/Flat) + dua alat baru `ToolKind::DisjointChannel` & `ToolKind::FlatChannel` (label objek per varian, `Degree::Channel` sama, tanpa hotkey — TV juga tidak memetakannya); geometri bersama `channel_segments` dipakai painter **dan** hit-test (batas 0 = a→b, batas 1 = lewat c sejajar dalam rentang x sama untuk paralel / salinan tergeser untuk disjoint / horizontal di harga c untuk flat)
- [x] **Opsi Extend per objek channel** (`ExtendMode` Nihil/Kiri/Kanan/Dua arah, `DrawingItem.extend`, dua centang di panel): default **tidak diperpanjang** seperti TV ("has an option of being extended … if the user wants the channel to be continued indefinitely"); perpanjangan mengikuti sumbu waktu (kiri = mundur dari ujung kiri, kanan = maju dari ujung kanan); garis tinggi vertikal tidak diperpanjang (tidak ada komponen waktu); konsekuensi: garis channel kini **segmen yang benar-benar digambar** (bukan lagi garis tak hingga) dan hit-test memakai segmen yang sama (`EXTEND_PX` 20.000 px mengalahkan pemotongan plot)
- [x] **Katalog template gaya bernama**: `StyleTemplate { name, style, opacity }` + `ChartState::{save_template,apply_template,remove_template}` (nama dipangkas & dipotong 32 karakter, kosong ditolak, nama sama diperbarui di tempat, katalog 16 entri, "Terapkan" ke seluruh seleksi batch 6); UI panel: kombo + kolom nama + tombol Simpan/Terapkan/Hapus + notifikasi hasil
- [x] Persistensi `schema:6`: `"variant"` per channel (hanya bila ≠ paralel), `"extend"` per objek (hanya bila ≠ nihil), `"templates":[…]` di layout; reader menerima v1–v6, entri template rusak/nama ganda dilewati & katalog dibatasi; **roundtrip byte-identik** untuk berkas v6 dan v5→v6 hanya nomor schema
- [x] Demo render headless menambah Disjoint Channel (Extend kanan) + Flat Top/Bottom Channel sebagai bukti visual; geometri & opsi Extend diverifikasi lewat 5 render PNG terfokus (satu per varian + extend kiri/kanan)

Batch 8 (selesai):
- [x] **10 alat baru** di `chart::drawing` (geometri dipakai bersama painter & hit-test): **Regression Channel** (2 titik; regresi kuadrat-terkecil close × indeks bar atas seluruh bar di rentang + dua batas ±σ, memakai `channel_segments`/`ExtendMode` batch 7), **Fib Trend Extension** (3 titik; level 0/61,8/100/127,2/161,8/261,8% horizontal dari titik ketiga ke kanan, level 100% disorot), **Elliott Impulse** (5 titik, label 1–5), **Elliott Correction** (3 titik, label A–C), **XABCD** (5 titik), **ABCD** (4 titik), **Gann Fan** (2 titik; 9 rasio 1×8…8×1, garis 1×1 tepat a→b, rasio lain menskalakan komponen harga), **Brush** & **Highlighter** (goresan ber-titik banyak; highlighter tebal ×3 & alpha 35%), **Sticker** (14 ikon vektor orisinal)
- [x] **Penempatan goresan & sticker**: `ToolKind::{Brush,Highlighter}` tidak lewat jalur klik (klik no-op) — titik disampel mentah saat seret (tanpa magnet, ambang ≥3 px, preview hover ikut raw), dibangun `build_stroke` di release; `Sticker` = sekali klik; `MAX_STROKE_POINTS = 4000`
- [x] **UI/UX teks**: `TextSize` (S/M/L = 12/16/22 px) & `TextAlign` (kiri/tengah/kanan) sebagai properti per objek Text/Callout (`DrawingItem.text_size`/`text_align`), kontrol di panel, `draw_note` menghormati keduanya
- [x] **Ctrl+seret badan objek = klon lalu geser** (konvensi TV): `Gesture::Press { toggle: true }` menunda keputusan sampai pointer dilepas (lepas tanpa seret = toggle seleksi; seret >4 px = `ChartState::duplicate_for_drag` → klon `group=None`, objek terkunci dilewati, seleksi pindah ke klon, lalu geser)
- [x] **Strip alat ber-area-gulir** (`ScrollArea::vertical().auto_shrink([false,false])`) karena entri alat kini 35; urutan `ToolKind::ALL` dipertahankan sehingga digit `1`–`9` tetap menunjuk sembilan entri pertama (Select…Arrow) — tanpa hotkey baru (sama seperti TV)
- [x] Persistensi `schema:7`: 10 kind baru (`regression`, `fibtrend`, `elliott_impulse`, `elliott_correction`, `xabcd`, `abcd`, `gannfan`, `brush`/`highlighter` ber-`pts`, `sticker` ber-`icon`) + meta `font`/`align` (hanya bila ≠ bawaan, ditulis setelah `fib`); reader menerima v1–v7, menolak skema > 7, `pts`/ikon rusak aman (ikon tak dikenal → `ArrowUp`, `pts` dipotong ke 4000); roundtrip **byte-identik**
- [x] Render headless: opsi `--demo batch8` (scene bersih) + demo penuh diperkaya → `docs/shot-batch8.png` sebagai bukti visual permanen
- [x] Kinerja (temuan bench): cache `ChartState::hover_pick` + pemakaian bersama blok kursor/render menurunkan pemanggilan `regression_fit` dari ~1,9× → **1,0×** per objek per frame (scene berat 36–38 → 13,6–14,1 ms p50)

Exit batch 1: 150 test lolos (naik dari 116), clippy `-D warnings` bersih,
`check-offline.sh` LULUS, e2e layout 14 jenis identik byte-for-byte, render PNG,
bench GUI 1M bar 5,0 ms/frame (tanpa regresi vs 4,9 ms). Bukti: `docs/TV-PARITY.md` §7.

Exit batch 2: 163 test lolos (naik dari 150; termasuk dobel-klik sumbu lewat event
sintetis, pick anti objek terkunci/sembunyi, parser tanggal, safety `i64::MIN` fib
extension), clippy bersih, `check-offline.sh` LULUS, e2e layout 18 jenis + metadata
identik byte-for-byte, berkas v1 naik otomatis ke v2, render PNG memuat channel/
pitchfork/fib ext/fib time, bench GUI 1M bar 4,9 ms/frame (p50 4,8 / p95 5,4).

Exit batch 3: 174 test lolos (naik dari 163; termasuk roundtrip skala log/persen/
invert, drag sumbu & drag grup lewat event sintetis, snapshot PNG valid, layout v3
roundtrip identik, tolak skala tak dikenal tanpa mengubah state), clippy bersih,
`check-offline.sh` LULUS, e2e layout 21 objek (teks/callout/dash/grup + log/invert +
overlay) identik byte-for-byte, berkas v1/v2 naik otomatis ke v3, render PNG memuat
overlay + teks/callout, bench GUI 1M bar 5,0 ms/frame; dengan overlay SMA/EMA/VWAP
5,2 ms/frame (p50 5,0 / p95 6,1).
Bukti: `docs/TV-PARITY.md` §8.

Exit batch 4: 180 test lolos (naik dari 174; termasuk tata letak pane + strip sumbu,
`macd_series` acuan eksak, EMA standar, pane RSI/MACD lewat `ChartState`, layout v4
roundtrip + pane tak dikenal dilewati, snapshot 2× = PNG 2× piksel, dan pane bukan
sumbu harga lewat event sintetis), clippy bersih, `check-offline.sh` LULUS, e2e layout
v4 (pane RSI 14 + MACD 12/26/9 + overlay) identik byte-for-byte, berkas v3 naik
otomatis ke v4 (hanya nomor schema yang berubah) dan `schema:5` ditolak tanpa mengubah
state, render PNG memuat pane RSI/MACD + strip sumbu, bench GUI 1M bar 4,7 ms/frame;
dengan overlay 5,1 ms; dengan overlay + 2 pane 5,2 ms.
Bukti: `docs/TV-PARITY.md` §10.

Exit batch 5: 190 test lolos (naik dari 180; termasuk geometri eksak 4 varian
pitchfork, hit-test yang membedakan varian & level fib kustom, opasitas konsisten
(alpha ≈ 40%), roundtrip metadata objek v5 + `default_style`, pewarisan gaya default &
"terapkan ke sejenis", serta panel properti batch 5 lewat event egui), clippy bersih,
`check-offline.sh` LULUS, e2e layout v5 (pitchfork Schiff 40%, fib kustom + level
berwarna, opacity liar 250 dijepit, pitchfork "noise" diabaikan) roundtrip **identik
byte-for-byte** (`cmp`, 963 B), berkas v4 naik otomatis ke v5 (hanya nomor schema yang
berubah) dan `schema:6` ditolak tanpa mengubah state, render PNG memuat fib 5 level
kustom + fork Schiff (19210/151816 segitiga; delta −48/+40 diverifikasi lewat dua build
eksperimen), bench GUI 1M bar + 2 pane 5,2 ms/frame (p50 5,0 / p95 5,8 — tanpa regresi).
Bukti: `docs/TV-PARITY.md` §11.

Exit batch 6: 204 test lolos (naik dari 190; termasuk `rubber_select` (anchor di
dalam / segmen memotong / garis panjang / sembunyi-terkunci), `move_items_z` banyak
objek (urutan relatif & mentok), `remove_many`/`duplicate_many`/grup banyak (satu
langkah undo), `sync_style_from_primary`, toggle seleksi & perpindahan primer,
perbaikan indeks seleksi setelah hapus, tes event egui nyata **Ctrl+klik →
seleksi 2 objek → drag badan menggeser keduanya → Ctrl+klik melepas satu** dan
**Ctrl+seret kotak seleksi → 2 objek terpilih / kotak kosong membersihkan**, serta
tes painter multi-seleksi (2 objek ember + 1 slate, kotak seleksi isi + tepi
putus-putus, kotak nol tidak digambar)), clippy `-D warnings` bersih,
`check-offline.sh` LULUS, e2e layout v5 roundtrip tetap **identik byte-for-byte**
(963 B, `cmp`) karena `schema:5` tidak berubah, render PNG memuat multi-seleksi +
kotak seleksi (19990/151890 segitiga; delta +780 / +74 = murni kotak seleksi —
dibuktikan dua build eksperimen yang mengembalikan tepat angka batch 5
19210/151816), bench GUI 1M bar + 2 pane 5,4–5,9 ms/frame dari 3 run (p50 5,2–5,4;
dalam variasi antar-run ±10%).
Bukti: `docs/TV-PARITY.md` §12.

Exit batch 7: 210 test lolos (naik dari 204; termasuk geometri eksak tiga varian
`channel_segments` (rentang x paralel = dasar, disjoint tergeser oleh c−a, flat
horizontal), perpanjangan mengikuti sumbu waktu & kasus garis vertikal, label/degree/
`translated` per varian, hit-test objek dengan `extend`, `StyleTemplate` (simpan/
perbarui/potong nama/penuh/hapus/pilihan panel), roundtrip `schema:6` (varian +
extend + template, byte-identik), migrasi v5→v6 (hanya nomor schema), nilai
`variant`/`extend` asing diabaikan, template rusak/ganda/kapasitas di layout, mapping
alat baru → varian, dan panel batch 7 lewat event egui), clippy `-D warnings` bersih,
`check-offline.sh` LULUS, e2e CLI (dataset 1M bar): berkas v5 lama → v6 **hanya nomor
schema yang berubah**, berkas v6 dengan varian+extend+template roundtrip **identik
byte-for-byte** dan idempoten, render PNG memuat tiga varian channel (18510/148850
segitiga; atribusi: channel kini dibatasi −1520/−3080 lalu +40 dari dua channel demo
baru — diukur lewat build eksperimen), bench GUI 1M bar + 2 pane 6,1/5,4/5,5 ms dari
3 run (p50 5,2 ketiganya; dalam variasi batch 6) dan 5,2/5,3 ms dengan 3 channel di
layar.
Bukti: `docs/TV-PARITY.md` §13.

Exit batch 8: 231 test lolos (naik dari 210; chart 60 (+11), ui 84 (+6), desktop 26
(+4)). Tes baru: geometri eksak regression (fit/σ/segmen/Extend, rentang terbalik,
pemangkasan indeks), level fib trend dari titik ketiga (proyeksi a→b ber-origin c),
sembilan garis Gann (1×1 = a→b, label unik), pola 5/3/5/4 titik + label titik,
goresan (batas 4000 titik, buang titik berdekatan), 14 ikon sticker + fallback,
`TextSize`/`TextAlign`, roundtrip `schema:7` byte-identik (10 kind + `font`/`align`),
reader menolak skema di luar 1–7 & data rusak, cache hover-pick (invalidasi objek/
undo/viewport/dataset), seret membuat goresan brush, Ctrl+seret pada objek mengklon
lalu menggeser klon, Ctrl+klik tanpa seret tetap toggle seleksi, panel properti batch 8
lewat event egui. `cargo fmt --check` & clippy `-D warnings` bersih,
`check-offline.sh` LULUS. E2E CLI (dataset 1 jt bar): berkas v6 batch 7 → v7 **hanya
nomor schema yang berubah**; berkas v7 baru (13 objek: 10 kind baru + `icon` +
`font`/`align` + `extend` + gaya) roundtrip **identik byte-for-byte** (`cmp`) dan
idempoten; berkas v1/v3 lama tetap termuat. Render PNG: `docs/shot-batch8.png` baru
(semua bentuk batch 8 + baris 14 ikon sticker), `docs/shot.png` **18583** dan
`docs/shot-wide.png` **148923** segitiga — atribusi +73/+73 dari sisipan demo batch 8
(build eksperimen tanpa sisipan mengembalikan tepat angka batch 7: 18510/148850), dan
build ulang menghasilkan PNG identik byte-for-byte. Bench GUI 1M bar + 2 pane
5,5/5,4/5,5 ms (p50 5,1–5,2) = setara batch 7; 3 channel di layar 5,3 ms
(p50 4,9–5,1); scene berat batch 8 14,0/14,6/14,5 ms (p50 13,6–14,1) dengan atribusi
biaya murni `regression_fit` O(rentang) (1,85 ms per 600k bar) dan perbaikan
hover-pick (36–38 → 13,6–14,1 ms).
Bukti: `docs/TV-PARITY.md` §14.

## Log perubahan

- 2026-10-19: Preview Windows x86_64 dibangun (paket portable `dist/sokanlab-0.1.0-windows-x86_64-preview.zip`) via cross-compile `x86_64-pc-windows-gnu` + mingw-w64, lalu di-smoke-test dengan Wine 10.0 (`--version`, render headless byte-identik dengan referensi Linux, roundtrip layout byte-identik, CLI import + backtest, GUI OpenGL 5 frame avg 5,5 ms; tangkapan layar `docs/preview-win-gui.png`). Backend DX12 di Wine belum terbukti dan Windows asli belum diuji — lihat `docs/WINDOWS-PREVIEW.md`. Temuan pengemasan: `scripts/package.sh` tidak menyertakan `config/default.toml`, sehingga GUI biner paket Linux hasil ekstraksi gagal ("config gagal"); skrip diperbaiki (ikut `config/default.toml` + `THIRD-PARTY.md`) dan paket Linux dibangun ulang (sha256 `3f110c28716583581ad1c2036dc2c627e30e117a73a8aba35544046eee8e7034`).
- 2026-10-19: Fase 6 batch 8 (parity TradingView: 10 alat lanjutan + UI/UX teks & klon Ctrl+seret) selesai. Perubahan saat eksekusi: (a) `chart::drawing`: `Drawing` +10 varian — `RegressionChannel{a,b}` (+`RegressionFit`/`regression_fit` dua lintasan O(rentang) dengan `partition_point`, `regression_segments` 3 segmen ±σ dibulatkan), `FibTrendExtension{a,b,c}` (level dari x(c) ke kanan, extend kiri), `ElliottImpulse{points}`/`ElliottCorrection{points}`/`Xabcd{points}`/`Abcd{points}` (polyline + label titik, jumlah efektif divalidasi), `GannFan{a,b}` (`gann_fan_lines` 9 rasio, `EXTEND_PX`), `Brush{points}`/`Highlighter{points}` (`MAX_STROKE_POINTS=4000`, `push_stroke_point` buang titik <3 px) dan `Sticker{at,icon}` (`StickerIcon::all()` 14 ikon + `from_key` fallback `ArrowUp`); `TextSize`/`TextAlign`; `hit_test_item` semua bentuk baru; (b) `ui::tools`: 10 `ToolKind` baru (label/singkatan strip, `is_stroke`, `build_stroke`, klik no-op untuk goresan), pratinjau goresan; (c) `ui::painter`: arm `draw_one` tiap varian (regresi lewat `PlotProj`, fib trend 6 level + label, pola polyline + label, Gann 9 garis + label tepi kanan, goresan `Shape::line` tebal ×3 alpha 35% untuk highlighter, `draw_sticker`), `draw_note` ukuran/perataan; (d) `ui`: `ChartState.{duplicate_for_drag}` (klon `group=None`, seleksi pindah ke klon), `Gesture::Press{toggle}` (tunda keputusan toggle vs klon sampai lepas/seret >4 px), `hover_pick` + cache (kunci: posisi lokal, viewport, jumlah objek, kedalaman undo, sembunyi, `bars_rev`) menggantikan `pick` per frame, writer/reader `schema:7` (`pts`/`icon`/`font`/`align`, clamp & tolak data rusak, reader v1–v7); (e) `app`: sampling goresan raw (tanpa magnet) untuk hover/drag + `build_stroke` saat lepas, Ctrl+seret klon, strip alat ber-`ScrollArea`, kontrol panel (Extend regression, centang perluasan kiri fib trend, kombo ikon sticker, kombo ukuran & perataan teks); (f) `headless`: opsi `--demo batch8` (scene bersih + baris 14 ikon sticker) dan sisipan batch 8 di demo penuh. Konvensi diverifikasi dari dokumentasi publik TV (Trend-based Fib Extension, Gann Fan, Elliott & pola, Linear Regression Channel) — perilaku saja, bukan kode/aset. Temuan bench: hit-test hover + blok kursor memanggil `regression_fit` dua kali per objek per frame (instrumentasi sementara membuktikan ~1,9×/frame) → kini 1,0× (scene berat 36–38 → 13,6–14,1 ms p50); rencana batch 9: prefix-sum O(1) bila rentang raksasa perlu. Verifikasi: 231 test (naik dari 210), fmt/clippy bersih, offline LULUS, e2e v6→v7 hanya nomor schema + berkas v7 (10 kind + font/align/icon/extend) idempoten **identik byte-for-byte**, render `docs/shot-batch8.png` + delta segitiga shot/shot-wide +73/+73 (eksperimen tanpa sisipan = 18510/148850) dan PNG deterministik, bench 3 run. Bukti: `docs/TV-PARITY.md` §14 (paket sha256 `3f110c28716583581ad1c2036dc2c627e30e117a73a8aba35544046eee8e7034`).
- 2026-10-19: Fase 6 batch 7 (parity TradingView: varian channel + opsi Extend + katalog template gaya bernama) selesai. Perubahan saat eksekusi: (a) `chart::drawing`: `ChannelKind` (Parallel/Disjoint/Flat) + `ExtendMode` (None/Left/Right/Both) + `EXTEND_PX` (20.000 px) + `channel_segments` — geometri dua batas channel (paralel: batas kedua sejajar dengan rentang x batas pertama; disjoint: salinan batas pertama yang digeser oleh `c−a`; flat: horizontal di `c.y` sepanjang rentang batas pertama; perpanjangan mengikuti sumbu waktu) dipakai bersama painter **dan** hit-test; `Drawing::ParallelChannel` bertambah `kind`; `DrawingItem.extend`; `StyleTemplate` (`from_item`/`apply_to`, nama maks 32 karakter, katalog 16 entri) + helper JSON; (b) `ui::tools`: `ToolKind::{DisjointChannel,FlatChannel}` → varian yang sesuai, label & singkatan strip (DJ/FL), tanpa hotkey (TV juga tidak memetakan tombolnya); (c) `ui::painter`: channel menggambar `channel_segments` dengan `it.extend` (garis channel kini segmen yang benar-benar digambar, bukan garis tak hingga) dan konstanta `wide` lama diganti `EXTEND_PX`; (d) `ui`: `ChartState.{templates,template_sel,template_name}` + `save_template`/`apply_template` (ke seluruh seleksi)/`remove_template`; writer JSON `"variant"` (hanya bila ≠ paralel), `"extend"` (hanya bila ≠ nihil), `"templates":[…]`; layout `schema:6` dengan reader v1–v6 (entri template rusak/nama ganda dilewati, katalog dipotong); (e) `app`: kombo varian channel + dua centang Extend (kiri/kanan) + seksi "Template gaya" (kombo + kolom nama + Simpan/Terapkan/Hapus + notifikasi; tombol dinonaktifkan saat belum berlaku); (f) `headless`: demo Disjoint Channel (Extend kanan) + Flat Top/Bottom Channel. Konvensi diverifikasi dari dokumentasi publik TV (Parallel/Disjoint/Flat Top-Bottom drawing tool) — perilaku saja, bukan kode/aset. Verifikasi: 210 test (naik dari 204), fmt/clippy bersih, offline LULUS, e2e v5→v6 hanya nomor schema + berkas v6 (varian+extend+template) idempoten **identik byte-for-byte** (912 B), verifikasi visual 5 render fokus (bentuk tiap varian + Extend kiri/kanan), delta segitiga render −1520/−3080 (channel dibatasi) lalu +40/+40 (dua channel demo baru) diverifikasi lewat build eksperimen, bench 6,1/5,4/5,5 ms (p50 5,2) dan 5,2/5,3 ms dengan 3 channel di layar. Bukti: `docs/TV-PARITY.md` §13 (paket sha256 `23847e1f85b7a5fd6e9be0938766e14b0788bfa0895233be94c106d88346c066`).

- 2026-10-18: Fase 6 batch 6 (parity TradingView: multi-seleksi & operasi kolektif) selesai. Perubahan saat eksekusi: (a) `chart::drawing`: `rubber_select` (+ helper kotak ternormalisasi, titik-dalam-kotak, perpotongan segmen-kotak) memilih objek yang tersentuh kotak seleksi dengan penanganan khusus garis horizontal/vertikal serta pelewatannya untuk objek sembunyi/terkunci; `History` bertambah `remove_many` (satu snapshot), `move_items_z` (banyak objek naik/turun satu langkah, urutan relatif tetap, tanpa snapshot saat mentok), `duplicate_many`, `assign_group_many`/`clear_group_many` (versi satu objek kini pembungkusnya); (b) `ui`: `ChartState.selected_multi` + `selection`/`is_selected`/`select_only`/`clear_selection`/`set_selection` (primer = indeks teratas seperti `pick`)/`toggle_in_selection`/`delete_selection`/`sync_style_from_primary`/`selection_after_removal`; `move_selected_z`/`duplicate_selected`/`toggle_group_selected`/`Command::RemoveSelected` kini bekerja pada seluruh seleksi; `Gesture::Rubber { origin, current }`; (c) `ui::painter`: `draw_drawings` menerima irisan objek terpilih (semua ditandai Ember) + `draw_rubber_band` (isi tembus pandang 10% + tepi putus-putus, klik tanpa seret tidak menggambar); (d) `app`: Ctrl/⌘+klik = toggle seleksi, Ctrl+seret di area kosong = kotak seleksi, drag badan tidak mengkolaps seleksi (geser semua terpilih), panel properti menyinkronkan gaya ke seluruh seleksi pada perubahan kontrol, daftar objek multi-pilih + label jumlah, menu klik kanan kunci/sembunyi massal, `Esc` membersihkan seleksi, entri F1 baru untuk konvensi mouse TV; (e) `headless`: demo multi-seleksi + kotak seleksi, `render_chart_png` ikut menggambar kotak saat gestur aktif. Konvensi diverifikasi dari dokumentasi publik TV ("Select multiple drawings: Hold Ctrl + Click", "hold CTRL and drag → box"), bukan kode TV. Verifikasi: 204 test (naik dari 190), clippy/fmt bersih, offline LULUS, e2e v5 roundtrip tetap byte-identik, delta segitiga render +780/+74 murni kotak seleksi (dua build eksperimen mengembalikan 19210/151816), bench 5,4–5,9 ms/frame. Bukti: `docs/TV-PARITY.md` §12 (paket sha256 `9356c41ddf3c35a2d0e1cccf8e32626fcb6a1eb4f12bb04951ccfdd9035b6c47`).

- 2026-10-16: pindah `PRODUCT.md` + plan ke `sokanlab/`; jadikan ruang kerja; buat roadmap fase 0–5.
- 2026-10-17: Fase 3 selesai (chart, drawing, render headless) — status + catatan jujur dicatat.
- 2026-10-17: Fase 4 selesai. Perubahan spec saat eksekusi: (a) sinyal bar `t` disimpan `pending` dan baru dieksekusi di open bar `t+1` — memperbaiki equity bar sinyal yang sebelumnya ikut terisi; (b) Sharpe ditambahkan sebagai metrik per-bar integer (belum annualized); (c) CLI `backtest` ditambah di samping panel GUI; (d) `StrategyKind::build/engine_stops` jadi API publik crate `ui` agar CLI dan GUI memakai jalur strategi yang sama; (e) RSI Wilder disimpan dalam milli (×1000) setelah ditemukan truncation integer membekukan RSI di 100 pada data langkah 0/±1 tick (regresi: `rsi_not_stuck_on_one_tick_steps`). Bukti di `docs/BENCH.md` bagian Fase 4.
- 2026-10-18: Fase 5 berjalan. Perubahan saat eksekusi: (a) fuzz-lite deterministik ditambahkan dan langsung menemukan panic `i64::MIN` di parser CSV — diperbaiki `unsigned_abs` + `checked_mul`; (b) katalog SQLite dapat migrasi v0→v1 dan menolak schema lebih baru; (c) resource limits: `MAX_ROWS` 20 juta + batas berkas impor 1 GiB; (d) layout chart persist ke disk (`SOKANLAB_LAYOUT`, flag `--layout`/`--save-layout`) dengan roundtrip JSON berversi (drawing ikut tersimpan; format lama kompatibel); (e) profiling menemukan 166 ms/frame pada 1M bar → decimation `draw_step` + spacing sub-pixel → 4,9 ms/frame, RSS GUI 808→251 MB; (f) fix pemetaan viewport x/y ke ukuran pane (lebar area + `price_pane_height`) — candle tak lagi jatuh ke pane volume dan crosshair/hit-test selaras dengan posisi candle; PNG docs dirender ulang; (g) `--bench-frames` untuk profiling frame-time; (h) dokumen rilis `README.md`, `docs/BACKTEST.md`, `THIRD-PARTY.md`, skrip `check-offline.sh` & `package.sh`, CI matrix diperbarui (impor/backtest/layout/render/headless/audit offline). Bukti di `docs/BENCH.md` bagian Fase 5.
- 2026-10-18: Fase 6 batch 5 (parity TradingView: properti objek lanjutan — opasitas, level fib kustom, varian pitchfork, template gaya) selesai. Perubahan saat eksekusi: (a) `chart::drawing`: `PitchforkKind` (Andrew/Schiff/ModifiedSchiff/Inside) + `FibLevel { milli, color }` + `ForkLines`/`pitchfork_lines` (geometri median/tine/segmen bantu dipakai bersama painter & hit-test) + `DrawingItem.{opacity,pitchfork,fib_levels}` + `hit_test_item` (varian pitchfork & level fib kustom; `pick` memakainya); (b) `ui::painter`: opasitas per objek via `gamma_multiply` di seluruh warna objek (level fib berwarna, info box range/posisi ikut memudar), fib menggambar level kustom bila ada (warna per level, sisanya aturan bawaan), pitchfork memakai `pitchfork_lines` (dukung 4 varian); (c) `ui`: `ChartState.default_style` + `add_drawing` mewariskan gaya default + `set_default_style_from_selected`/`apply_style_to_same_kind` (berdasarkan `degree`), parser gaya `parse_item_style` dipakai ulang, layout `schema:5` (`opacity`, `pitchfork`, `fib`, `default_style`) dengan reader v1–v5 & penjepitan nilai liar; (d) `app`: slider opasitas, combo varian pitchfork, editor level fib (tambah/hapus/persen/warna siklus palet/kembali bawaan), tombol "Jadikan default" & "Terapkan ke sejenis" + notifikasi; (e) `headless`: demo fib 5 level kustom & fork Schiff 70%. Verifikasi: e2e v5 roundtrip byte-identik (963 B), v4→v5 hanya nomor schema, delta segitiga render −48 (fib 7→5 level) dan +40 (fork tambahan) diverifikasi lewat dua build eksperimen, bench 5,2 ms/frame (p50 5,0 / p95 5,8). Bukti: `docs/TV-PARITY.md` §11 (190 test, clippy bersih, offline LULUS, paket sha256 `36dc2ed9bd97df42c0fa38139416d307a2f8c42251a3349aef87fa34d42043ff`).
- 2026-10-18: Fase 6 batch 4 (parity TradingView: pane indikator + sumbu berlabel + resolusi snapshot) selesai. Perubahan saat eksekusi: (a) `indicators::macd_series` (MACD + sinyal dalam milli) dan **perbaikan bug nyata EMA** — rumus lama `value + k*(prev−value)` membalik bobot (α ke nilai lama); kini `prev + k*(value−prev)` sesuai definisi standar sehingga overlay EMA & MACD benar (regresi: `ema_series([10,12,14],2) = [None,11,13]`); (b) `painter`: `PlotLayout` bertambah `panes`/`axis_y`/`axis_x` + `split_plot_panes`/`plot_width`/`price_pane_height` (pane harga menyusut saat pane aktif; strip kanan 64 px, bawah 22 px), `draw_axes` (label harga & waktu UTC via `civil_from_millis`), tag crosshair + tag harga terakhir ber-kotak, label "Vol", `draw_indicator_panes` (RSI 0–100 dengan panduan; MACD simetris 0 di tengah + histogram), dan objek gambar di-clip ke pane harga; (c) `ui`: `PaneKind`/`PaneSetting`/`PaneSeries`, `ChartState::{set_panes,rebuild_panes}` + cache `panes`, `snapshot_scale`, layout `schema:4` (`panes`) dengan reader v1–v4; (d) `app`: panel samping RSI/MACD + pemilih resolusi snapshot, gestur sumbu memakai strip `axis_y`/`axis_x`; (e) `headless`: rasterizer menghormati `clip_rect` dan mendukung skala piksel snapshot (batas ~8K). Bukti: `docs/TV-PARITY.md` §10 (180 test, clippy bersih, offline LULUS, e2e v4 identik, v3→v4 hanya nomor schema, bench 4,7/5,1/5,2 ms, paket sha256 `7733b5cbfa0952be4aea0804d6a2a665a5d12a1b63dce7ef86eb06c0e1af88b0`).
- 2026-10-18: Fase 6 batch 3 (parity TradingView: skala/overlay/snapshot/teks/z-grup) selesai. Perubahan saat eksekusi: (a) `chart`: `ScaleMode` (Linear/Log/Persen) + `Viewport.scale/invert/pct_base` dengan transformasi v-space (`v_for`/`v_range`/`price_from_v`/`y_eff`) — jalur linear dipertahankan byte-identik; `Drawing::Text/Callout` (String di varian) sehingga `Drawing` tidak lagi `Copy`; `DashStyle` (solid/putus/titik); `DrawingItem.group`; helper History (`move_item`/`duplicate`/`group_members`/`assign_group`/`clear_group`); (b) `indicators`: `vwap_series` jangkar (i128); (c) `ui`: `painter` pola garis + kotak teks/callout + `draw_overlays` (hanya bar terlihat, ikut decimation); `tools.rs` Text/Callout + `Action::Snapshot` (`Alt+S`); `ChartState` skala/overlay/grup/z + `Gesture::ScalePrice`/`ScaleTime` + `MoveBody` multi-target (grup); layout `schema:3` dengan reader v1/v2/v3; (d) `app`: drag sumbu, tombol skala/invert/snapshot, panel overlay SMA/EMA/VWAP, properti pola garis + editor teks + z/duplikat/grup, menu klik kanan z/grup; (e) snapshot GUI memakai `render_chart_png` (refactor rasterizer headless bersama `--render`). Bukti: `docs/TV-PARITY.md` §9 (174 test, clippy bersih, offline LULUS, e2e v3 identik, PNG triangles 9594/61136, bench 5,0/5,2 ms, paket sha256 `0fd385db…`).
- 2026-10-18: Fase 6 batch 2 (parity TradingView: properti objek + object tree + alat lanjutan) selesai. Perubahan saat eksekusi: (a) `Drawing` 14 → 18 jenis: Parallel Channel, Pitchfork Andrew, Fib Extension (`fib_ext_price` i128-safe, level 0–261,8%), Fib Time (`fib_time_ts`, bar +1..+144, aman di ujung data); (b) model objek baru `DrawingItem` + `ItemStyle` di `chart` — `History` menyimpan metadata (tampil/kunci/nama/gaya), `pick`/`snap` melewati objek sembunyi & terkunci, `set` mempertahankan metadata saat drag; (c) `painter` memakai gaya per objek + palet 8 warna (orisinal) + gambar 4 jenis baru (channel/median pitchfork, label fib extension, garis fib time); (d) `tools.rs`: ToolKind 17 → 21 varian + `Action::OpenGoToDate` (`Alt+G`); (e) `app.rs`: panel properti (nama/warna/tebal/default/tampil/kunci/hapus), object tree dengan toggle per baris, dobel-klik sumbu (fit harga / rentang penuh), dialog Alt+G, menu klik kanan + kunci/sembunyi objek; (f) layout `schema:2` dengan metadata per objek — reader menerima v1, e2e roundtrip byte-identik; (g) `parse_ts_input` (epoch s/ms + `YYYY-MM-DD[ HH:MM[:SS]]` UTC, days-from-civil Hinnant) dengan 6 tes batas; (h) tes event sintetis baru: objek terkunci/sembunyi tidak terpilih, dobel-klik sumbu mengembalikan skala, panel objek + strip alat merender headless. Bukti: `docs/TV-PARITY.md` §8 (163 test, clippy bersih, offline LULUS, e2e identik, PNG, bench 4,9 ms, paket sha256 `d5b5cc00…`).
- 2026-10-18: Fase 6 batch 1 (parity TradingView: alat gambar + pintasan) selesai. Perubahan saat eksekusi: (a) `drawing.rs` diperluas ke 14 jenis + helper fib/position/range (i128-safe), hit-test semua jenis, snap magnet (weak: OHLC dalam toleransi; strong: anchor objek → harga hline → OHLC terdekat), constrain 45°/H/V, History undo/redo; (b) `crates/ui/src/tools.rs` baru: ToolKind (17 varian), ToolState (klik/seret/stay/preview), peta `action_for` hotkey TV; (c) `painter.rs` menggambar semua jenis + pratinjau + handle + label (level fib %, Δharga/Δ%/bar, risk-reward posisi); (d) `app.rs`: toolbar kiri, dispatch keyboard, interaksi penuh (penempatan, magnet, constrain, drag, pan, klik kanan, daftar objek, dialog F1), `ChartState::plot_rect`; (e) layout `schema:1` + `tools`/`hidden`; (f) bug `ToolKind::build` (eager `?` → alat 2-anchor selalu `None`) ditemukan tes event-driven dan diperbaiki — regresi dijaga tes klik/seret. Divergensi terdokumentasi: `1`–`9` = alat (TV: timeframe; resampling belum ada di MVP). Bukti: `docs/TV-PARITY.md` (150 test, clippy bersih, offline LULUS, e2e layout identik, render PNG, bench 5,0 ms).
