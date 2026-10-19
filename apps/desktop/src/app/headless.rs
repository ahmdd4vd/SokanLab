// SokanLab headless render — Apache-2.0.
// Verifikasi nyata: chart dirender melalui pipeline tessellator egui tanpa window,
// lalu PNG ditulis dari triangle hasil tessellasi (fallback CPU untuk backend GPU).
use egui::{Context, RawInput, Rect};
use sokanlab_chart::drawing::{
    Anchor, ChannelKind, DashStyle, Drawing, ExtendMode, FibLevel, ItemStyle,
};
use sokanlab_chart::visible_indices;
use sokanlab_domain::Bar;
use sokanlab_ui::painter::{
    draw_axes, draw_candles, draw_crosshair, draw_drawings, draw_grid, draw_indicator_panes,
    draw_last_price, draw_overlays, draw_rubber_band, draw_volume, fit_spacing, plot_width,
    split_plot_panes,
};
use sokanlab_ui::theme;
use sokanlab_ui::{ChartState, Gesture, OverlaySettings, PaneKind, PaneSetting};

/// Raster triangle-filled sederhana (RGBA8) untuk verifikasi visual.
struct Canvas {
    w: usize,
    h: usize,
    px: Vec<u8>,
}

impl Canvas {
    fn new(w: usize, h: usize) -> Self {
        Self {
            w,
            h,
            px: [0x14, 0x18, 0x1d, 0xff].repeat(w * h),
        }
    }

    fn set(&mut self, x: i64, y: i64, c: [u8; 4]) {
        if x < 0 || y < 0 || x >= self.w as i64 || y >= self.h as i64 {
            return;
        }
        let i = (y as usize * self.w + x as usize) * 4;
        // alpha blend sederhana
        let a = c[3] as u32;
        for (k, &src) in c.iter().take(3).enumerate() {
            let dst = self.px[i + k] as u32;
            self.px[i + k] = (((src as u32) * a + dst * (255 - a)) / 255) as u8;
        }
        self.px[i + 3] = 0xff;
    }

    /// Isi triangle dengan warna `c`, dibatasi `clip` (piksel): clip_rect dari
    /// tessellator (dipakai egui untuk pane/drawing) harus dihormati jalur CPU.
    fn fill_tri(&mut self, p: [(f32, f32); 3], c: [u8; 4], clip: (i64, i64, i64, i64)) {
        let minx = (p.iter().map(|q| q.0).fold(f32::MAX, f32::min).floor() as i64).max(clip.0);
        let maxx = (p.iter().map(|q| q.0).fold(f32::MIN, f32::max).ceil() as i64).min(clip.2);
        let miny = (p.iter().map(|q| q.1).fold(f32::MAX, f32::min).floor() as i64).max(clip.1);
        let maxy = (p.iter().map(|q| q.1).fold(f32::MIN, f32::max).ceil() as i64).min(clip.3);
        let (ax, ay) = (p[0].0, p[0].1);
        let (bx, by) = (p[1].0, p[1].1);
        let (cx, cy) = (p[2].0, p[2].1);
        let area = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
        if area.abs() < 1e-9 {
            return;
        }
        for y in miny..=maxy {
            for x in minx..=maxx {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let w0 = ((bx - ax) * (py - ay) - (by - ay) * (px - ax)) / area;
                let w1 = ((cx - bx) * (py - by) - (cy - by) * (px - bx)) / area;
                let w2 = ((ax - cx) * (py - cy) - (ay - cy) * (px - cx)) / area;
                if (w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0) || (w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0) {
                    self.set(x, y, c);
                }
            }
        }
    }
}

