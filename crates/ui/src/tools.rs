// SokanLab ui — alat gambar interaktif + peta pintasan keyboard.
// Keputusan desain: klik-klik (atau seret sekali) dengan pratinjau; Esc membatalkan.
// Pintasan yang sama dengan TradingView ditandai di `hotkey_hint`; sisanya orisinal.
use sokanlab_chart::drawing::{
    Anchor, ChannelKind, Drawing, Magnet, StickerIcon, MAX_STROKE_POINTS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Select,
    Trend,
    Ray,
    ExtendedLine,
    HorizontalLine,
    HorizontalRay,
    VerticalLine,
    CrossLine,
    Arrow,
    Rectangle,
    Ellipse,
    Triangle,
    FibRetracement,
    RangeMeasure,
    LongPosition,
    ShortPosition,
    /// Channel paralel (3 anchor: dasar a→b + lebar lewat c).
    ParallelChannel,
    /// Channel disjoint (3 anchor; batas kedua = salinan dasar yang digeser).
    DisjointChannel,
    /// Channel flat top/bottom (3 anchor; batas kedua horizontal di harga c).
    FlatChannel,
    /// Kanal regresi (2 anchor: rentang bar; garis regresi + batas ±σ).
    RegressionChannel,
    /// Trend-based fib extension (3 anchor: trendline + level dari titik ketiga).
    FibTrendExtension,
    /// Elliott impulse wave 1·2·3·4·5 (5 anchor).
    ElliottImpulse,
    /// Elliott correction wave A·B·C (3 anchor).
    ElliottCorrection,
    /// XABCD pattern (5 anchor).
    Xabcd,
    /// ABCD pattern (4 anchor).
    Abcd,
    /// Gann fan (2 anchor; sembilan sudut 1x8 … 8x1).
    GannFan,
    /// Brush: garis bebas hasil seret (titik disampel saat seret).
    Brush,
    /// Highlighter: garis bebas tebal semi-transparan.
    Highlighter,
    /// Sticker: ikon vektor pada satu klik.
    Sticker,
    /// Pitchfork Andrew (3 anchor).
    Pitchfork,
    /// Fibonacci extension (3 anchor: rentang a→b diproyeksikan dari c).
    FibExtension,
    /// Fibonacci time (1 anchor; garis di bar +1,+2,+3,+5,+8,…).
    FibTime,
    /// Catatan teks (1 anchor; isi teks diedit dari panel properti).
    Text,
    /// Callout: teks + ekor penunjuk (1 anchor).
    Callout,
    /// Alat ukur cepat: tidak pernah disimpan, hanya pratinjau sampai klik berikutnya.
    Measure,
}

