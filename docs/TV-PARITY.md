# Parity TradingView: alat gambar & pintasan keyboard

Dokumen ini menjawab satu pertanyaan jujur: **seberapa dekat SokanLab dengan TradingView
untuk alat gambar (drawing) dan pintasan keyboard?** Isinya: metode riset, apa yang sudah
sama, apa yang belum, dan rencana batch berikutnya.

Bukan klon: SokanLab memakai Rust, offline, fokus backtest personal. Kita meniru
**perilaku/konvensi** (klik-titik, magnet, hotkey) — bukan kode, aset, atau ikon TV.

## 1. Metode riset (dan batasnya)

Artifact yang diperiksa:

- `TradingView.msix` (TradingView Desktop, Electron) → 119 file; `resources/app.asar`
  38.119.804 byte. Diekstrak ke `/tmp/opencode/tv/app` (234 file, 3 file unpacked:
  `native/tvdbridge.node`, `keytar.node`, `build/Release/classic_level.node`).
- Analisis REA `analyze_javascript_application` pada `app.asar` (index.js + package.json):
  artifact sha256 `a70a18bfd8f1db8d113993709d8294344545a29b3ccf7f7c3436667907023906`,
  166 file relevan, 590.597 node AST, 908 modul, 2.839 findings, 3 kanal IPC literal +
  3 observasi validasi sender (evidence `ev_a5deb67cfdb47ace47e756b4c4838f7800c99bb9b8438598e61f492e643bc637`).

**Temuan penting:** bundle desktop TV *tidak memuat mesin chart*. Yang dikirim hanya shell
(window/tab, menu, toast, splash) + jembatan native. Toolbar drawing, hotkey chart, dan
seluruh mesin render dimuat dari server saat runtime (offline tidak bisa diandalkan untuk
mempelajarinya dari artifact).

Karena itu daftar otoritatif diambil dari **dokumentasi publik/panduan resmi TV**
(perilaku antarmuka — bukan kode):

- Halaman *Keyboard shortcuts* chart TV (Alt+T/H/V/C/F, Ctrl+Alt+H, Alt+R, Ctrl+Z/Y,
  panah, Ctrl+S, digit timeframe, dsb).
- *Drawings List* charting library (daftar lengkap ~60+ alat: trend line → Gann,
  Elliott, pattern, sticker, dst).
- Halaman *Desktop app keyboard shortcuts* (Ctrl+T/N/W/,, F5 — level aplikasi).

Semua pemetaan tombol di SokanLab diimplementasikan ulang dari nol (cleanroom); literal
accelerator shell TV dipakai hanya untuk mencocokkan konvensi.

## 2. Kondisi SokanLab: awal → batch 1 → … → batch 7 → batch 8

| Aspek | Awal (Fase 5) | Batch 1 | Batch 2 | Batch 3 | Batch 4 | Batch 5 | Batch 6 | Batch 7 | Batch 8 (kini) |
|---|---|---|---|---|---|---|---|---|---|
| Jenis alat | 5 | 14 | 18 | **20** (+teks, callout) | 20 (fokus batch 4 di indikator & sumbu) | 20 (batch 5 melengkapi properti objek) | 20 (batch 6 = seleksi banyak & operasi kolektif) | **22 (+Disjoint Channel, Flat Top/Bottom Channel)**  **33 alat tersimpan (+10: Regression, Fib Trend Extension, Elliott Impulse, Elliott Correction, XABCD, ABCD, Gann Fan, Brush, Highlighter, Sticker)** |
| Penempatan di GUI | 1 tombol | klik-klik / seret-satu-kali + pratinjau | + 3 alat ber-anchor-tiga, fib time | sama; teks diisi dari panel properti | sama | sama | sama | sama (varian channel = jenis alat terpisah)  sama; goresan Brush/Highlighter dibuat dengan seret (titik disampel tiap ≥3 px), Sticker sekali klik; strip alat kini dapat di-scroll (35 entri) |
| Magnet/snap | tidak ada | Off/Weak/Strong (OHLC + anchor objek) | magnet melewati objek tersembunyi/terkunci | sama | sama | sama | sama | sama  sama; goresan sengaja **tanpa** magnet (titik mentah, sama seperti TV) |
| Constrain | tidak ada | Shift: H/V/45° | sama | sama | sama | sama | sama | sama  sama (tidak berlaku untuk goresan) |
| Edit objek | pilih + hapus | drag handle & badan, daftar objek | + gaya per objek (8 warna, tebal 1–4) | + pola garis (solid/putus/titik), duplikat, z-order, grup | + objek di-clip ke pane harga | + opasitas 0–100%, level fib kustom, varian pitchfork | **+ seleksi banyak (Ctrl+klik, kotak seleksi Ctrl+seret): geser/hapus/z-order/duplikat/grup/gaya sekaligus** | **+ varian channel + opsi Extend per objek + katalog template gaya bernama (Simpan/Terapkan/Hapus)**  **+ Ctrl+seret badan objek = klon lalu geser (konvensi TV), ukuran (S/M/L) & perataan (kiri/tengah/kanan) teks per objek, ikon sticker dari kombo (14 ikon vektor), centang perluasan-kiri fib trend** |
| Object tree | tidak ada | daftar + pilih + hapus | + tampil/kunci/nama + panel properti | + penanda grup, tombol z/duplikat/grup | sama | + "Jadikan default" & "Terapkan ke sejenis" | **+ baris terpilih ditandai, Ctrl+klik di daftar, catatan jumlah terpilih** | + seksi "Template gaya" (kombo + nama + tombol)  sama |
| Indikator | tidak ada | tidak ada | tidak ada | overlay SMA/EMA/VWAP jangkar | **+ pane terpisah RSI & MACD** | sama | sama | sama  sama |
| Sumbu | tidak ada interaksi | tidak ada interaksi | dobel-klik reset (fit harga / rentang penuh) | + skala log/persen/invert; drag sumbu = skala langsung | **+ strip sumbu berlabel** | sama | sama | sama  sama |
| Snapshot | — | — | — | Alt+S (area chart, tanpa panel) | **+ opsi resolusi 1×/2×/4×** | sama | sama | sama  sama |
| Pintasan keyboard | 0 | 15+ aksi, dialog F1 | + Alt+G, dobel-klik sumbu | + Alt+S (snapshot PNG) | sama (tidak ada hotkey baru) | sama | **+ tidak ada hotkey baru; Ctrl+klik & Ctrl+seret = konvensi mouse TV (tercatat di dialog F1)** | **+ tidak ada hotkey baru: TV juga tidak memetakan tombol untuk Disjoint/Flat Channel maupun template**  **tidak ada hotkey baru (TV juga tidak memetakan); digit 1–9 tetap 9 entri pertama (Select…Arrow) — alat baru masuk setelahnya sehingga peta lama tidak bergeser** |
| Objek & persistensi | `drawings` sederhana | 14 jenis, `schema:1` | metadata per objek di `schema:2` | teks/callout + dash + grup + skala + overlay di `schema:3` | pane indikator di `schema:4`; berkas v1–v3 tetap termuat | opasitas/fib kustom/varian pitchfork/gaya default di `schema:5` | **`schema:5` tetap (seleksi tidak dipersistensi); berkas v1–v4 tetap termuat** | **`schema:6`: varian channel (`variant`), perpanjangan (`extend`), katalog `templates`; berkas v1–v5 tetap termuat**  **`schema:7`: meta `font`/`align` per teks (hanya bila ≠ bawaan) + 10 kind baru (`regression`, `fibtrend`, pola ber-`pts`, `gannfan`, `brush`/`highlighter` (≤4000 titik), `sticker` + `icon`); berkas v1–v6 tetap termuat** |

### Alat yang sudah ada (33 + quick measure)