/// Tulis PNG tanpa dependensi eksternal (zlib stored block + CRC32).
fn write_png(path: &std::path::Path, w: usize, h: usize, rgba: &[u8]) -> std::io::Result<()> {
    fn crc32(data: &[u8]) -> u32 {
        let mut c = 0xffff_ffffu32;
        for &b in data {
            c ^= b as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xedb8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
        }
        !c
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc32(&body).to_be_bytes());
    }
    // scanline: filter byte 0 + RGB
    let mut raw = Vec::with_capacity(h * (w * 3 + 1));
    for y in 0..h {
        raw.push(0u8);
        for x in 0..w {
            let i = (y * w + x) * 4;
            raw.extend_from_slice(&rgba[i..i + 3]);
        }
    }
    // zlib stored
    let mut z = vec![0x78u8, 0x01];
    let mut off = 0usize;
    while off < raw.len() {
        let n = (raw.len() - off).min(65535);
        let last = if off + n >= raw.len() { 1u8 } else { 0u8 };
        z.push(last);
        z.extend_from_slice(&(n as u16).to_le_bytes());
        z.extend_from_slice(&(!(n as u16)).to_le_bytes());
        z.extend_from_slice(&raw[off..off + n]);
        off += n;
    }
    let mut adler_a: u32 = 1;
    let mut adler_b: u32 = 0;
    for &b in &raw {
        adler_a = (adler_a + b as u32) % 65521;
        adler_b = (adler_b + adler_a) % 65521;
    }
    z.extend_from_slice(&((adler_b << 16) | adler_a).to_be_bytes());

    let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8-bit, truecolor
    chunk(&mut png, b"IHDR", &ihdr);
    chunk(&mut png, b"IDAT", &z);
    chunk(&mut png, b"IEND", &[]);
    std::fs::write(path, png)
}

fn synth_bars(n: usize) -> Vec<Bar> {
    // Harga sintetik deterministik (bukan data pasar nyata): mean-reverting + noise.
    let mut seed: i64 = 20_260_101;
    let mut next = move || {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (seed >> 33) % 7 - 3 // -3..3
    };
    let mut p = 100i64;
    (0..n)
        .map(|i| {
            let drift = ((i as f64 / 18.0).sin() * 2.0).round() as i64; // osilasi
            let noise = next();
            let o = p;
            let c = o + drift + noise;
            let h = o.max(c) + 1 + (next().abs());
            let l = o.min(c) - 1 - (next().abs());
            p = c;
            Bar {
                ts_millis: 1_700_000_000_000 + i as i64 * 60_000,
                open_ticks: o,
                high_ticks: h,
                low_ticks: l,
                close_ticks: c,
                volume_milli: 400 + (i as i64 * 53 % 1100),
            }
        })
        .collect()
}

