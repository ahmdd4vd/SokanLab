# Baseline Fase 1 — 2026-10-16 (Linux x86_64)

## Build & test
- Toolchain: cargo 1.99.1 / rustc 1.99.1 stable.
- `cargo fmt --check`: OK (setelah `cargo fmt`).
- `cargo clippy --workspace --all-targets -- -D warnings`: OK, 0 warning.
- `cargo test --workspace`: 17 unit test lolos, 19 target `test result: ok`.
- `cargo build --release`: OK.

## Smoke offline
- `./target/release/sokanlab-desktop --version` → `0.1.0 (schema 1)`.
- `./target/release/sokanlab-desktop --smoke` → `smoke-ok`, exit 0, tanpa jaringan.
- Biner release: 466K. Max RSS smoke: 2164 kB. Wall time: ~0.00 s.

## Privasi
- `grep -rn TcpStream|TcpListener|UdpSocket|reqwest|hyper apps crates` → tidak ada (exit 1).
- Dependensi eksternal Fase 1: nol (workspace std-only).

## Viewport (proksi Python dari rumus Rust)
- 1.000.000 transformasi `x_for_time` dalam 101,6 ms → 9,84 M/s.
- Catatan jujur: ini proksi Python, bukan ukur Rust. Bench Rust asli (`cargo bench`/criterion) ditunda ke Fase 3 saat renderer GPU/CPU dipilih.

---

## Baseline Fase 3 — 2026-10-17 (chart CPU renderer)

### Mutu
- `cargo fmt --check`: OK. `cargo clippy --workspace --all-targets -- -D warnings`: 0 warning.
- `cargo test --workspace`: 51 test lolos.
- GUI native eframe jalan di Xvfb `:99` (LIBGL_ALWAYS_SOFTWARE=1), window `SokanLab` 1200x800 tercatat.
- `sokanlab-desktop --headless`: muat 3 bar dari Parquet, layout JSON schema 1.

### Render headless → PNG (fallback CPU, tessellator egui)
Command: `sokanlab-desktop --render --out docs/shot.png --bars N --size 1200x800`

| bars | triangle | wall | RSS |
|---|---|---|---|
| 10.000 | 36.252 | 0,05 s | 32,7 MB |
| 100.000 | 36.256 | 0,09 s | 77,0 MB |
| 1.000.000 | 36.260 | 0,62 s | 667,7 MB |

Baca jujur: jumlah triangle hampir konstan (36k) walau bars naik 100x — bukti hanya bar
terlihat yang digambar, sesuai kriteria exit Fase 3. RSS 1M bar = 668 MB adalah batas MVP
(dataset penuh di memory); perlu SoA/streaming untuk dataset > 1M bar.

### Artefak visual
- `docs/shot.png` — 120 bar, ember/slate orisinal, crosshair + garis harga + objek demo:
  trendline, fib retracement, long position, date & price range (batch 1) + parallel
  channel bergaya kustom (biru, tebal 2), pitchfork Andrew, fib extension, fib time
  (batch 2) + text & callout (dash titik), overlay SMA 20 / EMA 9 / VWAP jangkar
  (batch 3) + pane RSI 14 & MACD 12/26/9, strip sumbu harga kanan + sumbu waktu bawah
  berlabel (batch 4) + **fib retracement 5 level kustom, pitchfork Schiff 70%
  (batch 5)** + **multi-seleksi dua objek + kotak seleksi putus-putus (batch 6)**.
  Dirender ulang Fase 6 batch 6 (2026-10-18); label teks tampil
  sebagai kotak karena rasterizer headless hanya mengisi triangle (font = quad
  bertekstur), dan objek gambar ter-clip di pane harga.
- `docs/shot-batch8.png` — 400 bar, 1200×700, `--demo batch8`: scene bersih khusus
  alat batch 8 (regression channel, fib trend extension, Elliott impulse/correction,
  XABCD, ABCD, Gann fan, brush, highlighter) + **baris 14 ikon sticker** untuk
  membuktikan setiap ikon digambar. Dirender ulang batch 8 (2026-10-19).
