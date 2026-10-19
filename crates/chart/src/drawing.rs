// SokanLab chart — model drawing orisinal. Anchor harga/waktu, hit-test konsisten.
// Meniru *perilaku* alat gambar chart populer (klik-titik, magnet, konstrain sudut),
// bukan kode/aset pihak lain. Semua matematika integer murni kecuali proyeksi layar.

use sokanlab_domain::Bar;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchor {
    pub ts_millis: i64,
    pub price_ticks: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Drawing {
    /// Garis horizontal pada harga tertentu (matematika garis harga publik).
    HorizontalLine {
        price_ticks: i64,
    },
    /// Sinar horizontal: mulai dari satu titik, memanjang ke kanan.
    HorizontalRay {
        origin: Anchor,
    },
    /// Garis vertikal pada waktu tertentu.
    VerticalLine {
        ts_millis: i64,
    },
    /// Crossline: garis horizontal + vertikal melalui satu titik.
    CrossLine {
        anchor: Anchor,
    },
    Trendline {
        a: Anchor,
        b: Anchor,
    },
    Ray {
        origin: Anchor,
        through: Anchor,
    },
    /// Garis penuh dua arah (extended line).
    ExtendedLine {
        a: Anchor,
        b: Anchor,
    },
    /// Segmen dengan mata panah di ujung kedua.
    Arrow {
        a: Anchor,
        b: Anchor,
    },
    Rectangle {
        a: Anchor,
        b: Anchor,
    },
    Ellipse {
        a: Anchor,
        b: Anchor,
    },
    Triangle {
        a: Anchor,
        b: Anchor,
        c: Anchor,
    },
    /// Fibonacci retracement: 7 level dari 0% (a) sampai 100% (b).
    FibRetracement {
        a: Anchor,
        b: Anchor,
    },
    /// Rentang tanggal + harga (mengukur Δt bar, Δharga, Δ%).
    RangeMeasure {
        a: Anchor,
        b: Anchor,
    },
    /// Long/short position: entry (a) dan target (b); stop dicerminkan dari entry.
    Position {
        long: bool,
        entry: Anchor,
        target: Anchor,
    },
    /// Channel: garis dasar a→b + batas kedua melalui c; `kind` memilih varian
    /// (paralel / disjoint / flat top-bottom).
    ParallelChannel {
        a: Anchor,
        b: Anchor,
        c: Anchor,
        kind: ChannelKind,
    },
    /// Pitchfork Andrew: median a→titik tengah (b,c) + dua tine sejajar.
    Pitchfork {
        a: Anchor,
        b: Anchor,
        c: Anchor,
    },
    /// Fibonacci extension: rentang a→b diproyeksikan dari c (0, 61.8, 100, 127.2, 161.8, 261.8%).
    FibExtension {
        a: Anchor,
        b: Anchor,
        c: Anchor,
    },
    /// Fibonacci time: garis vertikal pada bar +1, +2, +3, +5, +8, … dari a.
    FibTime {
        a: Anchor,
    },
    /// Catatan teks pada satu titik (teks bisa diedit dari panel properti).
    Text {
        at: Anchor,
        text: String,
    },
    /// Callout: teks + ekor penunjuk ke titik anchor.
    Callout {
        at: Anchor,
        text: String,
    },
    /// Kanal regresi: regresi linear close pada rentang a–b + batas ±σ.
    RegressionChannel {
        a: Anchor,
        b: Anchor,
    },
    /// Trend-based fib extension: trendline a→b→c + level horizontal dari c
    /// ke kanan (level fib pada sekuens 0/61,8/100/127,2/161,8/261,8%).
    FibTrendExtension {
        a: Anchor,
        b: Anchor,
        c: Anchor,
    },
    /// Elliott impulse wave (1·2·3·4·5): 5 titik, label angka.
    ElliottImpulse {
        points: Vec<Anchor>,
    },
    /// Elliott correction wave (A·B·C): 3 titik, label huruf.
    ElliottCorrection {
        points: Vec<Anchor>,
    },
    /// XABCD pattern: 5 titik, label X/A/B/C/D.
    Xabcd {
        points: Vec<Anchor>,
    },
    /// ABCD pattern: 4 titik, label A/B/C/D.
    Abcd {
        points: Vec<Anchor>,
    },
    /// Gann fan: sembilan sudut 1x8 … 8x1 dari titik a (garis 1x1 = a→b).
    GannFan {
        a: Anchor,
        b: Anchor,
    },
    /// Garis bebas (brush): titik-titik hasil seret.
    Brush {
        points: Vec<Anchor>,
    },
    /// Stabilo (highlighter): seperti brush, garis tebal semi-transparan.
    Highlighter {
        points: Vec<Anchor>,
    },
    /// Sticker: satu ikon vektor pada satu titik.
    Sticker {
        at: Anchor,
        icon: StickerIcon,
    },
}

/// Varian pitchfork (konvensi publik, sama seperti tool "Auto Pitchfork"
/// pada chart populer):
/// - Andrew: median dari A ke titik tengah B–C; tine sejajar lewat B dan C.
/// - Schiff: origin = waktu A, harga rata-rata A–B; median ke titik tengah B–C.
/// - ModifiedSchiff: origin = titik tengah A–B (waktu & harga); median sama.
/// - Inside: median dari titik tengah A–B ke C; tine sejajar lewat A dan B.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PitchforkKind {
    #[default]
    Andrew,
    Schiff,
    ModifiedSchiff,
    Inside,
}

impl PitchforkKind {
    pub fn key(self) -> &'static str {
        match self {
            PitchforkKind::Andrew => "andrew",
            PitchforkKind::Schiff => "schiff",
            PitchforkKind::ModifiedSchiff => "modified_schiff",
            PitchforkKind::Inside => "inside",
        }
    }

    pub fn from_key(k: &str) -> Option<Self> {
        match k {
            "andrew" => Some(PitchforkKind::Andrew),
            "schiff" => Some(PitchforkKind::Schiff),
            "modified_schiff" | "modified-schiff" => Some(PitchforkKind::ModifiedSchiff),
            "inside" => Some(PitchforkKind::Inside),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PitchforkKind::Andrew => "Andrew",
            PitchforkKind::Schiff => "Schiff",
            PitchforkKind::ModifiedSchiff => "Modified Schiff",
            PitchforkKind::Inside => "Inside",
        }
    }

    pub fn all() -> [Self; 4] {
        [
            PitchforkKind::Andrew,
            PitchforkKind::Schiff,
            PitchforkKind::ModifiedSchiff,
            PitchforkKind::Inside,
        ]
    }
}

/// Varian channel (tiga alat channel populer, konvensi publik):
/// - Parallel: batas kedua sejajar batas pertama, rentang waktunya sama.
/// - Disjoint: batas kedua = salinan batas pertama yang digeser, jadi titik
///   awalnya boleh berbeda di sumbu waktu.
/// - Flat: batas kedua horizontal di harga titik c (satu sisi rata).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChannelKind {
    #[default]
    Parallel,
    Disjoint,
    Flat,
}

impl ChannelKind {
    pub fn key(self) -> &'static str {
        match self {
            ChannelKind::Parallel => "parallel",
            ChannelKind::Disjoint => "disjoint",
            ChannelKind::Flat => "flat",
        }
    }

    pub fn from_key(k: &str) -> Option<Self> {
        match k {
            "parallel" => Some(ChannelKind::Parallel),
            "disjoint" => Some(ChannelKind::Disjoint),
            "flat" => Some(ChannelKind::Flat),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ChannelKind::Parallel => "Parallel Channel",
            ChannelKind::Disjoint => "Disjoint Channel",
            ChannelKind::Flat => "Flat Top/Bottom Channel",
        }
    }

    pub fn all() -> [Self; 3] {
        [
            ChannelKind::Parallel,
            ChannelKind::Disjoint,
            ChannelKind::Flat,
        ]
    }
}

/// Perpanjangan garis channel (padanan opsi "Extend" pada alat channel populer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtendMode {
    #[default]
    None,
    Left,
    Right,
    Both,
}

impl ExtendMode {
    pub fn key(self) -> &'static str {
        match self {
            ExtendMode::None => "none",
            ExtendMode::Left => "left",
            ExtendMode::Right => "right",
            ExtendMode::Both => "both",
        }
    }

    pub fn from_key(k: &str) -> Option<Self> {
        match k {
            "none" => Some(ExtendMode::None),
            "left" => Some(ExtendMode::Left),
            "right" => Some(ExtendMode::Right),
            "both" => Some(ExtendMode::Both),
            _ => None,
        }
    }

    pub fn left(self) -> bool {
        matches!(self, ExtendMode::Left | ExtendMode::Both)
    }

    pub fn right(self) -> bool {
        matches!(self, ExtendMode::Right | ExtendMode::Both)
    }

    /// Dari dua centang panel properti (TV memakai dua checkbox serupa).
    pub fn from_checks(left: bool, right: bool) -> Self {
        match (left, right) {
            (true, true) => ExtendMode::Both,
            (true, false) => ExtendMode::Left,
            (false, true) => ExtendMode::Right,
            (false, false) => ExtendMode::None,
        }
    }
}

/// Titik di ruang layar (x, y).
pub type Pt = (f64, f64);

/// Perpanjang pasangan titik mengikuti sumbu waktu (opsi "Extend" alat garis):
/// arah "kanan" = arah yang menaikkan x. Garis hampir vertikal tidak
/// diperpanjang (tidak ada komponen waktu). Dipakai bersama painter + hit-test.
pub fn extend_pair(p0: Pt, p1: Pt, extend: ExtendMode) -> (Pt, Pt) {
    let (dx, dy) = (p1.0 - p0.0, p1.1 - p0.1);
    let len = (dx * dx + dy * dy).sqrt().max(1e-9);
    let (ux, uy) = (dx / len, dy / len);
    let s = if ux >= 0.0 { 1.0 } else { -1.0 };
    let (rx, ry) = (s * ux, s * uy);
    let e = if rx.abs() < 1e-6 { 0.0 } else { EXTEND_PX };
    let (l, r) = if s > 0.0 { (p0, p1) } else { (p1, p0) };
    let el = if extend.left() { e } else { 0.0 };
    let er = if extend.right() { e } else { 0.0 };
    (
        (l.0 - el * rx, l.1 - el * ry),
        (r.0 + er * rx, r.1 + er * ry),
    )
}

/// Rasio Gann fan: (label, pengganda kemiringan terhadap garis 1x1).
/// Konvensi publik: sembilan sudut 1x8 … 8x1 dengan 1x1 sebagai acuan.
pub const GANN_RATIOS: [(&str, f64); 9] = [
    ("1x8", 0.125),
    ("1x4", 0.25),
    ("1x3", 1.0 / 3.0),
    ("1x2", 0.5),
    ("1x1", 1.0),
    ("2x1", 2.0),
    ("3x1", 3.0),
    ("4x1", 4.0),
    ("8x1", 8.0),
];

/// Batas jumlah titik garis bebas (brush/highlighter) — pengaman parser/painter.
pub const MAX_STROKE_POINTS: usize = 4000;

/// Ikon "sticker": ikon vektor bawaan SokanLab (bukan set emoji pihak lain).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StickerIcon {
    #[default]
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    TriangleUp,
    TriangleDown,
    Diamond,
    Circle,
    Square,
    Star,
    Check,
    Cross,
    Plus,
    Flag,
}

impl StickerIcon {
    pub fn key(self) -> &'static str {
        match self {
            StickerIcon::ArrowUp => "arrow_up",
            StickerIcon::ArrowDown => "arrow_down",
            StickerIcon::ArrowLeft => "arrow_left",
            StickerIcon::ArrowRight => "arrow_right",
            StickerIcon::TriangleUp => "triangle_up",
            StickerIcon::TriangleDown => "triangle_down",
            StickerIcon::Diamond => "diamond",
            StickerIcon::Circle => "circle",
            StickerIcon::Square => "square",
            StickerIcon::Star => "star",
            StickerIcon::Check => "check",
            StickerIcon::Cross => "cross",
            StickerIcon::Plus => "plus",
            StickerIcon::Flag => "flag",
        }
    }

    pub fn from_key(k: &str) -> Option<Self> {
        Some(match k {
            "arrow_up" => StickerIcon::ArrowUp,
            "arrow_down" => StickerIcon::ArrowDown,
            "arrow_left" => StickerIcon::ArrowLeft,
            "arrow_right" => StickerIcon::ArrowRight,
            "triangle_up" => StickerIcon::TriangleUp,
            "triangle_down" => StickerIcon::TriangleDown,
            "diamond" => StickerIcon::Diamond,
            "circle" => StickerIcon::Circle,
            "square" => StickerIcon::Square,
            "star" => StickerIcon::Star,
            "check" => StickerIcon::Check,
            "cross" => StickerIcon::Cross,
            "plus" => StickerIcon::Plus,
            "flag" => StickerIcon::Flag,
            _ => return None,
        })
    }

    pub fn label(self) -> &'static str {
        match self {
            StickerIcon::ArrowUp => "Panah atas",
            StickerIcon::ArrowDown => "Panah bawah",
            StickerIcon::ArrowLeft => "Panah kiri",
            StickerIcon::ArrowRight => "Panah kanan",
            StickerIcon::TriangleUp => "Segitiga atas",
            StickerIcon::TriangleDown => "Segitiga bawah",
            StickerIcon::Diamond => "Belah ketupat",
            StickerIcon::Circle => "Lingkaran",
            StickerIcon::Square => "Kotak",
            StickerIcon::Star => "Bintang",
            StickerIcon::Check => "Centang",
            StickerIcon::Cross => "Silang",
            StickerIcon::Plus => "Plus",
            StickerIcon::Flag => "Bendera",
        }
    }

    pub fn all() -> [Self; 14] {
        [
            StickerIcon::ArrowUp,
            StickerIcon::ArrowDown,
            StickerIcon::ArrowLeft,
            StickerIcon::ArrowRight,
            StickerIcon::TriangleUp,
            StickerIcon::TriangleDown,
            StickerIcon::Diamond,
            StickerIcon::Circle,
            StickerIcon::Square,
            StickerIcon::Star,
            StickerIcon::Check,
            StickerIcon::Cross,
            StickerIcon::Plus,
            StickerIcon::Flag,
        ]
    }
}

/// Ukuran font catatan teks (px pada 1× zoom): S=12, M=16, L=22.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextSize {
    S,
    #[default]
    M,
    L,
}

impl TextSize {
    pub fn key(self) -> &'static str {
        match self {
            TextSize::S => "s",
            TextSize::M => "m",
            TextSize::L => "l",
        }
    }

    pub fn from_key(k: &str) -> Option<Self> {
        match k {
            "s" => Some(TextSize::S),
            "m" => Some(TextSize::M),
            "l" => Some(TextSize::L),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            TextSize::S => "S",
            TextSize::M => "M",
            TextSize::L => "L",
        }
    }

    pub fn px(self) -> f32 {
        match self {
            TextSize::S => 12.0,
            TextSize::M => 16.0,
            TextSize::L => 22.0,
        }
    }

    pub fn all() -> [Self; 3] {
        [TextSize::S, TextSize::M, TextSize::L]
    }
}

/// Perataan catatan teks relatif terhadap anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

impl TextAlign {
    pub fn key(self) -> &'static str {
        match self {
            TextAlign::Left => "left",
            TextAlign::Center => "center",
            TextAlign::Right => "right",
        }
    }

    pub fn from_key(k: &str) -> Option<Self> {
        match k {
            "left" => Some(TextAlign::Left),
            "center" => Some(TextAlign::Center),
            "right" => Some(TextAlign::Right),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            TextAlign::Left => "Kiri",
            TextAlign::Center => "Tengah",
            TextAlign::Right => "Kanan",
        }
    }

    pub fn all() -> [Self; 3] {
        [TextAlign::Left, TextAlign::Center, TextAlign::Right]
    }
}

/// Hasil regresi linear close pada rentang bar (untuk kanal regresi).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegressionFit {
    /// Titik garis regresi di bar pertama rentang: (ts, harga ticks).
    pub start: (i64, i64),
    /// Titik garis regresi di bar terakhir rentang: (ts, harga ticks).
    pub end: (i64, i64),
    /// Deviasi standar residual (ticks, ≥ 0) — jarak garis batas ±σ.
    pub sigma: f64,
}