/// Render chart ke PNG. Mengembalikan jumlah triangle ter-tessellasi.
/// Demo scene penuh (batch 0–8): semua alat lama + sisipan batch 8.
fn demo_full(st: &mut ChartState, bars: &[Bar], area: Rect) {
    st.add_drawing(sokanlab_ui::horizontal_line(
        bars[bars.len() / 2].close_ticks,
    ));
    // Demo alat (bukti render): trend, fib, posisi long, rentang ukur.
    st.add_drawing(Drawing::Trendline {
        a: Anchor {
            ts_millis: bars[10].ts_millis,
            price_ticks: bars[10].low_ticks,
        },
        b: Anchor {
            ts_millis: bars[bars.len() - 6].ts_millis,
            price_ticks: bars[bars.len() - 6].high_ticks,
        },
    });
    st.add_drawing(Drawing::FibRetracement {
        a: Anchor {
            ts_millis: bars[20].ts_millis,
            price_ticks: bars[20].high_ticks,
        },
        b: Anchor {
            ts_millis: bars[40].ts_millis,
            price_ticks: bars[40].low_ticks,
        },
    });
    // Batch 5: level fib kustom (0/38,2/50/61,8/100) + warna khusus di 50%.
    if let Some(it) = st.drawings.items.last_mut() {
        it.fib_levels = vec![
            FibLevel {
                milli: 0,
                color: None,
            },
            FibLevel {
                milli: 382,
                color: None,
            },
            FibLevel {
                milli: 500,
                color: Some([0x4d, 0x9c, 0xf2]),
            },
            FibLevel {
                milli: 618,
                color: None,
            },
            FibLevel {
                milli: 1000,
                color: None,
            },
        ];
    }
    st.add_drawing(Drawing::Position {
        long: true,
        entry: Anchor {
            ts_millis: bars[30].ts_millis,
            price_ticks: bars[30].close_ticks,
        },
        target: Anchor {
            ts_millis: bars[55].ts_millis,
            price_ticks: bars[55].high_ticks + 4,
        },
    });
    st.add_drawing(Drawing::RangeMeasure {
        a: Anchor {
            ts_millis: bars[5].ts_millis,
            price_ticks: bars[5].low_ticks,
        },
        b: Anchor {
            ts_millis: bars[15].ts_millis,
            price_ticks: bars[15].high_ticks,
        },
    });
    // Demo batch 2 (butuh dataset cukup panjang): channel, pitchfork, fib
    // extension, fib time — plus satu objek bergaya khusus.
    if bars.len() >= 100 {
        st.add_drawing(Drawing::ParallelChannel {
            a: Anchor {
                ts_millis: bars[8].ts_millis,
                price_ticks: bars[8].low_ticks,
            },
            b: Anchor {
                ts_millis: bars[48].ts_millis,
                price_ticks: bars[48].high_ticks,
            },
            c: Anchor {
                ts_millis: bars[8].ts_millis,
                price_ticks: bars[8].low_ticks - 6,
            },
            kind: ChannelKind::Parallel,
        });
        if let Some(it) = st.drawings.items.last_mut() {
            it.style = Some(ItemStyle {
                color: [0x7f, 0xb2, 0xd9],
                width: 2,
                dash: DashStyle::Dashed,
            });
        }
        // Batch 7: varian channel — disjoint dengan perpanjangan kanan, dan
        // flat top/bottom (batas kedua horizontal).
        st.add_drawing(Drawing::ParallelChannel {
            a: Anchor {
                ts_millis: bars[12].ts_millis,
                price_ticks: bars[12].high_ticks + 2,
            },
            b: Anchor {
                ts_millis: bars[52].ts_millis,
                price_ticks: bars[52].low_ticks - 2,
            },
            c: Anchor {
                ts_millis: bars[30].ts_millis,
                price_ticks: bars[30].high_ticks + 14,
            },
            kind: ChannelKind::Disjoint,
        });
        if let Some(it) = st.drawings.items.last_mut() {
            it.extend = ExtendMode::Right;
        }
        st.add_drawing(Drawing::ParallelChannel {
            a: Anchor {
                ts_millis: bars[66].ts_millis,
                price_ticks: bars[66].low_ticks,
            },
            b: Anchor {
                ts_millis: bars[92].ts_millis,
                price_ticks: bars[92].high_ticks,
            },
            c: Anchor {
                ts_millis: bars[70].ts_millis,
                price_ticks: bars[70].high_ticks + 10,
            },
            kind: ChannelKind::Flat,
        });
        st.add_drawing(Drawing::Pitchfork {
            a: Anchor {
                ts_millis: bars[60].ts_millis,
                price_ticks: bars[60].high_ticks + 6,
            },
            b: Anchor {
                ts_millis: bars[75].ts_millis,
                price_ticks: bars[75].low_ticks,
            },
            c: Anchor {
                ts_millis: bars[90].ts_millis,
                price_ticks: bars[90].low_ticks + 4,
            },
        });
        if let Some(it) = st.drawings.items.last_mut() {
            it.opacity = 70; // batch 5: objek semi-transparan
        }
        // Batch 5: varian pitchfork Schiff pada rentang awal.
        st.add_drawing(Drawing::Pitchfork {
            a: Anchor {
                ts_millis: bars[20].ts_millis,
                price_ticks: bars[20].low_ticks,
            },
            b: Anchor {
                ts_millis: bars[35].ts_millis,
                price_ticks: bars[35].high_ticks,
            },
            c: Anchor {
                ts_millis: bars[50].ts_millis,
                price_ticks: bars[50].high_ticks - 3,
            },
        });
        if let Some(it) = st.drawings.items.last_mut() {
            it.pitchfork = sokanlab_chart::drawing::PitchforkKind::Schiff;
        }
        st.add_drawing(Drawing::FibExtension {
            a: Anchor {
                ts_millis: bars[35].ts_millis,
                price_ticks: bars[35].high_ticks,
            },
            b: Anchor {
                ts_millis: bars[50].ts_millis,
                price_ticks: bars[50].low_ticks,
            },
            c: Anchor {
                ts_millis: bars[65].ts_millis,
                price_ticks: bars[65].close_ticks,
            },
        });
        st.add_drawing(Drawing::FibTime {
            a: Anchor {
                ts_millis: bars[55].ts_millis,
                price_ticks: 0,
            },
        });
    }
    // Demo batch 3: teks/callout (gaya garis titik), dan indikator overlay.
    if bars.len() >= 40 {
        st.add_drawing(Drawing::Text {
            at: Anchor {
                ts_millis: bars[12].ts_millis,
                price_ticks: bars[12].high_ticks + 8,
            },
            text: "zona resisten".into(),
        });
        st.add_drawing(Drawing::Callout {
            at: Anchor {
                ts_millis: bars[26].ts_millis,
                price_ticks: bars[26].low_ticks - 6,
            },
            text: "entry pullback".into(),
        });
        if let Some(it) = st.drawings.items.last_mut() {
            it.style = Some(ItemStyle {
                color: [0xf2, 0xb0, 0x4d],
                width: 2,
                dash: DashStyle::Dotted,
            });
        }
    }
    st.set_overlay_settings(OverlaySettings {
        sma: Some(20),
        ema: Some(9),
        vwap: true,
    });
    // Demo batch 4: pane indikator RSI + MACD.
    st.set_panes(vec![
        PaneSetting {
            kind: PaneKind::Rsi,
            period: 14,
            slow: 0,
            signal: 0,
        },
        PaneSetting {
            kind: PaneKind::Macd,
            period: 12,
            slow: 26,
            signal: 9,
        },
    ]);

    // Demo batch 8 (ringkas di sela objek lama): regression channel, sticker,
    // goresan brush + highlighter.
    if bars.len() >= 120 {
        let last = bars.len() - 1;
        let i0 = last * 3 / 5;
        st.add_drawing(Drawing::RegressionChannel {
            a: Anchor {
                ts_millis: bars[i0].ts_millis,
                price_ticks: bars[i0].close_ticks,
            },
            b: Anchor {
                ts_millis: bars[last - 2].ts_millis,
                price_ticks: bars[last - 2].close_ticks,
            },
        });
        if let Some(it) = st.drawings.items.last_mut() {
            it.style = Some(ItemStyle {
                color: [0x8f, 0xc9, 0x8f],
                width: 2,
                dash: DashStyle::Solid,
            });
            it.extend = ExtendMode::Right;
        }
        st.add_drawing(Drawing::Sticker {
            at: Anchor {
                ts_millis: bars[last - 6].ts_millis,
                price_ticks: bars[last - 6].high_ticks + 4,
            },
            icon: sokanlab_chart::drawing::StickerIcon::ArrowDown,
        });
        st.add_drawing(Drawing::Brush {
            points: vec![
                Anchor {
                    ts_millis: bars[last - 30].ts_millis,
                    price_ticks: bars[last - 30].low_ticks - 6,
                },
                Anchor {
                    ts_millis: bars[last - 24].ts_millis,
                    price_ticks: bars[last - 24].low_ticks - 2,
                },
                Anchor {
                    ts_millis: bars[last - 18].ts_millis,
                    price_ticks: bars[last - 18].low_ticks - 8,
                },
            ],
        });
        st.add_drawing(Drawing::Highlighter {
            points: vec![
                Anchor {
                    ts_millis: bars[last - 40].ts_millis,
                    price_ticks: bars[last - 40].high_ticks + 6,
                },
                Anchor {
                    ts_millis: bars[last - 22].ts_millis,
                    price_ticks: bars[last - 22].high_ticks + 4,
                },
            ],
        });
    }
    // Peta Y memakai tinggi pane harga (konsisten dengan split_plot_panes).
    let layout0 = split_plot_panes(area, 8.0, st.panes.len());
    let vp = st
        .viewport
        .resize(layout0.price.width() as f64, layout0.price.height() as f64)
        .fit_price(bars);
    // Demo batch 6: multi-seleksi (dua objek terakhir ditandai ember) + kotak
    // seleksi Ctrl+seret yang melingkupi keduanya.
    let n_items = st.drawings.items.len();
    if n_items >= 2 {
        st.set_selection(&[n_items - 2, n_items - 1]);
        let (b_hi, b_lo) = (12usize.min(bars.len() - 1), 26usize.min(bars.len() - 1));
        let xa = area.min.x + vp.x_for_time(bars[b_hi.saturating_sub(4)].ts_millis) as f32 - 16.0;
        let xb = area.min.x + vp.x_for_time(bars[b_lo.saturating_add(4)].ts_millis) as f32 + 16.0;
        let ya = area.min.y + vp.y_for_price(bars[b_hi].high_ticks + 12) as f32;
        let yb = area.min.y + vp.y_for_price(bars[b_lo].low_ticks - 10) as f32;
        st.gesture = Gesture::Rubber {
            origin: egui::pos2(xa, ya.min(yb)),
            current: egui::pos2(xb, ya.max(yb)),
        };
    }
}