impl ToolKind {
    pub const ALL: &'static [ToolKind] = &[
        ToolKind::Select,
        ToolKind::Trend,
        ToolKind::Ray,
        ToolKind::ExtendedLine,
        ToolKind::HorizontalLine,
        ToolKind::HorizontalRay,
        ToolKind::VerticalLine,
        ToolKind::CrossLine,
        ToolKind::Arrow,
        ToolKind::Rectangle,
        ToolKind::Ellipse,
        ToolKind::Triangle,
        ToolKind::FibRetracement,
        ToolKind::RangeMeasure,
        ToolKind::LongPosition,
        ToolKind::ShortPosition,
        ToolKind::ParallelChannel,
        ToolKind::DisjointChannel,
        ToolKind::FlatChannel,
        ToolKind::RegressionChannel,
        ToolKind::Pitchfork,
        ToolKind::FibExtension,
        ToolKind::FibTrendExtension,
        ToolKind::FibTime,
        ToolKind::ElliottImpulse,
        ToolKind::ElliottCorrection,
        ToolKind::Xabcd,
        ToolKind::Abcd,
        ToolKind::GannFan,
        ToolKind::Text,
        ToolKind::Callout,
        ToolKind::Brush,
        ToolKind::Highlighter,
        ToolKind::Sticker,
        ToolKind::Measure,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ToolKind::Select => "Select / Pan",
            ToolKind::Trend => "Trend Line",
            ToolKind::Ray => "Ray",
            ToolKind::ExtendedLine => "Extended Line",
            ToolKind::HorizontalLine => "Horizontal Line",
            ToolKind::HorizontalRay => "Horizontal Ray",
            ToolKind::VerticalLine => "Vertical Line",
            ToolKind::CrossLine => "Crossline",
            ToolKind::Arrow => "Arrow",
            ToolKind::Rectangle => "Rectangle",
            ToolKind::Ellipse => "Ellipse",
            ToolKind::Triangle => "Triangle",
            ToolKind::FibRetracement => "Fib Retracement",
            ToolKind::RangeMeasure => "Date & Price Range",
            ToolKind::LongPosition => "Long Position",
            ToolKind::ShortPosition => "Short Position",
            ToolKind::ParallelChannel => "Parallel Channel",
            ToolKind::DisjointChannel => "Disjoint Channel",
            ToolKind::FlatChannel => "Flat Top/Bottom Channel",
            ToolKind::RegressionChannel => "Regression Channel",
            ToolKind::Pitchfork => "Pitchfork",
            ToolKind::FibExtension => "Fib Extension",
            ToolKind::FibTrendExtension => "Trend-Based Fib Extension",
            ToolKind::FibTime => "Fib Time",
            ToolKind::ElliottImpulse => "Elliott Impulse Wave (1-5)",
            ToolKind::ElliottCorrection => "Elliott Correction Wave (A-C)",
            ToolKind::Xabcd => "XABCD Pattern",
            ToolKind::Abcd => "ABCD Pattern",
            ToolKind::GannFan => "Gann Fan",
            ToolKind::Text => "Text",
            ToolKind::Callout => "Callout",
            ToolKind::Brush => "Brush",
            ToolKind::Highlighter => "Highlighter",
            ToolKind::Sticker => "Sticker",
            ToolKind::Measure => "Measure (quick)",
        }
    }

    /// Label pendek untuk tombol toolbar (2–3 huruf, orisinal).
    pub fn short(self) -> &'static str {
        match self {
            ToolKind::Select => "SEL",
            ToolKind::Trend => "TL",
            ToolKind::Ray => "RY",
            ToolKind::ExtendedLine => "EX",
            ToolKind::HorizontalLine => "HL",
            ToolKind::HorizontalRay => "HR",
            ToolKind::VerticalLine => "VL",
            ToolKind::CrossLine => "XL",
            ToolKind::Arrow => "AR",
            ToolKind::Rectangle => "RC",
            ToolKind::Ellipse => "EL",
            ToolKind::Triangle => "TR",
            ToolKind::FibRetracement => "FB",
            ToolKind::RangeMeasure => "RNG",
            ToolKind::LongPosition => "LNG",
            ToolKind::ShortPosition => "SHT",
            ToolKind::ParallelChannel => "CH",
            ToolKind::DisjointChannel => "DJ",
            ToolKind::FlatChannel => "FL",
            ToolKind::RegressionChannel => "REG",
            ToolKind::Pitchfork => "PF",
            ToolKind::FibExtension => "FE",
            ToolKind::FibTrendExtension => "FTX",
            ToolKind::FibTime => "FT",
            ToolKind::ElliottImpulse => "EI5",
            ToolKind::ElliottCorrection => "EC3",
            ToolKind::Xabcd => "XAB",
            ToolKind::Abcd => "ABC",
            ToolKind::GannFan => "GF",
            ToolKind::Text => "TXT",
            ToolKind::Callout => "CAL",
            ToolKind::Brush => "BR",
            ToolKind::Highlighter => "HI",
            ToolKind::Sticker => "STK",
            ToolKind::Measure => "MEA",
        }
    }

    /// Pintasan keyboard; sama dengan TradingView bila alat ini ada di sana.
    pub fn hotkey_hint(self) -> &'static str {
        match self {
            ToolKind::Trend => "Alt+T",
            ToolKind::HorizontalLine => "Alt+H",
            ToolKind::VerticalLine => "Alt+V",
            ToolKind::CrossLine => "Alt+C",
            ToolKind::FibRetracement => "Alt+F",
            _ => "",
        }
    }

    pub fn needed(self) -> usize {
        match self {
            ToolKind::Select => 0,
            ToolKind::HorizontalLine
            | ToolKind::HorizontalRay
            | ToolKind::VerticalLine
            | ToolKind::CrossLine => 1,
            ToolKind::Triangle
            | ToolKind::ParallelChannel
            | ToolKind::DisjointChannel
            | ToolKind::FlatChannel
            | ToolKind::Pitchfork
            | ToolKind::FibExtension
            | ToolKind::FibTrendExtension
            | ToolKind::ElliottCorrection => 3,
            ToolKind::ElliottImpulse | ToolKind::Xabcd => 5,
            ToolKind::Abcd => 4,
            ToolKind::FibTime | ToolKind::Text | ToolKind::Callout | ToolKind::Sticker => 1,
            // Brush/highlighter tidak memakai hitungan klik (diseret).
            ToolKind::Brush | ToolKind::Highlighter => 0,
            _ => 2,
        }
    }

    pub fn is_draw_mode(self) -> bool {
        self != ToolKind::Select
    }

    /// Garis bebas: dibuat dengan menyeret (titik disampel), bukan klik-klik.
    pub fn is_stroke(self) -> bool {
        matches!(self, ToolKind::Brush | ToolKind::Highlighter)
    }

    /// Bangun garis bebas dari titik hasil seret (≥ 2 titik).
    pub fn build_stroke(self, points: &[Anchor]) -> Option<Drawing> {
        if points.len() < 2 {
            return None;
        }
        let pts: Vec<Anchor> = points.iter().take(MAX_STROKE_POINTS).copied().collect();
        Some(match self {
            ToolKind::Brush => Drawing::Brush { points: pts },
            ToolKind::Highlighter => Drawing::Highlighter { points: pts },
            _ => return None,
        })
    }

    /// Bangun drawing dari anchor terkumpul (urutan klik).
    pub fn build(self, anchors: &[Anchor]) -> Option<Drawing> {
        // Ambil anchor sesuai kebutuhan tiap alat (jangan eager `?` — alat satu
        // anchor tidak butuh a1/a2).
        let a0 = || anchors.first().copied();
        let a1 = || anchors.get(1).copied();
        let a2 = || anchors.get(2).copied();
        Some(match self {
            ToolKind::Select | ToolKind::Measure => return None,
            ToolKind::Trend => Drawing::Trendline { a: a0()?, b: a1()? },
            ToolKind::Ray => Drawing::Ray {
                origin: a0()?,
                through: a1()?,
            },
            ToolKind::ExtendedLine => Drawing::ExtendedLine { a: a0()?, b: a1()? },
            ToolKind::HorizontalLine => Drawing::HorizontalLine {
                price_ticks: a0()?.price_ticks,
            },
            ToolKind::HorizontalRay => Drawing::HorizontalRay { origin: a0()? },
            ToolKind::VerticalLine => Drawing::VerticalLine {
                ts_millis: a0()?.ts_millis,
            },
            ToolKind::CrossLine => Drawing::CrossLine { anchor: a0()? },
            ToolKind::Arrow => Drawing::Arrow { a: a0()?, b: a1()? },
            ToolKind::Rectangle => Drawing::Rectangle { a: a0()?, b: a1()? },
            ToolKind::Ellipse => Drawing::Ellipse { a: a0()?, b: a1()? },
            ToolKind::Triangle => Drawing::Triangle {
                a: a0()?,
                b: a1()?,
                c: a2()?,
            },
            ToolKind::FibRetracement => Drawing::FibRetracement { a: a0()?, b: a1()? },
            ToolKind::RangeMeasure => Drawing::RangeMeasure { a: a0()?, b: a1()? },
            ToolKind::LongPosition => Drawing::Position {
                long: true,
                entry: a0()?,
                target: a1()?,
            },
            ToolKind::ShortPosition => Drawing::Position {
                long: false,
                entry: a0()?,
                target: a1()?,
            },
            ToolKind::ParallelChannel => Drawing::ParallelChannel {
                a: a0()?,
                b: a1()?,
                c: a2()?,
                kind: ChannelKind::Parallel,
            },
            ToolKind::DisjointChannel => Drawing::ParallelChannel {
                a: a0()?,
                b: a1()?,
                c: a2()?,
                kind: ChannelKind::Disjoint,
            },
            ToolKind::FlatChannel => Drawing::ParallelChannel {
                a: a0()?,
                b: a1()?,
                c: a2()?,
                kind: ChannelKind::Flat,
            },
            ToolKind::RegressionChannel => Drawing::RegressionChannel { a: a0()?, b: a1()? },
            ToolKind::FibTrendExtension => Drawing::FibTrendExtension {
                a: a0()?,
                b: a1()?,
                c: a2()?,
            },
            ToolKind::ElliottImpulse => {
                if anchors.len() < 5 {
                    return None;
                }
                Drawing::ElliottImpulse {
                    points: anchors.iter().take(5).copied().collect(),
                }
            }
            ToolKind::ElliottCorrection => {
                if anchors.len() < 3 {
                    return None;
                }
                Drawing::ElliottCorrection {
                    points: anchors.iter().take(3).copied().collect(),
                }
            }
            ToolKind::Xabcd => {
                if anchors.len() < 5 {
                    return None;
                }
                Drawing::Xabcd {
                    points: anchors.iter().take(5).copied().collect(),
                }
            }
            ToolKind::Abcd => {
                if anchors.len() < 4 {
                    return None;
                }
                Drawing::Abcd {
                    points: anchors.iter().take(4).copied().collect(),
                }
            }
            ToolKind::GannFan => Drawing::GannFan { a: a0()?, b: a1()? },
            ToolKind::Brush | ToolKind::Highlighter => return None,
            ToolKind::Sticker => Drawing::Sticker {
                at: a0()?,
                icon: StickerIcon::default(),
            },
            ToolKind::Pitchfork => Drawing::Pitchfork {
                a: a0()?,
                b: a1()?,
                c: a2()?,
            },
            ToolKind::FibExtension => Drawing::FibExtension {
                a: a0()?,
                b: a1()?,
                c: a2()?,
            },
            ToolKind::FibTime => Drawing::FibTime { a: a0()? },
            ToolKind::Text => Drawing::Text {
                at: a0()?,
                text: String::new(),
            },
            ToolKind::Callout => Drawing::Callout {
                at: a0()?,
                text: String::new(),
            },
        })
    }
}