/// Regresi linear least-squares atas close bar dalam rentang [t0, t1].
/// Rentang di luar data memakai bar terdekat; butuh minimal 2 bar.
pub fn regression_fit(bars: &[Bar], t0: i64, t1: i64) -> Option<RegressionFit> {
    if bars.len() < 2 {
        return None;
    }
    let (lo, hi) = (t0.min(t1), t0.max(t1));
    let mut i0 = bars.partition_point(|b| b.ts_millis < lo);
    let mut i1 = bars.partition_point(|b| b.ts_millis <= hi);
    if i0 >= bars.len() - 1 {
        i0 = bars.len() - 2;
    }
    if i1 <= i0 + 1 {
        i1 = (i0 + 2).min(bars.len());
    }
    let n = (i1 - i0) as f64;
    let (mut sx, mut sy, mut sxx, mut sxy) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for (k, b) in bars[i0..i1].iter().enumerate() {
        let x = k as f64;
        let y = b.close_ticks as f64;
        sx += x;
        sy += y;
        sxx += x * x;
        sxy += x * y;
    }
    let denom = n * sxx - sx * sx;
    let (slope, intercept) = if denom.abs() < 1e-9 {
        (0.0, sy / n)
    } else {
        ((n * sxy - sx * sy) / denom, (sy * sxx - sx * sxy) / denom)
    };
    let mut ss = 0.0f64;
    for (k, b) in bars[i0..i1].iter().enumerate() {
        let d = b.close_ticks as f64 - (intercept + slope * k as f64);
        ss += d * d;
    }
    let sigma = (ss / n).sqrt();
    let round = |v: f64| -> i64 {
        if !v.is_finite() {
            return 0;
        }
        let r = if v >= 0.0 {
            (v + 0.5).floor()
        } else {
            (v - 0.5).ceil()
        };
        r.clamp(i64::MIN as f64, i64::MAX as f64) as i64
    };
    Some(RegressionFit {
        start: (bars[i0].ts_millis, round(intercept)),
        end: (bars[i1 - 1].ts_millis, round(intercept + slope * (n - 1.0))),
        sigma,
    })
}

/// Tiga garis kanal regresi (tengah, atas +σ, bawah −σ) di ruang layar,
/// sudah termasuk perpanjangan `extend`. Offset σ dihitung per titik ujung
/// agar tetap benar pada skala log.
pub fn regression_segments<P: Projector>(
    fit: &RegressionFit,
    p: &P,
    extend: ExtendMode,
) -> [((f64, f64), (f64, f64)); 3] {
    let sigma = fit.sigma.round().clamp(0.0, 1.0e12) as i64;
    let mk = |d: i64| -> ((f64, f64), (f64, f64)) {
        let a = (p.x(fit.start.0), p.y(fit.start.1.saturating_add(d)));
        let b = (p.x(fit.end.0), p.y(fit.end.1.saturating_add(d)));
        extend_pair(a, b, extend)
    };
    [mk(0), mk(sigma), mk(-sigma)]
}

/// Sembilan garis Gann fan di ruang layar: (label, titik asal, titik jauh).
/// Garis 1x1 = arah a→b; rasio lain mengubah komponen harga (kemiringan).
pub fn gann_fan_lines(a: Pt, b: Pt) -> [(&'static str, Pt, Pt); 9] {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    GANN_RATIOS.map(|(label, r)| {
        let d = (dx, dy * r);
        let len = (d.0 * d.0 + d.1 * d.1).sqrt().max(1e-9);
        let u = (d.0 / len, d.1 / len);
        (label, a, (a.0 + u.0 * EXTEND_PX, a.1 + u.1 * EXTEND_PX))
    })
}

/// Panjang perpanjangan garis panjang (px). Dipakai bersama painter & hit-test
/// supaya "yang terlihat" = "yang bisa diklik"; nilai besar menggantikan
/// pemotongan di tepi plot.
pub const EXTEND_PX: f64 = 20_000.0;

/// Dua segmen batas channel di ruang layar: batas 0 (garis dasar a→b) dan
/// batas 1 (tergantung varian). Segmen sudah termasuk perpanjangan `extend`.
/// Dipakai bersama painter + hit-test agar keduanya konsisten.
pub fn channel_segments(
    kind: ChannelKind,
    extend: ExtendMode,
    a: (f64, f64),
    b: (f64, f64),
    c: (f64, f64),
) -> [((f64, f64), (f64, f64)); 2] {
    // Perpanjangan & arah: dipakai bersama helper `extend_pair` supaya
    // semantiknya identik dengan alat garis lain.
    let (p0, p1) = extend_pair(a, b, extend);
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dy * dy).sqrt().max(1e-9);
    let (ux, uy) = (dx / len, dy / len);
    let s = if ux >= 0.0 { 1.0 } else { -1.0 };
    let (rx, ry) = (s * ux, s * uy);
    let batas1 = match kind {
        // Batas kedua sejajar; rentang waktunya sama dengan batas pertama.
        ChannelKind::Parallel => {
            if rx.abs() < 1e-6 {
                // Dasar vertikal: batas kedua vertikal di c, rentang y sama.
                ((c.0, p0.1), (c.0, p1.1))
            } else {
                let slope = ry / rx;
                let y_at = |x: f64| c.1 + (x - c.0) * slope;
                ((p0.0, y_at(p0.0)), (p1.0, y_at(p1.0)))
            }
        }
        // Batas kedua = salinan garis dasar yang digeser (c − awal dasar):
        // titik awalnya boleh berbeda di sumbu waktu.
        ChannelKind::Disjoint => (
            (p0.0 + c.0 - a.0, p0.1 + c.1 - a.1),
            (p1.0 + c.0 - a.0, p1.1 + c.1 - a.1),
        ),
        // Batas kedua horizontal di harga c, sepanjang rentang batas pertama.
        ChannelKind::Flat => ((p0.0, c.1), (p1.0, c.1)),
    };
    [(p0, p1), batas1]
}

/// Satu level fibonacci kustom (per-mil). `color` None = warna otomatis level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FibLevel {
    pub milli: i32,
    pub color: Option<[u8; 3]>,
}

/// Derajat yang bisa dipakai key (menghindari `f64: Eq` yang tidak ada).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Degree {
    Horizontal,
    HorizontalRay,
    Vertical,
    Cross,
    Trendline,
    Ray,
    ExtendedLine,
    Arrow,
    Rectangle,
    Ellipse,
    Triangle,
    Fib,
    Range,
    Position,
    Channel,
    Pitchfork,
    FibExtension,
    FibTime,
    Text,
    Callout,
    Pattern,
    Gann,
    Stroke,
    Sticker,
}

/// Level Fibonacci (dalam per-mil) yang digambar.
pub const FIB_LEVELS_MILLI: [i32; 7] = [0, 236, 382, 500, 618, 786, 1000];

/// Level Fibonacci extension (per-mil): rentang a→b diproyeksikan dari c.
pub const FIB_EXT_MILLI: [i32; 6] = [0, 618, 1000, 1272, 1618, 2618];

/// Urutan bar Fibonacci untuk Fib Time.
pub const FIB_TIME_BARS: [usize; 11] = [1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144];

/// Harga level fib ke-`milli` dari a→b. Integer murni, aman overflow.
pub fn fib_level_price(a_ticks: i64, b_ticks: i64, milli: i32) -> i64 {
    let num = (b_ticks as i128 - a_ticks as i128) * milli as i128;
    let rounded = if num >= 0 {
        (num + 500) / 1000
    } else {
        (num - 500) / 1000
    };
    let out = a_ticks as i128 + rounded;
    out.clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

/// Harga level fib extension ke-`milli`: c + (b − a)·milli/1000. Integer aman.
pub fn fib_ext_price(a_ticks: i64, b_ticks: i64, c_ticks: i64, milli: i32) -> i64 {
    let base = b_ticks as i128 - a_ticks as i128;
    let num = base * milli as i128;
    let rounded = if num >= 0 {
        (num + 500) / 1000
    } else {
        (num - 500) / 1000
    };
    let out = c_ticks as i128 + rounded;
    out.clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

/// Waktu bar untuk Fib Time: bar ke-+k (1,2,3,5,8,…) setelah bar anchor.
/// Bar di luar dataset dilewati (bukan panic).
pub fn fib_time_ts(anchor_ts: i64, bars: &[Bar]) -> Vec<(usize, i64)> {
    let at = bars.partition_point(|b| b.ts_millis < anchor_ts);
    let base = if at < bars.len() && bars[at].ts_millis == anchor_ts {
        at
    } else if at == 0 {
        0
    } else {
        at - 1
    };
    FIB_TIME_BARS
        .iter()
        .filter_map(|k| bars.get(base + k).map(|b| (*k, b.ts_millis)))
        .collect()
}

/// Level target + stop untuk Position: stop dicerminkan dari entry.
pub fn position_levels(entry_ticks: i64, target_ticks: i64) -> (i64, i64) {
    let stop = entry_ticks as i128 - (target_ticks as i128 - entry_ticks as i128);
    (
        target_ticks,
        stop.clamp(i64::MIN as i128, i64::MAX as i128) as i64,
    )
}

impl Drawing {
    pub fn degree(&self) -> Degree {
        match self {
            Self::HorizontalLine { .. } => Degree::Horizontal,
            Self::HorizontalRay { .. } => Degree::HorizontalRay,
            Self::VerticalLine { .. } => Degree::Vertical,
            Self::CrossLine { .. } => Degree::Cross,
            Self::Trendline { .. } => Degree::Trendline,
            Self::Ray { .. } => Degree::Ray,
            Self::ExtendedLine { .. } => Degree::ExtendedLine,
            Self::Arrow { .. } => Degree::Arrow,
            Self::Rectangle { .. } => Degree::Rectangle,
            Self::Ellipse { .. } => Degree::Ellipse,
            Self::Triangle { .. } => Degree::Triangle,
            Self::FibRetracement { .. } => Degree::Fib,
            Self::RangeMeasure { .. } => Degree::Range,
            Self::Position { .. } => Degree::Position,
            Self::ParallelChannel { .. } => Degree::Channel,
            Self::RegressionChannel { .. } => Degree::Channel,
            Self::Pitchfork { .. } => Degree::Pitchfork,
            Self::FibExtension { .. } | Self::FibTrendExtension { .. } => Degree::FibExtension,
            Self::FibTime { .. } => Degree::FibTime,
            Self::ElliottImpulse { .. }
            | Self::ElliottCorrection { .. }
            | Self::Xabcd { .. }
            | Self::Abcd { .. } => Degree::Pattern,
            Self::GannFan { .. } => Degree::Gann,
            Self::Brush { .. } | Self::Highlighter { .. } => Degree::Stroke,
            Self::Sticker { .. } => Degree::Sticker,
            Self::Text { .. } => Degree::Text,
            Self::Callout { .. } => Degree::Callout,
        }
    }

    /// Label manusia untuk daftar objek/tooltip.
    pub fn kind_label(&self) -> &'static str {
        match self {
            Self::HorizontalLine { .. } => "Horizontal Line",
            Self::HorizontalRay { .. } => "Horizontal Ray",
            Self::VerticalLine { .. } => "Vertical Line",
            Self::CrossLine { .. } => "Crossline",
            Self::Trendline { .. } => "Trend Line",
            Self::Ray { .. } => "Ray",
            Self::ExtendedLine { .. } => "Extended Line",
            Self::Arrow { .. } => "Arrow",
            Self::Rectangle { .. } => "Rectangle",
            Self::Ellipse { .. } => "Ellipse",
            Self::Triangle { .. } => "Triangle",
            Self::FibRetracement { .. } => "Fib Retracement",
            Self::RangeMeasure { .. } => "Date & Price Range",
            Self::Position { long: true, .. } => "Long Position",
            Self::Position { long: false, .. } => "Short Position",
            Self::ParallelChannel { kind, .. } => kind.label(),
            Self::Pitchfork { .. } => "Pitchfork",
            Self::FibExtension { .. } => "Fib Extension",
            Self::FibTime { .. } => "Fib Time",
            Self::Text { .. } => "Text",
            Self::Callout { .. } => "Callout",
            Self::RegressionChannel { .. } => "Regression Channel",
            Self::FibTrendExtension { .. } => "Trend-Based Fib Extension",
            Self::ElliottImpulse { .. } => "Elliott Impulse Wave",
            Self::ElliottCorrection { .. } => "Elliott Correction Wave",
            Self::Xabcd { .. } => "XABCD Pattern",
            Self::Abcd { .. } => "ABCD Pattern",
            Self::GannFan { .. } => "Gann Fan",
            Self::Brush { .. } => "Brush",
            Self::Highlighter { .. } => "Highlighter",
            Self::Sticker { .. } => "Sticker",
        }
    }

    /// Label titik pola (Elliott/XABCD/ABCD) untuk painter.
    pub fn pattern_labels(&self) -> Option<&'static [&'static str]> {
        match self {
            Self::ElliottImpulse { .. } => Some(&["1", "2", "3", "4", "5"]),
            Self::ElliottCorrection { .. } => Some(&["A", "B", "C"]),
            Self::Xabcd { .. } => Some(&["X", "A", "B", "C", "D"]),
            Self::Abcd { .. } => Some(&["A", "B", "C", "D"]),
            _ => None,
        }
    }

    /// Anchor yang bisa di-drag (titik edit). Beberapa alat memakai anchor sintetis.
    pub fn anchors(&self) -> Vec<Anchor> {
        match self {
            Self::HorizontalLine { price_ticks } => vec![Anchor {
                ts_millis: 0,
                price_ticks: *price_ticks,
            }],
            Self::HorizontalRay { origin } => vec![*origin],
            Self::VerticalLine { ts_millis } => vec![Anchor {
                ts_millis: *ts_millis,
                price_ticks: 0,
            }],
            Self::CrossLine { anchor } => vec![*anchor],
            Self::Trendline { a, b }
            | Self::ExtendedLine { a, b }
            | Self::Arrow { a, b }
            | Self::Rectangle { a, b }
            | Self::Ellipse { a, b }
            | Self::FibRetracement { a, b }
            | Self::RangeMeasure { a, b } => vec![*a, *b],
            Self::Ray { origin, through } => vec![*origin, *through],
            Self::Triangle { a, b, c } => vec![*a, *b, *c],
            Self::Position { entry, target, .. } => vec![*entry, *target],
            Self::ParallelChannel { a, b, c, .. }
            | Self::Pitchfork { a, b, c }
            | Self::FibExtension { a, b, c } => vec![*a, *b, *c],
            Self::FibTime { a } => vec![*a],
            Self::RegressionChannel { a, b } | Self::GannFan { a, b } => vec![*a, *b],
            Self::FibTrendExtension { a, b, c } => vec![*a, *b, *c],
            Self::ElliottImpulse { points }
            | Self::ElliottCorrection { points }
            | Self::Xabcd { points }
            | Self::Abcd { points }
            | Self::Brush { points }
            | Self::Highlighter { points } => points.clone(),
            Self::Sticker { at, .. } => vec![*at],
            Self::Text { at, .. } | Self::Callout { at, .. } => vec![*at],
        }
    }

    /// Ganti satu anchor (drag handle). `false` bila indeks di luar rentang.
    pub fn set_anchor(&mut self, idx: usize, a: Anchor) -> bool {
        match self {
            Self::HorizontalLine { price_ticks } if idx == 0 => {
                *price_ticks = a.price_ticks;
                true
            }
            Self::HorizontalRay { origin } if idx == 0 => {
                *origin = a;
                true
            }
            Self::VerticalLine { ts_millis } if idx == 0 => {
                *ts_millis = a.ts_millis;
                true
            }
            Self::CrossLine { anchor } if idx == 0 => {
                *anchor = a;
                true
            }
            Self::Trendline { a: x, b: y }
            | Self::ExtendedLine { a: x, b: y }
            | Self::Arrow { a: x, b: y }
            | Self::Rectangle { a: x, b: y }
            | Self::Ellipse { a: x, b: y }
            | Self::FibRetracement { a: x, b: y }
            | Self::RangeMeasure { a: x, b: y } => match idx {
                0 => {
                    *x = a;
                    true
                }
                1 => {
                    *y = a;
                    true
                }
                _ => false,
            },
            Self::Ray { origin, through } => match idx {
                0 => {
                    *origin = a;
                    true
                }
                1 => {
                    *through = a;
                    true
                }
                _ => false,
            },
            Self::Triangle { a: x, b: y, c: z } => match idx {
                0 => {
                    *x = a;
                    true
                }
                1 => {
                    *y = a;
                    true
                }
                2 => {
                    *z = a;
                    true
                }
                _ => false,
            },
            Self::Position { entry, target, .. } => match idx {
                0 => {
                    *entry = a;
                    true
                }
                1 => {
                    *target = a;
                    true
                }
                _ => false,
            },
            Self::ParallelChannel {
                a: x, b: y, c: z, ..
            }
            | Self::Pitchfork { a: x, b: y, c: z }
            | Self::FibExtension { a: x, b: y, c: z } => match idx {
                0 => {
                    *x = a;
                    true
                }
                1 => {
                    *y = a;
                    true
                }
                2 => {
                    *z = a;
                    true
                }
                _ => false,
            },
            Self::FibTime { a: x } if idx == 0 => {
                *x = a;
                true
            }
            Self::RegressionChannel { a: x, b: y } | Self::GannFan { a: x, b: y } => match idx {
                0 => {
                    *x = a;
                    true
                }
                1 => {
                    *y = a;
                    true
                }
                _ => false,
            },
            Self::FibTrendExtension { a: x, b: y, c: z } => match idx {
                0 => {
                    *x = a;
                    true
                }
                1 => {
                    *y = a;
                    true
                }
                2 => {
                    *z = a;
                    true
                }
                _ => false,
            },
            Self::ElliottImpulse { points }
            | Self::ElliottCorrection { points }
            | Self::Xabcd { points }
            | Self::Abcd { points }
            | Self::Brush { points }
            | Self::Highlighter { points } => {
                if idx < points.len() {
                    points[idx] = a;
                    true
                } else {
                    false
                }
            }
            Self::Sticker { at, .. } if idx == 0 => {
                *at = a;
                true
            }
            Self::Text { at, .. } | Self::Callout { at, .. } if idx == 0 => {
                *at = a;
                true
            }
            _ => false,
        }
    }

    /// Geser seluruh objek (drag body) sebesar Δt/Δharga.
    pub fn translated(&self, dts: i64, dp: i64) -> Drawing {
        let sh = |a: &Anchor| Anchor {
            ts_millis: a.ts_millis.saturating_add(dts),
            price_ticks: a.price_ticks.saturating_add(dp),
        };
        // Clone sekali agar varian ber-String bisa dicocokkan tanpa Copy.
        match self.clone() {
            Self::HorizontalLine { price_ticks } => Self::HorizontalLine {
                price_ticks: price_ticks.saturating_add(dp),
            },
            Self::HorizontalRay { origin } => Self::HorizontalRay {
                origin: sh(&origin),
            },
            Self::VerticalLine { ts_millis } => Self::VerticalLine {
                ts_millis: ts_millis.saturating_add(dts),
            },
            Self::CrossLine { anchor } => Self::CrossLine {
                anchor: sh(&anchor),
            },
            Self::Trendline { a, b } => Self::Trendline {
                a: sh(&a),
                b: sh(&b),
            },
            Self::Ray { origin, through } => Self::Ray {
                origin: sh(&origin),
                through: sh(&through),
            },
            Self::ExtendedLine { a, b } => Self::ExtendedLine {
                a: sh(&a),
                b: sh(&b),
            },
            Self::Arrow { a, b } => Self::Arrow {
                a: sh(&a),
                b: sh(&b),
            },
            Self::Rectangle { a, b } => Self::Rectangle {
                a: sh(&a),
                b: sh(&b),
            },
            Self::Ellipse { a, b } => Self::Ellipse {
                a: sh(&a),
                b: sh(&b),
            },
            Self::Triangle { a, b, c } => Self::Triangle {
                a: sh(&a),
                b: sh(&b),
                c: sh(&c),
            },
            Self::FibRetracement { a, b } => Self::FibRetracement {
                a: sh(&a),
                b: sh(&b),
            },
            Self::RangeMeasure { a, b } => Self::RangeMeasure {
                a: sh(&a),
                b: sh(&b),
            },
            Self::Position {
                long,
                entry,
                target,
            } => Self::Position {
                long,
                entry: sh(&entry),
                target: sh(&target),
            },
            Self::ParallelChannel { a, b, c, kind } => Self::ParallelChannel {
                a: sh(&a),
                b: sh(&b),
                c: sh(&c),
                kind,
            },
            Self::Pitchfork { a, b, c } => Self::Pitchfork {
                a: sh(&a),
                b: sh(&b),
                c: sh(&c),
            },
            Self::FibExtension { a, b, c } => Self::FibExtension {
                a: sh(&a),
                b: sh(&b),
                c: sh(&c),
            },
            Self::FibTime { a } => Self::FibTime { a: sh(&a) },
            Self::RegressionChannel { a, b } => Self::RegressionChannel {
                a: sh(&a),
                b: sh(&b),
            },
            Self::FibTrendExtension { a, b, c } => Self::FibTrendExtension {
                a: sh(&a),
                b: sh(&b),
                c: sh(&c),
            },
            Self::ElliottImpulse { points } => Self::ElliottImpulse {
                points: points.iter().map(&sh).collect(),
            },
            Self::ElliottCorrection { points } => Self::ElliottCorrection {
                points: points.iter().map(&sh).collect(),
            },
            Self::Xabcd { points } => Self::Xabcd {
                points: points.iter().map(&sh).collect(),
            },
            Self::Abcd { points } => Self::Abcd {
                points: points.iter().map(&sh).collect(),
            },
            Self::GannFan { a, b } => Self::GannFan {
                a: sh(&a),
                b: sh(&b),
            },
            Self::Brush { points } => Self::Brush {
                points: points.iter().map(&sh).collect(),
            },
            Self::Highlighter { points } => Self::Highlighter {
                points: points.iter().map(&sh).collect(),
            },
            Self::Sticker { at, icon } => Self::Sticker { at: sh(&at), icon },
            Self::Text { at, text } => Self::Text {
                at: sh(&at),
                text: text.clone(),
            },
            Self::Callout { at, text } => Self::Callout {
                at: sh(&at),
                text: text.clone(),
            },
        }
    }
}

