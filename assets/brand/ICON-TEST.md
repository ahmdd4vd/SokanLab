# Uji ikon SokanLab — 2026-10-16

Metode: cek struktur SVG + proksi lebar-garis pada 16px (lebar_512 * 16/512), dan kontras WCAG.
Bukan uji render piksel penuh (tanpa rasterizer di env ini).

## Hasil

- Kontras: Ember di atas Ink 8.27 (lolos), Paper di atas Ink 16.08 (lolos), Ember di atas Paper 1.95 (gagal — jangan pakai blok Ember besar di atas Paper).
- `icon-a-sokan-pillar.svg`: body 80px → 2.5px di 16px (ok), wick 20px → 0.62px (hilang di 16px).
- `icon-b-dual-bar.svg`: wick 20px → 0.62px (hilang); plus masalah kontras Ember-di-Paper. Status: perlu rework, bukan pilihan utama.
- `icon-c-grid-mark.svg`: grid 12px → 0.38px (hilang di 16px); body 80px ok.
- `icon-small-mono.svg`: lebar min 160px → 5.0px di 16px (lolos). Tanpa garis tipis, tanpa teks.

## Keputusan

- Primer: A atau C untuk ukuran ≥32px dengan background Ink.
- Ukuran 16–32px: wajib pakai `icon-small-mono.svg` (tanpa wick/grid tipis).
- Varian B ditolak sebagai primer sampai kontras + ketebalan diperbaiki.
- Semua file: tanpa teks, tanpa aset tiruan TradingView/Muse, digambar dari nol.