- `docs/shot-wide.png` — 100.000 bar (uji only-visible-bars).
- Verifikasi batch 2: `render-ok triangles=4762` (120 bar) dan `triangles=36876`
  (100.000 bar), 1200×800. Batch 3: `triangles=9594` dan `triangles=61136`.
  Batch 4: `triangles=19218` (120 bar, 1200×800) dan `triangles=151824`
  (100.000 bar, 1600×900). Batch 5: `triangles=19210` dan `triangles=151816`
  (dengan fib 5 level kustom + fork Schiff tambahan). Batch 6: **19990** dan
  **151890** (delta +780/+74 murni kotak seleksi; rincian + bukti eksperimen di
  bagian baseline batch 6 di bawah). Batch 7: **18510** dan **148850** (channel kini
  segmen berbatas). Batch 8: **18583** dan **148923** (delta +73/+73 dari sisipan
  demo batch 8: regression, sticker, goresan brush + highlighter; build eksperimen
  tanpa sisipan mengembalikan tepat angka batch 7 — rincian di baseline batch 8).

### Batas
- Renderer memakai `egui::Painter` (CPU tessellation), bukan instancing GPU. Ini yang
  memungkinkan verifikasi headless; jalur GPU belum diimplementasi.
- Screenshot native GPU belum diambil (tidak ada tool xwd/import di env ini).

---

## Baseline Fase 4 — 2026-10-17 (backtest + impor 1M bar, release)

Mesin uji: 2 vCPU, RAM 7,6 GB, Linux x86_64, biner `--release`.
Dataset: sintetis deterministik 1.000.000 bar (CSV 38 MB, `sin` campuran, harga bulat tick),
parquet hasil impor 13,9 MB.

| Tahap | Wall | Max RSS | Catatan |
|---|---|---|---|
| `import` 1M CSV → Parquet + katalog | 0,35 s | 163 MB | parse + validasi + sha256 + SQLite |
| `backtest --strategy buy_and_hold` 1M bar | 0,30 s | 149 MB | 1 trade, fee 19 tick |
| `backtest --strategy ma_cross 9/21` 1M bar | 0,31 s | 149 MB | 160 trade, return 2520 bps, Sharpe 500 mili |
| `backtest --strategy rsi_threshold 14/30/70` 1M bar | 0,36 s | 145 MB | 211 trade, return −2615 bps (strategi rugi di data sin ini — hasil apa adanya) |
| `backtest --strategy sl_tp 200/600` 1M bar | 0,30 s | 146 MB | 1 trade, return 5 bps |

Konteks mutu: `cargo fmt` OK, `clippy --workspace --all-targets -D warnings` 0 warning,
`cargo test --workspace` 95 test lolos. Dua run `ma_cross` pada dataset 200 bar menghasilkan
direktori laporan byte-identik (`diff -r` bersih) — bukti determinisme end-to-end.

Temuan saat benchmark (dan perbaikannya):
- RSI Wilder versi integer naif terjebak di 100 pada dataset ini karena langkah harga hanya
  0/±1 tick per bar: `avg_loss = (0*13 + 1)/14 = 0` selamanya (truncation). Terdeteksi saat
  `rsi_threshold` memberi 0 trade di 1M bar padahal 4 trade di 200 bar.
- Perbaikan: rata-rata gain/loss RSI disimpan dalam milli (×1000) sehingga presisi
  1/1000 tick; regresi dijaga test `rsi_not_stuck_on_one_tick_steps`. Setelah perbaikan
  strategi yang sama menghasilkan 211 trade (angka di tabel).

Baca jujur:
- Angka di atas termasuk baca Parquet + tulis 5 file laporan, bukan hanya loop engine.
- Metrik bps di-truncate integer: dengan modal 1.000.000 tick, drawdown di bawah ~125 tick
  terbaca 0 bps (`ma_cross` 1M bar: maxdd 0 bps walau ada 1 trade rugi kecil). Untuk uji
  presisi, gunakan modal kecil (lihat `hand_calculated_four_bars` di crate backtest).