/// Pemetaan koordinat layar (dipakai UI dan hit-test agar konsisten).
pub trait Projector {
    fn x(&self, ts_millis: i64) -> f64;
    fn y(&self, price_ticks: i64) -> f64;
}

pub fn hit_test<P: Projector>(
    drawing: &Drawing,
    bars: &[Bar],
    p: &P,
    mouse: (f64, f64),
    tol_px: f64,
) -> bool {
    match drawing {
        Drawing::HorizontalLine { price_ticks } => (p.y(*price_ticks) - mouse.1).abs() <= tol_px,
        Drawing::HorizontalRay { origin } => {
            (p.y(origin.price_ticks) - mouse.1).abs() <= tol_px
                && mouse.0 >= p.x(origin.ts_millis) - tol_px
        }
        Drawing::VerticalLine { ts_millis } => (p.x(*ts_millis) - mouse.0).abs() <= tol_px,
        Drawing::CrossLine { anchor } => {
            (p.y(anchor.price_ticks) - mouse.1).abs() <= tol_px
                || (p.x(anchor.ts_millis) - mouse.0).abs() <= tol_px
        }
        Drawing::Trendline { a, b } | Drawing::Arrow { a, b } => {
            seg_dist(
                mouse,
                (p.x(a.ts_millis), p.y(a.price_ticks)),
                (p.x(b.ts_millis), p.y(b.price_ticks)),
            ) <= tol_px
        }
        Drawing::Ray { origin, through } => {
            let (x0, y0) = (p.x(origin.ts_millis), p.y(origin.price_ticks));
            let (x1, y1) = (p.x(through.ts_millis), p.y(through.price_ticks));
            ray_dist(mouse, (x0, y0), (x1, y1)) <= tol_px
        }
        Drawing::ExtendedLine { a, b } => {
            inf_line_dist(
                mouse,
                (p.x(a.ts_millis), p.y(a.price_ticks)),
                (p.x(b.ts_millis), p.y(b.price_ticks)),
            ) <= tol_px
        }
        Drawing::Rectangle { a, b } => {
            let (x0, y0) = (p.x(a.ts_millis), p.y(a.price_ticks));
            let (x1, y1) = (p.x(b.ts_millis), p.y(b.price_ticks));
            rect_edge_dist(mouse, x0, y0, x1, y1, tol_px)
        }
        Drawing::Ellipse { a, b } => {
            let (x0, y0) = (p.x(a.ts_millis), p.y(a.price_ticks));
            let (x1, y1) = (p.x(b.ts_millis), p.y(b.price_ticks));
            ellipse_edge_dist(mouse, x0, y0, x1, y1, tol_px)
        }
        Drawing::Triangle { a, b, c } => {
            let pa = (p.x(a.ts_millis), p.y(a.price_ticks));
            let pb = (p.x(b.ts_millis), p.y(b.price_ticks));
            let pc = (p.x(c.ts_millis), p.y(c.price_ticks));
            seg_dist(mouse, pa, pb) <= tol_px
                || seg_dist(mouse, pb, pc) <= tol_px
                || seg_dist(mouse, pc, pa) <= tol_px
        }
        Drawing::FibRetracement { a, b } => {
            let near_seg = seg_dist(
                mouse,
                (p.x(a.ts_millis), p.y(a.price_ticks)),
                (p.x(b.ts_millis), p.y(b.price_ticks)),
            ) <= tol_px;
            near_seg
                || FIB_LEVELS_MILLI.iter().any(|m| {
                    let price = fib_level_price(a.price_ticks, b.price_ticks, *m);
                    (p.y(price) - mouse.1).abs() <= tol_px
                })
        }
        Drawing::RangeMeasure { a, b } => {
            let (x0, y0) = (p.x(a.ts_millis), p.y(a.price_ticks));
            let (x1, y1) = (p.x(b.ts_millis), p.y(b.price_ticks));
            rect_edge_dist(mouse, x0, y0, x1, y1, tol_px)
                || seg_dist(mouse, (x0, y0), (x1, y1)) <= tol_px
        }
        Drawing::Position { entry, target, .. } => {
            let (x0, y0) = (p.x(entry.ts_millis), p.y(entry.price_ticks));
            let (x1, y1) = (p.x(target.ts_millis), p.y(target.price_ticks));
            let (_, stop) = position_levels(entry.price_ticks, target.price_ticks);
            let ys = p.y(stop);
            rect_edge_dist(mouse, x0, y0, x1, y1, tol_px)
                || (mouse.0 - x0).abs() <= tol_px
                || (mouse.0 - x1).abs() <= tol_px
                || ((mouse.1 - ys).abs() <= tol_px && span_contains(mouse.0, x0, x1))
        }
        Drawing::ParallelChannel { a, b, c, kind } => {
            let pa = (p.x(a.ts_millis), p.y(a.price_ticks));
            let pb = (p.x(b.ts_millis), p.y(b.price_ticks));
            let pc = (p.x(c.ts_millis), p.y(c.price_ticks));
            // Batas channel diuji sebagai segmen yang benar-benar digambar
            // (perpanjangan default: tanpa perpanjangan; versi item memakai
            // `extend` objek lewat `hit_test_item`).
            channel_segments(*kind, ExtendMode::None, pa, pb, pc)
                .iter()
                .any(|(s0, s1)| seg_dist(mouse, *s0, *s1) <= tol_px)
        }
        Drawing::Pitchfork { a, b, c } => {
            let pa = (p.x(a.ts_millis), p.y(a.price_ticks));
            let pb = (p.x(b.ts_millis), p.y(b.price_ticks));
            let pc = (p.x(c.ts_millis), p.y(c.price_ticks));
            let mid = ((pb.0 + pc.0) / 2.0, (pb.1 + pc.1) / 2.0);
            let (dx, dy) = (mid.0 - pa.0, mid.1 - pa.1);
            let tine = |o: (f64, f64)| inf_line_dist(mouse, o, (o.0 + dx, o.1 + dy)) <= tol_px;
            inf_line_dist(mouse, pa, mid) <= tol_px || tine(pb) || tine(pc)
        }
        Drawing::FibExtension { a, b, c } => {
            let pa = (p.x(a.ts_millis), p.y(a.price_ticks));
            let pb = (p.x(b.ts_millis), p.y(b.price_ticks));
            let pc = (p.x(c.ts_millis), p.y(c.price_ticks));
            seg_dist(mouse, pa, pb) <= tol_px
                || seg_dist(mouse, pb, pc) <= tol_px
                || FIB_EXT_MILLI.iter().any(|m| {
                    let price = fib_ext_price(a.price_ticks, b.price_ticks, c.price_ticks, *m);
                    (p.y(price) - mouse.1).abs() <= tol_px
                })
        }
        Drawing::FibTime { a } => fib_time_ts(a.ts_millis, bars)
            .iter()
            .any(|(_, ts)| (p.x(*ts) - mouse.0).abs() <= tol_px),
        Drawing::RegressionChannel { a, b } => match regression_fit(bars, a.ts_millis, b.ts_millis)
        {
            Some(fit) => regression_segments(&fit, p, ExtendMode::None)
                .iter()
                .any(|(s0, s1)| seg_dist(mouse, *s0, *s1) <= tol_px),
            None => false,
        },
        Drawing::FibTrendExtension { a, b, c } => {
            let pa = (p.x(a.ts_millis), p.y(a.price_ticks));
            let pb = (p.x(b.ts_millis), p.y(b.price_ticks));
            let pc = (p.x(c.ts_millis), p.y(c.price_ticks));
            // Level horizontal dari x(c) ke kanan.
            seg_dist(mouse, pa, pb) <= tol_px
                || seg_dist(mouse, pb, pc) <= tol_px
                || FIB_EXT_MILLI.iter().any(|m| {
                    let price = fib_ext_price(a.price_ticks, b.price_ticks, c.price_ticks, *m);
                    (p.y(price) - mouse.1).abs() <= tol_px && mouse.0 >= pc.0 - tol_px
                })
        }
        Drawing::ElliottImpulse { points }
        | Drawing::ElliottCorrection { points }
        | Drawing::Xabcd { points }
        | Drawing::Abcd { points }
        | Drawing::Brush { points }
        | Drawing::Highlighter { points } => points.windows(2).any(|w| {
            seg_dist(
                mouse,
                (p.x(w[0].ts_millis), p.y(w[0].price_ticks)),
                (p.x(w[1].ts_millis), p.y(w[1].price_ticks)),
            ) <= tol_px
        }),
        Drawing::GannFan { a, b } => {
            let pa = (p.x(a.ts_millis), p.y(a.price_ticks));
            let pb = (p.x(b.ts_millis), p.y(b.price_ticks));
            gann_fan_lines(pa, pb)
                .iter()
                .any(|(_, o, far)| ray_dist(mouse, *o, *far) <= tol_px)
        }
        Drawing::Sticker { at, .. } => {
            let c = (p.x(at.ts_millis), p.y(at.price_ticks));
            ((mouse.0 - c.0).powi(2) + (mouse.1 - c.1).powi(2)).sqrt() <= 13.0 + tol_px
        }
        // Teks/callout: kotak perkiraan di kanan-atas anchor (ukuran font tetap).
        Drawing::Text { at, .. } | Drawing::Callout { at, .. } => {
            let x = p.x(at.ts_millis);
            let y = p.y(at.price_ticks);
            (mouse.0 - x).abs() <= 70.0 && (mouse.1 - (y - 8.0)).abs() <= 20.0
        }
    }
}

fn span_contains(v: f64, x0: f64, x1: f64) -> bool {
    v >= x0.min(x1) && v <= x0.max(x1)
}

fn rect_edge_dist(m: (f64, f64), x0: f64, y0: f64, x1: f64, y1: f64, tol: f64) -> bool {
    let (l, r) = (x0.min(x1), x0.max(x1));
    let (t, b) = (y0.min(y1), y0.max(y1));
    let inside = m.0 >= l - tol && m.0 <= r + tol && m.1 >= t - tol && m.1 <= b + tol;
    inside
        && ((m.0 - l).abs() <= tol
            || (r - m.0).abs() <= tol
            || (m.1 - t).abs() <= tol
            || (b - m.1).abs() <= tol)
}

fn ellipse_edge_dist(m: (f64, f64), x0: f64, y0: f64, x1: f64, y1: f64, tol: f64) -> bool {
    let cx = (x0 + x1) / 2.0;
    let cy = (y0 + y1) / 2.0;
    let rx = (x1 - x0).abs() / 2.0;
    let ry = (y1 - y0).abs() / 2.0;
    if rx < 0.5 || ry < 0.5 {
        return ((m.0 - cx).powi(2) + (m.1 - cy).powi(2)).sqrt() <= tol.max(rx.max(ry));
    }
    let nx = (m.0 - cx) / rx;
    let ny = (m.1 - cy) / ry;
    let k = (nx * nx + ny * ny).sqrt();
    // Perkiraan jarak ke kurva (cukup untuk toleransi klik).
    (k - 1.0).abs() * rx.min(ry) <= tol
}

fn seg_dist(m: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (vx, vy) = (b.0 - a.0, b.1 - a.1);
    let len2 = vx * vx + vy * vy;
    if len2 <= f64::EPSILON {
        return ((m.0 - a.0).powi(2) + (m.1 - a.1).powi(2)).sqrt();
    }
    let t = (((m.0 - a.0) * vx + (m.1 - a.1) * vy) / len2).clamp(0.0, 1.0);
    let cx = a.0 + t * vx;
    let cy = a.1 + t * vy;
    ((m.0 - cx).powi(2) + (m.1 - cy).powi(2)).sqrt()
}

/// Jarak ke garis penuh (ray = segmen dari origin ke titik mana pun di depan through).
fn ray_dist(m: (f64, f64), o: (f64, f64), through: (f64, f64)) -> f64 {
    let (dx, dy) = (through.0 - o.0, through.1 - o.1);
    let len = (dx * dx + dy * dy).sqrt();
    if len <= f64::EPSILON {
        return ((m.0 - o.0).powi(2) + (m.1 - o.1).powi(2)).sqrt();
    }
    let (ux, uy) = (dx / len, dy / len);
    let along = (m.0 - o.0) * ux + (m.1 - o.1) * uy;
    if along >= 0.0 {
        let px = o.0 + along * ux;
        let py = o.1 + along * uy;
        ((m.0 - px).powi(2) + (m.1 - py).powi(2)).sqrt()
    } else {
        ((m.0 - o.0).powi(2) + (m.1 - o.1).powi(2)).sqrt()
    }
}