/// State alat aktif + anchor yang sedang ditempatkan.
#[derive(Debug, Clone)]
pub struct ToolState {
    pub kind: ToolKind,
    /// Tetap di mode gambar setelah satu objek selesai (TV: "stay in drawing mode").
    pub stay: bool,
    pub magnet: Magnet,
    /// Anchor yang sudah diklik untuk objek yang sedang dibuat.
    pub pending: Vec<Anchor>,
    /// Hasil ukur cepat terakhir (Measure) — tampil sampai klik berikutnya.
    pub last_measure: Option<(Anchor, Anchor)>,
    /// Posisi hover terakhir (untuk pratinjau).
    pub hover: Option<Anchor>,
    /// Hover sedang menempel ke data (indikator visual magnet).
    pub hover_snapped: bool,
}

impl Default for ToolState {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolState {
    pub fn new() -> Self {
        Self {
            kind: ToolKind::Select,
            stay: false,
            magnet: Magnet::Off,
            pending: Vec::new(),
            last_measure: None,
            hover: None,
            hover_snapped: false,
        }
    }

    pub fn select(&mut self, kind: ToolKind) {
        if self.kind != kind {
            self.pending.clear();
        }
        self.kind = kind;
    }

    /// Esc: batalkan penempatan/last measure → keluar mode gambar → sinyal deselect.
    pub fn cancel(&mut self) -> bool {
        if !self.pending.is_empty() || self.last_measure.is_some() {
            self.pending.clear();
            self.last_measure = None;
            return true;
        }
        if self.kind != ToolKind::Select {
            self.kind = ToolKind::Select;
            return true;
        }
        false
    }