- RSS 149–163 MB adalah dataset penuh di memory (48 byte/bar + overhead Parquet/decoder).
- Sharpe dihitung per-bar (belum annualized); pada 1M bar nilai Sharpe mengecil karena
  return per bar sangat kecil relatif derau — ini ekspektasi, bukan bug.

---

## Baseline Fase 5 — 2026-10-18 (hardening: fuzz, limits, profiling, persistence)

Mesin uji sama (2 vCPU, RAM 7,6 GB, biner `--release`). GUI diukur di Xvfb `:99`
dengan `LIBGL_ALWAYS_SOFTWARE=1` (llvmpipe; angka GPU nyata bisa berbeda).

### Mutu
- `cargo fmt --check` OK, `clippy --workspace --all-targets -D warnings` 0 warning,
  `cargo test --workspace` **116 test lolos** (dari 95 di Fase 4).
- `scripts/check-offline.sh`: 0 pola jaringan di kode, 0 klien jaringan di graph build,
  smoke impor→backtest→render lulus.
- E2E rilis: 4 strategi exit 0; dua run `ma_cross` 1M bar `diff -r` identik;
  fixture valid exit 0, 3 fixture rusak exit 2.

### CLI (1M bar, /usr/bin/time -v)

| Tahap | Wall | Max RSS | Catatan |
|---|---|---|---|
| `import` 1M CSV (38 MB) | 0,40 s | 162 MB | sama dengan Fase 4 (±noise) |
| `backtest ma_cross 9/21` | 0,31 s | 149 MB | sama dengan Fase 4 |
| `--render` 1M bar 1600x1000 | **0,15 s** | **131 MB** | Fase 4: 0,62 s / 668 MB (decimation + fix pane) |
| `--headless` (3 bar) | ~0,00 s | 10 MB | muat katalog + layout |

### GUI 1M bar (300 frame, `--bench-frames 300`)

| Metrik | Sebelum fix | Sesudah fix |
|---|---|---|
| Frame pertama | 268 ms | **9 ms** |
| Rata-rata frame | 165,9 ms (p50 164,8; p95 170,1) | **4,9 ms** (p50 4,8; p95 5,7) |
| Max RSS proses | 808 MB | **251 MB** |

GUI 200 bar: frame pertama 4 ms, rata-rata 0,2 ms, RSS 148 MB.
`--bench-frames` memaksa repaint kontinu (egui reaktif) dan mencetak
`avg/p50/p95`; waktu diukur di dalam `update()` (bangun shape), **bukan** GL swap.

### Temuan hardening (dan perbaikannya)
1. **Fuzz-lite menemukan panic nyata**: `parse_csv_ohlcv` panic `attempt to negate
   with overflow` pada timestamp `i64::MIN` (`ts_raw.abs()` lalu `* 1000`).
   Perbaikan: `unsigned_abs()` + `checked_mul`; regresi dijaga
   `timestamp_ekstrem_i64_tidak_panic` (3 test fuzz deterministik, 2301 kasus/run).
2. **Zoom-out ekstrem tidak melukis seluruh rentang**: `fit_spacing` clamp 1 px
   membuat 1M bar tampil sebagai potongan tertua + 166 ms/frame (3 shape/bar).
   Perbaikan: spacing sub-pixel (floor 1e-9) + `draw_step` (sampel tiap k-bar,
   k = ceil(1/spacing)); uji `candles_decimated_saat_subpixel`.
3. **Pemetaan viewport tidak mengikuti ukuran pane**: headless memakai tinggi
   jendela penuh (candle jatuh ke pane volume), GUI memakai ukuran basi
   (height 400 px, width 800 px) sehingga crosshair/hit-test bisa meleset dari
   posisi candle saat ukuran pane nyata berbeda.
   Perbaikan: `price_pane_height` dipakai bersama oleh `split_plot`, anchor zoom,
   `chart_panel` (resize lebar & tinggi pane tiap frame), dan render headless;
   PNG docs dirender ulang dan diverifikasi visual.