Trend Line, Ray, Extended Line, Horizontal Line, Horizontal Ray, Vertical Line,
Crossline, Arrow, Rectangle, Ellipse, Triangle, Fib Retracement (7 level bawaan
0/23,6/38,2/50/61,8/78,6/100% — **bisa diganti level kustom**), Date & Price Range
(Δharga, Δ%, jumlah bar), Long Position, Short Position (stop dicerminkan dari entry),
Parallel Channel, **Disjoint Channel** (batas kedua = salinan yang digeser, jadi titik
awalnya boleh berbeda di sumbu waktu) dan **Flat Top/Bottom Channel** (batas kedua
horizontal di harga titik ketiga) — ketiganya 3 titik, sama-sama punya opsi
**Extend** kiri/kanan/keduanya, Pitchfork (3 titik; **varian Andrew/Schiff/Modified
Schiff/Inside**), Fib Extension (0/61,8/100/127,2/161,8/261,8%), Fib Time (bar
+1,+2,+3,+5,+8,…+144), **Text**, **Callout (teks + ekor penunjuk)**, **Regression Channel** (2 titik; garis
regresi kuadrat-terkecil atas close seluruh bar di rentang × indeks bar, plus dua batas
±σ), **Fib Trend Extension** (3 titik; level 0/61,8/100/127,2/161,8/261,8% digambar
horizontal dari titik ketiga ke kanan, level 100% disorot),
**Elliott Impulse** (5 titik berlabel 1–5), **Elliott Correction** (3 titik berlabel
A–C), **XABCD** (5 titik berlabel X-A-B-C-D), **ABCD** (4 titik), **Gann Fan** (2 titik;
9 rasio 1×8…8×1, garis 1×1 tepat a→b), **Brush** dan **Highlighter** (goresan bebas
ber-titik banyak; highlighter tebal ×3 & alpha 35%), **Sticker** (14 ikon vektor
bawaan), Measure (kilat, tidak disimpan).

Setiap objek punya metadata: **tampil/sembunyi, kunci, nama kustom, warna (8 pilihan),
ketebalan (1–4), pola garis (solid/putus/titik), opasitas (0–100%), grup**; channel
juga menyimpan **varian** dan **perpanjangan**, teks menyimpan **ukuran** dan
**perataan** — semuanya ikut tersimpan di layout `schema:7`.

Indikator (batch 4): **pane RSI** (0–100, panduan 30/50/70) dan **pane MACD**
(garis + sinyal + histogram, skala simetris otomatis) di bawah pane volume —
maksimum 3 pane, periode diatur dari panel samping; ditambah overlay SMA/EMA/VWAP
di atas candle. Objek gambar kini di-clip ke pane harga seperti di TV.

Properti objek lanjutan (batch 5): **opasitas 0–100%** per objek (berlaku ke seluruh
garis & label objek, termasuk level fib berwarna), **level fib kustom** (tambah/hapus,
geser persen satuan 0,1%, warna per level dengan siklus otomatis↔8 warna palet,
"Kembali bawaan" = 7 level standar), **varian pitchfork** dengan definisi publik
(median Andrew A→titik tengah B–C; Schiff origin = waktu A & harga rata-rata A–B;
Modified Schiff = titik tengah A–B; Inside = titik tengah A–B → C, tine lewat A & B;
garis tine selalu sejajar median; geometri dipakai bersama painter & hit-test), dan
**template gaya**: "Jadikan default" (objek baru mewarisi gaya) + "Terapkan ke
sejenis" (samakan gaya & opasitas ke semua objek ber-`degree` sama).

Seleksi banyak (batch 6): **Ctrl+klik** (atau ⌘+klik di macOS) menambah/melepas
objek dari seleksi — persis konvensi mouse TV ("Select multiple drawings: Hold
Ctrl + Click") — dan **Ctrl+seret di area kosong** menggambar **kotak seleksi**
(TV: "hold CTRL and drag → box that you can drag over multiple drawings"). Objek
tersentuh kotak = titik anchor-nya di dalam kotak atau segmennya memotong kotak
(garis horizontal/vertikal dinilai dari posisi garisnya); objek tersembunyi/
terkunci dilewati. Objek terpilih ditandai ember; **primer** (indeks teratas)
yang memegang handle edit & panel properti. Operasi kolektif: geser bersama saat
badan di-drag, hapus (satu langkah undo), z-order naik/turun satu langkah dengan
urutan relatif tetap, duplikat sekaligus, grup satu id, serta edit
warna/tebal/pola/opasitas yang diterapkan ke semua yang terpilih.

Varian channel + template gaya bernama (batch 7):
- **Disjoint Channel** dan **Flat Top/Bottom Channel** mengikuti deskripsi publik TV
  masing-masing: disjoint = "two parallel trendlines that do not necessarily share the
  same starting points on the time axis" (batas kedua digambar sebagai **salinan**
  batas pertama yang digeser oleh titik ketiga, sehingga titik awalnya boleh berbeda di
  sumbu waktu); flat = "horizontal boundary with an angled trendline on the opposite
  side" (batas kedua horizontal di harga titik ketiga, sepanjang rentang batas
  pertama). Varian disimpan di dalam objek (`variant`) — label objek otomatis
  menyesuaikan (Disjoint Channel / Flat Top/Bottom Channel).