    /// Hapus anchor terakhir saat menempatkan (Backspace).
    pub fn undo_anchor(&mut self) -> bool {
        self.pending.pop().is_some()
    }

    /// Proses satu klik. Mengembalikan drawing bila objek selesai dibangun.
    pub fn click(&mut self, a: Anchor) -> Option<Drawing> {
        match self.kind {
            ToolKind::Select => None,
            // Garis bebas tidak memakai jalur klik: titik disampel saat seret
            // (lihat PlaceDrag) — klik diabaikan tanpa mengubah state.
            ToolKind::Brush | ToolKind::Highlighter => None,
            ToolKind::Measure => {
                if self.pending.is_empty() {
                    self.last_measure = None;
                    self.pending.push(a);
                } else {
                    let first = self.pending[0];
                    self.last_measure = Some((first, a));
                    self.pending.clear();
                }
                None
            }
            kind => {
                self.pending.push(a);
                if self.pending.len() >= kind.needed() {
                    let d = kind.build(&self.pending);
                    self.pending.clear();
                    if d.is_some() && !self.stay {
                        self.kind = ToolKind::Select;
                    }
                    d
                } else {
                    None
                }
            }
        }
    }

    /// Objek pratinjau dari anchor terkumpul + posisi hover.
    pub fn preview(&self, hover: Anchor) -> Option<Drawing> {
        if self.kind == ToolKind::Measure {
            if let Some(first) = self.pending.first() {
                return ToolKind::RangeMeasure.build(&[*first, hover]);
            }
            if let Some((a, b)) = self.last_measure {
                return ToolKind::RangeMeasure.build(&[a, b]);
            }
            return None;
        }
        if self.kind.is_stroke() {
            // Garis bebas: pratinjau = titik tersampel + posisi hover.
            if self.pending.is_empty() {
                return None;
            }
            let mut pts = self.pending.clone();
            pts.push(hover);
            return self.kind.build_stroke(&pts);
        }
        if self.pending.is_empty() {
            return None;
        }
        if self.pending.len() + 1 < self.kind.needed() {
            return None; // triangle butuh 2 klik dulu; pratinjau setelah itu
        }
        let mut anchors = self.pending.clone();
        anchors.push(hover);
        self.kind.build(&anchors)
    }
}

/// Aksi tingkat aplikasi hasil pemetaan tombol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Tool(ToolKind),
    Undo,
    Redo,
    DeleteSelected,
    Cancel,
    ResetView,
    SaveLayout,
    ToggleHideAll,
    ZoomIn,
    ZoomOut,
    PanBars(i64),
    OpenShortcuts,
    /// Dialog "pergi ke tanggal/waktu" (Alt+G).
    OpenGoToDate,
    /// Simpan snapshot PNG chart (Alt+S; padanan TV snapshot).
    Snapshot,
}