### Batasan pembacaan angka
- Software GL: frame-time nyata di GPU terdedikasi bisa jauh lebih cepat; angka ini
  batas atas CPU-side pada mesin uji.
- 116 test termasuk fuzz-lite deterministik (bukan cargo-fuzz); 2301 kasus/run
  dengan seed tetap, bukan liputan input tak terbatas.
- Layout persistence diuji lewat `--save-layout`/`--headless` (jalur sama dengan
  `on_exit`); penutupan GUI normal tak bisa diotomasi di env ini (tanpa WM control).

---

## Baseline Fase 6 batch 3 — 2026-10-18 (skala, overlay, snapshot)

Mesin uji sama (2 vCPU, RAM 7,6 GB, biner `--release`, Xvfb `:99`, llvmpipe).
Dataset: `SYNTHUSDT-1m` 1.000.000 bar (13,9 MB parquet, harga 8.735–11.265 tick).

### GUI 1M bar (200 frame, `--bench-frames 200`, viewport penuh)

| Skenario | Frame pertama | Rata-rata | p50 | p95 |
|---|---|---|---|---|
| Tanpa overlay (baseline batch 2) | 9 ms | **5,0 ms** | 4,9 | 5,4 |
| Overlay SMA 20 + EMA 9 + VWAP | 13 ms | **5,2 ms** | 5,0 | 6,1 |

Baca jujur:
- Overlay dihitung **sekali** saat data/setelan berubah (3 seri × 1M bar, ~4 ms di
  frame pertama), bukan tiap frame; gambar per frame memakai decimation yang sama
  dengan candle, jadi biayanya tetap ~0,2 ms pada viewport penuh.
- Snapshot `Alt+S` memakai rasterizer headless yang sama dengan `--render`; tidak
  menggambar font bertekstur (teks = kotak di PNG) dan tanpa panel samping — itu batas
  jalur CPU headless, bukan regresi.
- Angka frame-time adalah CPU-side pembangunan shape (bukan GL swap), batas atas pada
  mesin uji tanpa GPU.

## Baseline Fase 6 batch 4 — 2026-10-18 (pane indikator + sumbu berlabel + resolusi snapshot)

Mesin & dataset sama dengan batch 3 (Xvfb `:99`, llvmpipe, `--release`,
`SYNTHUSDT-1m` 1.000.000 bar). Run: `--bench-frames 120` dari `/tmp/opencode/prof`.

| Skenario | Frame pertama | Rata-rata | p50 | p95 |
|---|---|---|---|---|
| Tanpa overlay | 12 ms | **4,7 ms** | 4,6 | 5,2 |
| Overlay SMA 20 + EMA 9 + VWAP | 13 ms | **5,1 ms** | 4,9 | 6,0 |
| Overlay + pane RSI 14 + MACD 12/26/9 | 12 ms | **5,2 ms** | 5,1 | 5,8 |

Baca jujur:
- Pane indikator menambah **O(bar terlihat) per pane** saat menggambar: auto-scale
  MACD memindai nilai terlihat mengikuti `draw_step` (decimation yang sama dengan
  candle), jadi biayanya tetap submilisekon pada viewport penuh — 0,1 ms vs tanpa pane.
- Sumbu berlabel (harga kanan + waktu bawah) hanya menggambar ~6×2 teks + 2 garis;
  tidak terukur menambah frame time.
- Snapshot resolusi 2×/4× meraster ulang pada skala piksel (bukan upscale), jadi biaya
  waktunya ~skala². Batas aman ~8K (mis. 1200×700 @4× = 4800×2800).
- Angka frame-time tetap CPU-side pembangunan shape (bukan GL swap).

## Baseline Fase 6 batch 5 — 2026-10-18 (opasitas, level fib kustom, varian pitchfork, template gaya)

