# Preview Windows x86_64 (paket portable)

Status: **preview / belum dirilis resmi**. Biner Windows dibangun dari Linux dengan
cross-compile `x86_64-pc-windows-gnu` + mingw-w64, lalu di-smoke-test memakai Wine 10.0.
**Belum pernah dijalankan di Windows asli** — inilah yang perlu kamu uji.

- Paket: `dist/sokanlab-0.1.0-windows-x86_64-preview.zip` (~16 MB, portable, tanpa installer)
- SHA-256 ZIP: lihat `dist/sokanlab-0.1.0-windows-x86_64-preview.zip.sha256`
  (tidak ditulis di dokumen ini agar tidak sirkular dengan isi ZIP)
- SHA-256 biner `sokanlab-desktop.exe`:
  `9b61ea80e4a03168ffaa6126c05de7afcf762555668e04ca3d98fdda09a01d73`
  (51.272.480 byte, PE32+ console x86-64)

## Isi paket

```
sokanlab-0.1.0-windows-x86_64-preview/
  sokanlab-desktop.exe        biner utama
  SokanLab.bat                buka chart bersih (SMA/EMA/VWAP + RSI/MACD)
  SokanLab-demo.bat           buka chart contoh 10 alat gambar
  SokanLab-OpenGL.bat         fallback backend OpenGL
  Impor-CSV.bat               impor CSV sendiri (interaktif)
  MULAI-DI-SINI.txt           panduan singkat + pintasan
  CATATAN-BUILD.txt           provenance build + hasil uji Wine
  config/default.toml         konfigurasi wajib (dibaca relatif ke folder ini)
  data/
    BTCUSDT-1h.parquet        contoh data SINTETIS 2.400 bar 1 jam
    contoh-BTCUSDT-1h.csv     CSV sumber contoh
    catalog.db                katalog SQLite contoh
    layout.json               layout bersih
    layout-demo.json          layout demo 10 objek
  docs/
    BACKTEST.md, TV-PARITY.md, preview-win-gui.png
  README.md, LICENSE, NOTICE, THIRD-PARTY.md
```

## Cara pakai di Windows

1. Extract **seluruh** isi ZIP ke satu folder (jangan jalankan dari dalam ZIP).
2. Klik dua kali `SokanLab.bat` (chart bersih) atau `SokanLab-demo.bat` (demo).
3. Jika SmartScreen muncul: **More info → Run anyway** (biner belum ditandatangani).
4. Jangan memindahkan `sokanlab-desktop.exe` sendirian; folder `config/` dan `data/`
   harus tetap di sebelahnya.
5. Untuk data sendiri: jalankan `Impor-CSV.bat` (format
   `timestamp,open,high,low,close,volume`, semua integer; timeframe `1m`/`1h`/`1D`).
6. Jika layar hitam / GPU gagal: coba `SokanLab-OpenGL.bat`.

Pintasan penting: `F1` (daftar lengkap), `Alt+T/H/V/C/F` (trend/horizontal/vertical/
crossline/fib), `1`–`9` pilih alat, `Ctrl+Z`/`Ctrl+Y`, `Alt+R` fit, `Alt+S` snapshot,
`Ctrl+S` simpan layout, `Del` hapus objek terpilih, `Ctrl+klik` multi-seleksi.

## Bukti verifikasi (Wine 10.0 di Linux, bukan Windows asli)

Lingkungan: Xvfb + Openbox, Mesa llvmpipe/lavapipe, prefix Wine terisolasi.

Lulus:

| Uji | Hasil |
|---|---|
| `--version` | `sokanlab-desktop 0.1.0 (schema 1)` |
| Render headless batch 8 (400 bar, 1200x700) | 13.338 segitiga; PNG **byte-identik** dengan referensi Linux (`5cdffff2…`) |
| `--headless` dataset contoh + layout demo | `headless-ok bars=2400`, 10 drawings, watchlist `BTC/USDT-1h` |
| Save/restore layout (schema 7) | byte-identik dengan hasil build Linux |
| CLI `import` CSV 2.400 bar | Parquet + katalog SQLite, sha256 provenance sama dengan Linux |
| CLI `backtest ma_cross` | 5 berkas laporan (equity, ledger, metrics, report.json, trades) |
| GUI (`WGPU_BACKEND=opengl`) | 5 frame, avg 5,5 ms, p50 0,9 ms, p95 18,0 ms; screenshot `docs/preview-win-gui.png` |
| `SokanLab-OpenGL.bat` | membuka GUI, memuat dataset + layout |

Belum bisa diverifikasi di sini:

- Backend default **DX12** di Wine gagal (`Parent device is lost`, keterbatasan
  vkd3d/lavapipe headless). Di Windows asli DX12 adalah jalur normal; fallback OpenGL
  sudah terbukti di Wine.
- Build belum dijalankan di Windows asli; belum ada installer/code-signing/auto-update.

## Reproduksi build

```bash
sudo apt-get install -y gcc-mingw-w64-x86-64
rustup target add x86_64-pc-windows-gnu
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
cargo build --release -p sokanlab-desktop --target x86_64-pc-windows-gnu
# biner: target/x86_64-pc-windows-gnu/release/sokanlab-desktop.exe
```

Catatan: build ini butuh RAM cukup (crate `windows` besar); di VM 8 GB perlu swap
sementara atau `CARGO_BUILD_JOBS=1`.

Dependensi DLL biner hanya DLL sistem Windows (KERNEL32, USER32, GDI32, DXGI, DWrite,
UIAutomation, ws2_32, dst) — tidak ada `libgcc_s`/`libwinpthread`.

## Rilis otomatis

Tag `v*` memicu workflow `release` (`.github/workflows/release.yml`) yang:

1. membangun biner native untuk Linux (`x86_64-unknown-linux-gnu`), Windows
   (`x86_64-pc-windows-msvc`), dan macOS (`aarch64-apple-darwin`);
2. membuat data contoh deterministik — `scripts/gen-sample-data.py` memakai seed
   tetap, jadi CSV/Parquet-nya byte-identik di semua mesin;
3. menjalankan `import` CLI agar paket memuat `catalog.db` + Parquet;
4. mengemas paket portable + `SHA256SUMS.txt` lewat `scripts/package-preview.py`
   dan mengunggah `sokanlab-<tag>-<platform>.zip|tar.gz` + `.sha256` ke release.

Uji pengemasan lokal (tanpa CI):

```bash
python3 scripts/gen-sample-data.py --out stage/data
./target/release/sokanlab-desktop import --file stage/data/contoh-BTCUSDT-1h.csv \
  --symbol BTC/USDT --timeframe 1h --db stage/data/catalog.db --out stage/data/BTCUSDT-1h.parquet
python3 scripts/package-preview.py --name sokanlab-demo-linux-x86_64 \
  --bin target/release/sokanlab-desktop --format tar.gz --out dist --data-dir stage/data
```

Paket preview ini dibangun manual lewat cross-compile `x86_64-pc-windows-gnu` +
skrip di `packaging/windows/`; paket dari workflow memakai target MSVC native
dengan berkas launcher yang sama.

## Lihat juga

- `MULAI-DI-SINI.txt` (di dalam paket) — panduan pemakaian singkat
- `docs/TV-PARITY.md` — fitur yang sudah/belum setara TradingView
- `docs/BACKTEST.md` — semantik backtest deterministik