fn plain(m: egui::Modifiers) -> bool {
    !m.command && !m.alt && !m.shift
}

fn ctrl_only(m: egui::Modifiers) -> bool {
    m.command && !m.alt && !m.shift
}

fn alt_only(m: egui::Modifiers) -> bool {
    m.alt && !m.command && !m.shift
}

/// Peta tombol → aksi. Hanya kombinasi yang dikenali yang mengembalikan Some.
pub fn action_for(key: egui::Key, mods: egui::Modifiers) -> Option<Action> {
    use egui::Key;
    // Chart (sama dengan TradingView bila ada padanannya).
    if plain(mods) {
        match key {
            Key::Escape => return Some(Action::Cancel),
            Key::Delete | Key::Backspace => return Some(Action::DeleteSelected),
            Key::ArrowLeft => return Some(Action::PanBars(-1)),
            Key::ArrowRight => return Some(Action::PanBars(1)),
            Key::F1 => return Some(Action::OpenShortcuts),
            Key::Num1 => return Some(Action::Tool(ToolKind::ALL[0])),
            Key::Num2 => return Some(Action::Tool(ToolKind::ALL[1])),
            Key::Num3 => return Some(Action::Tool(ToolKind::ALL[2])),
            Key::Num4 => return Some(Action::Tool(ToolKind::ALL[3])),
            Key::Num5 => return Some(Action::Tool(ToolKind::ALL[4])),
            Key::Num6 => return Some(Action::Tool(ToolKind::ALL[5])),
            Key::Num7 => return Some(Action::Tool(ToolKind::ALL[6])),
            Key::Num8 => return Some(Action::Tool(ToolKind::ALL[7])),
            Key::Num9 => return Some(Action::Tool(ToolKind::ALL[8])),
            _ => {}
        }
    }
    if ctrl_only(mods) {
        match key {
            Key::Z => return Some(Action::Undo),
            Key::Y => return Some(Action::Redo),
            Key::S => return Some(Action::SaveLayout),
            Key::ArrowLeft => return Some(Action::PanBars(-10)),
            Key::ArrowRight => return Some(Action::PanBars(10)),
            Key::ArrowUp => return Some(Action::ZoomIn),
            Key::ArrowDown => return Some(Action::ZoomOut),
            _ => {}
        }
    }
    if mods.command && mods.shift && !mods.alt && key == Key::Z {
        return Some(Action::Redo);
    }
    if mods.command && mods.alt && !mods.shift && key == Key::H {
        return Some(Action::ToggleHideAll);
    }
    if alt_only(mods) {
        match key {
            Key::R => return Some(Action::ResetView),
            Key::T => return Some(Action::Tool(ToolKind::Trend)),
            Key::H => return Some(Action::Tool(ToolKind::HorizontalLine)),
            Key::V => return Some(Action::Tool(ToolKind::VerticalLine)),
            Key::C => return Some(Action::Tool(ToolKind::CrossLine)),
            Key::F => return Some(Action::Tool(ToolKind::FibRetracement)),
            Key::G => return Some(Action::OpenGoToDate),
            Key::S => return Some(Action::Snapshot),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use sokanlab_chart::drawing::Drawing;

    fn anchor(t: i64, p: i64) -> Anchor {
        Anchor {
            ts_millis: t,
            price_ticks: p,
        }
    }

    #[test]
    fn klik_dua_kali_membangun_trendline() {
        let mut t = ToolState::new();
        t.select(ToolKind::Trend);
        assert!(t.click(anchor(1, 2)).is_none());
        let d = t.click(anchor(3, 4)).expect("trendline selesai");
        assert_eq!(
            d,
            Drawing::Trendline {
                a: anchor(1, 2),
                b: anchor(3, 4)
            }
        );
        assert_eq!(t.kind, ToolKind::Select); // kembali ke Select
        assert!(t.pending.is_empty());
    }

    #[test]
    fn stay_mode_tetap_di_alat() {
        let mut t = ToolState::new();
        t.stay = true;
        t.select(ToolKind::Rectangle);
        t.click(anchor(0, 0));
        let d = t.click(anchor(1, 1));
        assert!(d.is_some());
        assert_eq!(t.kind, ToolKind::Rectangle);
    }

    #[test]
    fn alat_satu_klik_langsung_selesai() {
        for kind in [
            ToolKind::HorizontalLine,
            ToolKind::HorizontalRay,
            ToolKind::VerticalLine,
            ToolKind::CrossLine,
            ToolKind::Text,
            ToolKind::Callout,
        ] {
            let mut t = ToolState::new();
            t.select(kind);
            assert!(t.click(anchor(7, 9)).is_some(), "{kind:?}");
            assert!(t.pending.is_empty());
        }
    }

    #[test]
    fn channel_pitchfork_fibext_tiga_klik_dan_fibtime_satu_klik() {
        for kind in [
            ToolKind::ParallelChannel,
            ToolKind::DisjointChannel,
            ToolKind::FlatChannel,
            ToolKind::Pitchfork,
            ToolKind::FibExtension,
        ] {
            let mut t = ToolState::new();
            t.select(kind);
            assert!(t.click(anchor(0, 0)).is_none(), "{kind:?}");
            assert!(t.click(anchor(1, 0)).is_none(), "{kind:?}");
            let d = t.click(anchor(1, 1)).expect("objek 3 anchor");
            assert_eq!(d.anchors().len(), 3, "{kind:?}");
        }
        // Alat channel memetakan varian masing-masing.
        for (tool, want) in [
            (ToolKind::ParallelChannel, ChannelKind::Parallel),
            (ToolKind::DisjointChannel, ChannelKind::Disjoint),
            (ToolKind::FlatChannel, ChannelKind::Flat),
        ] {
            let d = tool
                .build(&[anchor(0, 0), anchor(1, 0), anchor(1, 1)])
                .expect("channel");
            match d {
                Drawing::ParallelChannel { kind, .. } => assert_eq!(kind, want, "{tool:?}"),
                other => panic!("bukan channel: {other:?}"),
            }
        }
        let mut t = ToolState::new();
        t.select(ToolKind::FibTime);
        let d = t.click(anchor(5, 5)).expect("fib time satu klik");
        assert!(matches!(d, Drawing::FibTime { .. }));
    }

    #[test]
    fn triangle_tiga_klik() {
        let mut t = ToolState::new();
        t.select(ToolKind::Triangle);
        assert!(t.click(anchor(0, 0)).is_none());
        assert!(t.click(anchor(1, 0)).is_none());
        let d = t.click(anchor(1, 1)).expect("triangle");
        assert!(matches!(d, Drawing::Triangle { .. }));
    }

    #[test]
    fn semua_alat_membangun_dari_anchor() {
        let a = [
            anchor(0, 100),
            anchor(10, 50),
            anchor(5, 80),
            anchor(7, 60),
            anchor(3, 70),
        ];
        for kind in ToolKind::ALL {
            let d = kind.build(&a);
            match kind {
                ToolKind::Select => assert!(d.is_none()),
                // Measure tidak disimpan, build-nya None; pratinjau memakai RangeMeasure.
                ToolKind::Measure => assert!(d.is_none()),
                // Brush/highlighter dibuat dari seret (`build_stroke`), bukan klik.
                ToolKind::Brush | ToolKind::Highlighter => assert!(d.is_none()),
                _ => assert!(d.is_some(), "gagal build {kind:?}"),
            }
        }
        // jumlah anchor kurang → None, bukan panic
        for kind in ToolKind::ALL {
            assert!(
                kind.build(&[]).is_none()
                    || *kind == ToolKind::Select
                    || *kind == ToolKind::Measure
                    || *kind == ToolKind::Brush
                    || *kind == ToolKind::Highlighter
            );
        }
    }

    #[test]
    fn garis_bebas_dibangun_dari_seret() {
        let pts = [anchor(0, 100), anchor(4, 90), anchor(9, 95)];
        assert!(ToolKind::Brush.build_stroke(&pts).is_some());
        assert!(ToolKind::Highlighter.build_stroke(&pts).is_some());
        // < 2 titik → bukan goresan.
        assert!(ToolKind::Brush.build_stroke(&pts[..1]).is_none());
        // klik diabaikan (titik disampel hanya saat seret).
        let mut t = ToolState::new();
        t.select(ToolKind::Brush);
        assert!(t.click(anchor(0, 100)).is_none());
        assert!(t.pending.is_empty());
        // pratinjau memakai titik tersampel + posisi hover.
        t.pending.push(anchor(0, 100));
        assert!(matches!(
            t.preview(anchor(2, 90)),
            Some(Drawing::Brush { .. })
        ));
    }

    #[test]
    fn sticker_satu_klik() {
        let mut t = ToolState::new();
        t.select(ToolKind::Sticker);
        let d = t.click(anchor(5, 5)).expect("sticker satu klik");
        match d {
            Drawing::Sticker { at, icon } => {
                assert_eq!(at.price_ticks, 5);
                assert_eq!(icon, sokanlab_chart::drawing::StickerIcon::ArrowUp);
            }
            other => panic!("bukan sticker: {other:?}"),
        }
    }

    #[test]
    fn esc_membatalkan_penempatan_lalu_keluar_mode() {
        let mut t = ToolState::new();
        t.select(ToolKind::Trend);
        t.click(anchor(1, 1));
        assert!(t.cancel()); // batalkan pending
        assert_eq!(t.kind, ToolKind::Trend);
        assert!(t.cancel()); // keluar mode gambar
        assert_eq!(t.kind, ToolKind::Select);
        assert!(!t.cancel()); // tidak ada apa-apa lagi → app deselect
    }

    #[test]
    fn measure_tidak_pernah_disimpan() {
        let mut t = ToolState::new();
        t.select(ToolKind::Measure);
        assert!(t.click(anchor(1, 1)).is_none());
        assert!(t.click(anchor(2, 2)).is_none());
        assert_eq!(t.last_measure, Some((anchor(1, 1), anchor(2, 2))));
        assert!(t.preview(anchor(9, 9)).is_some());
        // klik berikutnya memulai pengukuran baru
        t.click(anchor(5, 5));
        assert!(t.last_measure.is_none());
        assert!(t.preview(anchor(6, 6)).is_some());
    }

    #[test]
    fn preview_mengikuti_hover() {
        let mut t = ToolState::new();
        t.select(ToolKind::FibRetracement);
        assert!(t.preview(anchor(1, 1)).is_none()); // belum ada anchor
        t.click(anchor(0, 100));
        let d = t.preview(anchor(10, 50)).expect("pratinjau fib");
        match d {
            Drawing::FibRetracement { a, b } => {
                assert_eq!(a, anchor(0, 100));
                assert_eq!(b, anchor(10, 50));
            }
            other => panic!("jenis salah: {other:?}"),
        }
    }

    #[test]
    fn undo_anchor_saat_menempatkan() {
        let mut t = ToolState::new();
        t.select(ToolKind::Triangle);
        t.click(anchor(1, 1));
        t.click(anchor(2, 2));
        assert!(t.undo_anchor());
        assert_eq!(t.pending.len(), 1);
        assert!(!t.undo_anchor() || t.pending.is_empty());
    }

    #[test]
    fn hotkey_sesuai_tradingview() {
        use egui::{Key, Modifiers};
        let alt = Modifiers {
            alt: true,
            ..Default::default()
        };
        assert_eq!(action_for(Key::T, alt), Some(Action::Tool(ToolKind::Trend)));
        assert_eq!(
            action_for(Key::H, alt),
            Some(Action::Tool(ToolKind::HorizontalLine))
        );
        assert_eq!(
            action_for(Key::V, alt),
            Some(Action::Tool(ToolKind::VerticalLine))
        );
        assert_eq!(
            action_for(Key::C, alt),
            Some(Action::Tool(ToolKind::CrossLine))
        );
        assert_eq!(
            action_for(Key::F, alt),
            Some(Action::Tool(ToolKind::FibRetracement))
        );
        assert_eq!(action_for(Key::R, alt), Some(Action::ResetView));
        // Alt+G = go to date (sama dengan TV).
        assert_eq!(action_for(Key::G, alt), Some(Action::OpenGoToDate));
        assert_eq!(action_for(Key::S, alt), Some(Action::Snapshot));
    }

    #[test]
    fn hotkey_chart_dan_undo_redo() {
        use egui::{Key, Modifiers};
        let none = Modifiers::default();
        let ctrl = Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        let ctrl_shift = Modifiers {
            ctrl: true,
            command: true,
            shift: true,
            ..Default::default()
        };
        let ctrl_alt = Modifiers {
            ctrl: true,
            command: true,
            alt: true,
            ..Default::default()
        };
        assert_eq!(action_for(Key::Z, ctrl), Some(Action::Undo));
        assert_eq!(action_for(Key::Z, ctrl_shift), Some(Action::Redo));
        assert_eq!(action_for(Key::Y, ctrl), Some(Action::Redo));
        assert_eq!(action_for(Key::S, ctrl), Some(Action::SaveLayout));
        assert_eq!(action_for(Key::H, ctrl_alt), Some(Action::ToggleHideAll));
        assert_eq!(action_for(Key::Escape, none), Some(Action::Cancel));
        assert_eq!(action_for(Key::Delete, none), Some(Action::DeleteSelected));
        assert_eq!(action_for(Key::ArrowLeft, none), Some(Action::PanBars(-1)));
        assert_eq!(action_for(Key::ArrowRight, none), Some(Action::PanBars(1)));
        assert_eq!(action_for(Key::ArrowLeft, ctrl), Some(Action::PanBars(-10)));
        assert_eq!(action_for(Key::ArrowUp, ctrl), Some(Action::ZoomIn));
        assert_eq!(action_for(Key::ArrowDown, ctrl), Some(Action::ZoomOut));
        assert_eq!(action_for(Key::F1, none), Some(Action::OpenShortcuts));
        assert_eq!(
            action_for(Key::Num1, none),
            Some(Action::Tool(ToolKind::Select))
        );
        assert_eq!(
            action_for(Key::Num2, none),
            Some(Action::Tool(ToolKind::Trend))
        );
    }

    #[test]
    fn tombol_tak_dikenal_tidak_memakan_input() {
        use egui::{Key, Modifiers};
        let none = Modifiers::default();
        assert_eq!(action_for(Key::A, none), None);
        assert_eq!(action_for(Key::Z, none), None);
        assert_eq!(action_for(Key::S, none), None);
        // Alt+A (alert di TV) tidak dipakai SokanLab → jangan telan tombol.
        let alt = Modifiers {
            alt: true,
            ..Default::default()
        };
        assert_eq!(action_for(Key::A, alt), None);
    }

    #[test]
    fn hotkey_alat_tidak_duplikat() {
        let hints: Vec<&str> = ToolKind::ALL
            .iter()
            .filter_map(|k| {
                let h = k.hotkey_hint();
                if h.is_empty() {
                    None
                } else {
                    Some(h)
                }
            })
            .collect();
        let mut sorted = hints.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), hints.len(), "ada hotkey alat duplikat");
    }
}