/// Demo khusus batch 8 (bukti visual bersih): semua jenis objek baru, plus
/// baris 14 ikon sticker di pita bawah supaya tiap ikon terbaca di PNG.
fn demo_batch8(st: &mut ChartState, bars: &[Bar]) {
    let n = bars.len();
    if n < 120 {
        return;
    }
    let last = n - 1;
    let idx = |f: f64| ((n as f64 * f).round() as usize).min(last);
    let hi = |f: f64| bars[idx(f)].high_ticks;
    let lo = |f: f64| bars[idx(f)].low_ticks;
    let close = |f: f64| bars[idx(f)].close_ticks;
    let an = |f: f64, p: i64| Anchor {
        ts_millis: bars[idx(f)].ts_millis,
        price_ticks: p,
    };
    let min_low = bars.iter().map(|b| b.low_ticks).min().unwrap_or(0);
    let style = |st: &mut ChartState, c: [u8; 3], w: u8| {
        if let Some(it) = st.drawings.items.last_mut() {
            it.style = Some(ItemStyle {
                color: c,
                width: w,
                dash: DashStyle::Solid,
            });
        }
    };
    // 0: regression channel (kuadrat-terkecil close), diperpanjang ke kanan.
    st.add_drawing(Drawing::RegressionChannel {
        a: an(0.62, close(0.62)),
        b: an(0.94, close(0.94)),
    });
    st.drawings.items.last_mut().unwrap().extend = ExtendMode::Right;
    style(st, [0x8f, 0xc9, 0x8f], 2);
    // 1: fib trend extension 3 titik — level horizontal dari titik ketiga.
    st.add_drawing(Drawing::FibTrendExtension {
        a: an(0.44, lo(0.44) - 2),
        b: an(0.52, hi(0.52) + 2),
        c: an(0.57, close(0.57)),
    });
    style(st, [0xd9, 0xa6, 0x6b], 2);
    // 2: Elliott impulse 1–5 (zigzag pada ayunan harga kiri).
    st.add_drawing(Drawing::ElliottImpulse {
        points: vec![
            an(0.01, lo(0.01) - 4),
            an(0.05, hi(0.05) + 5),
            an(0.09, lo(0.09) - 2),
            an(0.13, hi(0.13) + 9),
            an(0.17, lo(0.17) + 1),
        ],
    });
    style(st, [0xf2, 0xb0, 0x4d], 2);
    // 3: koreksi A–C.
    st.add_drawing(Drawing::ElliottCorrection {
        points: vec![
            an(0.19, hi(0.19) + 7),
            an(0.23, lo(0.23) - 3),
            an(0.27, hi(0.27) + 3),
        ],
    });
    style(st, [0x4d, 0x9c, 0xf2], 2);
    // 4: XABCD lima titik.
    st.add_drawing(Drawing::Xabcd {
        points: vec![
            an(0.29, lo(0.29) - 3),
            an(0.33, hi(0.33) + 7),
            an(0.37, lo(0.37) + 2),
            an(0.41, hi(0.41) + 10),
            an(0.45, lo(0.45) + 5),
        ],
    });
    style(st, [0xc9, 0x8f, 0xc9], 2);
    // 5: ABCD empat titik.
    st.add_drawing(Drawing::Abcd {
        points: vec![
            an(0.47, lo(0.47) - 4),
            an(0.50, hi(0.50) + 6),
            an(0.53, lo(0.53) - 2),
            an(0.56, hi(0.56) + 8),
        ],
    });
    style(st, [0x66, 0xc2, 0xa4], 2);
    // 6: Gann fan sembilan sudut (1x1 = a→b).
    st.add_drawing(Drawing::GannFan {
        a: an(0.58, lo(0.58) - 1),
        b: an(0.64, hi(0.64) + 3),
    });
    style(st, [0xc9, 0x8f, 0xc9], 1);
    // 7: brush — goresan tangan di ruang kosong tengah-kiri.
    st.add_drawing(Drawing::Brush {
        points: vec![
            an(0.02, lo(0.02) - 14),
            an(0.06, lo(0.06) - 8),
            an(0.10, lo(0.10) - 18),
            an(0.14, lo(0.14) - 10),
        ],
    });
    style(st, [0x9a, 0xa5, 0xb2], 2);
    // 8: highlighter (tebal & tembus pandang).
    st.add_drawing(Drawing::Highlighter {
        points: vec![an(0.03, lo(0.03) - 26), an(0.22, lo(0.22) - 22)],
    });
    style(st, [0x4d, 0x9c, 0xf2], 1);
    // 9–22: baris 14 ikon sticker (bukan emoji pihak lain — vektor bawaan).
    let icons = sokanlab_chart::drawing::StickerIcon::all();
    for (k, icon) in icons.iter().enumerate() {
        let f = 0.03 + 0.86 * (k as f64) / ((icons.len() - 1) as f64);
        st.add_drawing(Drawing::Sticker {
            at: an(f, min_low + 4),
            icon: *icon,
        });
    }
    // Seleksi ganda (ember): jalur seleksi/hit-test jenis baru.
    st.set_selection(&[1, 4]);
}