/// Jarak ke garis tak berujung (extended line).
fn inf_line_dist(m: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (vx, vy) = (b.0 - a.0, b.1 - a.1);
    let len = (vx * vx + vy * vy).sqrt();
    if len <= f64::EPSILON {
        return ((m.0 - a.0).powi(2) + (m.1 - a.1).powi(2)).sqrt();
    }
    ((m.0 - a.0) * vy - (m.1 - a.1) * vx).abs() / len
}

// ---------------------------------------------------------------------------
// Geometri pitchfork (dipakai painter + hit-test agar keduanya konsisten).
// ---------------------------------------------------------------------------

/// Tiga garis pitchfork di ruang layar: median + dua tine sejajar + segmen bantu.
#[derive(Debug, Clone, PartialEq)]
pub struct ForkLines {
    /// Titik awal median.
    pub median_from: (f64, f64),
    /// Titik awal tine pertama.
    pub tine1_from: (f64, f64),
    /// Titik awal tine kedua.
    pub tine2_from: (f64, f64),
    /// Arah median (bukan unit) — dipakai semua garis.
    pub dir: (f64, f64),
    /// Segmen bantu putus-putus: B–C untuk varian klasik, A–B untuk Inside.
    pub helper: ((f64, f64), (f64, f64)),
}

/// Hitung garis pitchfork untuk varian tertentu. Input titik layar (x, y).
pub fn pitchfork_lines(
    kind: PitchforkKind,
    a: (f64, f64),
    b: (f64, f64),
    c: (f64, f64),
) -> ForkLines {
    let mid_bc = ((b.0 + c.0) / 2.0, (b.1 + c.1) / 2.0);
    let mid_ab = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    match kind {
        PitchforkKind::Andrew => ForkLines {
            median_from: a,
            tine1_from: b,
            tine2_from: c,
            dir: (mid_bc.0 - a.0, mid_bc.1 - a.1),
            helper: (b, c),
        },
        PitchforkKind::Schiff => {
            // Origin: waktu A, harga rata-rata A–B.
            let o = (a.0, (a.1 + b.1) / 2.0);
            ForkLines {
                median_from: o,
                tine1_from: b,
                tine2_from: c,
                dir: (mid_bc.0 - o.0, mid_bc.1 - o.1),
                helper: (b, c),
            }
        }
        PitchforkKind::ModifiedSchiff => ForkLines {
            median_from: mid_ab,
            tine1_from: b,
            tine2_from: c,
            dir: (mid_bc.0 - mid_ab.0, mid_bc.1 - mid_ab.1),
            helper: (b, c),
        },
        PitchforkKind::Inside => ForkLines {
            // Median: titik tengah A–B → C; tine lewat A dan B.
            median_from: mid_ab,
            tine1_from: a,
            tine2_from: b,
            dir: (c.0 - mid_ab.0, c.1 - mid_ab.1),
            helper: (a, b),
        },
    }
}

/// Hit-test versi objek: menghormati varian pitchfork, varian channel +
/// perpanjangannya, dan level fib kustom. Kasus lain sama dengan `hit_test`
/// pada geometri drawing-nya.
pub fn hit_test_item<P: Projector>(
    it: &DrawingItem,
    bars: &[Bar],
    p: &P,
    mouse: (f64, f64),
    tol_px: f64,
) -> bool {
    match &it.drawing {
        Drawing::ParallelChannel { a, b, c, kind } => {
            let pa = (p.x(a.ts_millis), p.y(a.price_ticks));
            let pb = (p.x(b.ts_millis), p.y(b.price_ticks));
            let pc = (p.x(c.ts_millis), p.y(c.price_ticks));
            channel_segments(*kind, it.extend, pa, pb, pc)
                .iter()
                .any(|(s0, s1)| seg_dist(mouse, *s0, *s1) <= tol_px)
        }
        Drawing::Pitchfork { a, b, c } => {
            let pa = (p.x(a.ts_millis), p.y(a.price_ticks));
            let pb = (p.x(b.ts_millis), p.y(b.price_ticks));
            let pc = (p.x(c.ts_millis), p.y(c.price_ticks));
            let f = pitchfork_lines(it.pitchfork, pa, pb, pc);
            let d = f.dir;
            let hit = |o: (f64, f64)| inf_line_dist(mouse, o, (o.0 + d.0, o.1 + d.1)) <= tol_px;
            hit(f.median_from) || hit(f.tine1_from) || hit(f.tine2_from)
        }
        Drawing::FibRetracement { a, b } => {
            let pa = (p.x(a.ts_millis), p.y(a.price_ticks));
            let pb = (p.x(b.ts_millis), p.y(b.price_ticks));
            if seg_dist(mouse, pa, pb) <= tol_px {
                return true;
            }
            if it.fib_levels.is_empty() {
                FIB_LEVELS_MILLI.iter().any(|m| {
                    let price = fib_level_price(a.price_ticks, b.price_ticks, *m);
                    (p.y(price) - mouse.1).abs() <= tol_px
                })
            } else {
                it.fib_levels.iter().any(|l| {
                    let price = fib_level_price(a.price_ticks, b.price_ticks, l.milli);
                    (p.y(price) - mouse.1).abs() <= tol_px
                })
            }
        }
        Drawing::RegressionChannel { a, b } => match regression_fit(bars, a.ts_millis, b.ts_millis)
        {
            Some(fit) => regression_segments(&fit, p, it.extend)
                .iter()
                .any(|(s0, s1)| seg_dist(mouse, *s0, *s1) <= tol_px),
            None => false,
        },
        Drawing::FibTrendExtension { a, b, c } => {
            let pa = (p.x(a.ts_millis), p.y(a.price_ticks));
            let pb = (p.x(b.ts_millis), p.y(b.price_ticks));
            let pc = (p.x(c.ts_millis), p.y(c.price_ticks));
            let x0 = if it.extend.left() { f64::MIN } else { pc.0 };
            seg_dist(mouse, pa, pb) <= tol_px
                || seg_dist(mouse, pb, pc) <= tol_px
                || FIB_EXT_MILLI.iter().any(|m| {
                    let price = fib_ext_price(a.price_ticks, b.price_ticks, c.price_ticks, *m);
                    (p.y(price) - mouse.1).abs() <= tol_px && mouse.0 >= x0 - tol_px
                })
        }
        Drawing::Brush { points } | Drawing::Highlighter { points } => {
            let w = f64::from(it.style.map(|s| s.width).unwrap_or(2)) * 1.5;
            points.windows(2).any(|seg| {
                seg_dist(
                    mouse,
                    (p.x(seg[0].ts_millis), p.y(seg[0].price_ticks)),
                    (p.x(seg[1].ts_millis), p.y(seg[1].price_ticks)),
                ) <= tol_px + w
            })
        }
        _ => hit_test(&it.drawing, bars, p, mouse, tol_px),
    }
}

/// Indeks objek di bawah kursor (teratas lebih dulu). Objek tersembunyi dan
/// terkunci tidak bisa dipilih dengan klik (padanan "lock" TV).
pub fn pick<P: Projector>(
    items: &[DrawingItem],
    bars: &[Bar],
    p: &P,
    mouse: (f64, f64),
    tol_px: f64,
) -> Option<usize> {
    items
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, it)| it.visible && !it.locked)
        .find(|(_, it)| hit_test_item(it, bars, p, mouse, tol_px))
        .map(|(i, _)| i)
}

/// Seleksi kotak (rubber band, Ctrl+seret di area kosong): indeks objek yang
/// tersentuh kotak layar `rect = (x1, y1, x2, y2)` (urutan bebas). Objek
/// dianggap tersentuh bila titik anchor-nya di dalam kotak, atau segmen antar
/// anchor memotong kotak. Garis panjang (horizontal/vertikal) dinilai dari
/// posisi garisnya, bukan anchor sintetisnya. Objek tersembunyi/terkunci
/// dilewati (padanan `pick`).
pub fn rubber_select<P: Projector>(
    items: &[DrawingItem],
    p: &P,
    rect: (f64, f64, f64, f64),
) -> Vec<usize> {
    let r = norm_rect(rect);
    items
        .iter()
        .enumerate()
        .filter(|(_, it)| it.visible && !it.locked && touched_by_rect(it, p, r))
        .map(|(i, _)| i)
        .collect()
}

/// Kotak ternormalisasi (x1 ≤ x2, y1 ≤ y2).
fn norm_rect((x1, y1, x2, y2): (f64, f64, f64, f64)) -> (f64, f64, f64, f64) {
    (x1.min(x2), y1.min(y2), x1.max(x2), y1.max(y2))
}

fn point_in_rect(q: (f64, f64), r: (f64, f64, f64, f64)) -> bool {
    q.0 >= r.0 && q.0 <= r.2 && q.1 >= r.1 && q.1 <= r.3
}

/// Perpotongan dua segmen (kasus kolinear/bersinggungan ujung diabaikan:
/// anchor di dalam kotak sudah ditangani terpisah sebelum ini).
fn segments_cross(a1: (f64, f64), a2: (f64, f64), b1: (f64, f64), b2: (f64, f64)) -> bool {
    let cross = |o: (f64, f64), a: (f64, f64), b: (f64, f64)| {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };
    let d1 = cross(b1, b2, a1);
    let d2 = cross(b1, b2, a2);
    let d3 = cross(a1, a2, b1);
    let d4 = cross(a1, a2, b2);
    ((d1 > 0.0) != (d2 > 0.0)) && ((d3 > 0.0) != (d4 > 0.0))
}

/// Segmen a1–a2 menyentuh kotak r?
fn segment_hits_rect(a1: (f64, f64), a2: (f64, f64), r: (f64, f64, f64, f64)) -> bool {
    if point_in_rect(a1, r) || point_in_rect(a2, r) {
        return true;
    }
    let (x1, y1, x2, y2) = r;
    [
        ((x1, y1), (x2, y1)),
        ((x2, y1), (x2, y2)),
        ((x2, y2), (x1, y2)),
        ((x1, y2), (x1, y1)),
    ]
    .iter()
    .any(|(b1, b2)| segments_cross(a1, a2, *b1, *b2))
}

/// Objek tersentuh kotak seleksi?
fn touched_by_rect<P: Projector>(it: &DrawingItem, p: &P, r: (f64, f64, f64, f64)) -> bool {
    match &it.drawing {
        // Garis panjang: hanya satu sumbu yang relevan.
        Drawing::HorizontalLine { price_ticks } => {
            let y = p.y(*price_ticks);
            y >= r.1 && y <= r.3
        }
        Drawing::VerticalLine { ts_millis } => {
            let x = p.x(*ts_millis);
            x >= r.0 && x <= r.2
        }
        d => {
            let pts: Vec<(f64, f64)> = d
                .anchors()
                .iter()
                .map(|a| (p.x(a.ts_millis), p.y(a.price_ticks)))
                .collect();
            match pts.len() {
                0 => false,
                1 => point_in_rect(pts[0], r),
                _ => {
                    pts.iter().any(|q| point_in_rect(*q, r))
                        || pts.windows(2).any(|w| segment_hits_rect(w[0], w[1], r))
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Magnet (snap) — meniru magnet lemah/kuat: menempel ke harga OHLC bar terdekat.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Magnet {
    Off,
    Weak,
    Strong,
}

impl Magnet {
    pub fn label(self) -> &'static str {
        match self {
            Magnet::Off => "Magnet Off",
            Magnet::Weak => "Magnet Weak",
            Magnet::Strong => "Magnet Strong",
        }
    }
    pub fn next(self) -> Magnet {
        match self {
            Magnet::Off => Magnet::Weak,
            Magnet::Weak => Magnet::Strong,
            Magnet::Strong => Magnet::Off,
        }
    }
    pub fn code(self) -> &'static str {
        match self {
            Magnet::Off => "off",
            Magnet::Weak => "weak",
            Magnet::Strong => "strong",
        }
    }
    pub fn from_code(s: &str) -> Option<Magnet> {
        match s {
            "off" => Some(Magnet::Off),
            "weak" => Some(Magnet::Weak),
            "strong" => Some(Magnet::Strong),
            _ => None,
        }
    }
}

/// Konteks snap: semua data yang dipakai untuk menghitung anchor magnet.
pub struct SnapContext<'a> {
    pub bars: &'a [Bar],
    pub viewport: &'a crate::Viewport,
    pub spacing_px: f64,
    pub drawings: &'a [DrawingItem],
    pub tol_px: f64,
}

fn raw_anchor(mouse: (f64, f64), v: &crate::Viewport) -> Anchor {
    Anchor {
        ts_millis: v.time_at_x(mouse.0),
        price_ticks: v.price_at_y(mouse.1),
    }
}

/// Indeks bar terdekat di layar (dibatasi rentang terlihat).
fn nearest_bar(ctx: &SnapContext, mouse_x: f64) -> Option<usize> {
    let range = crate::visible_indices(ctx.bars, ctx.viewport.t_min, ctx.viewport.t_max);
    if range.is_empty() {
        return None;
    }
    let start = range.start;
    let len = range.end - start;
    let idx = (mouse_x / ctx.spacing_px.max(1e-9)).floor() as i64 - start as i64;
    let mut best = start;
    let mut best_d = f64::MAX;
    for delta in -2i64..=2 {
        let cand = (start as i64 + idx + delta).clamp(start as i64, (start + len - 1) as i64);
        let x = bar_center_x(cand as usize, start, ctx.spacing_px);
        let d = (x - mouse_x).abs();
        if d < best_d {
            best_d = d;
            best = cand as usize;
        }
    }
    Some(best)
}

pub fn bar_center_x(index: usize, visible_start: usize, spacing_px: f64) -> f64 {
    (index - visible_start) as f64 * spacing_px + spacing_px / 2.0
}

fn bar_x(ctx: &SnapContext, index: usize) -> f64 {
    let range = crate::visible_indices(ctx.bars, ctx.viewport.t_min, ctx.viewport.t_max);
    bar_center_x(index, range.start, ctx.spacing_px)
}

/// Kandidat harga OHLC sebuah bar + jarak piksel ke mouse_y.
fn ohlc_nearest(bar: &Bar, mouse_y: f64, v: &crate::Viewport) -> (i64, f64) {
    let mut best = bar.open_ticks;
    let mut best_d = f64::MAX;
    for p in [
        bar.open_ticks,
        bar.high_ticks,
        bar.low_ticks,
        bar.close_ticks,
    ] {
        let d = (v.y_for_price(p) - mouse_y).abs();
        if d < best_d {
            best_d = d;
            best = p;
        }
    }
    (best, best_d)
}

/// Anchor magnetik + apakah benar-benar menempel (bukan posisi bebas).
pub fn snap_anchor(mouse: (f64, f64), ctx: &SnapContext, magnet: Magnet) -> (Anchor, bool) {
    match magnet {
        Magnet::Off => (raw_anchor(mouse, ctx.viewport), false),
        Magnet::Weak => {
            let Some(i) = nearest_bar(ctx, mouse.0) else {
                return (raw_anchor(mouse, ctx.viewport), false);
            };
            let dx = (bar_x(ctx, i) - mouse.0).abs();
            let (price, dy) = ohlc_nearest(&ctx.bars[i], mouse.1, ctx.viewport);
            if dx <= ctx.tol_px && dy <= ctx.tol_px {
                (
                    Anchor {
                        ts_millis: ctx.bars[i].ts_millis,
                        price_ticks: price,
                    },
                    true,
                )
            } else {
                (raw_anchor(mouse, ctx.viewport), false)
            }
        }
        Magnet::Strong => {
            // 1) anchor drawing terdekat (objek tersembunyi/terkunci dilewati)
            let mut best: Option<(Anchor, f64)> = None;
            for it in ctx.drawings.iter().filter(|i| i.visible && !i.locked) {
                for a in it.drawing.anchors() {
                    let ax = ctx.viewport.x_for_time(a.ts_millis);
                    let ay = ctx.viewport.y_for_price(a.price_ticks);
                    let dist = ((ax - mouse.0).powi(2) + (ay - mouse.1).powi(2)).sqrt();
                    if dist <= ctx.tol_px && best.map(|(_, bd)| dist < bd).unwrap_or(true) {
                        best = Some((a, dist));
                    }
                }
            }
            if let Some((a, _)) = best {
                return (a, true);
            }
            // 2) harga garis horizontal dalam toleransi (waktu tetap bebas)
            for it in ctx.drawings.iter().filter(|i| i.visible && !i.locked) {
                if let Drawing::HorizontalLine { price_ticks } = &it.drawing {
                    let dy = (ctx.viewport.y_for_price(*price_ticks) - mouse.1).abs();
                    if dy <= ctx.tol_px {
                        return (
                            Anchor {
                                ts_millis: ctx.viewport.time_at_x(mouse.0),
                                price_ticks: *price_ticks,
                            },
                            true,
                        );
                    }
                }
            }
            // 3) selalu tempel ke OHLC bar terdekat
            match nearest_bar(ctx, mouse.0) {
                Some(i) => {
                    let (price, _) = ohlc_nearest(&ctx.bars[i], mouse.1, ctx.viewport);
                    (
                        Anchor {
                            ts_millis: ctx.bars[i].ts_millis,
                            price_ticks: price,
                        },
                        true,
                    )
                }
                None => (raw_anchor(mouse, ctx.viewport), false),
            }
        }
    }
}

/// Konstrain sudut saat menekan Shift: horizontal / vertikal / 45° (piksel).
pub fn constrain_angle(a: Anchor, mouse: (f64, f64), v: &crate::Viewport) -> Anchor {
    let ax = v.x_for_time(a.ts_millis);
    let ay = v.y_for_price(a.price_ticks);
    let dx = mouse.0 - ax;
    let dy = mouse.1 - ay;
    let sign = if dy >= 0.0 { 1.0 } else { -1.0 };
    let diag = ay + sign * dx.abs();
    let d_h = dy * dy;
    let d_v = dx * dx;
    let d_d = (mouse.1 - diag) * (mouse.1 - diag);
    if d_h <= d_v && d_h <= d_d {
        Anchor {
            ts_millis: v.time_at_x(mouse.0),
            price_ticks: a.price_ticks,
        }
    } else if d_v <= d_d {
        Anchor {
            ts_millis: a.ts_millis,
            price_ticks: v.price_at_y(mouse.1),
        }
    } else {
        Anchor {
            ts_millis: v.time_at_x(mouse.0),
            price_ticks: v.price_at_y(diag),
        }
    }
}

/// Jumlah bar dalam rentang waktu (dipakai label measure).
pub fn bars_between(bars: &[Bar], t0: i64, t1: i64) -> usize {
    let (lo, hi) = (t0.min(t1), t0.max(t1));
    crate::visible_indices(bars, lo, hi).len()
}

/// Pola garis objek (padanan TV: solid/dashed/dotted).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DashStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

impl DashStyle {
    pub fn key(self) -> &'static str {
        match self {
            DashStyle::Solid => "solid",
            DashStyle::Dashed => "dashed",
            DashStyle::Dotted => "dotted",
        }
    }

    pub fn from_key(k: &str) -> Option<Self> {
        match k {
            "solid" => Some(DashStyle::Solid),
            "dashed" => Some(DashStyle::Dashed),
            "dotted" => Some(DashStyle::Dotted),
            _ => None,
        }
    }
}

/// Gaya khusus objek (None = default per jenis dari painter).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemStyle {
    pub color: [u8; 3],
    pub width: u8,
    pub dash: DashStyle,
}