Mesin & dataset sama dengan batch 4 (Xvfb `:99`, llvmpipe, `--release`,
`SYNTHUSDT-1m` 1.000.000 bar). Run: `--bench-frames 120` dari `/tmp/opencode/prof`
(layout `bench-panes.json`: overlay + 2 pane).

| Skenario | Frame pertama | Rata-rata | p50 | p95 |
|---|---|---|---|---|
| Overlay + pane RSI 14 + MACD 12/26/9 | 11 ms | **5,2 ms** | 5,0 | 5,8 |

Baca jujur:
- Opasitas per objek hanya mengalikan warna efektif (`gamma_multiply`) — nol biaya
  tambahan. Level fib kustom & varian pitchfork mengubah jumlah garis/level, bukan
  algoritmanya; geometri tetap O(1) per objek.
- Tidak ada regresi terukur vs batch 4 (5,2 ms; p50 5,1 → 5,0; p95 5,8 sama).

### Artefak visual batch 5 (delta terukur)

Demo headless menambah: fib retracement dengan 5 level kustom (2 level lebih sedikit
dari bawaan) dan satu pitchfork Schiff semi-transparan (70%).

- `docs/shot.png` (120 bar, 1200×800): **19210** segitiga (batch 4: 19218).
- `docs/shot-wide.png` (100.000 bar, 1600×900): **151816** segitiga (batch 4: 151824).
- Delta diverifikasi lewat eksperimen build terpisah: level fib kustom (7→5 level)
  **−48** segitiga (2 garis + 2 label; tiap glyph label = 1 quad) dan pitchfork
  tambahan **+40** segitiga (4 garis panjang; tessellator memecah segmen sangat
  panjang menjadi beberapa quad) → 19218 − 48 + 40 = 19210 ✓ konsisten.

### Perbaikan yang ditemukan saat verifikasi batch 4

- **EMA**: rumus lama `value + k*(prev - value)` membalik bobot (α ke nilai lama);
  kini `prev + k*(value - prev)`. Overlay EMA dan dasar perhitungan MACD ikut berubah —
  ini perbaikan kebenaran, bukan perubahan kosmetik.
- **Rasterizer headless**: mengabaikan `clip_rect` dari tessellator sehingga objek
  gambar menembus pane di PNG; sekarang clip dihormati (backend GPU GUI tidak pernah
  terpengaruh).

## Baseline Fase 6 batch 6 — 2026-10-18 (multi-seleksi & operasi kolektif)

Mesin & dataset sama dengan batch 5 (Xvfb `:99`, llvmpipe, `--release`,
`SYNTHUSDT-1m` 1.000.000 bar). Run: `--bench-frames 40` dari `/tmp/opencode/prof`
(layout `bench-panes.json`: overlay + 2 pane), **tiga kali** untuk melihat variasi
antar-run.

| Skenario | Frame pertama | Rata-rata | p50 | p95 |
|---|---|---|---|---|
| Overlay + pane RSI 14 + MACD 12/26/9 (run 1) | 11 ms | **5,4 ms** | 5,2 | 6,3 |
| Overlay + pane RSI 14 + MACD 12/26/9 (run 2) | 14 ms | **5,9 ms** | 5,4 | 9,6 |
| Overlay + pane RSI 14 + MACD 12/26/9 (run 3) | 13 ms | **5,6 ms** | 5,2 | 9,4 |

Baca jujur:
- Batch 5 mengukur **5,2 ms** (p50 5,0) di mesin yang sama; ketiga run batch 6
  (5,4/5,9/5,6 ms) berada di dalam variasi antar-run mesin virtual ini (±10%) —
  jangan dibaca sebagai regresi pasti. Yang berubah di jalur frame: `ChartState`
  memanggil `selection()` (satu `Vec` kecil, ≤ jumlah objek) saat menggambar dan
  panel membaca seleksi 1–2 kali; pencocokan objek terpilih pakai `contains` O(n)
  dengan n = jumlah objek (belasan). Tidak ada alokasi per objek per frame.