pub fn render(
    out: &std::path::Path,
    bars_n: usize,
    width: usize,
    height: usize,
    demo: Option<&str>,
) -> std::io::Result<usize> {
    let bars = synth_bars(bars_n);
    let area = Rect::from_min_size(
        egui::pos2(0.0, 0.0),
        egui::vec2(width as f32, height as f32),
    );
    let mut st = ChartState::new();
    st.load(bars.clone());
    match demo {
        Some("batch8") => demo_batch8(&mut st, &bars),
        _ => demo_full(&mut st, &bars, area),
    }
    // Peta Y memakai tinggi pane harga (konsisten dengan split_plot_panes).
    let layout0 = split_plot_panes(area, 8.0, st.panes.len());
    let vp = st
        .viewport
        .resize(layout0.price.width() as f64, layout0.price.height() as f64)
        .fit_price(&bars);
    let bars_ref = &bars;
    let st_ref = &st;
    let raster = rasterize(width, height, 1.0, theme::Theme::Dark, |ui, area| {
        let visible = visible_indices(bars_ref, vp.t_min, vp.t_max);
        let spacing = fit_spacing(
            plot_width(area) as f64,
            (visible.end - visible.start).max(1),
        );
        let layout = split_plot_panes(area, spacing, st_ref.panes.len());
        let painter = ui.painter_at(area);
        draw_grid(&painter, &layout, 5);
        draw_axes(&painter, &layout, &vp);
        draw_candles(&painter, &layout, &vp, bars_ref, visible.start);
        draw_volume(
            &painter,
            &layout,
            bars_ref,
            vp.t_min,
            vp.t_max,
            visible.start,
        );
        draw_overlays(
            &painter,
            &layout,
            &vp,
            bars_ref,
            visible.start,
            &st_ref.overlay_lines,
        );
        draw_indicator_panes(
            &painter,
            &layout,
            &vp,
            bars_ref,
            visible.start,
            &st_ref.panes,
        );
        draw_drawings(
            &painter,
            &layout,
            &vp,
            bars_ref,
            &st_ref.drawings.items,
            &st_ref.selection(),
            None,
            false,
        );
        if let Gesture::Rubber { origin, current } = &st_ref.gesture {
            draw_rubber_band(
                &painter,
                egui::Rect::from_two_pos(*origin, *current),
                theme::EMBER,
            );
        }
        draw_last_price(&painter, &layout, &vp, st_ref.last_close());
        draw_crosshair(
            &painter,
            &layout,
            &vp,
            Some(egui::pos2(area.min.x + 220.0, area.min.y + 180.0)),
        );
    });
    write_png(out, width, height, &raster.px)?;
    Ok(raster.tris)
}