/// Objek gambar + metadata tampilan: visibilitas, kunci, nama kustom, gaya.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawingItem {
    pub drawing: Drawing,
    pub visible: bool,
    pub locked: bool,
    pub name: Option<String>,
    pub style: Option<ItemStyle>,
    /// Id grup (objek bergrup ikut bergeser bersama saat badan digeser).
    pub group: Option<u32>,
    /// Opasitas objek dalam persen (0 = tembus pandang penuh, 100 = penuh).
    pub opacity: u8,
    /// Varian pitchfork (hanya dipakai objek `Drawing::Pitchfork`).
    pub pitchfork: PitchforkKind,
    /// Level fib kustom; kosong = pakai level bawaan `FIB_LEVELS_MILLI`.
    pub fib_levels: Vec<FibLevel>,
    /// Perpanjangan garis channel (hanya dipakai objek `Drawing::ParallelChannel`).
    pub extend: ExtendMode,
    /// Ukuran font catatan teks (hanya dipakai objek Text/Callout).
    pub text_size: TextSize,
    /// Perataan catatan teks (hanya dipakai objek Text/Callout).
    pub text_align: TextAlign,
}

impl DrawingItem {
    pub fn new(drawing: Drawing) -> Self {
        Self {
            drawing,
            visible: true,
            locked: false,
            name: None,
            style: None,
            group: None,
            opacity: 100,
            pitchfork: PitchforkKind::Andrew,
            fib_levels: Vec::new(),
            extend: ExtendMode::None,
            text_size: TextSize::M,
            text_align: TextAlign::Left,
        }
    }

    /// Faktor opasitas 0.0–1.0 dari persen (nilai >100 dianggap 100).
    pub fn opacity_factor(&self) -> f32 {
        f32::from(self.opacity.min(100)) / 100.0
    }

    /// Label untuk daftar objek: nama kustom bila ada, jika tidak jenis alat.
    pub fn label(&self) -> String {
        match &self.name {
            Some(n) if !n.is_empty() => n.clone(),
            _ => self.drawing.kind_label().to_string(),
        }
    }
}

/// Template gaya bernama: cuplikan warna/tebal/pola + opasitas sebuah objek,
/// bisa diterapkan ke objek terpilih (padanan katalog template alat gambar).
#[derive(Debug, Clone, PartialEq)]
pub struct StyleTemplate {
    pub name: String,
    pub style: Option<ItemStyle>,
    pub opacity: u8,
}

impl StyleTemplate {
    /// Panjang nama maksimum (dipotong saat simpan/muat).
    pub const NAME_MAX: usize = 32;
    /// Jumlah maksimum entri katalog.
    pub const MAX: usize = 16;

    /// Cuplikan dari sebuah objek.
    pub fn from_item(name: &str, it: &DrawingItem) -> Self {
        Self {
            name: name.to_string(),
            style: it.style,
            opacity: it.opacity.min(100),
        }
    }

    /// Terapkan ke sebuah objek (gaya + opasitas).
    pub fn apply_to(&self, it: &mut DrawingItem) {
        it.style = self.style;
        it.opacity = self.opacity.min(100);
    }
}

/// Riwayat undo/redo untuk tambah/pindah/hapus.
#[derive(Debug, Default, Clone)]
pub struct History {
    pub items: Vec<DrawingItem>,
    pub undo: Vec<Vec<DrawingItem>>,
    pub redo: Vec<Vec<DrawingItem>>,
}

impl History {
    pub fn snapshot(&mut self) {
        self.undo.push(self.items.clone());
        self.redo.clear();
    }

    pub fn add(&mut self, d: Drawing) {
        self.add_item(DrawingItem::new(d));
    }

    pub fn add_item(&mut self, item: DrawingItem) {
        self.snapshot();
        self.items.push(item);
    }

    pub fn remove(&mut self, idx: usize) -> bool {
        if idx >= self.items.len() {
            return false;
        }
        self.snapshot();
        self.items.remove(idx);
        true
    }

    pub fn clear(&mut self) -> bool {
        if self.items.is_empty() {
            return false;
        }
        self.snapshot();
        self.items.clear();
        true
    }

    /// Hapus banyak objek sekaligus (satu snapshot untuk undo). Return jumlah
    /// objek yang benar-benar terhapus.
    pub fn remove_many(&mut self, idxs: &[usize]) -> usize {
        let mut v: Vec<usize> = idxs
            .iter()
            .copied()
            .filter(|i| *i < self.items.len())
            .collect();
        v.sort_unstable();
        v.dedup();
        if v.is_empty() {
            return 0;
        }
        self.snapshot();
        for i in v.iter().rev() {
            self.items.remove(*i);
        }
        v.len()
    }

    /// Snapshot sekali di awal drag, lalu `set` berkali-kali tanpa snapshot.
    pub fn begin_edit(&mut self) {
        self.snapshot();
    }

    /// Ganti geometri objek, metadata (visible/locked/nama/gaya) tetap.
    pub fn set(&mut self, idx: usize, d: Drawing) -> bool {
        if let Some(it) = self.items.get_mut(idx) {
            it.drawing = d;
            true
        } else {
            false
        }
    }

    pub fn set_item(&mut self, idx: usize, item: DrawingItem) -> bool {
        if idx >= self.items.len() {
            return false;
        }
        self.items[idx] = item;
        true
    }

    /// Geser urutan z objek (dir<0 = ke belakang, dir>0 = ke depan). Return indeks baru.
    pub fn move_item(&mut self, idx: usize, dir: isize) -> Option<usize> {
        if idx >= self.items.len() || dir == 0 {
            return None;
        }
        let to = idx as isize + dir.signum();
        if to < 0 || to as usize >= self.items.len() {
            return None;
        }
        self.snapshot();
        self.items.swap(idx, to as usize);
        Some(to as usize)
    }

    /// Duplikat objek idx, digeser (dts, dp); objek baru di atas dan terpilih.
    /// Salinan tidak membawa grup (agar drag tidak menyeret dua objek sekaligus).
    pub fn duplicate(&mut self, idx: usize, dts: i64, dp: i64) -> Option<usize> {
        self.duplicate_many(&[idx], dts, dp).first().copied()
    }

    /// Duplikat banyak objek (satu snapshot), semua digeser (dts, dp). Salinan
    /// berada di urutan yang sama di atas tumpukan; return indeks barunya.
    pub fn duplicate_many(&mut self, idxs: &[usize], dts: i64, dp: i64) -> Vec<usize> {
        let mut sel: Vec<usize> = idxs
            .iter()
            .copied()
            .filter(|i| *i < self.items.len())
            .collect();
        sel.sort_unstable();
        sel.dedup();
        if sel.is_empty() {
            return Vec::new();
        }
        self.snapshot();
        let mut out = Vec::with_capacity(sel.len());
        for i in sel {
            let mut item = self.items[i].clone();
            item.drawing = item.drawing.translated(dts, dp);
            item.group = None;
            self.items.push(item);
            out.push(self.items.len() - 1);
        }
        out
    }

    /// Geser z banyak objek satu langkah (dir>0 = ke depan) tanpa mengubah
    /// urutan relatif di antara mereka. Return indeks baru tiap objek (urut
    /// sesuai masukan), atau None bila tidak ada yang bisa pindah.
    pub fn move_items_z(&mut self, idxs: &[usize], dir: isize) -> Option<Vec<usize>> {
        let step = dir.signum();
        let n = self.items.len();
        if step == 0 || n < 2 {
            return None;
        }
        let mut order: Vec<usize> = idxs.iter().copied().filter(|i| *i < n).collect();
        order.sort_unstable();
        order.dedup();
        if order.is_empty() {
            return None;
        }
        let mut flag = vec![false; n];
        for i in &order {
            flag[*i] = true;
        }
        let movable = order.iter().any(|i| {
            let t = *i as isize + step;
            t >= 0 && (t as usize) < n && !flag[t as usize]
        });
        if !movable {
            return None;
        }
        self.snapshot();
        let mut pos = order.clone();
        if step > 0 {
            for i in (0..n - 1).rev() {
                if flag[i] && !flag[i + 1] {
                    self.items.swap(i, i + 1);
                    flag.swap(i, i + 1);
                    for q in pos.iter_mut() {
                        if *q == i {
                            *q = i + 1;
                        }
                    }
                }
            }
        } else {
            for i in 1..n {
                if flag[i] && !flag[i - 1] {
                    self.items.swap(i - 1, i);
                    flag.swap(i - 1, i);
                    for q in pos.iter_mut() {
                        if *q == i {
                            *q = i - 1;
                        }
                    }
                }
            }
        }
        Some(pos)
    }

    /// Indeks semua anggota grup objek idx (termasuk idx), atau hanya idx bila
    /// tidak bergrup.
    pub fn group_members(&self, idx: usize) -> Vec<usize> {
        let Some(item) = self.items.get(idx) else {
            return Vec::new();
        };
        match item.group {
            None => vec![idx],
            Some(g) => self
                .items
                .iter()
                .enumerate()
                .filter(|(_, it)| it.group == Some(g))
                .map(|(i, _)| i)
                .collect(),
        }
    }

    /// Lepas objek idx dari grupnya. Return true bila sebelumnya bergrup.
    pub fn clear_group(&mut self, idx: usize) -> bool {
        self.clear_group_many(&[idx]) > 0
    }

    /// Lepas beberapa objek dari grupnya (satu snapshot). Return jumlah yang
    /// sebelumnya bergrup.
    pub fn clear_group_many(&mut self, idxs: &[usize]) -> usize {
        let mut sel: Vec<usize> = idxs
            .iter()
            .copied()
            .filter(|i| *i < self.items.len())
            .collect();
        sel.sort_unstable();
        sel.dedup();
        let grouped = sel
            .iter()
            .filter(|i| self.items[**i].group.is_some())
            .count();
        if grouped == 0 {
            return 0;
        }
        self.snapshot();
        for i in sel {
            self.items[i].group = None;
        }
        grouped
    }

    /// Beri id grup baru untuk objek idx (id = maks yang ada + 1). Return id.
    pub fn assign_group(&mut self, idx: usize) -> Option<u32> {
        self.assign_group_many(&[idx])
    }

    /// Beri satu id grup baru untuk beberapa objek sekaligus (satu snapshot).
    pub fn assign_group_many(&mut self, idxs: &[usize]) -> Option<u32> {
        let mut sel: Vec<usize> = idxs
            .iter()
            .copied()
            .filter(|i| *i < self.items.len())
            .collect();
        sel.sort_unstable();
        sel.dedup();
        if sel.is_empty() {
            return None;
        }
        self.snapshot();
        let next = self
            .items
            .iter()
            .filter_map(|it| it.group)
            .max()
            .unwrap_or(0)
            + 1;
        for i in sel {
            self.items[i].group = Some(next);
        }
        Some(next)
    }

    pub fn do_undo(&mut self) -> bool {
        if let Some(prev) = self.undo.pop() {
            self.redo.push(self.items.clone());
            self.items = prev;
            true
        } else {
            false
        }
    }