- Kotak seleksi **hanya** digambar saat gestur `Ctrl+seret` aktif (`Gesture::Rubber`);
  pada bench (tidak ada interaksi) biayanya nol.
- Biaya operasi kolektif juga O(n): `move_items_z` hanya menukar dua posisi per
  langkah dan `remove_many`/`duplicate_many` mengambil satu snapshot untuk seluruh
  seleksi (sebelumnya satu snapshot per objek).

### Artefak visual batch 6 (delta terukur)

Demo headless menambah: multi-seleksi dua objek terakhir (ditandai ember, satu
primer ber-handle) dan **kotak seleksi** putus-putus Ember di area objek tersebut.

- `docs/shot.png` (120 bar, 1200×800): **19990** segitiga (batch 5: 19210).
- `docs/shot-wide.png` (100.000 bar, 1600×900): **151890** segitiga (batch 5: 151816).
- Delta diverifikasi lewat dua build eksperimen (menghapus hanya pemanggilan
  `draw_rubber_band` di jalur render): hasilnya **persis angka batch 5** —
  19210 dan 151816 — sehingga delta **+780** (120 bar; kotak ≈ 255×75 px, isi 1 quad
  + tepi putus-putus ~4,75 quad/dash) dan **+74** (100.000 bar; kotak jauh lebih
  sempit karena 22 bar dari 100.000) murni berasal dari kotak seleksi. Perubahan
  seleksi dari 1 objek ke 2 objek tidak menambah segitiga (hanya warna).

## Baseline Fase 6 batch 7 — 2026-10-19 (varian channel + Extend + katalog template)

Mesin & dataset sama dengan batch 6 (Xvfb `:99`, llvmpipe, `--release`,
`SYNTHUSDT-1m` 1.000.000 bar). Run: `--bench-frames 40` dari `/tmp/opencode/prof`.

| Skenario | Frame pertama | Rata-rata | p50 | p95 |
|---|---|---|---|---|
| Overlay + pane RSI 14 + MACD 12/26/9 (run 1) | 36 ms | **6,1 ms** | 5,2 | 7,1 |
| Overlay + pane RSI 14 + MACD 12/26/9 (run 2) | 13 ms | **5,4 ms** | 5,2 | 5,8 |
| Overlay + pane RSI 14 + MACD 12/26/9 (run 3) | 15 ms | **5,5 ms** | 5,2 | 6,2 |
| **+ 3 channel di layar** (`bench-channels7.json`: disjoint `extend:right`, flat, paralel `extend:both`) run 1 | 12 ms | **5,2 ms** | 5,0 | 5,7 |
| idem run 2 | 13 ms | **5,3 ms** | 5,0 | 6,7 |

Baca jujur:
- Batch 6 mengukur 5,4–5,9 ms (p50 5,2–5,4) di mesin yang sama; tiga run batch 7
  (6,1/5,4/5,5; p50 5,2 ketiganya) masih di dalam variasi antar-run (±10%). Run 1
  dengan frame pertama 36 ms adalah outlier pemuatan awal Xvfb (bukan jalur frame).
- Channel tidak menambah biaya terukur: 3 channel (satu dengan `extend:both`) = 5,2–5,3
  ms vs 5,4–6,1 ms tanpa channel. Tiap channel menggambar **2 segmen** (O(1));
  perpanjangan hanya memperpanjang segmen, dan clipping plot membatasi pikselnya.
- `channel_segments` dihitung per frame per channel (beberapa operasi f64) — tidak
  terukur menambah waktu pada 1M bar karena jumlah objek kecil (hitungan lusinan).

### Artefak visual batch 7 (delta terukur)

Demo headless menambah **Disjoint Channel** (3 titik, `extend` kanan) dan **Flat
Top/Bottom Channel** (batas kedua horizontal). Perubahan kedua yang terlihat di PNG:
garis channel sekarang **segmen berbatas** (opsi Extend per objek, default tidak
diperpanjang seperti TV) — bukan lagi garis tak hingga seperti batch 1–6.