- **Extend** per objek channel (tidak diperpanjang / kiri / kanan / keduanya) dengan
  dua centang di panel properti, seperti opsi Extend pada alat channel TV. Default
  **tidak diperpanjang** (sama dengan TV: perlu diperpanjang "if the user wants the
  channel to be continued indefinitely"). Perpanjangan mengikuti sumbu waktu: "kiri" =
  mundur dari ujung kiri, "kanan" = maju dari ujung kanan; garis hampir vertikal tidak
  bisa diperpanjang kiri/kanan (tidak ada komponen waktu).
- **Katalog template gaya bernama**: simpan cuplikan warna/tebal/pola + opasitas dari
  objek primer (nama dipangkas, maksimum 32 karakter, katalog 16 entri, nama sama =
  diperbarui), **Terapkan** ke seluruh objek terpilih (melengkapi multi-seleksi batch 6),
  dan **Hapus**. TV menempatkan template di dialog pengaturan objek dan menerapkannya
  per jenis alat; SokanLab menyederhanakannya menjadi katalog generik (gaya + opasitas,
  tidak terikat jenis alat) yang ikut tersimpan di layout.

Alat & interaksi batch 8:
- **Regression Channel**: regresi kuadrat-terkecil atas **close** bar di rentang a→b
  dengan x = indeks bar; garis tengah + dua batas ±σ (σ = simpangan baku residual,
  dibulatkan ke tick). Opsi **Extend** sama seperti channel lain, memakai
  `channel_segments` yang dipakai bersama painter & hit-test.
- **Fib Trend Extension** mengikuti deskripsi publik TV ("levels are drawn to the right
  of the third point"): level horizontal digambar dari x titik ketiga ke tepi kanan
  plot; Extend kiri memperpanjang ke tepi kiri, Extend kanan tidak mengubah apa pun
  karena level memang sudah mencapai tepi kanan (perilaku baku TV).
- **Elliott** (impulse 1–5, koreksi A–B–C) dan **pattern** (XABCD, ABCD) = polyline
  berlabel titik, satu sumber geometri untuk painter dan hit-test; jumlah titik efektif
  divalidasi saat memuat JSON (pola butuh tepat 5/3/5/4; data rusak ditolak).
- **Gann Fan**: sembilan rasio TV (1×8, 1×4, 1×3, 1×2, 1×1, 2×1, 3×1, 4×1, 8×1)
  dihitung dari vektor a→b; garis 1×1 tepat a→b, rasio lain menskalakan komponen harga.
  Sudut layarnya bergantung aspek plot (persis catatan TV: "the angle depends on the
  chart's aspect ratio"), jadi yang disimpan adalah rasio harga↔waktu yang deterministik.
- **Brush / Highlighter**: titik disampel dari pointer saat seret (**tanpa** magnet,
  ambang 3 px), batas 4000 titik per objek; highlighter digambar tebal ×3 dengan alpha
  35%. Tool klik-untuk-menempatkan sengaja tidak dipakai untuk kedua alat ini.
- **Sticker**: 14 ikon vektor orisinal (ArrowUp/Down/Left/Right, TriangleUp/Down,
  Diamond, Circle, Square, Star, Check, Cross, Plus, Flag) berukuran tetap; ikon tak
  dikenal di berkas JSON jatuh ke ArrowUp (bukan emoji pihak lain).
- **UI/UX**: ukuran teks (S/M/L = 12/16/22 px) dan perataan (kiri/tengah/kanan) per
  objek Text/Callout; **Ctrl+seret pada objek = klon lalu geser** (TV: Ctrl+drag untuk
  duplikat) — keputusan "toggle seleksi" vs "klon" ditunda sampai pointer dilepas atau
  bergerak >4 px, dan objek terkunci dilewati; strip alat dibungkus area gulir karena
  entri kini 35.

## 3. Peta pintasan: SokanLab vs TradingView

Yang **sama dengan TV** (dipakai hanya bila fiturnya ada di SokanLab):

| Tombol | Aksi | Sumber TV |
|---|---|---|
| `Alt+T` | Trend Line | ✓ |
| `Alt+H` | Horizontal Line | ✓ |
| `Alt+V` | Vertical Line | ✓ |
| `Alt+C` | Crossline | ✓ |
| `Alt+F` | Fib Retracement | ✓ |
| `Ctrl+Alt+H` | Sembunyikan/tampilkan semua drawing | ✓ |
| `Alt+R` | Reset tampilan (seluruh data) | ✓ |
| `Alt+G` | Pergi ke tanggal/waktu (dialog: `YYYY-MM-DD[ HH:MM]` atau epoch) | ✓ (TV: go to date) |
| `Alt+S` | Snapshot chart → PNG di `data/` | ✓ (TV: snapshot) |
| Drag sumbu harga / waktu | Skala vertikal / horizontal langsung (dobel-klik = reset) | ✓ |
| Dobel-klik sumbu harga / waktu | Fit ulang skala harga / rentang waktu penuh | ✓ |
| `Ctrl+Z` / `Ctrl+Y` / `Ctrl+Shift+Z` | Undo / redo | ✓ |
| `Ctrl+S` | Simpan layout | ✓ (TV: save layout) |
| `←` / `→` | Geser 1 bar | ✓ |
| `Ctrl+←` / `Ctrl+→` | Geser 10 bar | ✓ (TV: "move further") |
| `Ctrl+↑` / `Ctrl+↓` | Zoom in / out | ✓ |
| `Shift` + scroll | Geser horizontal (scroll = zoom di kursor) | ✓ |
| `Shift` + seret saat menggambar | Kunci horizontal/vertikal/45° | ✓ |
| `Esc` | Batalkan penempatan → keluar mode → lepas pilihan | ✓ |
| `Del` / `Backspace` | Hapus semua objek terpilih | ✓ |
| `Ctrl` + klik objek | Tambah/lepas objek dari seleksi (multi-seleksi) | ✓ |
| `Ctrl` + seret di area kosong | Kotak seleksi (rubber band) untuk banyak objek | ✓ |

Tambahan **khusus SokanLab** (didokumentasikan terbuka karena fitur TV-nya belum ada):

- `1`–`9` memilih alat. TV memakai angka untuk *timeframe* — SokanLab belum punya
  resampling timeframe di MVP, jadi angka dialihfungsikan (akan dikembalikan ke
  timeframe saat fitur itu ada).
- `F1` membuka dialog pintasan.

Batch 7 dan batch 8 **tidak menambah hotkey**: TV juga tidak memetakan tombol untuk
Disjoint/Flat Channel, template, maupun sepuluh alat batch 8 (Regression, Fib Trend
Extension, Elliott, XABCD/ABCD, Gann Fan, Brush, Highlighter, Sticker) — semuanya
dipilih dari strip alat. Digit `1`–`9` tetap menunjuk sembilan entri pertama
(`Select`…`Arrow`); alat baru masuk **setelah** entri ke-9 sehingga peta digit lama
tidak bergeser.

Tombol TV yang **sengaja tidak dipetakan** (fitur belum ada — jangan menelan tombol):
`Alt+A` (alert), `Alt+W` (watchlist), `Alt+I`/`Alt+L`/`Alt+P` (invert/log/persen
tersedia lewat tombol panel skala, bukan hotkey TV), `Alt+X`/`Alt+Y` (split/date range
di mode replay), `Ctrl+K` / `/` (quick search & indikator), `.` (layout dialog),
`Ctrl+Z`-family di luar undo/redo chart.

## 4. Gap yang masih tersisa (jujur)

**Belum ada di SokanLab (rencana batch berikutnya):**

1. **Alat lanjutan**: varian channel (batch 7) dan sepuluh alat batch 8 (Regression,
   Fib Trend Extension, Elliott, XABCD/ABCD, Gann Fan, Brush, Highlighter, Sticker)
   **sudah ada**; sisa: projection/forecast, Gann Box/Square & Gann angles, keluarga
   pola harmonic lain (cypher/shark/…), serta editor teks lebih dalam — ukuran (S/M/L)
   & perataan per objek ada di batch 8, sisanya pilihan font, pivot, dan multi-baris.
2. **Properti objek lanjutan**: panel format penuh ala TV (tab
   Style/Text/Coordinates/Visibility) masih belum; **multi-pilih objek + edit gaya
   kolektif sudah ada (batch 6)**; level fib kustom + warna per level dan opasitas
   ada di batch 5; **katalog template bernama sudah ada (batch 7)** — bedanya dengan TV:
   template SokanLab generik (gaya + opasitas, bisa ke jenis alat apa pun) dan tanpa
   dialog per-tab, sedangkan TV menyimpan template di dalam dialog alat dan menyediakan
   "apply to all <jenis>"; Extend masih **khusus channel** (TV memberi Extend juga untuk
   alat garis lain — SokanLab memakai alat Ray/Extended Line untuk kebutuhan itu);
   template **per jenis alat** (di dalam dialog alat) menyusul di batch 9.
3. **Object tree lanjutan**: **multi-seleksi + kotak seleksi sudah ada (batch 6)**;
   sisa: menu klik-kanan per baris, drag-urut z dari daftar, pencarian/filter objek
   (TV: "Object Tree" dengan hidden/locked group + rename inline).
4. **Skala & navigasi lanjutan**: auto-scale manual per pane (kunci skala), skala
   volume terpisah, drag *label sumbu* individual — batch 4: pane RSI/MACD sudah
   auto-scale (MACD simetris 0 di tengah) tapi belum bisa dikunci manual.
5. **Snapshot lanjutan**: opsi rentang & sembunyikan grid/panel (batch 4 sudah:
   resolusi 1×/2×/4×; batch 3: PNG area chart tanpa panel samping).
6. **Indikator lanjutan**: pengaturan warna per indikator & library indikator
   (batch 4 sudah: pane RSI/MACD + periode; overlay SMA/EMA/VWAP).
7. **Di luar scope MVP** (dicatat agar ekspektasi jelas): trading/broker, alert,
   replay bar, multi-chart (dua simbol dalam satu layout), watchlist hotkey,
   Pine Script, layout dialog multi-layout.

**Perkiraan kedekatan (diperbarui setelah batch 8):** untuk alur kerja gambar inti
yang dipakai sehari-hari (trend/garis/fib/channel lengkap dengan variannya/ukur/posisi
+ magnet + properti lanjutan + template bernama + object tree + seleksi banyak +
grup/z + skala/sumbu + pane indikator + hotkey + klon `Ctrl+seret`), SokanLab kini di
kisaran **~95% perilaku TV**; untuk seluruh katalog alat & fitur TV (~60+ alat — banyak
di antaranya varian dari keluarga fib/Gann/pola — + dialog format penuh), sekitar
**~50%** (33 alat gambar tersimpan). Hotkey yang berlaku untuk fitur kita sudah
**~95%** sama dengan TV (batch 6–8 tidak menambah hotkey: TV pun tidak memberi tombol
untuk multi-seleksi, Disjoint/Flat Channel, template, maupun sepuluh alat batch 8).

## 5. "Smoth" (kinerja) — angka terukur

Bukan klaim: diukur di lingkungan ini (Xvfb, software GL, CPU renderer).

| Metrik | Angka |
|---|---|
| GUI 1M bar, frame pertama | 9 ms |
| GUI 1M bar, rata-rata (tanpa overlay) | 5,0 ms/frame (p50 4,9 / p95 5,4) — batch 3; 4,7 ms (p50 4,6 / p95 5,2) pada run batch 4 |
| GUI 1M bar + SMA/EMA/VWAP overlay | 5,2 ms/frame (p50 5,0 / p95 6,1) — batch 3; 5,1 ms (p50 4,9 / p95 6,0) — batch 4; frame pertama 13 ms (hitung seri 1M bar saat muat) |
| GUI 1M bar + overlay + pane RSI 14 & MACD 12/26/9 | **5,2 ms/frame** (p50 5,1 / p95 5,8) — pane menambah O(bar terlihat) per pane saat menggambar (auto-scale MACD dihitung dari bar terlihat, ikut decimation) |
| GUI 1M bar + overlay + pane RSI 14 & MACD 12/26/9 (batch 5) | **5,2 ms/frame** (p50 5,0 / p95 5,8) — setara batch 4; opasitas/level fib/varian pitchfork tidak menambah biaya terukur |
| GUI 1M bar + overlay + pane RSI 14 & MACD 12/26/9 (batch 6) | **5,4–5,9 ms/frame** dari 3 run (p50 5,2–5,4) — di dalam variasi antar-run mesin ini (±10%); jalur gambar kini memanggil `selection()` (1 alokasi kecil/frame). Kotak seleksi hanya digambar saat gestur aktif. |
| GUI 1M bar + overlay + pane RSI 14 & MACD 12/26/9 (batch 7) | **6,1 / 5,4 / 5,5 ms/frame** dari 3 run (p50 5,2 ketiganya) — masih di dalam variasi batch 6; varian channel tidak menambah cabang per frame (2 segmen per channel, O(1)). |
| GUI 1M bar dengan 3 channel di layar (batch 7, `bench-channels7.json`) | **5,2 / 5,3 ms/frame** (p50 5,0) dari 2 run — praktis sama dengan tanpa channel; satu channel dengan `extend:both` hanya 2 garis yang di-clip ke plot. |
| GUI 1M bar + overlay + pane RSI 14 + MACD 12/26/9 (batch 8) | **5,5 / 5,4 / 5,5 ms/frame** dari 3 run (p50 5,1–5,2) — setara batch 7; jalur gambar batch 8 tidak menambah biaya pada scene biasa. |
| GUI 1M bar dengan 3 channel di layar, diulang di batch 8 (`bench-channels7.json`) | **5,3 ms/frame** (p50 4,9–5,1) dari 3 run — tidak ada regresi. |
| GUI 1M bar, scene batch 8 berat (`bench-batch8.json`: 3 regression channel rentang ~600k/750k/380k bar + 2 Gann fan + Elliott + XABCD + ABCD + brush 400 titik + highlighter + 4 sticker) | **14,0 / 14,6 / 14,5 ms/frame** (p50 13,6–14,1 / p95 19,4–20,2) dari 3 run — biaya murni `regression_fit` yang O(rentang): **1,85 ms per 600k bar** (micro-bench release) ×3 objek; atribusi terukur: scene tanpa regression (Gann/pola/goresan/sticker) = 4,9–5,1 ms (setara baseline), 3 regression berentang 100 bar = 5,0 ms, 1 regression 600k = 8,8–9,0 ms. |
| Perbaikan batch 8: hit-test hover di-cache + `pick` dipakai bersama kursor/render | scene batch 8 berat **36–38 ms → 13,6–14,1 ms** (p50); pemanggilan `regression_fit` per frame turun dari ~1,9× menjadi **1,0×** per objek (diukur lewat instrumentasi sementara lalu dilepas). |
| Render headless 1M bar | 0,15 s |
| Interaksi baru (magnet/drag/toolbar/properti objek) | tidak menambah biaya terukur vs bench sebelumnya (4,9 ms/frame) |

Yang membuat terasa "berat" di TV dan kita hindari: gambar setiap bar tiap frame
(decimation `draw_step`), dan pemetaan viewport yang tidak konsisten saat resize
(sudah diperbaiki Fase 5).

## 6. Rencana batch

- **Batch 1 (selesai, 2026-10-18)**: 14 alat + interaksi penuh + magnet + hotkey TV-inti
  + toolbar + menu klik kanan + daftar objek + persistensi layout v1 diperluas.
- **Batch 2 (selesai, 2026-10-18)**: properti objek (8 warna + tebal 1–4 + default
  per alat), object tree penuh (tampil/kunci/nama + panel properti), channel &
  pitchfork, fib extension/time, dobel-klik reset sumbu, Alt+G "go to date".
- **Batch 3 (selesai, 2026-10-18)**: skala log/persen/invert + drag sumbu, snapshot
  GUI (Alt+S), gaya garis putus-putus/titik, teks/callout, indikator overlay
  (SMA/EMA/VWAP jangkar), z-order/duplikat/grup.
- **Batch 4 (selesai, 2026-10-18)**: pane indikator RSI/MACD + pengaturan periode,
  strip sumbu harga/waktu berlabel + tag crosshair, objek di-clip ke pane harga,
  snapshot resolusi 1×/2×/4×, **perbaikan bug EMA** (bobot α), layout `schema:4`.
- **Batch 5 (selesai, 2026-10-18)**: opasitas per objek 0–100%, level fib kustom +
  warna per level, varian pitchfork (Schiff/Modified Schiff/Inside) dengan geometri
  bersama painter/hit-test, template gaya ("Jadikan default" + "Terapkan ke sejenis"),
  layout `schema:5` (reader v1–v5).
- **Batch 6 (selesai, 2026-10-18)**: **multi-seleksi** (Ctrl+klik, ⌘ di macOS, dan
  kotak seleksi Ctrl+seret) + operasi kolektif: geser bersama, hapus satu langkah
  undo, z-order tetap urut relatif, duplikat sekaligus, grup satu id, edit
  warna/tebal/pola/opasitas ke semua terpilih; daftar objek multi-pilih; **tidak ada
  hotkey baru** (konvensi mouse TV terdokumentasi di dialog F1); `schema:5` tidak
  berubah.
- **Batch 7 (selesai, 2026-10-19)**: **varian channel** — `ToolKind::DisjointChannel`
  & `ToolKind::FlatChannel` (geometri bersama painter/hit-test lewat
  `channel_segments`; label objek per varian; JSON `"variant"` hanya bila bukan
  paralel), **opsi Extend per objek channel** (tidak/kiri/kanan/keduanya; dua centang
  di panel; default tidak diperpanjang seperti TV — garis channel kini **segmen** yang
  benar-benar digambar, bukan lagi garis tak hingga), **katalog template gaya
  bernama** (Simpan dari objek primer / Terapkan ke seluruh seleksi / Hapus; nama
  maks 32 karakter, 16 entri, nama sama diperbarui) dengan persistensi
  `"templates"` di layout **`schema:6`** (reader v1–v6); **tidak ada hotkey baru**.
- **Batch 8 (selesai, 2026-10-19)**: **10 alat baru** — Regression Channel, Fib Trend
  Extension, Elliott Impulse, Elliott Correction, XABCD, ABCD, Gann Fan, Brush,
  Highlighter, Sticker (14 ikon vektor orisinal) — plus **UI/UX**: ukuran (S/M/L) &
  perataan (kiri/tengah/kanan) teks per objek, **Ctrl+seret badan objek = klon lalu
  geser**, strip alat ber-area-gulir, centang perluasan-kiri fib trend, kombo ikon
  sticker; layout **`schema:7`** (reader v1–v7). Bonus kinerja: cache hover-pick +
  `pick` sekali per frame menggantikan dua kali (lihat §5/§14).
- **Batch 9 (rencana)**: Gann Box/Square & Gann angles, **Projection/Forecast** (3
  titik), pola harmonic lain, template **per jenis alat** (dialog ala TV), editor teks
  lebih dalam (font/pivot/multi-baris), **prefix-sum** untuk regression channel agar
  O(1) per frame pada rentang raksasa, varian pitchfork sebagai entri alat terpisah,
  dan pengembalian digit `1`–`9` ke pemilih timeframe saat resampling tersedia.

## 7. Verifikasi batch 1 (bukti)

- `cargo test --workspace`: **150 lolos, 0 gagal** (sebelumnya 116) — termasuk tes
  interaksi *event-driven* tanpa display: klik-dua-titik membuat trendline, seret
  sekali membuat rectangle, klik menyeleksi + drag badan memindahkan objek, Esc
  membatalkan, hotkey Alt+T/Ctrl+Z mengubah state.
- `cargo clippy --workspace --all-targets -- -D warnings`: bersih.
- `scripts/check-offline.sh`: LULUS (0 pola jaringan; impor→backtest→render tanpa jaringan).
- E2E layout: file dengan 14 jenis alat + setelan `tools`/`hidden` direstore lewat CLI,
  disimpan ulang → **identik byte-for-byte** (`cmp`).
- Render headless: `render-ok triangles=15812` (1200×800, 500 bar) — PNG memuat
  trendline, fib, position, range.
- Bench GUI 1M bar (Xvfb): angka pada §5.

## 8. Verifikasi batch 2 (bukti)

- `cargo test --workspace`: **163 lolos, 0 gagal** (sebelumnya 150) — termasuk tes
  baru: geometri channel/pitchfork/fib extension/fib time (hit-test), `fib_ext_price`
  dengan `i64::MIN`/`i64::MAX` (dijepit, tanpa overflow), urutan bar fib time,
  pick melewati objek tersembunyi/terkunci, `set` mempertahankan metadata, parser
  tanggal (epoch detik/milidetik, `YYYY-MM-DD[ HH:MM[:SS]]`), render panel objek +
  strip alat headless, dan **dobel-klik sumbu harga mengembalikan skala** lewat event
  sintetis.
- `cargo fmt --all -- --check` + `cargo clippy --workspace --all-targets -- -D warnings`:
  bersih.
- `scripts/check-offline.sh`: LULUS.
- E2E layout v2: berkas dengan **18 jenis alat** + metadata (gaya, nama ber-escape,
  kunci, sembunyi) + `tools`/`hidden` direstore lewat CLI lalu disimpan ulang —
  **identik byte-for-byte**; berkas `schema:1` lama tetap termuat dan dinaikkan ke
  `schema:2` tanpa error.
- Render headless `docs/shot.png`: PNG memuat channel bergaya kustom (biru, tebal 2),
  pitchfork, fib extension (rail + label), fib time (garis vertikal berlabel).
- Bench GUI 1M bar (Xvfb) batch 2: avg 4,9 ms (p50 4,8 / p95 5,4) — tanpa regresi.
- Paket ulang: `dist/sokanlab-0.1.0-linux-x86_64.tar.gz`
  sha256 `d5b5cc006dac57c4ac8ecb1947d7fb96dd5eebbc7ec1aa7323cea809255092af`.

## 9. Verifikasi batch 3 (bukti)

- `cargo test --workspace`: **174 lolos, 0 gagal** (sebelumnya 163) — termasuk tes
  baru: skala log roundtrip & persen/invert (`y_for_price`/`price_at_y` konsisten),
  `MoveBody` bergrup menggeser dua objek sekaligus (event sintetis), drag sumbu harga
  (bawah = zoom out) & sumbu waktu (kanan = zoom in) lewat event sintetis, snapshot
  `render_chart_png` menulis PNG valid (header + >1000 byte), layout `schema:3`
  roundtrip byte-identik (skala log + invert + overlay + dash + grup + teks
  ber-escape), skala tak dikenal ditolak tanpa mengubah state, basis skala persen
  dari bar terlihat, duplikat/z/grup lewat `ChartState`, dan JSON teks/callout.
- `cargo fmt --all -- --check` + `cargo clippy --workspace --all-targets -- -D warnings`:
  bersih.
- `scripts/check-offline.sh`: LULUS.
- E2E layout v3 (CLI, dataset 1.000.000 bar): berkas dengan 21 objek (termasuk
  teks/callout, dash, grup) + `scale:log` + `invert` + overlay direstore lalu disimpan
  dua kali — **identik byte-for-byte** (`cmp`). Berkas `schema:1` dan `schema:2` lama
  tetap termuat dan dinaikkan ke `schema:3`; skala tak dikenal ditolak
  ("layout tidak dipulihkan: skala tidak dikenal: kubik") tanpa mengubah state.
- Render headless `docs/shot.png`: **triangles=9594** (120 bar) dan
  `docs/shot-wide.png` **triangles=61136** (100.000 bar) — PNG memuat SMA/EMA/VWAP
  overlay, channel putus-putus, kotak teks & callout ekor.
- Bench GUI 1M bar (Xvfb): tanpa overlay **5,0 ms** (p50 4,9 / p95 5,4); dengan
  overlay **5,2 ms** (p50 5,0 / p95 6,1) — overlay tidak menambah biaya per frame
  berarti (dihitung saat muat/perubahan setelan, digambar hanya bar terlihat).
- Paket ulang: `dist/sokanlab-0.1.0-linux-x86_64.tar.gz`
  sha256 `0fd385db3b5637c297226eba6666b44b05a69430b5c6554856b4c7229840a832`.

## 10. Verifikasi batch 4 (bukti)

- `cargo test --workspace`: **180 lolos, 0 gagal** (sebelumnya 174) — termasuk tes baru:
  tata letak `split_plot_panes` (pane tersusun, strip sumbu terpisah, batas 3 pane),
  format waktu UTC (`fmt_time`, termasuk pra-epoch), `macd_series` acuan eksak (ramp
  fast 3/slow 6 → MACD 1500 milli, sinyal konvergen, ramp turun −1500), **EMA standar**
  (`ema_series([10,12,14], 2) = [None, 11, 13]` — bug bobot α terbalik diperbaiki),
  pane RSI/MACD lewat `ChartState` (maks 3 pane, histogram = MACD − sinyal, RSI 0–100),
  layout v4 roundtrip (pane + overlay + dash + grup) dan entri pane tak dikenal
  dilewati, snapshot 2× menghasilkan PNG 2× piksel (IHDR 1280×800), serta event
  sintetis: klik di pane indikator bukan drag skala harga dan strip kanan tetap
  memicu `ScalePrice`.
- `cargo fmt --all -- --check` + `cargo clippy --workspace --all-targets -- -D warnings`:
  bersih.
- `scripts/check-offline.sh`: LULUS.
- E2E layout v4 (CLI, dataset 1.000.000 bar): berkas dengan pane RSI 14 + MACD
  12/26/9 dan overlay direstore → disimpan → dimuat ulang: **identik byte-for-byte**
  (`cmp`, 367 B). Berkas `schema:3` (21 objek) dinaikkan ke `schema:4` — **hanya field
  `schema` yang berubah** (viewport/overlay/watchlist/drawing sama); entri pane tak
  dikenal dilewati (hanya MACD yang tersisa) dan `schema:5` ditolak
  ("layout tidak dipulihkan: schema layout 5 tidak dikenal") tanpa mengubah state.
- Render headless `docs/shot.png`: **triangles=19218** (120 bar, 1200×800) dan
  `docs/shot-wide.png` **triangles=151824** (100.000 bar, 1600×900) — PNG memuat pane
  RSI (panduan 30/50/70), pane MACD (garis + histogram), strip sumbu harga kanan &
  sumbu waktu bawah, dan objek gambar yang kini ter-clip di pane harga (sebelumnya
  menembus pane volume).
- Dua bug nyata ditemukan lewat verifikasi render ini dan diperbaiki: (a) **EMA**
  (bobot α ke nilai lama — overlay EMA & dasar MACD salah), (b) **rasterizer headless
  mengabaikan `clip_rect`** sehingga objek gambar menembus pane di PNG (backend GPU
  GUI tidak terpengaruh); raster kini memakai clip rect seperti egui.
- Bench GUI 1M bar (Xvfb): tanpa overlay **4,7 ms** (p50 4,6 / p95 5,2); + overlay
  SMA/EMA/VWAP **5,1 ms** (p50 4,9 / p95 6,0); + overlay + pane RSI/MACD **5,2 ms**
  (p50 5,1 / p95 5,8).
- Paket ulang: `dist/sokanlab-0.1.0-linux-x86_64.tar.gz` sha256 `7733b5cbfa0952be4aea0804d6a2a665a5d12a1b63dce7ef86eb06c0e1af88b0`.

## 11. Verifikasi batch 5 (bukti)

- `cargo test --workspace`: **190 lolos, 0 gagal** (sebelumnya 180). Tes baru:
  - `chart`: geometri eksak 4 varian `pitchfork_lines` (Andrew/Schiff/Modified
    Schiff/Inside — origin, arah median, titik tine, segmen bantu, target median di
    t = 1), kestabilan kunci JSON varian, `hit_test_item` yang membedakan varian
    (titik yang kena median Inside tidak kena median Andrew, dan sebaliknya) serta
    level fib kustom (level yang dihapus tidak bisa di-hit, segmen a–b tetap aktif),
    dan penjepitan `opacity_factor` (0 → 0,0; 40 → 0,4; 100 → 1,0; 200 → 1,0).
  - `ui`: `fib_level_kustom_dan_opasitas` (7 garis horizontal bawaan → 2 garis untuk
    2 level kustom; alpha semua segmen ≈ 102 saat opasitas 40%; warna level kustom
    terpakai apa adanya) dan `varian_pitchfork_mengubah_geometri` (jarak titik ke
    garis median < 1 px untuk target yang benar — titik tengah B–C / C / origin
    Schiff — dan garis bantu berpindah b–c → a–b); roundtrip `schema:5` metadata objek
    + `default_style` (identik byte-for-byte), berkas v4/lama tanpa metadata baru
    tetap termuat dengan default, nilai liar dijepit & kunci pitchfork tak dikenal
    diabaikan, pewarisan gaya default ke objek baru, "terapkan ke sejenis" menghitung
    2 trendline dan tidak menyentuh rectangle, tanpa seleksi = 0 perubahan.
  - `app`: panel properti batch 5 merender (slider opasitas, combo varian, editor 3
    level fib) untuk pitchfork & fib, jalur "Jadikan default" (objek baru mewarisi),
    "Terapkan ke sejenis", dan siklus warna level fib auto → palet → auto.
- `cargo fmt --all -- --check` + `cargo clippy --workspace --all-targets -- -D warnings`:
  bersih. `scripts/check-offline.sh`: **LULUS** (tanpa dependensi/jejak jaringan baru).
- E2E layout (CLI, dataset 1M bar):
  - Berkas uji berisi pitchfork `"pitchfork":"schiff","opacity":40`, fib dengan
    `"fib":[{"r":500,"c":"#f2b04d"},{"r":425}]`, trendline `"opacity":250` (liar) dan
    pitchfork `"pitchfork":"noise"` → muat + simpan ulang: metadata sah tersimpan,
    opacity 250 **dijepit** (field hilang = 100), kunci `noise` **diabaikan** (Andrew),
    dan berkas keluaran v5 di-roundtrip lagi → **identik byte-for-byte** (`cmp`, 963 B).
  - Berkas v4 (21 objek, tanpa metadata baru) → v5: **hanya nomor schema yang berubah**
    (baris JSON tetap 1798 byte; berkas keluaran 1799 byte termasuk newline).
  - `schema:6` ditolak ("schema layout 6 tidak dikenal") **tanpa mengubah state**
    (state default dataset tetap tersimpan, exit 0).
- Render headless (regenerasi): `docs/shot.png` **19210** segitiga (120 bar,
  1200×800; fib 5 level kustom + pitchfork Schiff 70%), `docs/shot-wide.png`
  **151816** segitiga (100.000 bar, 1600×900). Delta vs batch 4 diverifikasi lewat dua
  build eksperimen: level fib 7→5 = **−48** segitiga (2 garis + 2 label; 1 glyph =
  1 quad), pitchfork tambahan = **+40** segitiga (4 garis; tessellator memecah segmen
  panjang) → 19218 − 48 + 40 = 19210 ✓.
- Bench GUI 1M bar (Xvfb, `bench-panes.json`): **5,2 ms/frame** (p50 5,0 / p95 5,8),
  frame pertama 11 ms — setara batch 4 (tanpa regresi).
- Paket ulang: `dist/sokanlab-0.1.0-linux-x86_64.tar.gz` sha256 `36dc2ed9bd97df42c0fa38139416d307a2f8c42251a3349aef87fa34d42043ff`
  (`sha256sum -c` OK).

## 12. Verifikasi batch 6 (bukti)

- Konvensi yang ditiru **diverifikasi dari dokumentasi publik TV**, bukan menebak:
  halaman Shortcuts TradingView memuat baris "Select multiple drawings — Hold Ctrl +
  Click" (di macOS ⌘), dan panduan Drawing memuat "hold CTRL and drag → box that you
  can drag over multiple drawings". Yang ditiru hanya konvensi mouse; tidak ada kode,
  aset, atau brand TV yang dipakai.
- `cargo test --workspace`: **204 lolos, 0 gagal** (sebelumnya 190). Tes baru:
  - `chart`: `rubber_select` (anchor di dalam kotak, kotak yang hanya **memotong
    segmen** tanpa memuat anchor, kotak jauh, garis horizontal dinilai dari posisi y,
    garis vertikal dari posisi x, crossline satu-anchor, objek **terkunci &
    tersembunyi dilewati**); `move_items_z` (urutan relatif 2 objek terjaga, indeks
    baru `[2,3]`, tidak bisa maju saat anggota teratas mentok **tanpa** membuang
    snapshot undo, mundur kembali `[1,2]`); `remove_many`/`duplicate_many` (satu
    langkah undo untuk banyak objek, hapus kosong tidak membuat snapshot);
    `assign_group_many`/`clear_group_many` (satu id grup untuk dua objek, lepas
    serentak).
  - `ui`: toggle seleksi & perpindahan primer (`[1,2,3] → lepas primer → primer
    `2` → lepas non-primer → `[2]` → seleksi kosong`), `set_selection` membuang
    duplikat/indeks tak valid dan memilih objek teratas sebagai primer, hapus banyak
    = satu `Command::Undo` mengembalikan keduanya lalu `RemoveSelected` kedua kali =
    `Ignored`, z/duplikat/grup pada seleksi banyak (seleksi mengikuti indeks baru
    `[1,2]`, duplikat `[3,4]`), `sync_style_from_primary` (primer default → anggota
    lain kembali default; primer bergaya → anggota ikut; seleksi tunggal = 0),
    `selection_after_removal` (indeks terpilih bergeser turun setelah hapus per
    baris).
  - `app` (event egui nyata, bukan mock): **Ctrl+klik memilih 2 objek → drag badan
    tanpa Ctrl tetap mempertahankan seleksi dan menggeser keduanya dengan satu
    langkah undo → Ctrl+klik anggota non-primer melepasnya**; **Ctrl+seret kotak di
    area kosong memilih 2 objek → kotak kecil di area kosong membersihkan seleksi →
    Ctrl+klik di area kosong tidak menyeleksi apa pun**; hapus kolektif lewat
    `Command::RemoveSelected` + undo + z-order + perbaikan indeks. Simulasi Ctrl
    memakai `Event::ModifiersChanged` (egui 0.37 tidak lagi punya `RawInput.modifiers`).
  - `ui::painter`: dua objek terpilih ditandai ember + satu objek tak terpilih tetap
    slate; kotak seleksi menggambar isi tembus pandang (`Shape::Rect`) + tepi
    putus-putus (≥3 segmen); kotak nol (klik tanpa seret) tidak menggambar isi/tepi.
- `cargo fmt --check` bersih dan `cargo clippy --workspace --all-targets -- -D
  warnings` bersih (0 peringatan).
- `scripts/check-offline.sh`: **LULUS** — 0 klien jaringan di seluruh kode
  (`grep` socket/TCP/HTTP), smoke impor → backtest → render tanpa jaringan.
- E2E CLI (CLI `--layout in5.json --save-layout out6.json`, dataset 1.000.000 bar):
  hasil **identik byte-for-byte** dengan keluaran batch 5 (`cmp out5.json out6.json`,
  963 B) — sesuai desain: seleksi tidak dipersistensi dan `schema` tetap **5**; berkas
  v1–v4 tetap termuat.
- Render headless demo kini memuat multi-seleksi + kotak seleksi (lihat gambar di
  `docs/shot.png`): `triangles=19990` (120 bar, 1200×800) dan `triangles=151890`
  (100.000 bar, 1600×900). Delta vs batch 5 (+780 / +74) **diatribusi lewat dua build
  eksperimen** yang hanya menghapus pemanggilan `draw_rubber_band`: hasilnya tepat
  `19210` dan `151816` (angka batch 5), jadi seluruh delta murni dari kotak seleksi —
  bukan dari perubahan gaya objek terpilih (warna tidak menambah segitiga).
- Bench GUI 1M bar (Xvfb, `bench-panes.json`, `--bench-frames 40` × 3 run):
  **5,4 / 5,9 / 5,6 ms** (p50 5,2 / 5,4 / 5,2), frame pertama 11 / 14 / 13 ms.
  Batch 5 mengukur 5,2 ms (p50 5,0) — selisih berada di dalam variasi antar-run
  mesin ini; tidak ada biaya per-frame dari kotak seleksi (hanya digambar saat
  gestur aktif). Rincian: `docs/BENCH.md` baseline batch 6.
- Batas verifikasi yang sama seperti batch sebelumnya: GUI nyata tidak bisa
  di-screenshot di lingkungan ini (tidak ada `xwd`/`import`/`scrot`, tidak ada
  `xdotool` untuk menyuntik input ke window nyata), jadi bukti interaksi = tes egui
  in-process (menjalankan `chart_panel` sungguhan) + PNG headless. Platform
  Windows/macOS belum diuji di sini.
- Paket ulang: `dist/sokanlab-0.1.0-linux-x86_64.tar.gz` sha256 `9356c41ddf3c35a2d0e1cccf8e32626fcb6a1eb4f12bb04951ccfdd9035b6c47`
  (`sha256sum -c` OK).

## 13. Verifikasi batch 7 (bukti)

- Konvensi yang ditiru **diverifikasi dari dokumentasi publik TV** (perilaku, bukan
  kode/aset): halaman *Parallel channel drawing tool* ("has an option of being
  extended either to the right, left or both directions, if the user wants the channel
  to be continued indefinitely"; level 1 diatur "in relation to level 0 using price"),
  *Disjoint channel drawing tool* ("two parallel trendlines that do not necessarily
  share the same starting points on the time axis … click a third time to place the
  parallel boundary at the desired distance"; Extend "to the left, right, or both
  sides, or not extend them at all") dan *Flat top/bottom drawing tool* ("completely
  flat at the top or bottom … click a third time to place the flat, horizontal
  boundary at the desired price level"; "horizontal boundary with an angled trendline
  on the opposite side").
- **Perubahan perilaku yang disengaja** (dan tercatat): garis channel kini digambar
  sebagai **segmen yang benar-benar terlihat** (dengan opsi Extend), bukan lagi garis
  tak hingga. Default "tidak diperpanjang" mengikuti TV; hit-test memakai segmen yang
  sama (`channel_segments` dipakai bersama painter & hit-test, `EXTEND_PX = 20.000 px`
  setara "diperpanjang penuh sampai tepi plot"). Tes lama yang mengandalkan garis tak
  hingga diperbarui — dan ditambah kasus baru yang menegaskan klik di luar bentuk
  **tidak** lagi memilih channel.
- `cargo test --workspace`: **210 lolos, 0 gagal** (sebelumnya 204). Tes baru:
  - `chart`: geometri eksak `channel_segments` untuk tiga varian (paralel: batas
    kedua sejajar dengan rentang x batas pertama; disjoint: batas kedua = salinan
    digeser oleh `c−a` → mulai di x lain; flat: horizontal di `c.y` sepanjang rentang
    batas pertama), **perpanjangan mengikuti sumbu waktu** (kiri/kanan/keduanya,
    termasuk batas kedua ikut diperpanjang dan kasus dasar vertikal yang tidak bisa
    diperpanjang), `ChannelKind` label/`degree`/`translated` per varian, hit-test objek
    dengan `extend` (klik di luar bentuk kena bila diperpanjang), `StyleTemplate`
    (potong nama 32 karakter, katalog penuh, perbarui di tempat, hapus), serta
    regresi hit-test tanpa perpanjangan.
  - `ui`: `save_template`/`apply_template`/`remove_template` (trim & potong nama,
    tolak kosong/tanpa objek, katalog 16 entri, terapkan ke **seluruh** seleksi,
    perbarui vs duplikat, pilihan panel ikut dibersihkan saat dihapus), roundtrip
    `schema:6` (varian + extend + template) **byte-identik**, migrasi berkas v5 → v6
    (**hanya nomor schema** yang berubah), nilai `"variant"`/`"extend"` asing
    diabaikan (bukan error), template rusak/nama kosong/ganda dilewati & katalog
    dipotong ke 16, dan mapping alat baru (`DisjointChannel`/`FlatChannel` →
    variannya) lewat `ToolKind::build`.
  - `app`: panel properti batch 7 merender dengan tiga varian channel + `extend`
    (kombo varian + dua centang + seksi "Template gaya" dengan kombo & tombol), dan
    `kind_label()` objek menyesuaikan variannya.
- `cargo fmt --check` bersih; `cargo clippy --workspace --all-targets -- -D warnings`
  bersih (0 peringatan); `scripts/check-offline.sh` **LULUS** (0 pola jaringan,
  0 klien jaringan di graph build, smoke impor→backtest→render).
- E2E layout lewat CLI (dataset `SYNTHUSDT-1m`, 1.000.000 bar):
  - Berkas v5 lama (4 objek: pitchfork Schiff, fib 2 level kustom, trend, pitchfork)
    → disimpan ulang: **hanya `"schema":5` → `"schema":6`** yang berubah
    (dibuktikan `sed 's/"schema":6/"schema":5/' out7.json | diff - out5.json` bersih).
  - Berkas v6 baru (3 channel: `"variant":"disjoint","extend":"right"`, `"variant":"flat"`,
    `"extend":"both"` + 3 template) → muat + simpan ulang, lalu muat + simpan lagi:
    **identik byte-for-byte** (`cmp`, 912 B) — idempoten, varian/extend/template
    bertahan penuh di jalur biner release.
- **Verifikasi visual geometri** (5 PNG fokus dari state nyata via
  `render_chart_png`, viewport di-zoom ke tiap channel, warna tegas): paralel =
  dua garis sejajar rentang x sama; disjoint = batas kedua salinan tergeser (mulai di
  x berbeda); flat = garis dasar miring + batas horizontal sepanjang rentang yang
  sama; `Extend` kiri/kanan = perpanjangan hanya ke arah tersebut sampai tepi plot.
  (Render ini eksperimen sesi, tidak disimpan di repo; bukti permanen = tes unit
  geometri + `docs/shot.png`.)
- Render headless (regenerasi): `docs/shot.png` **18510** segitiga (120 bar,
  1200×800) dan `docs/shot-wide.png` **148850** (100.000 bar, 1600×900). Atribusi
  delta lewat build eksperimen yang hanya **menghapus dua channel demo baru**:
  hasilnya **18470 / 148810** — jadi channel kini dibatasi (bukan garis tak hingga)
  mengubah −1520 / −3080, dan dua channel demo baru menambah +40 / +40
  (2 garis per channel). Demo headless memuat Disjoint Channel (`extend` kanan) dan
  Flat Top/Bottom Channel sebagai bukti visual permanen di `docs/shot.png`.
- Bench GUI 1M bar (Xvfb, `bench-panes.json`, `--bench-frames 40` × 3 run):
  **6,1 / 5,4 / 5,5 ms** (p50 5,2 ketiganya; frame pertama 36 / 13 / 15 ms) — batch 6
  mengukur 5,4–5,9 ms, jadi masih di dalam variasi antar-run. Dengan **3 channel di
  layar** (layout v6 `bench-channels7.json`, salah satunya `extend:both`):
  **5,2 / 5,3 ms** (p50 5,0) — varian channel tidak menambah biaya terukur
  (2 segmen O(1) per channel). Rincian: `docs/BENCH.md` baseline batch 7.
- Batas verifikasi yang sama seperti batch sebelumnya: interaksi GUI nyata tidak bisa
  di-screenshot/di-inject di lingkungan ini (tanpa `xwd`/`import`/`scrot`/`xdotool`),
  jadi bukti interaksi = tes egui in-process + PNG headless; platform Windows/macOS
  belum diuji di sini. Verifikasi visual varian memakai render fokus (temporary),
  bukan tangkapan window GUI.
- Paket ulang: `dist/sokanlab-0.1.0-linux-x86_64.tar.gz` sha256
  `23847e1f85b7a5fd6e9be0938766e14b0788bfa0895233be94c106d88346c066` (`sha256sum -c` OK; biner di dalam tar
  identik dengan `target/release` — sha256 `92396623…`).

## 14. Verifikasi batch 8 (bukti)

- Konvensi diverifikasi dari **dokumentasi publik TV** (perilaku saja, bukan kode/aset):
  *Trend-based Fib Extension* ("levels are drawn to the right of the third point"),
  *Gann Fan* (9 rasio dengan catatan "the angle depends on the chart's aspect ratio";
  garis 1×1 mengikuti kemiringan dasar), *Elliott Wave Theory and tools* (impulse 1–5,
  correction A–B–C, pola XABCD/ABCD), serta *Linear Regression Channel* (garis regresi
  kuadrat-terkecil dengan batas simpangan baku).
- `cargo test --workspace`: **231 lolos, 0 gagal** (naik dari 210; chart 60, ui 84,
  desktop 26, sisanya domain/engine/import/store/fuzz). Tes baru batch 8 (+21):
  - `chart` (+11): `regression_fit` eksak (kemiringan/σ pada data linear murni, rentang
    terbalik, < 2 bar, pemangkasan indeks di ujung), `regression_segments` (3 segmen,
    ±σ dibulatkan, Extend memakai `extend_pair`), level `fib_trend` (proyeksi a→b
    ber-origin c, sama dengan fib extension), sembilan garis `gann_fan_lines` (1×1
    tepat a→b, rasio lain menskalakan komponen harga, label unik), pola (5/3/5/4 titik
    efektif, label titik), goresan (`MAX_STROKE_POINTS` dipotong, titik berdekatan
    dibuang), `StickerIcon` (14 ikon, fallback `ArrowUp`, ukuran ikon tetap), hit-test
    jenis baru, serta `TextSize`/`TextAlign`.
  - `ui` (+6): dua tes `tools` (goresan dibangun dari seret; sticker sekali klik),
    satu tes painter (6 level fib trend, 9 garis Gann, 3 segmen regresi, polyline pola,
    sticker, dan 5 label XABCD benar-benar digambar), roundtrip `schema:7`
    byte-identik (10 kind baru + `font`/`align`), reader menolak data rusak & skema
    di luar 1–7, dan cache hover-pick (invalidasi saat objek dihapus/undo/viewport/
    dataset berubah).
  - `desktop` (+4): seret membuat goresan brush, `Ctrl+seret` pada objek mengklon lalu
    menggeser klon (objek terkunci dilewati), `Ctrl+klik` tanpa seret tetap toggle
    seleksi, panel properti batch 8 (Extend regression, centang perluasan fib trend,
    kombo ikon sticker, kombo ukuran & perataan teks).
- `cargo fmt --check` bersih; `cargo clippy --workspace --all-targets -- -D warnings`
  bersih (0 peringatan); `scripts/check-offline.sh` **LULUS** (0 pola jaringan, 0 klien
  jaringan di graph build, smoke impor → backtest → render).
- E2E layout lewat CLI biner release (dataset 1 jt bar):
  - Berkas v6 batch 7 → disimpan ulang: **hanya `"schema":6` → `"schema":7`** yang
    berubah (`sed 's/"schema":7/"schema":6/' … | diff -` bersih).
  - Berkas v7 baru (13 objek: 10 kind baru, satu `sticker` dengan `icon` dan satu tanpa
    ikon, `text`/`callout` ber-`font`/`align`, `extend` kanan/kiri, gaya eksplisit) →
    muat → simpan → muat → simpan: **identik byte-for-byte** (`cmp`) dan idempoten;
    nilai bawaan tidak ditulis ulang (`icon` hilang → `arrow_up`, `font`/`align` hanya
    muncul bila ≠ bawaan).
  - Berkas v1 dan v3 lama tetap termuat dan naik otomatis ke v7.
- Render headless (bukti visual): `docs/shot-batch8.png` (400 bar, 1200×700,
  `--demo batch8`) memuat semua bentuk baru — 6 level fib trend dari titik ketiga,
  9 garis Gann, 3 segmen regresi, polyline + label Elliott/XABCD/ABCD, brush +
  highlighter, dan **baris 14 ikon sticker** untuk membuktikan setiap ikon digambar;
  `docs/shot.png` (120 bar) dan `docs/shot-wide.png` (100.000 bar) memuat sisipan
  batch 8 (regression, sticker, goresan). Determinisme: build ulang dari sumber yang
  sama menghasilkan PNG **identik byte-for-byte** dengan kedua berkas itu.
  Atribusi segitiga: build eksperimen yang hanya mematikan sisipan demo batch 8
  mengembalikan **18510 / 148850** — tepat angka batch 7 — sehingga sisipan batch 8
  menyumbang **+73 / +73** (18583 / 148923).
- Bench GUI 1M bar (Xvfb `:99`, llvmpipe, `--bench-frames 40` × 3 run): baseline batch 8
  **5,5 / 5,4 / 5,5 ms** (p50 5,1–5,2) = setara batch 7; 3 channel di layar 5,3 ms
  (p50 4,9–5,1) = tidak ada regresi. Scene berat `bench-batch8.json` (3 regression
  channel rentang ~600k/750k/380k bar + 2 Gann fan + Elliott + XABCD + ABCD + brush 400
  titik + highlighter + 4 sticker): **14,0 / 14,6 / 14,5 ms** (p50 13,6–14,1 /
  p95 19,4–20,2). Atribusi terukur: scene tanpa regression = 4,9–5,1 ms (setara
  baseline), 3 regression berentang 100 bar = 5,0 ms, 1 regression 600k bar =
  8,8–9,0 ms; micro-bench release `regression_fit` = **0,003 ms @1k / 0,34 ms @100k /
  1,85 ms @600k bar** (dua lintasan O(rentang)). Konsekuensi jujur: memakai regression
  channel pada rentang ratusan ribu bar memakan ~2 ms per objek per frame; kandidat
  optimasi (prefix-sum → O(1)) dicatat untuk batch 9.
- **Perbaikan nyata temuan bench batch 8**: hit-test hover ternyata dijalankan tiap
  frame (blok kursor + pemilihan) sehingga `regression_fit` dieksekusi **dua kali per
  objek per frame** (instrumentasi sementara: ~1,9×/frame). Kini
  `ChartState::hover_pick` menyimpan hasilnya dengan kunci lengkap (posisi lokal,
  viewport, jumlah objek, kedalaman undo, sembunyi-semua, revisi dataset) dan hasil itu
  dipakai bersama blok kursor & render → **1,0×/frame** per objek; scene berat turun
  **36–38 ms → 13,6–14,1 ms** (p50) tanpa mengubah perilaku di jalur lain (kondisi
  mode gambar/gestur tetap memakai `pick` langsung seperti sebelumnya).
- Batas verifikasi yang sama seperti batch sebelumnya: interaksi GUI nyata tidak bisa
  di-screenshot/di-inject di lingkungan ini (tanpa `xwd`/`import`/`scrot`/`xdotool`),
  jadi bukti interaksi = tes egui in-process + PNG headless; teks tetap berupa kotak
  pada raster headless (jalur font bertekstur belum ada); platform Windows/macOS belum
  diuji di sini.
- Paket ulang: `dist/sokanlab-0.1.0-linux-x86_64.tar.gz` — sha256 `3f110c28716583581ad1c2036dc2c627e30e117a73a8aba35544046eee8e7034`
  (`sha256sum -c` OK; biner di dalam tar identik dengan `target/release`, sha256
  `bb1bfc58310bbde671e71960857ce8f6a838e2f54ad7a2a5c2909b4b2d411316`).
- Tambahan 2026-10-19 (setelah batch 8): preview Windows x86_64 di-cross-compile
  (`x86_64-pc-windows-gnu`) dan di-smoke-test via Wine — lihat `docs/WINDOWS-PREVIEW.md`.
  Saat pengemasan ditemukan `scripts/package.sh` tidak menyertakan `config/default.toml`
  sehingga GUI biner paket hasil ekstraksi gagal ("config gagal"); skrip diperbaiki
  (ikut `config/default.toml` + `THIRD-PARTY.md`) dan paket Linux dibangun ulang dengan
  sha256 di atas. Bukti GUI Wine (OpenGL) kini dapat di-screenshot via `xwd`+`xdotool`.