    pub fn do_redo(&mut self) -> bool {
        if let Some(next) = self.redo.pop() {
            self.undo.push(self.items.clone());
            self.items = next;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct P {
        t0: i64,
        p0: i64,
    }

    impl Projector for P {
        fn x(&self, ts_millis: i64) -> f64 {
            (ts_millis - self.t0) as f64 * 2.0
        }
        fn y(&self, price_ticks: i64) -> f64 {
            (self.p0 - price_ticks) as f64
        }
    }

    fn p() -> P {
        P { t0: 0, p0: 0 }
    }

    fn bars(n: usize) -> Vec<Bar> {
        (0..n)
            .map(|i| Bar {
                ts_millis: 1_000 + i as i64 * 60_000,
                open_ticks: 100 + i as i64 * 2,
                high_ticks: 110 + i as i64 * 2,
                low_ticks: 90 + i as i64 * 2,
                close_ticks: 105 + i as i64 * 2,
                volume_milli: 10,
            })
            .collect()
    }

    fn viewport(n: usize) -> crate::Viewport {
        let b = bars(n);
        crate::Viewport {
            width_px: 800.0,
            height_px: 400.0,
            t_min: b[0].ts_millis,
            t_max: b[n - 1].ts_millis,
            p_min: 80,
            p_max: 130,
            ..Default::default()
        }
    }

    #[test]
    fn horizontal_line_hit_near_y() {
        let d = Drawing::HorizontalLine { price_ticks: 100 };
        assert!(hit_test(&d, &[], &p(), (500.0, -99.0), 4.0));
        assert!(!hit_test(&d, &[], &p(), (500.0, 50.0), 4.0));
    }

    #[test]
    fn vertical_line_hit_near_x() {
        let d = Drawing::VerticalLine { ts_millis: 50 };
        assert!(hit_test(&d, &[], &p(), (99.0, 300.0), 4.0));
        assert!(!hit_test(&d, &[], &p(), (500.0, 300.0), 4.0));
    }

    #[test]
    fn trendline_hit_on_segment() {
        let d = Drawing::Trendline {
            a: Anchor {
                ts_millis: 0,
                price_ticks: 0,
            },
            b: Anchor {
                ts_millis: 10,
                price_ticks: 0,
            },
        };
        assert!(hit_test(&d, &[], &p(), (10.0, 1.0), 4.0));
        assert!(!hit_test(&d, &[], &p(), (10.0, 100.0), 4.0));
    }

    #[test]
    fn ray_hit_only_forward_of_origin() {
        let d = Drawing::Ray {
            origin: Anchor {
                ts_millis: 10,
                price_ticks: 0,
            },
            through: Anchor {
                ts_millis: 20,
                price_ticks: 0,
            },
        };
        assert!(hit_test(&d, &[], &p(), (60.0, 0.0), 2.0)); // di depan
        assert!(!hit_test(&d, &[], &p(), (0.0, 0.0), 2.0)); // di belakang
    }

    #[test]
    fn rectangle_hits_edge_not_center() {
        let d = Drawing::Rectangle {
            a: Anchor {
                ts_millis: 0,
                price_ticks: 0,
            },
            b: Anchor {
                ts_millis: 10,
                price_ticks: 10,
            },
        };
        assert!(hit_test(&d, &[], &p(), (0.0, -5.0), 2.0));
        assert!(!hit_test(&d, &[], &p(), (10.0, -5.0), 2.0));
    }

    #[test]
    fn pick_returns_topmost() {
        let list = vec![
            DrawingItem::new(Drawing::HorizontalLine { price_ticks: 100 }),
            DrawingItem::new(Drawing::HorizontalLine { price_ticks: 100 }),
        ];
        assert_eq!(pick(&list, &[], &p(), (0.0, -100.0), 4.0), Some(1));
    }

    #[test]
    fn undo_redo_cycle() {
        let mut h = History::default();
        h.add(Drawing::HorizontalLine { price_ticks: 1 });
        h.add(Drawing::VerticalLine { ts_millis: 2 });
        assert_eq!(h.items.len(), 2);
        assert!(h.do_undo());
        assert_eq!(h.items.len(), 1);
        assert!(h.do_undo());
        assert_eq!(h.items.len(), 0);
        assert!(h.do_redo());
        assert_eq!(h.items.len(), 1);
    }

    #[test]
    fn remove_out_of_range_returns_false() {
        let mut h = History::default();
        assert!(!h.remove(5));
    }

    #[test]
    fn anchors_exposed() {
        let d = Drawing::Trendline {
            a: Anchor {
                ts_millis: 1,
                price_ticks: 2,
            },
            b: Anchor {
                ts_millis: 3,
                price_ticks: 4,
            },
        };
        assert_eq!(d.anchors().len(), 2);
    }

    #[test]
    fn new_kinds_hit_test() {
        let hray = Drawing::HorizontalRay {
            origin: Anchor {
                ts_millis: 10,
                price_ticks: 0,
            },
        };
        assert!(hit_test(&hray, &[], &p(), (60.0, 0.5), 2.0)); // kanan origin
        assert!(!hit_test(&hray, &[], &p(), (0.0, 0.5), 2.0)); // kiri origin

        let cross = Drawing::CrossLine {
            anchor: Anchor {
                ts_millis: 10,
                price_ticks: 10,
            },
        };
        assert!(hit_test(&cross, &[], &p(), (20.0, -10.0), 2.0));
        assert!(hit_test(&cross, &[], &p(), (20.5, -300.0), 2.0));
        assert!(!hit_test(&cross, &[], &p(), (200.0, -300.0), 2.0));

        let ext = Drawing::ExtendedLine {
            a: Anchor {
                ts_millis: 0,
                price_ticks: 0,
            },
            b: Anchor {
                ts_millis: 10,
                price_ticks: 10,
            },
        };
        // Proyeksi: x = 2·ts, y = -price → a=(0,0), b=(20,-10); garis y = -x/2.
        // Di luar segmen pun kena (x=100 → y=-50).
        assert!(hit_test(&ext, &[], &p(), (100.0, -50.5), 2.0));
        assert!(!hit_test(&ext, &[], &p(), (100.0, -200.0), 2.0));

        let tri = Drawing::Triangle {
            a: Anchor {
                ts_millis: 0,
                price_ticks: 0,
            },
            b: Anchor {
                ts_millis: 10,
                price_ticks: 0,
            },
            c: Anchor {
                ts_millis: 5,
                price_ticks: 10,
            },
        };
        // tepi c-a: garis y = -x → titik (5,-5) tepat di tepi
        assert!(hit_test(&tri, &[], &p(), (5.0, -5.0), 2.0));
        // di dalam tapi >2px dari semua tepi
        assert!(!hit_test(&tri, &[], &p(), (10.0, -4.0), 2.0));

        let fib = Drawing::FibRetracement {
            a: Anchor {
                ts_millis: 0,
                price_ticks: 100,
            },
            b: Anchor {
                ts_millis: 10,
                price_ticks: 0,
            },
        };
        // level 50% → harga 50 → y -50
        assert!(hit_test(&fib, &[], &p(), (500.0, -50.0), 2.0));

        let pos = Drawing::Position {
            long: true,
            entry: Anchor {
                ts_millis: 0,
                price_ticks: 100,
            },
            target: Anchor {
                ts_millis: 10,
                price_ticks: 120,
            },
        };
        // garis target y=-120 di dalam rentang x
        assert!(hit_test(&pos, &[], &p(), (10.0, -120.0), 2.0));
        // stop = 80 → y=-80
        assert!(hit_test(&pos, &[], &p(), (10.0, -80.0), 2.0));
        // jauh dari apa pun
        assert!(!hit_test(&pos, &[], &p(), (100.0, -400.0), 2.0));
    }

    #[test]
    fn fib_level_math_integer_aman() {
        assert_eq!(fib_level_price(0, 1000, 0), 0);
        assert_eq!(fib_level_price(0, 1000, 500), 500);
        assert_eq!(fib_level_price(0, 1000, 1000), 1000);
        assert_eq!(fib_level_price(100, 0, 500), 50);
        // i64 ekstrem tidak overflow
        let lo = fib_level_price(i64::MIN, i64::MAX, 1000);
        assert_eq!(lo, i64::MAX);
        let mid = fib_level_price(i64::MIN, i64::MAX, 500);
        assert!(mid.abs() <= 1);
    }

    #[test]
    fn pitchfork_geometri_empat_varian() {
        let a = (100.0, 200.0);
        let b = (300.0, 100.0);
        let c = (500.0, 160.0);
        // Andrew: median A → titik tengah B–C = (400, 130); dir = (300, −70).
        let f = pitchfork_lines(PitchforkKind::Andrew, a, b, c);
        assert_eq!(f.median_from, a);
        assert_eq!(f.dir, (300.0, -70.0));
        assert_eq!(f.tine1_from, b);
        assert_eq!(f.tine2_from, c);
        assert_eq!(f.helper, (b, c));
        // Schiff: origin (waktu A, harga rata-rata A–B) = (100, 150).
        let f = pitchfork_lines(PitchforkKind::Schiff, a, b, c);
        assert_eq!(f.median_from, (100.0, 150.0));
        assert_eq!(f.dir, (300.0, -20.0));
        assert_eq!(f.tine1_from, b);
        assert_eq!(f.tine2_from, c);
        // Modified Schiff: origin = titik tengah A–B = (200, 150).
        let f = pitchfork_lines(PitchforkKind::ModifiedSchiff, a, b, c);
        assert_eq!(f.median_from, (200.0, 150.0));
        assert_eq!(f.dir, (200.0, -20.0));
        assert_eq!(f.tine1_from, b);
        // Inside: median titik tengah A–B → C; tine lewat A dan B.
        let f = pitchfork_lines(PitchforkKind::Inside, a, b, c);
        assert_eq!(f.median_from, (200.0, 150.0));
        assert_eq!(f.dir, (300.0, 10.0));
        assert_eq!(f.tine1_from, a);
        assert_eq!(f.tine2_from, b);
        assert_eq!(f.helper, (a, b));
        // Median tiap varian berakhir tepat di targetnya (t = 1).
        for (kind, target) in [
            (PitchforkKind::Andrew, (400.0, 130.0)),
            (PitchforkKind::Schiff, (400.0, 130.0)),
            (PitchforkKind::ModifiedSchiff, (400.0, 130.0)),
            (PitchforkKind::Inside, c),
        ] {
            let f = pitchfork_lines(kind, a, b, c);
            let end = (f.median_from.0 + f.dir.0, f.median_from.1 + f.dir.1);
            assert_eq!(end, target, "median {:?}", kind);
        }
    }

    #[test]
    fn varian_pitchfork_kunci_stabil() {
        for k in PitchforkKind::all() {
            assert_eq!(PitchforkKind::from_key(k.key()), Some(k));
        }
        assert_eq!(
            PitchforkKind::from_key("modified-schiff"),
            Some(PitchforkKind::ModifiedSchiff)
        );
        assert_eq!(PitchforkKind::from_key("noise"), None);
        assert_eq!(PitchforkKind::default(), PitchforkKind::Andrew);
    }

    #[test]
    fn hit_test_menghormati_varian_pitchfork_dan_level_fib() {
        let a = Anchor {
            ts_millis: 0,
            price_ticks: 200,
        };
        let b = Anchor {
            ts_millis: 100,
            price_ticks: 100,
        };
        let c = Anchor {
            ts_millis: 200,
            price_ticks: 150,
        };
        let mut it = DrawingItem::new(Drawing::Pitchfork { a, b, c });
        // Proyeksi tes: x = t·2, y = −harga. Titik tengah A–B = (100, −150);
        // median Andrew di x=100 ada di y=−200+(100/300)·75 = −175.
        assert!(
            !hit_test_item(&it, &[], &p(), (100.0, -150.0), 4.0),
            "median Andrew tidak lewat situ"
        );
        assert!(hit_test_item(&it, &[], &p(), (100.0, -175.0), 4.0));
        it.pitchfork = PitchforkKind::Inside;
        assert!(hit_test_item(&it, &[], &p(), (100.0, -150.0), 4.0));
        assert!(!hit_test_item(&it, &[], &p(), (100.0, -175.0), 4.0));
        // Level fib kustom menggantikan level bawaan untuk hit-test.
        let mut f = DrawingItem::new(Drawing::FibRetracement {
            a: Anchor {
                ts_millis: 0,
                price_ticks: 100,
            },
            b: Anchor {
                ts_millis: 300,
                price_ticks: 300,
            },
        });
        // Bawaan: 0,618 → harga 100 + round(200·618/1000) = 224.
        assert!(hit_test_item(&f, &[], &p(), (10.0, -224.0), 4.0));
        f.fib_levels = vec![FibLevel {
            milli: 500,
            color: None,
        }];
        assert!(hit_test_item(&f, &[], &p(), (10.0, -200.0), 4.0));
        assert!(!hit_test_item(&f, &[], &p(), (10.0, -224.0), 4.0));
        // Segmen a–b tetap bisa di-hit (titik tengah (300, −200)).
        assert!(hit_test_item(&f, &[], &p(), (300.0, -200.0), 4.0));
    }

    #[test]
    fn opacity_factor_mengikuti_persen() {
        let mut it = DrawingItem::new(Drawing::VerticalLine { ts_millis: 0 });
        assert_eq!(it.opacity_factor(), 1.0);
        it.opacity = 40;
        assert!((it.opacity_factor() - 0.4).abs() < 1e-6);
        it.opacity = 0;
        assert_eq!(it.opacity_factor(), 0.0);
        it.opacity = 200; // nilai liar tetap dijepit ke 100
        assert_eq!(it.opacity_factor(), 1.0);
    }

    #[test]
    fn position_stop_dicerminkan() {
        let (t, s) = position_levels(100, 120);
        assert_eq!((t, s), (120, 80));
        // Ekstrem i64 tidak overflow — hasil dijepit, bukan panic.
        let (_t, s) = position_levels(i64::MIN + 1, i64::MAX);
        assert_eq!(s, i64::MIN);
    }

    #[test]
    fn set_anchor_dan_translate() {
        let mut d = Drawing::FibRetracement {
            a: Anchor {
                ts_millis: 1,
                price_ticks: 2,
            },
            b: Anchor {
                ts_millis: 3,
                price_ticks: 4,
            },
        };
        assert!(d.set_anchor(
            1,
            Anchor {
                ts_millis: 9,
                price_ticks: 9
            }
        ));
        assert!(!d.set_anchor(
            2,
            Anchor {
                ts_millis: 0,
                price_ticks: 0
            }
        ));
        let t = d.translated(10, -2);
        assert_eq!(t.anchors()[0].ts_millis, 11);
        assert_eq!(t.anchors()[1].price_ticks, 7);
    }

    #[test]
    fn history_set_dan_clear() {
        let mut h = History::default();
        h.add(Drawing::VerticalLine { ts_millis: 1 });
        h.add(Drawing::VerticalLine { ts_millis: 2 });
        h.begin_edit();
        assert!(h.set(0, Drawing::VerticalLine { ts_millis: 99 }));
        assert_eq!(h.items[0].drawing, Drawing::VerticalLine { ts_millis: 99 });
        assert!(h.do_undo());
        assert_eq!(h.items[0].drawing, Drawing::VerticalLine { ts_millis: 1 });
        assert!(h.clear());
        assert!(h.items.is_empty());
        assert!(h.do_undo());
        assert_eq!(h.items.len(), 2);
        // clear pada objek kosong tidak error dan tidak mengubah riwayat.
        let mut empty = History::default();
        assert!(!empty.clear());
    }

    #[test]
    fn magnet_off_mengembalikan_posisi_bebas() {
        let b = bars(20);
        let v = viewport(20);
        let ctx = SnapContext {
            bars: &b,
            viewport: &v,
            spacing_px: 800.0 / 20.0,
            drawings: &[],
            tol_px: 6.0,
        };
        let (a, snapped) = snap_anchor((10.0, 10.0), &ctx, Magnet::Off);
        assert!(!snapped);
        assert_eq!(a.ts_millis, v.time_at_x(10.0));
        assert_eq!(a.price_ticks, v.price_at_y(10.0));
    }

    #[test]
    fn magnet_weak_menempel_ke_ohlc() {
        let b = bars(20);
        let v = viewport(20);
        let spacing = 800.0 / 20.0;
        let ctx = SnapContext {
            bars: &b,
            viewport: &v,
            spacing_px: spacing,
            drawings: &[],
            tol_px: 6.0,
        };
        // x tepat di tengah bar #5, y = harga close bar #5 → harus menempel penuh
        let x = bar_center_x(5, 0, spacing);
        let y = v.y_for_price(b[5].close_ticks);
        let (a, snapped) = snap_anchor((x, y), &ctx, Magnet::Weak);
        assert!(snapped);
        assert_eq!(a.ts_millis, b[5].ts_millis);
        assert_eq!(a.price_ticks, b[5].close_ticks);
        // jauh dari bar → bebas
        let (_, snapped) = snap_anchor((x, 0.0), &ctx, Magnet::Weak);
        assert!(!snapped);
    }

    #[test]
    fn magnet_strong_prioritas_anchor_drawing() {
        let b = bars(20);
        let v = viewport(20);
        let spacing = 800.0 / 20.0;
        let d = Drawing::Trendline {
            a: Anchor {
                ts_millis: b[3].ts_millis,
                price_ticks: b[3].high_ticks,
            },
            b: Anchor {
                ts_millis: b[10].ts_millis,
                price_ticks: b[10].low_ticks,
            },
        };
        let ctx = SnapContext {
            bars: &b,
            viewport: &v,
            spacing_px: spacing,
            drawings: &[DrawingItem::new(d.clone())],
            tol_px: 6.0,
        };
        let x = v.x_for_time(d.anchors()[0].ts_millis);
        let y = v.y_for_price(d.anchors()[0].price_ticks) + 2.0;
        let (a, snapped) = snap_anchor((x, y), &ctx, Magnet::Strong);
        assert!(snapped);
        assert_eq!(a, d.anchors()[0]);
        // jauh dari anchor: tetap menempel ke OHLC bar terdekat
        let (_, snapped) = snap_anchor((x, 0.0), &ctx, Magnet::Strong);
        assert!(snapped);
    }

    #[test]
    fn constrain_shift_memilih_sudut_terdekat() {
        let v = viewport(20);
        let a = Anchor {
            ts_millis: v.t_min,
            price_ticks: v.p_min,
        };
        let ax = v.x_for_time(a.ts_millis);
        let ay = v.y_for_price(a.price_ticks);
        // hampir horizontal → harga sama
        let c = constrain_angle(a, (ax + 100.0, ay + 3.0), &v);
        assert_eq!(c.price_ticks, a.price_ticks);
        // hampir vertikal → waktu sama
        let c = constrain_angle(a, (ax + 3.0, ay + 100.0), &v);
        assert_eq!(c.ts_millis, a.ts_millis);
        // diagonal → |dy| ≈ |dx| (toleransi lebar: kuantisasi harga ke tick)
        let c = constrain_angle(a, (ax + 100.0, ay + 96.0), &v);
        let cy = v.y_for_price(c.price_ticks);
        let cx = v.x_for_time(c.ts_millis);
        assert!(((cy - ay).abs() - (cx - ax).abs()).abs() < 12.0);
    }

    #[test]
    fn bars_between_menghitung_cakupan() {
        let b = bars(10);
        assert_eq!(bars_between(&b, b[2].ts_millis, b[5].ts_millis), 4);
        assert_eq!(bars_between(&b, b[5].ts_millis, b[2].ts_millis), 4);
        assert_eq!(bars_between(&b, 0, 1), 0);
    }

    #[test]
    fn fib_ext_price_dan_safety() {
        assert_eq!(fib_ext_price(100, 200, 300, 0), 300);
        assert_eq!(fib_ext_price(100, 200, 300, 1000), 400);
        assert_eq!(fib_ext_price(100, 200, 300, 618), 362); // 61,8 → dibulatkan
        assert_eq!(fib_ext_price(200, 100, 300, 1618), 138); // arah turun
                                                             // i64 ekstrem dijepit, bukan overflow/panic
        assert_eq!(fib_ext_price(i64::MIN, i64::MAX, i64::MAX, 2618), i64::MAX);
        assert_eq!(fib_ext_price(i64::MIN, i64::MAX, i64::MIN, -2618), i64::MIN);
    }

    #[test]
    fn fib_time_mengikuti_bar() {
        let b = bars(30);
        let list = fib_time_ts(b[3].ts_millis, &b);
        // +1,+2,+3,+5,+8,+13,+21 dalam rentang 30 bar → 7 garis
        assert_eq!(list.len(), 7);
        assert_eq!(list[0], (1, b[4].ts_millis));
        assert_eq!(list[6], (21, b[24].ts_millis));
        // anchor bukan ts bar → pakai bar terdekat sebelumnya
        assert_eq!(fib_time_ts(b[3].ts_millis + 1, &b)[0].1, b[4].ts_millis);
        // bars kosong → kosong, tanpa panic
        assert!(fib_time_ts(0, &[]).is_empty());
    }

    #[test]
    fn channel_pitchfork_fibext_fibtime_hit() {
        let b = bars(20);
        // proyeksi uji: x = 2·ts, y = −price
        let ch = Drawing::ParallelChannel {
            a: Anchor {
                ts_millis: 0,
                price_ticks: 0,
            },
            b: Anchor {
                ts_millis: 10,
                price_ticks: 0,
            },
            c: Anchor {
                ts_millis: 0,
                price_ticks: 10,
            },
            kind: ChannelKind::Parallel,
        };
        assert_eq!(ch.kind_label(), "Parallel Channel");
        // Tanpa perpanjangan (default): hanya segmen batas yang bisa diklik.
        assert!(hit_test(&ch, &[], &p(), (10.0, 0.5), 2.0)); // batas 0 (y=0)
        assert!(hit_test(&ch, &[], &p(), (10.0, -10.5), 2.0)); // batas 1 (y=−10)
        assert!(!hit_test(&ch, &[], &p(), (500.0, 0.5), 2.0)); // di luar rentang x
        assert!(!hit_test(&ch, &[], &p(), (10.0, -5.0), 2.0)); // di antara batas

        // pitchfork: a=(0,0) b=(20,0) c=(20,−10) → median (0,0)→(20,−5)
        let pf = Drawing::Pitchfork {
            a: Anchor {
                ts_millis: 0,
                price_ticks: 0,
            },
            b: Anchor {
                ts_millis: 10,
                price_ticks: 0,
            },
            c: Anchor {
                ts_millis: 10,
                price_ticks: 10,
            },
        };
        assert!(hit_test(&pf, &[], &p(), (40.0, -10.0), 1.5)); // median
        assert!(hit_test(&pf, &[], &p(), (40.0, -5.0), 1.5)); // tine lewat b
        assert!(hit_test(&pf, &[], &p(), (40.0, -15.0), 1.5)); // tine lewat c
        assert!(!hit_test(&pf, &[], &p(), (40.0, -60.0), 1.5));

        // fib extension: base 100→0 dari c=50 → rail 0% di 50 (y −50), 100% di −50 (y 50)
        let fe = Drawing::FibExtension {
            a: Anchor {
                ts_millis: 0,
                price_ticks: 100,
            },
            b: Anchor {
                ts_millis: 10,
                price_ticks: 0,
            },
            c: Anchor {
                ts_millis: 5,
                price_ticks: 50,
            },
        };
        assert!(hit_test(&fe, &[], &p(), (500.0, -50.0), 2.0));
        assert!(hit_test(&fe, &[], &p(), (500.0, 50.0), 2.0));
        assert!(hit_test(&fe, &[], &p(), (10.0, -50.0), 2.0)); // konektor a→b
        assert!(!hit_test(&fe, &[], &p(), (500.0, 0.0), 2.0));

        // fib time: garis vertikal di bar +1,+2,+3,…
        let ft = Drawing::FibTime {
            a: Anchor {
                ts_millis: b[3].ts_millis,
                price_ticks: 0,
            },
        };
        let x5 = p().x(b[5].ts_millis);
        assert!(hit_test(&ft, &b, &p(), (x5 + 0.5, -200.0), 2.0));
        assert!(!hit_test(&ft, &b, &p(), (x5 + 9.0, -200.0), 2.0));
        // tanpa bars → tidak ada garis yang bisa dihit (tidak panic)
        assert!(!hit_test(&ft, &[], &p(), (x5, -200.0), 2.0));

        // anchors & translate 3-anchor
        assert_eq!(fe.anchors().len(), 3);
        assert_eq!(ft.anchors().len(), 1);
        let moved = pf.translated(5, -3);
        assert_eq!(moved.anchors()[2].price_ticks, 7);
    }

    /// Bandingkan dua titik layar dengan toleransi kecil.
    fn near(p: (f64, f64), q: (f64, f64)) -> bool {
        (p.0 - q.0).abs() < 1e-6 && (p.1 - q.1).abs() < 1e-6
    }

    #[test]
    fn channel_segments_paralel_disjoint_flat() {
        // Dasar menurun: a=(0,0) → b=(20,−20) (slope −1); c=(10,−30).
        let (a, b, c) = ((0.0, 0.0), (20.0, -20.0), (10.0, -30.0));
        let none = ExtendMode::None;
        // Parallel: batas kedua sejajar lewat c, rentang x sama dengan batas 0.
        let [s0, s1] = channel_segments(ChannelKind::Parallel, none, a, b, c);
        assert!(near(s0.0, a) && near(s0.1, b));
        assert!(near(s1.0, (0.0, -20.0)) && near(s1.1, (20.0, -40.0)));
        // Disjoint: batas kedua = salinan batas 0 yang digeser oleh (c − a).
        let [_, s1] = channel_segments(ChannelKind::Disjoint, none, a, b, c);
        assert!(near(s1.0, (10.0, -30.0)) && near(s1.1, (30.0, -50.0)));
        // Flat: batas kedua horizontal di harga c, rentang x batas 0.
        let [_, s1] = channel_segments(ChannelKind::Flat, none, a, b, c);
        assert!(near(s1.0, (0.0, -30.0)) && near(s1.1, (20.0, -30.0)));
    }

    #[test]
    fn channel_extend_mengikuti_sumbu_waktu() {
        // Dasar horizontal a=(0,0) → b=(20,0).
        let (a, b, c) = ((0.0, 0.0), (20.0, 0.0), (0.0, -8.0));
        let [s0, s1] = channel_segments(ChannelKind::Parallel, ExtendMode::Both, a, b, c);
        assert!(near(s0.0, (-EXTEND_PX, 0.0)) && near(s0.1, (20.0 + EXTEND_PX, 0.0)));
        assert!(near(s1.0, (-EXTEND_PX, -8.0)) && near(s1.1, (20.0 + EXTEND_PX, -8.0)));
        let [s0, _] = channel_segments(ChannelKind::Parallel, ExtendMode::Left, a, b, c);
        assert!(near(s0.0, (-EXTEND_PX, 0.0)) && near(s0.1, b));
        let [s0, _] = channel_segments(ChannelKind::Parallel, ExtendMode::Right, a, b, c);
        assert!(near(s0.0, a) && near(s0.1, (20.0 + EXTEND_PX, 0.0)));
        // Panjang perpanjangan juga mengikuti batas 1 pada varian disjoint/flat.
        let [_, s1] = channel_segments(ChannelKind::Disjoint, ExtendMode::Right, a, b, c);
        assert!(near(s1.0, (0.0, -8.0)) && near(s1.1, (20.0 + EXTEND_PX, -8.0)));
        let [_, s1] = channel_segments(ChannelKind::Flat, ExtendMode::Right, a, b, c);
        assert!(near(s1.0, (0.0, -8.0)) && near(s1.1, (20.0 + EXTEND_PX, -8.0)));
        // Dasar vertikal: perpanjangan kiri/kanan tidak bermakna → dilewati.
        let (av, bv, cv) = ((10.0, 0.0), (10.0, 20.0), (30.0, 0.0));
        let [s0, s1] = channel_segments(ChannelKind::Parallel, ExtendMode::Both, av, bv, cv);
        assert!(near(s0.0, av) && near(s0.1, bv));
        assert!(near(s1.0, (30.0, 0.0)) && near(s1.1, (30.0, 20.0)));
        assert_eq!(ExtendMode::from_checks(true, false), ExtendMode::Left);
        assert_eq!(ExtendMode::from_checks(false, true), ExtendMode::Right);
        assert_eq!(ExtendMode::from_checks(true, true), ExtendMode::Both);
        assert_eq!(ExtendMode::from_checks(false, false), ExtendMode::None);
        assert_eq!(ExtendMode::from_key("both"), Some(ExtendMode::Both));
        assert_eq!(ExtendMode::from_key("kiri"), None);
    }

    #[test]
    fn channel_varian_label_dan_hit_extend_objek() {
        assert_eq!(ChannelKind::all().len(), 3);
        assert_eq!(ChannelKind::from_key("flat"), Some(ChannelKind::Flat));
        assert_eq!(ChannelKind::from_key("regression"), None);
        for (k, label) in [
            (ChannelKind::Parallel, "Parallel Channel"),
            (ChannelKind::Disjoint, "Disjoint Channel"),
            (ChannelKind::Flat, "Flat Top/Bottom Channel"),
        ] {
            let d = Drawing::ParallelChannel {
                a: Anchor {
                    ts_millis: 0,
                    price_ticks: 0,
                },
                b: Anchor {
                    ts_millis: 10,
                    price_ticks: 0,
                },
                c: Anchor {
                    ts_millis: 3,
                    price_ticks: 10,
                },
                kind: k,
            };
            assert_eq!(d.kind_label(), label);
            assert_eq!(d.degree(), Degree::Channel);
            assert_eq!(d.anchors().len(), 3);
            let moved = d.translated(5, -3);
            match moved {
                Drawing::ParallelChannel { kind, .. } => assert_eq!(kind, k),
                _ => panic!("translated harus tetap channel"),
            }
        }
        // Objek dengan extend: klik di luar bentuk tetap kena selama garis
        // perpanjangannya melewati titik itu (padanan tv "Extend").
        let mut it = DrawingItem::new(Drawing::ParallelChannel {
            a: Anchor {
                ts_millis: 0,
                price_ticks: 0,
            },
            b: Anchor {
                ts_millis: 10,
                price_ticks: 0,
            },
            c: Anchor {
                ts_millis: 0,
                price_ticks: 10,
            },
            kind: ChannelKind::Parallel,
        });
        assert!(!hit_test_item(&it, &[], &p(), (500.0, 0.5), 2.0));
        it.extend = ExtendMode::Both;
        assert!(hit_test_item(&it, &[], &p(), (500.0, 0.5), 2.0));
        assert!(hit_test_item(&it, &[], &p(), (500.0, -10.5), 2.0));
        // Varian disjoint: batas kedua bergeser di sumbu waktu.
        if let Drawing::ParallelChannel { kind, .. } = &mut it.drawing {
            *kind = ChannelKind::Disjoint;
        }
        it.extend = ExtendMode::None;
        // Batas 1 disjoint mulai di x c=0 → sama dengan paralel di sini;
        // batas 0 tetap di (0..20) dan di luar itu tidak kena.
        assert!(hit_test_item(&it, &[], &p(), (10.0, -10.5), 2.0));
        assert!(!hit_test_item(&it, &[], &p(), (60.0, -10.5), 2.0));
    }

    #[test]
    fn pick_melewati_objek_tersembunyi_dan_terkunci() {
        let b = bars(5);
        let mut items = vec![
            DrawingItem::new(Drawing::HorizontalLine { price_ticks: 100 }),
            DrawingItem::new(Drawing::HorizontalLine { price_ticks: 200 }),
            DrawingItem::new(Drawing::HorizontalLine { price_ticks: 200 }),
        ];
        // teratas menang
        assert_eq!(pick(&items, &b, &p(), (0.0, -200.0), 4.0), Some(2));
        items[2].visible = false;
        assert_eq!(pick(&items, &b, &p(), (0.0, -200.0), 4.0), Some(1));
        items[1].locked = true;
        assert_eq!(pick(&items, &b, &p(), (0.0, -200.0), 4.0), None);
        items[2].visible = true;
        assert_eq!(pick(&items, &b, &p(), (0.0, -200.0), 4.0), Some(2));
    }

    #[test]
    fn set_menjaga_metadata_dan_label_nama() {
        let mut h = History::default();
        h.add(Drawing::HorizontalLine { price_ticks: 100 });
        h.items[0].locked = true;
        h.items[0].style = Some(ItemStyle {
            color: [1, 2, 3],
            width: 3,
            dash: DashStyle::Solid,
        });
        h.items[0].name = Some("support".into());
        assert!(h.set(0, Drawing::HorizontalLine { price_ticks: 123 }));
        assert!(h.items[0].locked);
        assert_eq!(h.items[0].style.unwrap().width, 3);
        assert_eq!(h.items[0].label(), "support");
        assert_eq!(
            h.items[0].drawing,
            Drawing::HorizontalLine { price_ticks: 123 }
        );
        // label default = jenis alat
        let it = DrawingItem::new(Drawing::VerticalLine { ts_millis: 5 });
        assert_eq!(it.label(), "Vertical Line");
        // set di luar rentang → false, tanpa panic
        assert!(!h.set(9, Drawing::VerticalLine { ts_millis: 1 }));
    }

    #[test]
    fn dash_group_duplikat_dan_z_order() {
        let mut h = History::default();
        h.add(Drawing::HorizontalLine { price_ticks: 1 });
        h.add(Drawing::HorizontalLine { price_ticks: 2 });
        h.add(Drawing::HorizontalLine { price_ticks: 3 });
        // z-order: objek 0 ke depan → tuker dengan 1
        assert_eq!(h.move_item(0, 1), Some(1));
        assert_eq!(
            h.items[0].drawing,
            Drawing::HorizontalLine { price_ticks: 2 }
        );
        assert_eq!(h.move_item(0, -1), None, "sudah paling belakang");
        assert_eq!(h.move_item(9, 1), None, "indeks di luar rentang");
        // duplikat dengan offset
        let di = h.duplicate(1, 60_000, 5).unwrap();
        assert_eq!(h.items.len(), 4);
        assert_eq!(
            h.items[di].drawing,
            Drawing::HorizontalLine { price_ticks: 6 }
        );
        // grup: assign id baru + anggota
        let g1 = h.assign_group(0).unwrap();
        assert_eq!(g1, 1);
        let g2 = h.assign_group(2).unwrap();
        assert_eq!(g2, 2, "id grup berikutnya naik");
        h.items[1].group = Some(g1); // objek kedua menyusul grup 1
        assert_eq!(h.group_members(0).len(), 2);
        assert_eq!(h.group_members(2), vec![2]);
        assert_eq!(h.group_members(3), vec![3], "tanpa grup = dirinya");
        assert_eq!(h.group_members(99), Vec::<usize>::new());
        // undo mengembalikan perubahan terakhir
        assert!(h.do_undo());
    }

    #[test]
    fn teks_callout_anchor_translate_dan_hit() {
        let d = Drawing::Text {
            at: Anchor {
                ts_millis: 0,
                price_ticks: 0,
            },
            text: "halo".into(),
        };
        assert_eq!(d.kind_label(), "Text");
        assert_eq!(d.degree(), Degree::Text);
        assert_eq!(d.anchors().len(), 1);
        // proyeksi uji: x = 2·ts, y = −price → anchor di (0, 0)
        assert!(hit_test(&d, &[], &p(), (40.0, -6.0), 4.0));
        assert!(!hit_test(&d, &[], &p(), (40.0, -80.0), 4.0));
        let t = d.translated(5, 3);
        assert_eq!(t.anchors()[0].price_ticks, 3);
        if let Drawing::Text { text, .. } = &t {
            assert_eq!(text, "halo");
        } else {
            panic!("translated mengubah jenis");
        }
        let mut m = d.clone();
        assert!(m.set_anchor(
            0,
            Anchor {
                ts_millis: 9,
                price_ticks: 9
            }
        ));
        assert_eq!(m.anchors()[0].ts_millis, 9);
        let c = Drawing::Callout {
            at: Anchor {
                ts_millis: 0,
                price_ticks: 0,
            },
            text: "titik".into(),
        };
        assert_eq!(c.kind_label(), "Callout");
        assert!(hit_test(&c, &[], &p(), (5.0, -8.0), 4.0));
    }

    #[test]
    fn magnet_kuat_melewati_objek_tersembunyi() {
        let b = bars(20);
        let v = viewport(20);
        let mut hid = DrawingItem::new(Drawing::HorizontalRay {
            origin: Anchor {
                ts_millis: b[7].ts_millis,
                price_ticks: 9_999,
            },
        });
        hid.visible = false;
        let ctx = SnapContext {
            bars: &b,
            viewport: &v,
            spacing_px: 800.0 / 20.0,
            drawings: &[hid],
            tol_px: 6.0,
        };
        let mx = v.x_for_time(b[7].ts_millis);
        let my = v.y_for_price(9_999);
        let (a, _) = snap_anchor((mx, my), &ctx, Magnet::Strong);
        assert_ne!(
            a.price_ticks, 9_999,
            "anchor objek tersembunyi tidak dipakai"
        );
    }

    // ---- Batch 6: seleksi kotak (rubber band) & operasi banyak objek ----

    fn tl(a: (i64, i64), b: (i64, i64)) -> DrawingItem {
        DrawingItem::new(Drawing::Trendline {
            a: Anchor {
                ts_millis: a.0,
                price_ticks: a.1,
            },
            b: Anchor {
                ts_millis: b.0,
                price_ticks: b.1,
            },
        })
    }

    #[test]
    fn rubber_select_anchor_di_dalam_dan_segmen_memotong() {
        // Peta tes: x = ts*2, y = -price.
        let p = p();
        let items = vec![tl((10, 10), (20, 20)), tl((100, 100), (110, 110))];
        // Kotak memuat kedua anchor objek 0 (x 20..40, y -10..-20).
        assert_eq!(
            rubber_select(&items, &p, (10.0, -30.0, 50.0, -5.0)),
            vec![0]
        );
        // Kotak hanya memotong segmen (tanpa anchor di dalam).
        assert_eq!(
            rubber_select(&items, &p, (25.0, -15.0, 30.0, -12.0)),
            vec![0]
        );
        // Kotak jauh dari semua objek.
        assert!(
            rubber_select(&items, &p, (0.0, 0.0, 5.0, 5.0)).is_empty(),
            "tidak ada objek tersentuh"
        );
    }

    #[test]
    fn rubber_select_garis_panjang_dan_lewati_terkunci_tersembunyi() {
        let p = p();
        let mut locked = tl((10, 10), (20, 20));
        locked.locked = true;
        let mut hidden = tl((12, 12), (18, 18));
        hidden.visible = false;
        let items = vec![
            DrawingItem::new(Drawing::HorizontalLine { price_ticks: -15 }), // y = 15
            DrawingItem::new(Drawing::VerticalLine { ts_millis: 100 }),     // x = 200
            DrawingItem::new(Drawing::CrossLine {
                anchor: Anchor {
                    ts_millis: 50,
                    price_ticks: -5,
                },
            }),
            locked,
            hidden,
        ];
        // Kotak pada y=15 memilih hline; hline di luar rentang y tidak.
        assert_eq!(rubber_select(&items, &p, (0.0, 10.0, 5.0, 20.0)), vec![0]);
        // Kotak pada x=200 memilih vline.
        assert_eq!(rubber_select(&items, &p, (195.0, 0.0, 205.0, 5.0)), vec![1]);
        // Crossline satu anchor: hanya bila anchor masuk kotak (x=100, y=5).
        assert_eq!(rubber_select(&items, &p, (95.0, 0.0, 105.0, 8.0)), vec![2]);
        // Kotak di area objek terkunci/tersembunyi: keduanya dilewati.
        assert!(
            rubber_select(&items, &p, (10.0, -30.0, 50.0, -5.0)).is_empty(),
            "locked & hidden tidak ikut terseleksi"
        );
    }

    #[test]
    fn z_multi_menjaga_urutan_dan_tidak_bisa_maju_saat_mentok() {
        let mut h = History::default();
        for i in 0..4 {
            h.add(Drawing::VerticalLine { ts_millis: i });
        }
        h.undo.clear();
        let new = h.move_items_z(&[1, 2], 1).expect("bisa maju");
        assert_eq!(new, vec![2, 3], "indeks baru kedua objek");
        assert!(matches!(
            h.items[2].drawing,
            Drawing::VerticalLine { ts_millis: 1 }
        ));
        assert!(matches!(
            h.items[3].drawing,
            Drawing::VerticalLine { ts_millis: 2 }
        ));
        // Sudah di puncak (anggota teratas mentok) → tidak ada yang pindah,
        // dan tidak ada snapshot undo yang terbuang.
        let undo_before = h.undo.len();
        assert!(h.move_items_z(&[2, 3], 1).is_none());
        assert_eq!(h.undo.len(), undo_before);
        let back = h.move_items_z(&[2, 3], -1).expect("bisa mundur");
        assert_eq!(back, vec![1, 2]);
        assert!(matches!(
            h.items[1].drawing,
            Drawing::VerticalLine { ts_millis: 1 }
        ));
    }

    #[test]
    fn hapus_dan_duplikat_banyak_satu_snapshot() {
        let mut h = History::default();
        for i in 0..4 {
            h.add(Drawing::VerticalLine { ts_millis: i });
        }
        h.undo.clear();
        let dup = h.duplicate_many(&[0, 1], 7, 7);
        assert_eq!(dup, vec![4, 5], "salinan di atas tumpukan, urutan sama");
        assert_eq!(h.items.len(), 6);
        assert_eq!(h.undo.len(), 1, "duplikat banyak = satu langkah undo");
        assert!(matches!(
            h.items[4].drawing,
            Drawing::VerticalLine { ts_millis: 7 }
        ));
        assert!(matches!(
            h.items[5].drawing,
            Drawing::VerticalLine { ts_millis: 8 }
        ));
        h.undo.clear();
        assert_eq!(h.remove_many(&[0, 4]), 2);
        assert_eq!(h.items.len(), 4);
        assert_eq!(h.undo.len(), 1, "hapus banyak = satu langkah undo");
        assert_eq!(h.remove_many(&[]), 0);
        assert_eq!(h.undo.len(), 1, "hapus kosong tidak membuat snapshot");
    }

    #[test]
    fn grup_banyak_sekaligus_dan_lepas_grup() {
        let mut h = History::default();
        for i in 0..3 {
            h.add(Drawing::VerticalLine { ts_millis: i });
        }
        h.undo.clear();
        let g = h.assign_group_many(&[0, 2]).expect("grup terbentuk");
        assert_eq!(g, 1);
        assert_eq!(h.group_members(0), vec![0, 2]);
        assert_eq!(h.undo.len(), 1);
        assert_eq!(h.clear_group_many(&[0, 2]), 2);
        assert!(h.items[0].group.is_none() && h.items[2].group.is_none());
    }
}

#[cfg(test)]
mod batch8_tests {
    use super::*;

    fn bars(n: usize) -> Vec<Bar> {
        (0..n)
            .map(|i| Bar {
                ts_millis: 1_000 + i as i64 * 60_000,
                open_ticks: 100 + i as i64,
                high_ticks: 112 + i as i64,
                low_ticks: 88 + i as i64,
                close_ticks: 100 + i as i64,
                volume_milli: 10,
            })
            .collect()
    }

    struct P;
    impl Projector for P {
        fn x(&self, ts: i64) -> f64 {
            ts as f64
        }
        fn y(&self, t: i64) -> f64 {
            t as f64
        }
    }

    #[test]
    fn extend_pair_mengikuti_sumbu_waktu() {
        let (p0, p1) = extend_pair((0.0, 0.0), (10.0, 5.0), ExtendMode::None);
        assert_eq!((p0, p1), ((0.0, 0.0), (10.0, 5.0)));
        let (p0, p1) = extend_pair((0.0, 0.0), (10.0, 5.0), ExtendMode::Right);
        assert_eq!(p0, (0.0, 0.0));
        assert!(p1.0 > 10.0 && p1.1 > 5.0, "kanan = menaikkan x");
        let (p0, p1) = extend_pair((0.0, 0.0), (10.0, 5.0), ExtendMode::Left);
        assert!(p0.0 < 0.0 && p0.1 < 0.0);
        assert_eq!(p1, (10.0, 5.0));
        // Garis hampir vertikal: tidak ada komponen waktu → tidak diperpanjang.
        let (p0, p1) = extend_pair((3.0, 0.0), (3.0, 10.0), ExtendMode::Both);
        assert_eq!((p0, p1), ((3.0, 0.0), (3.0, 10.0)));
    }

    #[test]
    fn channel_segments_tetap_memakai_helper_extend() {
        // Regresi batch 7: hasil helper harus sama dengan semantik lama.
        let segs = channel_segments(
            ChannelKind::Parallel,
            ExtendMode::Both,
            (0.0, 0.0),
            (10.0, 10.0),
            (0.0, 20.0),
        );
        // Batas 0 diperpanjang dua arah; batas 1 sejajar lewat c.
        assert!(segs[0].0 .0 < 0.0 && segs[0].1 .0 > 10.0);
        assert!(segs[1].0 .1 < 20.0 + 0.001);
        let (b0, b1) = (segs[1].0, segs[1].1);
        assert!(b1.1 > b0.1, "batas kedua naik sejajar dasar");
    }

    #[test]
    fn regresi_linear_close_eksak() {
        let b = bars(10);
        let fit = regression_fit(&b, b[0].ts_millis, b[9].ts_millis).unwrap();
        assert_eq!(fit.start, (b[0].ts_millis, 100));
        assert_eq!(fit.end, (b[9].ts_millis, 109));
        assert!(fit.sigma < 1e-9, "sigma={}", fit.sigma);
    }

    #[test]
    fn regresi_sigma_dan_rentang_di_luar_data() {
        let mut b = bars(20);
        b[5].close_ticks += 10;
        b[15].close_ticks -= 10;
        let fit = regression_fit(&b, b[0].ts_millis, b[19].ts_millis).unwrap();
        assert!(fit.sigma > 0.0, "noise → sigma > 0");
        let fit = regression_fit(&b, -1_000_000, -500_000).unwrap();
        assert_eq!(fit.start.0, b[0].ts_millis);
        assert_eq!(fit.end.0, b[1].ts_millis);
        let hi = b[19].ts_millis;
        let fit = regression_fit(&b, hi + 1_000_000, hi + 2_000_000).unwrap();
        assert_eq!(fit.start.0, b[18].ts_millis);
        assert_eq!(fit.end.0, b[19].ts_millis);
        assert!(regression_fit(&b[..1], 0, 1).is_none());
    }

    #[test]
    fn segmen_regresi_tiga_garis_dan_extend() {
        let b = bars(10);
        let fit = regression_fit(&b, b[0].ts_millis, b[9].ts_millis).unwrap();
        let segs = regression_segments(&fit, &P, ExtendMode::None);
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[0], segs[1], "sigma 0 → garis atas = tengah");
        assert_eq!(segs[0].0, (1_000.0, 100.0));
        assert_eq!(segs[0].1, (541_000.0, 109.0));
        let segs = regression_segments(&fit, &P, ExtendMode::Right);
        assert!(segs[0].1 .0 > 541_000.0);
    }

    #[test]
    fn fib_trend_level_dari_c() {
        assert_eq!(fib_ext_price(100, 200, 90, 1000), 190);
        assert_eq!(fib_ext_price(100, 200, 90, 618), 90 + 62);
        assert_eq!(fib_ext_price(100, 200, 90, 0), 90);
    }

    #[test]
    fn gann_fan_sembilan_garis_dan_rasio() {
        let lines = gann_fan_lines((0.0, 0.0), (100.0, 50.0));
        assert_eq!(lines.len(), 9);
        let far = |k: &str| lines.iter().find(|(l, _, _)| *l == k).unwrap().2;
        assert!(far("1x1").0 > 0.0, "arah kanan: x naik");
        assert!(far("2x1").1 > far("1x1").1, "2x1 lebih curam dari 1x1");
        assert!(far("1x2").1 < far("1x1").1, "1x2 lebih datar dari 1x1");
        // Drag vertikal: semua garis tetap vertikal (tidak ada x).
        let lines = gann_fan_lines((5.0, 5.0), (5.0, 55.0));
        assert!((lines[4].2 .0 - 5.0).abs() < 1e-6);
    }

    #[test]
    fn pola_label_anchor_dan_transformasi() {
        let pts: Vec<Anchor> = (0..5)
            .map(|i| Anchor {
                ts_millis: i as i64 * 100,
                price_ticks: 100 + i as i64,
            })
            .collect();
        let d = Drawing::ElliottImpulse {
            points: pts.clone(),
        };
        assert_eq!(d.pattern_labels(), Some(&["1", "2", "3", "4", "5"][..]));
        assert_eq!(d.degree(), Degree::Pattern);
        assert_eq!(d.anchors(), pts);
        assert_eq!(d.kind_label(), "Elliott Impulse Wave");
        let mut d2 = d.clone();
        assert!(d2.set_anchor(
            3,
            Anchor {
                ts_millis: 9,
                price_ticks: 9
            }
        ));
        assert!(!d2.set_anchor(
            9,
            Anchor {
                ts_millis: 9,
                price_ticks: 9
            }
        ));
        let t = d2.translated(10, 5);
        assert_eq!(
            t.anchors()[3],
            Anchor {
                ts_millis: 19,
                price_ticks: 14
            }
        );
        assert_eq!(
            Drawing::Xabcd { points: vec![] }.pattern_labels(),
            Some(&["X", "A", "B", "C", "D"][..])
        );
        assert_eq!(Drawing::Brush { points: vec![] }.pattern_labels(), None);
    }

    #[test]
    fn sticker_dan_brush_geometri() {
        assert_eq!(StickerIcon::from_key("star"), Some(StickerIcon::Star));
        assert_eq!(StickerIcon::from_key("nope"), None);
        assert_eq!(StickerIcon::all().len(), 14);
        assert_eq!(StickerIcon::Star.key(), "star");
        let d = Drawing::Sticker {
            at: Anchor {
                ts_millis: 5,
                price_ticks: 7,
            },
            icon: StickerIcon::Star,
        };
        assert_eq!(d.anchors().len(), 1);
        assert_eq!(d.kind_label(), "Sticker");
        assert_eq!(d.degree(), Degree::Sticker);
        let b = Drawing::Brush {
            points: vec![
                Anchor {
                    ts_millis: 0,
                    price_ticks: 0,
                },
                Anchor {
                    ts_millis: 10,
                    price_ticks: 10,
                },
            ],
        };
        assert_eq!(b.anchors().len(), 2);
        assert_eq!(
            b.translated(1, 2).anchors()[1],
            Anchor {
                ts_millis: 11,
                price_ticks: 12
            }
        );
        assert_eq!(b.degree(), Degree::Stroke);
    }

    #[test]
    fn hit_test_objek_batch8() {
        let b = bars(10);
        let brush = Drawing::Brush {
            points: vec![
                Anchor {
                    ts_millis: 0,
                    price_ticks: 0,
                },
                Anchor {
                    ts_millis: 100,
                    price_ticks: 100,
                },
            ],
        };
        assert!(hit_test(&brush, &b, &P, (50.0, 50.0), 2.0));
        assert!(!hit_test(&brush, &b, &P, (50.0, 90.0), 2.0));
        let st = Drawing::Sticker {
            at: Anchor {
                ts_millis: 10,
                price_ticks: 10,
            },
            icon: StickerIcon::Circle,
        };
        assert!(hit_test(&st, &b, &P, (16.0, 16.0), 2.0));
        assert!(!hit_test(&st, &b, &P, (40.0, 40.0), 2.0));
        let fe = Drawing::FibTrendExtension {
            a: Anchor {
                ts_millis: 0,
                price_ticks: 100,
            },
            b: Anchor {
                ts_millis: 200,
                price_ticks: 300,
            },
            c: Anchor {
                ts_millis: 400,
                price_ticks: 250,
            },
        };
        // Level 100% = c + (b−a) = 450; hanya kena di kanan x(c).
        assert!(hit_test(&fe, &b, &P, (500.0, 450.0), 3.0));
        assert!(!hit_test(&fe, &b, &P, (300.0, 450.0), 3.0));
        let gf = Drawing::GannFan {
            a: Anchor {
                ts_millis: 0,
                price_ticks: 0,
            },
            b: Anchor {
                ts_millis: 10,
                price_ticks: 10,
            },
        };
        assert!(hit_test(&gf, &b, &P, (100.0, 100.0), 2.0));
        // Regresi: garis close eksak; extend kanan menambah jangkauan klik.
        let rc = Drawing::RegressionChannel {
            a: Anchor {
                ts_millis: b[0].ts_millis,
                price_ticks: 0,
            },
            b: Anchor {
                ts_millis: b[9].ts_millis,
                price_ticks: 0,
            },
        };
        let y = |x: f64| 100.0 + (x - 1_000.0) * 9.0 / 540_000.0;
        assert!(hit_test(&rc, &b, &P, (241_000.0, y(241_000.0)), 1.0));
        let mut it = DrawingItem::new(rc);
        assert!(!hit_test_item(&it, &b, &P, (560_000.0, y(560_000.0)), 1.0));
        it.extend = ExtendMode::Right;
        assert!(hit_test_item(&it, &b, &P, (560_000.0, y(560_000.0)), 1.0));
    }

    #[test]
    fn item_meta_teks_default_dan_varian() {
        let it = DrawingItem::new(Drawing::Text {
            at: Anchor {
                ts_millis: 1,
                price_ticks: 2,
            },
            text: "halo".into(),
        });
        assert_eq!(it.text_size, TextSize::M);
        assert_eq!(it.text_align, TextAlign::Left);
        assert_eq!(TextSize::from_key("l"), Some(TextSize::L));
        assert_eq!(TextAlign::from_key("kanan"), None);
        assert_eq!(TextAlign::all().len(), 3);
    }
}