- `docs/shot.png` (120 bar, 1200×800): **18510** segitiga (batch 6: 19990).
- `docs/shot-wide.png` (100.000 bar, 1600×900): **148850** segitiga (batch 6: 151890).
- Atribusi lewat build eksperimen yang **hanya menghapus dua channel demo baru**
  (kode channel dibatasi tetap aktif): hasilnya **18470 / 148810** → jadi
  −1520 / −3080 berasal dari channel lama yang kini berbatas (channel demo batch 2
  bergaya putus-putus; setiap dash = beberapa quad) dan **+40 / +40** dari dua
  channel baru (2 garis × 2 channel; segmen pendek). 19990 − 1520 + 40 = 18510 ✓,
  151890 − 3080 + 40 = 148850 ✓. PNG deterministik: build ulang setelah restore
  menghasilkan PNG **identik byte-for-byte** dengan `docs/shot.png`.

### Mutu & e2e batch 7

- `cargo test --workspace`: **210 lolos, 0 gagal** (batch 6: 204); `cargo fmt --check`
  dan `clippy --workspace --all-targets -- -D warnings` bersih (0 peringatan);
  `scripts/check-offline.sh` LULUS.
- E2E CLI 1M bar: berkas v5 lama → v6 **hanya nomor schema** yang berubah; berkas v6
  (varian channel `disjoint`/`flat`, `extend` kanan/keduanya, 3 template) muat →
  simpan → muat → simpan: **identik byte-for-byte** (`cmp`, 912 B), idempoten.
- Verifikasi visual: 5 render PNG fokus (satu per varian + extend kiri/kanan, viewport
  di-zoom) dari state nyata; bentuk sesuai deskripsi publik TV. Render eksperimen itu
  tidak disimpan; bukti permanen = tes geometri `channel_segments` + `docs/shot.png`.
- Paket ulang: `dist/sokanlab-0.1.0-linux-x86_64.tar.gz` sha256
  `23847e1f85b7a5fd6e9be0938766e14b0788bfa0895233be94c106d88346c066` (`sha256sum -c`
  OK; biner di dalam tar identik dengan `target/release`).


## Baseline Fase 6 batch 8 — 2026-10-19 (10 alat lanjutan + UI/UX teks & klon Ctrl+seret)

Mesin & dataset sama dengan batch 6/7 (Xvfb `:99`, llvmpipe, `--release`,
`SYNTHUSDT-1m` 1.000.000 bar). Run: `--bench-frames 40` dari `/tmp/opencode/prof`.
Layout baru: `bench-batch8.json` (3 regression channel rentang ~600k/750k/380k bar +
2 Gann fan + Elliott impulse/correction + XABCD + ABCD + brush 400 titik +
highlighter 100 titik + 4 sticker), sub-layout atribusi `b8-reg`, `b8-gann`,
`b8-stroke`, `b8-sticker`, `r1-satu` (1 regression 600k bar), `r5-tiga-pendek`
(3 regression 100 bar).

| Skenario | Rata-rata | p50 | p95 |
|---|---|---|---|
| Overlay + pane RSI 14 + MACD 12/26/9 (run 1/2/3) | **5,5 / 5,4 / 5,5 ms** | 5,2 / 5,1 / 5,2 | 5,6 / 8,1 / 7,0 |
| + 3 channel di layar (`bench-channels7.json`, run 1/2/3) | **5,3 / 5,3 / 5,3 ms** | 4,9 / 5,1 / 5,0 | 6,9 / 7,1 / 7,7 |
| **Scene batch 8 berat** (`bench-batch8.json`, run 1/2/3) | **14,6 / 14,0 / 14,5 ms** | 14,1 / 13,6 / 14,1 | 20,0 / 19,4 / 20,2 |
| 1 regression channel, rentang 600k bar (`r1-satu`, 2 run) | 9,1 / 9,2 ms | 8,8 / 9,0 | 11,4 / 12,7 |
| 3 regression channel, rentang 100 bar (`r5-tiga-pendek`, 2 run) | 5,2 / 5,3 ms | 5,0 / 5,0 | 5,8 / 6,1 |
| 2 Gann fan + Elliott + XABCD + ABCD (`b8-gann`, 2 run) | 5,2 / 5,1 ms | 4,9 / 4,9 | 6,4 / 5,6 |
| Brush 400 titik + highlighter (`b8-stroke`, 2 run) | 5,1 / 5,2 ms | 4,9 / 4,9 | 6,4 / 5,7 |
| 4 sticker (`b8-sticker`, 2 run) | 5,2 / 5,4 ms | 5,0 / 5,1 | 5,9 / 7,8 |