/// Render state chart saat ini (tanpa window) ke PNG — dipakai snapshot GUI
/// (Alt+S). Rentang waktu & harga dari state dipertahankan; tinggi pane
/// disesuaikan dengan ukuran gambar. Return jumlah triangle ter-tessellasi.
pub fn render_chart_png(
    out: &std::path::Path,
    st: &ChartState,
    width: usize,
    height: usize,
) -> std::io::Result<usize> {
    let area = Rect::from_min_size(
        egui::pos2(0.0, 0.0),
        egui::vec2(width as f32, height as f32),
    );
    // Skala piksel snapshot (GUI: 1×/2×/4×), dibatasi agar raster ≤ ~8K.
    let scale = (st.snapshot_scale.max(1) as f32)
        .min(7680.0 / width.max(1) as f32)
        .min(4320.0 / height.max(1) as f32)
        .max(1.0);
    let layout0 = split_plot_panes(area, 8.0, st.panes.len());
    let vp = st
        .viewport
        .resize(layout0.price.width() as f64, layout0.price.height() as f64);
    let raster = rasterize(width, height, scale, st.theme, |ui, area| {
        let visible = visible_indices(&st.bars, vp.t_min, vp.t_max);
        let spacing = fit_spacing(
            plot_width(area) as f64,
            (visible.end - visible.start).max(1),
        );
        let layout = split_plot_panes(area, spacing, st.panes.len());
        let painter = ui.painter_at(area);
        draw_grid(&painter, &layout, 5);
        draw_axes(&painter, &layout, &vp);
        draw_candles(&painter, &layout, &vp, &st.bars, visible.start);
        draw_volume(
            &painter,
            &layout,
            &st.bars,
            vp.t_min,
            vp.t_max,
            visible.start,
        );
        draw_overlays(
            &painter,
            &layout,
            &vp,
            &st.bars,
            visible.start,
            &st.overlay_lines,
        );
        draw_indicator_panes(&painter, &layout, &vp, &st.bars, visible.start, &st.panes);
        draw_drawings(
            &painter,
            &layout,
            &vp,
            &st.bars,
            &st.drawings.items,
            &st.selection(),
            None,
            false,
        );
        if let Gesture::Rubber { origin, current } = &st.gesture {
            draw_rubber_band(
                &painter,
                egui::Rect::from_two_pos(*origin, *current),
                theme::EMBER,
            );
        }
        draw_last_price(&painter, &layout, &vp, st.last_close());
    });
    write_png(
        out,
        (width as f32 * scale) as usize,
        (height as f32 * scale) as usize,
        &raster.px,
    )?;
    Ok(raster.tris)
}

