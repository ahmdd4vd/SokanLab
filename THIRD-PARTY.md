# Pihak Ketiga & Lisensi

SokanLab sendiri dilisensikan Apache-2.0 (lihat `LICENSE`). Daftar berikut adalah
dependensi eksternal langsung yang dipakai biner/runtime, beserta lisensinya.
Dependensi transitif tercatat di `Cargo.lock`; seluruh graph build dapat diperiksa
dengan `cargo tree`.

| Paket | Versi | Lisensi | Dipakai untuk |
|---|---|---|---|
| `eframe` | 0.37 | MIT OR Apache-2.0 | Shell window desktop |
| `egui` | 0.37 | MIT OR Apache-2.0 | Widget + painter chart |
| `epaint` | 0.37 | MIT OR Apache-2.0 | Tipe primitif gambar |
| `log` | 0.4 | MIT OR Apache-2.0 | Logging lokal (teredaksi) |
| `arrow-array`, `arrow-schema` | 60 | Apache-2.0 | Kolom Parquet |
| `parquet` | 60 | Apache-2.0 | Baca/tulis berkas Parquet |
| `rusqlite` (fitur `bundled`) | 0.40 | MIT | Katalog SQLite (SQLite: public domain) |
| `sha2` | 0.11 | MIT OR Apache-2.0 | Checksum SHA-256 provenance |

Catatan:

- Tidak ada klien HTTP/TLS, tidak ada runtime async jaringan di graph build
  (diverifikasi `scripts/check-offline.sh`).
- Tidak ada dependensi yang membutuhkan layanan online saat runtime; pertama kali
  build memerlukan akses registry crates.io untuk mengunduh sumber paket.
- Font/ikon: ikon brand di `assets/brand/` adalah karya orisinal SokanLab;
  font default egui berasal dari dependensi egui (lisensi di atas).