Baca jujur:
- **Atribusi biaya**: hanya regression channel yang terukur menambah waktu, dan
  biayanya murni panjang rentang. Scene tanpa regression (Gann/pola/goresan/sticker)
  = 4,9–5,1 ms (setara baseline), 3 regression berentang 100 bar = 5,0 ms, sedangkan
  1 regression 600k bar = 8,8–9,0 ms. Micro-bench `regression_fit` (release):
  **0,003 ms @1k / 0,34 ms @100k / 1,85 ms @600k bar** — dua lintasan O(rentang)
  (jumlah, lalu residual untuk σ). Jadi objek regresi dengan rentang ratusan ribu bar
  adalah kasus terburuk yang disengaja (3 objek = ~6 ms matematika murni per frame);
  rentang wajar (≤10k bar) tidak terukur.
- **Temuan + perbaikan**: instrumentasi sementara menunjukkan `regression_fit`
  dipanggil **~1,9× per objek per frame** — sekali untuk menggambar, sekali lagi dari
  hit-test hover (blok kursor ikut memanggil `pick` tiap frame). Perbaikan batch 8:
  `ChartState::hover_pick` menyimpan hasil dengan kunci lengkap (posisi lokal,
  viewport, jumlah objek, kedalaman undo, sembunyi-semua, revisi dataset `bars_rev`)
  dan hasilnya dipakai bersama blok kursor + render → **1,0× per frame** per objek
  (dibuktikan hitungan ulang: 41 panggilan / 40 frame untuk 1 objek; 123 / 40 untuk
  3 objek). Scene berat turun **36–38 ms → 13,6–14,1 ms** (p50) tanpa mengubah
  perilaku jalur lain (saat mode gambar/gestur aktif, `pick` langsung tetap dipakai
  seperti sebelumnya).
- Kandidat batch 9: prefix-sum per dataset (`Σ1, Σi, Σi², Σi·y`) → `regression_fit`
  O(1) dan scene berat kembali ke baseline; harga: ~32 MB untuk 1M bar (RSS sekarang
  ~668 MB untuk dataset penuh).

### Artefak & determinisme batch 8

- Render headless: `docs/shot-batch8.png` (400 bar, 1200×700, `--demo batch8`)
  **13338** segitiga; `docs/shot.png` **18583** (120 bar, 1200×800) dan
  `docs/shot-wide.png` **148923** (100.000 bar, 1600×900).
- Atribusi delta: build eksperimen yang hanya mematikan sisipan demo batch 8 di scene
  penuh menghasilkan **18510 / 148850** — tepat angka batch 7 — sehingga sisipan batch 8
  (regression + sticker + goresan) menyumbang **+73 / +73**. Dua sticker batch 8 di
  `docs/shot.png` juga dijelaskan oleh +73 ini.
- Determinisme: build ulang dari sumber yang sama menghasilkan `docs/shot.png` dan
  `docs/shot-wide.png` **identik byte-for-byte** (`cmp` bersih).
- Batas: teks (label titik pola, label level fib, label sumbu) tetap digambar sebagai
  kotak pada raster headless — jalur font bertekstur belum ada; GUI normal menampilkan
  teks.