/// Hasil raster: piksel RGBA8 + jumlah triangle.
struct Raster {
    px: Vec<u8>,
    tris: usize,
}

/// Jalankan satu pass egui headless lalu raster hasil tessellasi ke RGBA8.
fn rasterize<F>(
    width: usize,
    height: usize,
    scale: f32,
    theme_now: theme::Theme,
    mut draw: F,
) -> Raster
where
    F: FnMut(&egui::Ui, Rect),
{
    let ctx = Context::default();
    theme::apply(&ctx, theme_now);
    let area = Rect::from_min_size(
        egui::pos2(0.0, 0.0),
        egui::vec2(width as f32, height as f32),
    );
    let input = RawInput {
        screen_rect: Some(area),
        ..Default::default()
    };
    let mut full = ctx.run_pass(input, |ctx| {
        ctx.root_ui(|ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                draw(ui, area);
            });
        });
    });
    let shapes = std::mem::take(&mut full.shapes);
    full.textures_delta.clear();
    drop(full);

    let prims = ctx.tessellate(shapes, scale);
    let mut canvas = Canvas::new(
        (width as f32 * scale) as usize,
        (height as f32 * scale) as usize,
    );
    let mut tris = 0usize;
    for p in &prims {
        let epaint::Primitive::Mesh(mesh) = &p.primitive else {
            continue;
        };
        // Tripel indeks dari tessellator: kelompokkan tiap 3 indeks.
        // Vertices dalam satuan point → dikalikan skala saat raster (HiDPI).
        let vertex = |i: u32| {
            let vtx = mesh.vertices[i as usize];
            (vtx.pos.x * scale, vtx.pos.y * scale)
        };
        let colour = |i: u32| {
            let c = mesh.vertices[i as usize].color;
            [c.r(), c.g(), c.b(), c.a()]
        };
        // Clip rect dari tessellator (satuan point → piksel); batasi area isi.
        let cr = p.clip_rect;
        let clip = (
            (cr.min.x * scale).floor() as i64,
            (cr.min.y * scale).floor() as i64,
            (cr.max.x * scale).ceil() as i64,
            (cr.max.y * scale).ceil() as i64,
        );
        let mut i = 0;
        while i + 2 < mesh.indices.len() {
            let (a, b, c) = (mesh.indices[i], mesh.indices[i + 1], mesh.indices[i + 2]);
            canvas.fill_tri([vertex(a), vertex(b), vertex(c)], colour(a), clip);
            tris += 1;
            i += 3;
        }
    }
    Raster {
        px: canvas.px,
        tris,
    }
}
