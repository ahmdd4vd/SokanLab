// SokanLab ui — command/event, layout persist, panel chart. Tanpa winit (headless-safe).
pub mod equity;
pub mod json;
pub mod painter;
pub mod runner;
pub mod theme;
pub mod tools;

use sokanlab_chart::drawing::{
    pick, snap_anchor, Anchor, ChannelKind, DashStyle, Drawing, DrawingItem, ExtendMode, FibLevel,
    History, ItemStyle, Magnet, PitchforkKind, Projector, SnapContext, StickerIcon, StyleTemplate,
    TextAlign, TextSize, MAX_STROKE_POINTS,
};
use sokanlab_chart::{visible_indices, ScaleMode, Viewport};
use sokanlab_domain::Bar;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    OpenDataset(String),
    FitToData,
    /// Kembalikan ke tampilan default: seluruh rentang data + fit harga (Alt+R).
    ResetView,
    Undo,
    Redo,
    RemoveSelected,
    SetTheme(theme::Theme),
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    DatasetOpened(String),
    ViewportChanged,
    DrawingsChanged,
    ThemeChanged(theme::Theme),
    Closed,
    Ignored,
}

pub fn handle(cmd: Command) -> Event {
    match cmd {
        Command::OpenDataset(p) => Event::DatasetOpened(p),
        Command::FitToData | Command::ResetView => Event::ViewportChanged,
        Command::Undo | Command::Redo | Command::RemoveSelected => Event::DrawingsChanged,
        Command::SetTheme(t) => Event::ThemeChanged(t),
        Command::Quit => Event::Closed,
    }
}

/// Jenis garis indikator overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayKind {
    Sma,
    Ema,
    Vwap,
}

impl OverlayKind {
    pub fn label(self) -> &'static str {
        match self {
            OverlayKind::Sma => "SMA",
            OverlayKind::Ema => "EMA",
            OverlayKind::Vwap => "VWAP",
        }
    }
}

/// Setelan overlay (None = nonaktif). Periode di-clamp `2..=400` saat dipakai.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OverlaySettings {
    pub sma: Option<usize>,
    pub ema: Option<usize>,
    pub vwap: bool,
}

/// Satu garis overlay: nilai harga (ticks) per bar, `None` saat warm-up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayLine {
    pub kind: OverlayKind,
    pub period: usize,
    pub values: Vec<Option<i64>>,
    pub color: [u8; 3],
}

/// Jenis pane indikator yang didukung (maks 3 pane sekaligus).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneKind {
    Rsi,
    Macd,
}

impl PaneKind {
    pub fn label(self) -> &'static str {
        match self {
            PaneKind::Rsi => "RSI",
            PaneKind::Macd => "MACD",
        }
    }
}

/// Setelan satu pane: `period` = periode RSI atau fast MACD;
/// `slow`/`signal` hanya dipakai MACD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneSetting {
    pub kind: PaneKind,
    pub period: usize,
    pub slow: usize,
    pub signal: usize,
}

/// Seri terhitung satu pane (cache; dibangun ulang saat data/setelan berubah).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneSeries {
    pub kind: PaneKind,
    pub label: String,
    /// Garis nilai: (nilai per bar, warna). RSI satu garis, MACD dua garis.
    pub lines: Vec<(Vec<Option<i64>>, [u8; 3])>,
    /// Histogram (MACD); kosong bila pane tidak memakai histogram.
    pub hist: Vec<Option<i64>>,
    /// true = skala tetap 0–100 (RSI); false = auto-simetris (MACD).
    pub fixed_rsi: bool,
}

/// Gestur pointer yang sedang berjalan di panel chart (state antar-frame).
#[derive(Debug, Clone, PartialEq)]
pub enum Gesture {
    Idle,
    /// Tombol kiri ditekan; tujuan (pan/pilih/move) ditentukan setelah bergerak.
    /// `toggle` = Ctrl ditekan saat tekan: lepas tanpa seret = tambah/lepas
    /// seleksi (TV), seret = klon lalu geser klon (`duplicate_for_drag`).
    Press {
        start: egui::Pos2,
        on_drawing: Option<usize>,
        toggle: bool,
    },
    Pan {
        last: egui::Pos2,
    },
    /// Drag handle anchor objek terpilih.
    MoveAnchor {
        idx: usize,
        anchor: usize,
        orig: Drawing,
    },
    /// Drag badan objek — sekaligus anggota grup: (indeks, drawing asli).
    MoveBody {
        targets: Vec<(usize, Drawing)>,
        start_local: (f64, f64),
    },
    /// Drag sumbu harga (skala vertikal).
    ScalePrice {
        last: egui::Pos2,
    },
    /// Drag sumbu waktu (skala horizontal).
    ScaleTime {
        last: egui::Pos2,
    },
    /// Seret-satu-kali untuk menempatkan objek (mode gambar).
    PlaceDrag {
        start_local: (f64, f64),
        start_anchor: Anchor,
        moved: bool,
    },
    /// Kotak seleksi (Ctrl+seret di area kosong) dari `origin` ke `current`
    /// (koordinat layar); lepas tombol = pilih objek yang tersentuh kotak.
    Rubber {
        origin: egui::Pos2,
        current: egui::Pos2,
    },
}

/// State chart: viewport, drawing, seleksi, tema, interaksi pointer.
/// Proyektor layar lokal (koordinat relatif area plot) untuk hit-test.
pub struct ScreenProj(pub Viewport);
impl Projector for ScreenProj {
    fn x(&self, ts_millis: i64) -> f64 {
        self.0.x_for_time(ts_millis)
    }
    fn y(&self, price_ticks: i64) -> f64 {
        self.0.y_for_price(price_ticks)
    }
}

/// Kunci cache hover-pick: semua masukan yang memengaruhi `pick`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct HoverKey {
    local: (f64, f64),
    viewport: Viewport,
    items: usize,
    undo: usize,
    hidden: bool,
    bars_rev: u64,
}

#[derive(Clone)]
pub struct ChartState {
    pub bars: Vec<Bar>,
    pub viewport: Viewport,
    pub drawings: History,
    pub selected: Option<usize>,
    /// Objek terpilih tambahan (multi-seleksi Ctrl+klik / kotak seleksi); tidak
    /// pernah memuat `selected` dan selalu urut naik.
    pub selected_multi: Vec<usize>,
    pub theme: theme::Theme,
    pub watchlist: Vec<String>,
    pub crosshair: Option<egui::Pos2>,
    /// Posisi pointer terakhir saat drag pan.
    pub drag: Option<egui::Pos2>,
    pub panning: bool,
    /// Alat gambar aktif + magnet + pending anchor.
    pub tool: tools::ToolState,
    /// Sembunyikan semua drawing (Ctrl+Alt+H).
    pub drawings_hidden: bool,
    /// Setelan indikator overlay (SMA/EMA/VWAP).
    pub overlay_settings: OverlaySettings,
    /// Garis overlay terhitung (cache; dibangun ulang saat data/setelan berubah).
    pub overlay_lines: Vec<OverlayLine>,
    /// Setelan pane indikator (RSI/MACD) di bawah volume.
    pub pane_settings: Vec<PaneSetting>,
    /// Seri pane terhitung (cache; dibangun ulang saat data/setelan berubah).
    pub panes: Vec<PaneSeries>,
    /// Skala snapshot PNG (1x/2x/4x; tidak dipersistensi).
    pub snapshot_scale: u32,
    /// Gaya default objek baru (diatur dari panel "Jadikan default").
    pub default_style: Option<ItemStyle>,
    /// Template gaya bernama yang disimpan pengguna (dipersistensi di layout).
    pub templates: Vec<StyleTemplate>,
    /// Template yang sedang dipilih di panel (nama; tidak dipersistensi).
    pub template_sel: Option<String>,
    /// Nama template untuk disimpan (input panel; tidak dipersistensi).
    pub template_name: String,
    /// Gestur pointer aktif.
    pub gesture: Gesture,
    /// Rect plotting harga terakhir (diisi chart_panel; dipakai pemetaan tes/koordinat).
    pub plot_rect: egui::Rect,
    /// Rect seluruh area chart terakhir (dipakai snapshot PNG).
    pub area_rect: egui::Rect,
    /// Revisi dataset (naik tiap `load`) — bagian kunci cache hover-pick.
    pub bars_rev: u64,
    /// Cache hover-pick (kunci → indeks objek). Tanpa ini, objek seperti
    /// regression channel di-hit-test O(rentang bar) tiap frame walau pointer
    /// diam — pengukuran: 1,85 ms/fit pada rentang 600k bar (1M dataset).
    hover_cache: Option<(HoverKey, Option<usize>)>,
}

impl Default for ChartState {
    fn default() -> Self {
        Self::new()
    }
}

impl ChartState {
    pub fn new() -> Self {
        Self {
            bars: Vec::new(),
            viewport: Viewport {
                width_px: 800.0,
                height_px: 400.0,
                t_min: 0,
                t_max: 1,
                p_min: 0,
                p_max: 1,
                ..Default::default()
            },
            drawings: History::default(),
            selected: None,
            selected_multi: Vec::new(),
            theme: theme::Theme::Dark,
            watchlist: Vec::new(),
            crosshair: None,
            drag: None,
            panning: false,
            tool: tools::ToolState::new(),
            drawings_hidden: false,
            overlay_settings: OverlaySettings::default(),
            overlay_lines: Vec::new(),
            pane_settings: Vec::new(),
            panes: Vec::new(),
            snapshot_scale: 1,
            default_style: None,
            templates: Vec::new(),
            template_sel: None,
            template_name: String::new(),
            gesture: Gesture::Idle,
            plot_rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 400.0)),
            area_rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0)),
            bars_rev: 0,
            hover_cache: None,
        }
    }

    pub fn load(&mut self, bars: Vec<Bar>) {
        self.bars = bars;
        // Rentang waktu dulu, baru fit harga (fit_price hanya melihat bar terlihat).
        if let (Some(first), Some(last)) = (self.bars.first(), self.bars.last()) {
            self.viewport.t_min = first.ts_millis;
            self.viewport.t_max = last.ts_millis;
        }
        self.viewport = self.viewport.fit_price(&self.bars);
        self.clear_selection();
        self.tool.pending.clear();
        self.gesture = Gesture::Idle;
        self.rebuild_overlays();
        self.rebuild_panes();
        self.refresh_pct_base();
        self.bars_rev = self.bars_rev.wrapping_add(1);
        self.hover_cache = None;
    }

    /// Hover pick ber-cache. `local = None` = pointer di luar area plot
    /// (cache dibersihkan). Pemilihan ulang hanya dijalankan bila masukan
    /// berubah: posisi lokal, viewport, jumlah objek, kedalaman undo,
    /// sembunyi-semua, atau revisi dataset — sehingga biaya hit-test
    /// O(rentang) milik objek regresi dibayar sekali per perubahan, bukan
    /// tiap frame.
    pub fn hover_pick(&mut self, local: Option<(f64, f64)>, tol_px: f64) -> Option<usize> {
        let Some(local) = local else {
            self.hover_cache = None;
            return None;
        };
        let key = HoverKey {
            local,
            viewport: self.viewport,
            items: self.drawings.items.len(),
            undo: self.drawings.undo.len(),
            hidden: self.drawings_hidden,
            bars_rev: self.bars_rev,
        };
        if let Some((k, v)) = &self.hover_cache {
            if *k == key {
                return *v;
            }
        }
        let v = pick(
            &self.drawings.items,
            &self.bars,
            &ScreenProj(self.viewport),
            local,
            tol_px,
        );
        self.hover_cache = Some((key, v));
        v
    }

    pub fn apply(&mut self, cmd: Command) -> Event {
        match cmd {
            Command::FitToData => {
                self.viewport = self.viewport.fit_price(&self.bars);
                Event::ViewportChanged
            }
            Command::ResetView => {
                self.reset_view();
                Event::ViewportChanged
            }
            Command::Undo => {
                if self.drawings.do_undo() {
                    self.clear_selection();
                }
                Event::DrawingsChanged
            }
            Command::Redo => {
                if self.drawings.do_redo() {
                    self.clear_selection();
                }
                Event::DrawingsChanged
            }
            Command::RemoveSelected => {
                if self.delete_selection() > 0 {
                    Event::DrawingsChanged
                } else {
                    Event::Ignored
                }
            }
            other => handle(other),
        }
    }

    /// Tampilan default: seluruh rentang waktu + fit harga.
    pub fn reset_view(&mut self) {
        if let (Some(first), Some(last)) = (self.bars.first(), self.bars.last()) {
            self.viewport.t_min = first.ts_millis;
            self.viewport.t_max = last.ts_millis;
        }
        self.viewport = self.viewport.fit_price(&self.bars);
    }

    /// Tambah objek; gaya default (bila diatur) langsung dipakai objek baru.
    pub fn add_drawing(&mut self, d: Drawing) {
        let mut item = DrawingItem::new(d);
        if let Some(st) = self.default_style {
            item.style = Some(st);
        }
        self.drawings.add_item(item);
        self.select_only(self.drawings.items.len() - 1);
    }

    // ---- Seleksi (tunggal & banyak) ----

    /// Indeks semua objek terpilih (urut naik, primer termasuk bila ada).
    pub fn selection(&self) -> Vec<usize> {
        let mut v = Vec::with_capacity(self.selected_multi.len() + 1);
        if let Some(p) = self.selected {
            if p < self.drawings.items.len() {
                v.push(p);
            }
        }
        for m in &self.selected_multi {
            if *m < self.drawings.items.len() && Some(*m) != self.selected {
                v.push(*m);
            }
        }
        v.sort_unstable();
        v.dedup();
        v
    }

    /// Objek idx ikut terpilih (primer atau tambahan)?
    pub fn is_selected(&self, idx: usize) -> bool {
        self.selected == Some(idx) || self.selected_multi.contains(&idx)
    }

    /// Pilih satu objek saja (seleksi banyak dibersihkan).
    pub fn select_only(&mut self, idx: usize) {
        self.selected = Some(idx);
        self.selected_multi.clear();
    }

    /// Bersihkan seluruh seleksi (primer + tambahan).
    pub fn clear_selection(&mut self) {
        self.selected = None;
        self.selected_multi.clear();
    }

    /// Pilih sekumpulan objek (kotak seleksi). Primer = indeks terbesar
    /// (paling atas z-order); duplikat & indeks tak valid dibuang.
    pub fn set_selection(&mut self, idxs: &[usize]) {
        let mut v: Vec<usize> = idxs
            .iter()
            .copied()
            .filter(|i| *i < self.drawings.items.len())
            .collect();
        v.sort_unstable();
        v.dedup();
        self.selected = v.pop();
        self.selected_multi = v;
    }

    /// Ctrl+klik: tambah objek ke seleksi, atau lepas bila sudah terpilih.
    /// Lepas objek primer → anggota teratas yang tersisa jadi primer.
    pub fn toggle_in_selection(&mut self, idx: usize) {
        if idx >= self.drawings.items.len() {
            return;
        }
        if self.selected == Some(idx) {
            self.selected = self.selected_multi.pop();
        } else if let Some(pos) = self.selected_multi.iter().position(|i| *i == idx) {
            self.selected_multi.remove(pos);
        } else if self.selected.is_none() {
            self.selected = Some(idx);
        } else {
            self.selected_multi.push(idx);
            self.selected_multi.sort_unstable();
            self.selected_multi.dedup();
        }
    }

    /// Hapus semua objek terpilih (satu langkah undo). Return jumlah terhapus.
    pub fn delete_selection(&mut self) -> usize {
        let sel = self.selection();
        let n = self.drawings.remove_many(&sel);
        if n > 0 {
            self.clear_selection();
        }
        n
    }

    /// Perbaiki seleksi setelah indeks `removed` dihapus dari daftar (dipakai
    /// tombol hapus per baris di daftar objek). Objek terpilih yang dihapus
    /// dibuang; indeks di atasnya bergeser turun satu.
    pub fn selection_after_removal(&mut self, removed: usize) {
        let cur = self.selection();
        let mapped: Vec<usize> = cur
            .into_iter()
            .filter(|i| *i != removed)
            .map(|i| if i > removed { i - 1 } else { i })
            .collect();
        self.set_selection(&mapped);
    }

    /// Samakan gaya + opasitas anggota seleksi dengan objek primer (dipakai
    /// saat panel properti mengubah gaya ketika multi-seleksi). Return jumlah
    /// objek yang ikut berubah (tanpa primer).
    pub fn sync_style_from_primary(&mut self) -> usize {
        let sel = self.selection();
        if sel.len() < 2 {
            return 0;
        }
        let Some(primary) = self.selected else {
            return 0;
        };
        let Some(src) = self.drawings.items.get(primary).cloned() else {
            return 0;
        };
        let mut n = 0;
        for i in sel {
            if i == primary {
                continue;
            }
            if let Some(it) = self.drawings.items.get_mut(i) {
                it.style = src.style;
                it.opacity = src.opacity;
                n += 1;
            }
        }
        n
    }

    /// Terapkan gaya + opasitas objek terpilih ke semua objek sejenis (degree
    /// sama). Mengembalikan jumlah objek yang disamakan (0 = tidak ada).
    pub fn apply_style_to_same_kind(&mut self) -> usize {
        let Some(sel) = self.selected else {
            return 0;
        };
        let Some(src) = self.drawings.items.get(sel).cloned() else {
            return 0;
        };
        let deg = src.drawing.degree();
        let mut n = 0;
        for it in self.drawings.items.iter_mut() {
            if it.drawing.degree() == deg {
                it.style = src.style;
                it.opacity = src.opacity;
                n += 1;
            }
        }
        n
    }

    /// Jadikan gaya objek terpilih sebagai gaya default objek baru berikutnya.
    pub fn set_default_style_from_selected(&mut self) -> bool {
        let Some(sel) = self.selected else {
            return false;
        };
        let Some(it) = self.drawings.items.get(sel) else {
            return false;
        };
        self.default_style = it.style;
        self.default_style.is_some()
    }

    // ---- Template gaya bernama (katalog, padanan template alat gambar) ----

    /// Simpan gaya + opasitas objek primer sebagai template bernama. Nama
    /// dipangkas dan dipotong `StyleTemplate::NAME_MAX`; nama kosong ditolak.
    /// Nama yang sudah ada diperbarui di tempat; katalog dibatasi
    /// `StyleTemplate::MAX` entri. Return pesan hasil untuk panel.
    pub fn save_template(&mut self, name: &str) -> Result<String, String> {
        let trimmed: String = name.trim().chars().take(StyleTemplate::NAME_MAX).collect();
        if trimmed.is_empty() {
            return Err("nama template kosong".into());
        }
        let Some(sel) = self.selected else {
            return Err("pilih objek dulu".into());
        };
        let Some(it) = self.drawings.items.get(sel) else {
            return Err("pilih objek dulu".into());
        };
        let tpl = StyleTemplate::from_item(&trimmed, it);
        if let Some(pos) = self.templates.iter().position(|t| t.name == trimmed) {
            self.templates[pos] = tpl;
            return Ok(format!("template \"{trimmed}\" diperbarui"));
        }
        if self.templates.len() >= StyleTemplate::MAX {
            return Err(format!("katalog penuh ({} template)", StyleTemplate::MAX));
        }
        self.templates.push(tpl);
        Ok(format!("template \"{trimmed}\" disimpan"))
    }

    /// Terapkan template ke seluruh objek terpilih (gaya + opasitas).
    /// Return jumlah objek yang berubah (0 = template/seleksi tidak ada).
    pub fn apply_template(&mut self, name: &str) -> usize {
        let Some(tpl) = self.templates.iter().find(|t| t.name == name).cloned() else {
            return 0;
        };
        let sel = self.selection();
        let mut n = 0;
        for i in sel {
            if let Some(it) = self.drawings.items.get_mut(i) {
                tpl.apply_to(it);
                n += 1;
            }
        }
        n
    }

    /// Hapus template dari katalog; pilihan panel ikut dibersihkan bila
    /// template itu yang sedang dipilih. `false` bila nama tidak ditemukan.
    pub fn remove_template(&mut self, name: &str) -> bool {
        let before = self.templates.len();
        self.templates.retain(|t| t.name != name);
        if self.template_sel.as_deref() == Some(name) {
            self.template_sel = None;
        }
        self.templates.len() != before
    }

    pub fn toggle_hide_all(&mut self) -> bool {
        self.drawings_hidden = !self.drawings_hidden;
        self.drawings_hidden
    }

    // ---- Skala harga (log/persen/invert) ----

    /// Ganti mode skala harga. Skala persen memakai close bar terlihat pertama
    /// sebagai basis 0%; mode lain membersihkan basis.
    pub fn set_price_scale(&mut self, mode: ScaleMode) {
        if self.viewport.scale == mode {
            return;
        }
        self.viewport.scale = mode;
        self.refresh_pct_base();
    }

    /// Balik arah sumbu harga; return status baru.
    pub fn toggle_invert(&mut self) -> bool {
        self.viewport.invert = !self.viewport.invert;
        self.viewport.invert
    }

    /// Basis skala persen = close bar terlihat pertama (atau bar pertama data).
    fn refresh_pct_base(&mut self) {
        if self.viewport.scale != ScaleMode::Percent {
            self.viewport.pct_base = 0;
            return;
        }
        let range = visible_indices(&self.bars, self.viewport.t_min, self.viewport.t_max);
        let first = if range.is_empty() {
            self.bars.first()
        } else {
            self.bars.get(range.start)
        };
        self.viewport.pct_base = first.map(|b| b.close_ticks).filter(|b| *b > 0).unwrap_or(1);
    }

    /// Drag sumbu harga (vertikal): turun = rentang membesar (zoom out).
    pub fn scale_price_drag(&mut self, dy_px: f64) {
        let factor = (dy_px * 0.005).exp();
        let anchor = self.viewport.height_px / 2.0;
        self.viewport = self.viewport.zoom_price(factor, anchor);
    }

    /// Drag sumbu waktu (horizontal): kanan = zoom in, kiri = zoom out.
    pub fn scale_time_drag(&mut self, dx_px: f64) {
        let factor = (-dx_px * 0.005).exp();
        let anchor = self.viewport.width_px;
        self.viewport = self.viewport.zoom_time(factor, anchor);
    }

    // ---- Objek: z-order, duplikat, grup ----

    /// Geser z semua objek terpilih (dir>0 = ke depan), urutan relatif tetap.
    /// Return true bila ada yang pindah.
    pub fn move_selected_z(&mut self, dir: isize) -> bool {
        let sel = self.selection();
        if sel.is_empty() {
            return false;
        }
        match self.drawings.move_items_z(&sel, dir) {
            Some(new_pos) => {
                self.set_selection(&new_pos);
                true
            }
            None => false,
        }
    }

    /// Id grup objek terpilih (None = tidak bergrup).
    pub fn selected_group(&self) -> Option<u32> {
        self.selected
            .and_then(|i| self.drawings.items.get(i))
            .and_then(|it| it.group)
    }

    /// Pasang/lepas grup untuk semua objek terpilih (satu id grup untuk
    /// seleksi banyak).
    pub fn toggle_group_selected(&mut self) -> bool {
        let sel = self.selection();
        if sel.is_empty() {
            return false;
        }
        match self.selected_group() {
            Some(_) => self.drawings.clear_group_many(&sel) > 0,
            None => self.drawings.assign_group_many(&sel).is_some(),
        }
    }

    /// Duplikat objek terpilih dengan offset 2 bar & 1/20 rentang harga.
    /// Seleksi banyak disalin sekaligus (satu langkah undo).
    pub fn duplicate_selected(&mut self) -> bool {
        let sel = self.selection();
        if sel.is_empty() {
            return false;
        }
        let step = if self.bars.len() >= 2 {
            (self.bars[self.bars.len() - 1].ts_millis - self.bars[self.bars.len() - 2].ts_millis)
                .max(1)
        } else {
            60_000
        };
        let dp = ((self.viewport.p_max - self.viewport.p_min) / 20).max(1);
        let new = self
            .drawings
            .duplicate_many(&sel, step.saturating_mul(2), dp);
        if new.is_empty() {
            return false;
        }
        self.set_selection(&new);
        true
    }

    /// Klon set drag (multi-seleksi atau anggota grup) untuk Ctrl+seret ala TV:
    /// klon tanpa grup ditambahkan & dipilih, indeks barunya dikembalikan.
    /// Objek terkunci dilewati (tidak bisa ikut digeser).
    pub fn duplicate_for_drag(&mut self, idx: usize) -> Vec<usize> {
        let sel = self.selection();
        let ids = if sel.len() > 1 && sel.contains(&idx) {
            sel
        } else {
            self.drawings.group_members(idx)
        };
        let ids: Vec<usize> = ids
            .into_iter()
            .filter(|i| {
                self.drawings
                    .items
                    .get(*i)
                    .map(|it| !it.locked)
                    .unwrap_or(false)
            })
            .collect();
        let new = self.drawings.duplicate_many(&ids, 0, 0);
        if !new.is_empty() {
            self.set_selection(&new);
        }
        new
    }

    // ---- Indikator overlay ----

    pub fn set_overlay_settings(&mut self, settings: OverlaySettings) {
        if settings == self.overlay_settings {
            return;
        }
        self.overlay_settings = settings;
        self.rebuild_overlays();
    }

    /// Hitung ulang garis overlay dari setelan aktif (SMA/EMA/VWAP).
    pub fn rebuild_overlays(&mut self) {
        self.overlay_lines.clear();
        if self.bars.is_empty() {
            return;
        }
        let closes: Vec<i64> = self.bars.iter().map(|b| b.close_ticks).collect();
        if let Some(p) = self.overlay_settings.sma {
            let period = p.clamp(2, 400);
            self.overlay_lines.push(OverlayLine {
                kind: OverlayKind::Sma,
                period,
                values: sokanlab_indicators::sma_series(&closes, period),
                color: [0x7f, 0xb2, 0xd9],
            });
        }
        if let Some(p) = self.overlay_settings.ema {
            let period = p.clamp(2, 400);
            self.overlay_lines.push(OverlayLine {
                kind: OverlayKind::Ema,
                period,
                values: sokanlab_indicators::ema_series(&closes, period),
                color: [0xf2, 0xb0, 0x4d],
            });
        }
        if self.overlay_settings.vwap {
            let volumes: Vec<i64> = self.bars.iter().map(|b| b.volume_milli).collect();
            self.overlay_lines.push(OverlayLine {
                kind: OverlayKind::Vwap,
                period: 0,
                values: sokanlab_indicators::vwap_series(&closes, &volumes),
                color: [0xb3, 0x8c, 0xf0],
            });
        }
    }

    // ---- Pane indikator (RSI/MACD) ----

    /// Setel pane indikator; hitung ulang cache bila setelan berubah.
    /// Dipotong ke maksimum 3 pane (batas layout `split_plot_panes`).
    pub fn set_panes(&mut self, settings: Vec<PaneSetting>) {
        let mut s = settings;
        s.truncate(3);
        if s == self.pane_settings {
            return;
        }
        self.pane_settings = s;
        self.rebuild_panes();
    }

    /// Hitung ulang seri pane dari setelan aktif + data bar.
    /// RSI dalam skala 0–100; MACD dalam milli (×1000 harga) dengan sinyal
    /// = EMA(signal) dari MACD, histogram = MACD − sinyal.
    pub fn rebuild_panes(&mut self) {
        self.panes.clear();
        if self.bars.is_empty() {
            return;
        }
        let closes: Vec<i64> = self.bars.iter().map(|b| b.close_ticks).collect();
        for st in &self.pane_settings {
            match st.kind {
                PaneKind::Rsi => {
                    let period = st.period.clamp(2, 100);
                    self.panes.push(PaneSeries {
                        kind: PaneKind::Rsi,
                        label: format!("RSI {period}"),
                        lines: vec![(
                            sokanlab_indicators::rsi_series(&closes, period),
                            [0xb3, 0x8c, 0xf0],
                        )],
                        hist: Vec::new(),
                        fixed_rsi: true,
                    });
                }
                PaneKind::Macd => {
                    let fast = st.period.clamp(1, 199);
                    let slow = st.slow.clamp(fast + 1, 200);
                    let signal = st.signal.clamp(1, 100);
                    let (macd, sig) = sokanlab_indicators::macd_series(&closes, fast, slow, signal);
                    let hist: Vec<Option<i64>> = macd
                        .iter()
                        .zip(sig.iter())
                        .map(|(m, s)| match (m, s) {
                            (Some(a), Some(b)) => Some(a - b),
                            _ => None,
                        })
                        .collect();
                    self.panes.push(PaneSeries {
                        kind: PaneKind::Macd,
                        label: format!("MACD {fast} {slow} {signal}"),
                        lines: vec![(macd, [0x7f, 0xb2, 0xd9]), (sig, [0xf2, 0xb0, 0x4d])],
                        hist,
                        fixed_rsi: false,
                    });
                }
            }
        }
    }

    pub fn zoom_at(&mut self, factor: f64, pos: egui::Pos2, area: egui::Rect) {
        // Peta Y memakai tinggi pane harga efektif (di-resize chart_panel tiap
        // frame), bukan tinggi jendela, agar anchor akurat saat ada pane indikator.
        let plot_h = self.viewport.height_px;
        self.viewport = self
            .viewport
            .zoom_time(factor, pos.x as f64 - area.min.x as f64)
            .zoom_price(factor, ((pos.y - area.min.y) as f64).min(plot_h));
    }

    /// Zoom di tengah pane (Ctrl+↑/↓).
    pub fn zoom_center(&mut self, factor: f64) {
        self.viewport = self
            .viewport
            .zoom_time(factor, self.viewport.width_px / 2.0)
            .zoom_price(factor, self.viewport.height_px / 2.0);
    }

    /// Geser chart sejumlah bar (anak panah kiri/kanan; TV: 1 bar, Ctrl: 10 bar).
    pub fn pan_bars(&mut self, n: i64) {
        let visible = visible_indices(&self.bars, self.viewport.t_min, self.viewport.t_max).len();
        let spacing = crate::painter::fit_spacing(self.viewport.width_px, visible.max(1));
        self.viewport = self.viewport.pan_px(-(n as f64) * spacing, 0.0);
    }

    pub fn pan(&mut self, delta: egui::Vec2, _area: egui::Rect) {
        self.viewport = self.viewport.pan_px(delta.x as f64, delta.y as f64);
    }

    pub fn visible_start(&self) -> usize {
        visible_indices(&self.bars, self.viewport.t_min, self.viewport.t_max).start
    }

    pub fn last_close(&self) -> Option<i64> {
        self.bars.last().map(|b| b.close_ticks)
    }

    /// Spacing bar saat ini (konsisten dengan painter).
    pub fn spacing(&self) -> f64 {
        let visible = visible_indices(&self.bars, self.viewport.t_min, self.viewport.t_max).len();
        crate::painter::fit_spacing(self.viewport.width_px, visible.max(1))
    }

    /// Anchor dari posisi lokal pointer, dengan magnet aktif (lemah/kuat).
    pub fn anchor_at(&self, local: (f64, f64), tol: f64) -> (Anchor, bool) {
        let ctx = SnapContext {
            bars: &self.bars,
            viewport: &self.viewport,
            spacing_px: self.spacing(),
            drawings: &self.drawings.items,
            tol_px: tol,
        };
        snap_anchor(local, &ctx, self.tool.magnet)
    }

    /// Handle anchor objek terpilih yang sedang di-hover/drag (lokal, toleransi px).
    pub fn handle_hit(
        &self,
        layout: &painter::PlotLayout,
        local: (f64, f64),
        tol: f64,
    ) -> Option<(usize, usize)> {
        let idx = self.selected?;
        let item = self.drawings.items.get(idx)?;
        if item.locked || !item.visible {
            return None;
        }
        let ox = layout.price.min.x as f64;
        let oy = layout.price.min.y as f64;
        let mut best: Option<((usize, usize), f64)> = None;
        for (ai, p) in painter::handle_points(layout, &self.viewport, &item.drawing) {
            let lx = p.x as f64 - ox;
            let ly = p.y as f64 - oy;
            let dist = ((lx - local.0).powi(2) + (ly - local.1).powi(2)).sqrt();
            if dist <= tol && best.map(|(_, bd)| dist < bd).unwrap_or(true) {
                best = Some(((idx, ai), dist));
            }
        }
        best.map(|(v, _)| v)
    }

    /// Layout JSON berversi (tanpa serde eksternal): tema, viewport (+skala/invert),
    /// watchlist, setelan alat (magnet/stay/hidden), overlay, pane indikator,
    /// katalog template gaya, dan seluruh objek gambar + metadata
    /// (gaya/visible/locked/nama/grup/varian).
    /// Writer menulis `schema:7`; reader menerima 1–7.
    pub fn layout_json(&self) -> String {
        let drawings: Vec<String> = self
            .drawings
            .items
            .iter()
            .map(drawing_item_to_json)
            .collect();
        let mut ov: Vec<String> = Vec::new();
        if let Some(p) = self.overlay_settings.sma {
            ov.push(format!("\"sma\":{p}"));
        }
        if let Some(p) = self.overlay_settings.ema {
            ov.push(format!("\"ema\":{p}"));
        }
        if self.overlay_settings.vwap {
            ov.push("\"vwap\":true".into());
        }
        let overlays = if ov.is_empty() {
            String::new()
        } else {
            format!(",\"overlays\":{{{}}}", ov.join(","))
        };
        let panes: Vec<String> = self
            .pane_settings
            .iter()
            .map(|p| match p.kind {
                PaneKind::Rsi => format!("{{\"kind\":\"rsi\",\"period\":{}}}", p.period),
                PaneKind::Macd => format!(
                    "{{\"kind\":\"macd\",\"fast\":{},\"slow\":{},\"signal\":{}}}",
                    p.period, p.slow, p.signal
                ),
            })
            .collect();
        let panes = if panes.is_empty() {
            String::new()
        } else {
            format!(",\"panes\":[{}]", panes.join(","))
        };
        let default_style = match self.default_style {
            Some(st) => {
                let dash = if st.dash == DashStyle::Solid {
                    String::new()
                } else {
                    format!(",\"dash\":\"{}\"", st.dash.key())
                };
                format!(
                    ",\"default_style\":{{\"color\":\"#{:02x}{:02x}{:02x}\",\"width\":{}{}}}",
                    st.color[0], st.color[1], st.color[2], st.width, dash
                )
            }
            None => String::new(),
        };
        let templates = if self.templates.is_empty() {
            String::new()
        } else {
            let ts: Vec<String> = self.templates.iter().map(style_template_to_json).collect();
            format!(",\"templates\":[{}]", ts.join(","))
        };
        format!(
            "{{\"schema\":7,\"theme\":\"{}\",\"viewport\":{{\"t_min\":{},\"t_max\":{},\"p_min\":{},\"p_max\":{},\"scale\":\"{}\",\"invert\":{}}},\"tools\":{{\"magnet\":\"{}\",\"stay\":{}}},\"hidden\":{}{overlays}{panes}{default_style}{templates},\"watchlist\":[{}],\"drawings\":[{}]}}",
            match self.theme {
                theme::Theme::Dark => "dark",
                theme::Theme::Light => "light",
            },
            self.viewport.t_min,
            self.viewport.t_max,
            self.viewport.p_min,
            self.viewport.p_max,
            self.viewport.scale.key(),
            self.viewport.invert,
            self.tool.magnet.code(),
            self.tool.stay,
            self.drawings_hidden,
            self.watchlist
                .iter()
                .map(|s| format!("\"{}\"", json::escape(s)))
                .collect::<Vec<_>>()
                .join(","),
            drawings.join(",")
        )
    }

    /// Pulihkan layout dari JSON. Menerima `schema:1`–`schema:7`; field yang
    /// hilang diabaikan (berkas lama tetap kompatibel,
    /// nilai default). Format lama (`"drawings":<angka>`) tetap diterima —
    /// drawing-nya saja yang dilewati. Tidak ada perubahan state yang diterapkan
    /// bila JSON/schema tidak valid (entry pane tak dikenal dilewati).
    pub fn restore_layout_json(&mut self, text: &str) -> Result<(), String> {
        let v = json::parse(text)?;
        match v.get("schema").and_then(json::Value::as_i64) {
            Some(1) | Some(2) | Some(3) | Some(4) | Some(5) | Some(6) | Some(7) => {}
            Some(other) => return Err(format!("schema layout {other} tidak dikenal")),
            None => return Err("layout tanpa schema".into()),
        }
        let mut theme = self.theme;
        if let Some(t) = v.get("theme").and_then(json::Value::as_str) {
            theme = match t {
                "dark" => theme::Theme::Dark,
                "light" => theme::Theme::Light,
                other => return Err(format!("tema tidak dikenal: {other}")),
            };
        }
        let mut viewport = self.viewport;
        if let Some(vp) = v.get("viewport") {
            let (t_min, t_max, p_min, p_max) = (
                vp.get("t_min").and_then(json::Value::as_i64),
                vp.get("t_max").and_then(json::Value::as_i64),
                vp.get("p_min").and_then(json::Value::as_i64),
                vp.get("p_max").and_then(json::Value::as_i64),
            );
            if let (Some(t_min), Some(t_max), Some(p_min), Some(p_max)) =
                (t_min, t_max, p_min, p_max)
            {
                if t_max > t_min && p_max > p_min {
                    viewport.t_min = t_min;
                    viewport.t_max = t_max;
                    viewport.p_min = p_min;
                    viewport.p_max = p_max;
                }
            }
            if let Some(k) = vp.get("scale").and_then(json::Value::as_str) {
                match ScaleMode::from_key(k) {
                    Some(m) => viewport.scale = m,
                    None => return Err(format!("skala tidak dikenal: {k}")),
                }
            }
            if let Some(b) = vp.get("invert").and_then(json::Value::as_bool) {
                viewport.invert = b;
            }
        }
        let mut magnet = self.tool.magnet;
        let mut stay = self.tool.stay;
        if let Some(t) = v.get("tools") {
            if let Some(code) = t.get("magnet").and_then(json::Value::as_str) {
                match Magnet::from_code(code) {
                    Some(m) => magnet = m,
                    None => return Err(format!("magnet tidak dikenal: {code}")),
                }
            }
            if let Some(s) = t.get("stay").and_then(json::Value::as_bool) {
                stay = s;
            }
        }
        let mut overlays = self.overlay_settings;
        if let Some(ov) = v.get("overlays") {
            if let Some(p) = ov.get("sma").and_then(json::Value::as_i64) {
                overlays.sma = Some(p.clamp(2, 400) as usize);
            }
            if let Some(p) = ov.get("ema").and_then(json::Value::as_i64) {
                overlays.ema = Some(p.clamp(2, 400) as usize);
            }
            if let Some(b) = ov.get("vwap").and_then(json::Value::as_bool) {
                overlays.vwap = b;
            }
        }
        let mut pane_settings = self.pane_settings.clone();
        if let Some(ps) = v.get("panes").and_then(json::Value::as_array) {
            let mut out = Vec::new();
            for p in ps {
                match p.get("kind").and_then(json::Value::as_str) {
                    Some("rsi") => {
                        let period = p
                            .get("period")
                            .and_then(json::Value::as_i64)
                            .unwrap_or(14)
                            .clamp(2, 100);
                        out.push(PaneSetting {
                            kind: PaneKind::Rsi,
                            period: period as usize,
                            slow: 0,
                            signal: 0,
                        });
                    }
                    Some("macd") => {
                        let fast = p
                            .get("fast")
                            .and_then(json::Value::as_i64)
                            .unwrap_or(12)
                            .clamp(1, 199) as usize;
                        let slow = p
                            .get("slow")
                            .and_then(json::Value::as_i64)
                            .unwrap_or(26)
                            .clamp(2, 200) as usize;
                        let slow = slow.max(fast + 1);
                        let signal = p
                            .get("signal")
                            .and_then(json::Value::as_i64)
                            .unwrap_or(9)
                            .clamp(1, 100) as usize;
                        out.push(PaneSetting {
                            kind: PaneKind::Macd,
                            period: fast,
                            slow,
                            signal,
                        });
                    }
                    // Jenis pane tak dikenal dilewati (berkas dari versi lebih baru).
                    _ => {}
                }
            }
            out.truncate(3);
            pane_settings = out;
        }
        let default_style = v
            .get("default_style")
            .and_then(parse_item_style)
            .or(self.default_style);
        let hidden = v
            .get("hidden")
            .and_then(json::Value::as_bool)
            .unwrap_or(self.drawings_hidden);
        let mut watchlist = self.watchlist.clone();
        if let Some(wl) = v.get("watchlist").and_then(json::Value::as_array) {
            watchlist = wl
                .iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect();
        }
        let mut drawings = self.drawings.clone();
        if let Some(ds) = v.get("drawings").and_then(json::Value::as_array) {
            let mut items = Vec::new();
            for d in ds {
                if let Some(dr) = drawing_item_from_value(d) {
                    items.push(dr);
                }
            }
            drawings = History {
                items,
                undo: Vec::new(),
                redo: Vec::new(),
            };
        }
        // Katalog template: entri rusak/nama ganda dilewati, dibatasi `MAX`.
        let mut templates = self.templates.clone();
        if let Some(ts) = v.get("templates").and_then(json::Value::as_array) {
            let mut out: Vec<StyleTemplate> = Vec::new();
            for t in ts {
                if let Some(tpl) = parse_style_template(t) {
                    if !out.iter().any(|x| x.name == tpl.name) {
                        out.push(tpl);
                    }
                }
            }
            out.truncate(StyleTemplate::MAX);
            templates = out;
        }

        self.theme = theme;
        self.viewport = viewport;
        self.watchlist = watchlist;
        self.drawings = drawings;
        self.clear_selection();
        self.drawings_hidden = hidden;
        self.tool.magnet = magnet;
        self.tool.stay = stay;
        self.tool.kind = tools::ToolKind::Select;
        self.tool.pending.clear();
        self.tool.last_measure = None;
        self.gesture = Gesture::Idle;
        self.overlay_settings = overlays;
        self.pane_settings = pane_settings;
        self.default_style = default_style;
        self.templates = templates;
        self.template_sel = None;
        self.rebuild_overlays();
        self.rebuild_panes();
        self.refresh_pct_base();
        Ok(())
    }

    /// Pusatkan tampilan waktu pada `ts_millis` tanpa mengubah skala (Alt+G).
    /// Memakai aritmetika saturating agar rentang ekstrem tidak panic.
    pub fn center_on_time(&mut self, ts_millis: i64) {
        let span = self
            .viewport
            .t_max
            .saturating_sub(self.viewport.t_min)
            .max(1);
        self.viewport.t_min = ts_millis.saturating_sub(span / 2);
        self.viewport.t_max = self.viewport.t_min.saturating_add(span);
        self.viewport = self.viewport.fit_price(&self.bars);
    }

    /// Double-click sumbu harga: fit ulang rentang harga ke bar terlihat.
    pub fn reset_price_scale(&mut self) {
        self.viewport = self.viewport.fit_price(&self.bars);
    }

    /// Double-click sumbu waktu: kembali ke rentang waktu penuh + fit harga.
    pub fn reset_time_scale(&mut self) {
        self.reset_view();
    }

    /// Tulis layout ke berkas (buat direktori induk bila perlu).
    pub fn save_layout(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, format!("{}\n", self.layout_json()))
    }

    /// Baca layout dari berkas lalu terapkan (atomik: state hanya berubah bila valid).
    pub fn restore_layout(&mut self, path: &std::path::Path) -> Result<(), String> {
        let text =
            std::fs::read_to_string(path).map_err(|e| format!("baca {}: {e}", path.display()))?;
        self.restore_layout_json(&text)
    }
}

fn anchor_json(a: &Anchor) -> String {
    format!("{{\"t\":{},\"p\":{}}}", a.ts_millis, a.price_ticks)
}

fn two_anchor_json(kind: &str, a: &Anchor, b: &Anchor) -> String {
    format!(
        "{{\"kind\":\"{kind}\",\"a\":{},\"b\":{}}}",
        anchor_json(a),
        anchor_json(b)
    )
}

/// Objek berdaftar titik (pola/garis bebas): `{"kind":"…","pts":[{t,p},…]}`.
fn points_json(kind: &str, points: &[Anchor]) -> String {
    let pts: Vec<String> = points.iter().map(anchor_json).collect();
    format!("{{\"kind\":\"{kind}\",\"pts\":[{}]}}", pts.join(","))
}

/// Serialisasi satu objek + metadata (metadata ditulis hanya bila non-default
/// agar roundtrip berkas default identik byte-for-byte).
fn drawing_item_to_json(it: &DrawingItem) -> String {
    let mut base = drawing_to_json(&it.drawing);
    let mut meta: Vec<String> = Vec::new();
    if let Some(st) = it.style {
        if st.dash == DashStyle::Solid {
            meta.push(format!(
                "\"style\":{{\"color\":\"#{:02x}{:02x}{:02x}\",\"width\":{}}}",
                st.color[0], st.color[1], st.color[2], st.width
            ));
        } else {
            meta.push(format!(
                "\"style\":{{\"color\":\"#{:02x}{:02x}{:02x}\",\"width\":{},\"dash\":\"{}\"}}",
                st.color[0],
                st.color[1],
                st.color[2],
                st.width,
                st.dash.key()
            ));
        }
    }
    if !it.visible {
        meta.push("\"visible\":false".into());
    }
    if it.locked {
        meta.push("\"locked\":true".into());
    }
    if let Some(n) = &it.name {
        if !n.is_empty() {
            meta.push(format!("\"name\":\"{}\"", json::escape(n)));
        }
    }
    if let Some(g) = it.group {
        meta.push(format!("\"group\":{g}"));
    }
    if it.opacity != 100 {
        meta.push(format!("\"opacity\":{}", it.opacity.min(100)));
    }
    if it.pitchfork != PitchforkKind::Andrew {
        meta.push(format!("\"pitchfork\":\"{}\"", it.pitchfork.key()));
    }
    if it.extend != ExtendMode::None {
        meta.push(format!("\"extend\":\"{}\"", it.extend.key()));
    }
    if !it.fib_levels.is_empty() {
        let ls: Vec<String> = it
            .fib_levels
            .iter()
            .map(|l| match l.color {
                Some(c) => format!(
                    "{{\"r\":{},\"c\":\"#{:02x}{:02x}{:02x}\"}}",
                    l.milli, c[0], c[1], c[2]
                ),
                None => format!("{{\"r\":{}}}", l.milli),
            })
            .collect();
        meta.push(format!("\"fib\":[{}]", ls.join(",")));
    }
    if it.text_size != TextSize::M {
        meta.push(format!("\"font\":\"{}\"", it.text_size.key()));
    }
    if it.text_align != TextAlign::Left {
        meta.push(format!("\"align\":\"{}\"", it.text_align.key()));
    }
    if !meta.is_empty() {
        base.pop(); // buang '}'
        base.push(',');
        base.push_str(&meta.join(","));
        base.push('}');
    }
    base
}

/// Satu template gaya → JSON (`name` wajib; `style`/`opacity` hanya bila bukan
/// default). Dipakai katalog di layout.
fn style_template_to_json(t: &StyleTemplate) -> String {
    let mut keys = vec![format!("\"name\":\"{}\"", json::escape(&t.name))];
    if let Some(st) = t.style {
        let dash = if st.dash == DashStyle::Solid {
            String::new()
        } else {
            format!(",\"dash\":\"{}\"", st.dash.key())
        };
        keys.push(format!(
            "\"style\":{{\"color\":\"#{:02x}{:02x}{:02x}\",\"width\":{}{dash}}}",
            st.color[0], st.color[1], st.color[2], st.width
        ));
    }
    if t.opacity != 100 {
        keys.push(format!("\"opacity\":{}", t.opacity.min(100)));
    }
    format!("{{{}}}", keys.join(","))
}

/// Template gaya dari JSON; nama kosong (setelah dipangkas) dilewati.
fn parse_style_template(v: &json::Value) -> Option<StyleTemplate> {
    let raw = v.get("name").and_then(json::Value::as_str)?;
    let name: String = raw.trim().chars().take(StyleTemplate::NAME_MAX).collect();
    if name.is_empty() {
        return None;
    }
    let style = v.get("style").and_then(parse_item_style);
    let opacity = v
        .get("opacity")
        .and_then(json::Value::as_i64)
        .unwrap_or(100)
        .clamp(0, 100) as u8;
    Some(StyleTemplate {
        name,
        style,
        opacity,
    })
}

fn parse_hex_rgb(s: &str) -> Option<[u8; 3]> {
    let t = s.strip_prefix('#').unwrap_or(s);
    if t.len() != 6 || !t.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let v = u32::from_str_radix(t, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

/// Gaya dari objek JSON (`{"color":"#rrggbb","width":n,"dash":"..."}`).
/// Warna wajib; tanpa warna dianggap tidak ada gaya.
fn parse_item_style(st: &json::Value) -> Option<ItemStyle> {
    let color = st
        .get("color")
        .and_then(json::Value::as_str)
        .and_then(parse_hex_rgb)?;
    let width = st
        .get("width")
        .and_then(json::Value::as_i64)
        .unwrap_or(2)
        .clamp(1, 4) as u8;
    let dash = st
        .get("dash")
        .and_then(json::Value::as_str)
        .and_then(DashStyle::from_key)
        .unwrap_or(DashStyle::Solid);
    Some(ItemStyle { color, width, dash })
}

/// Objek + metadata dari JSON. Metadata rusak diabaikan (default), bukan panic.
fn drawing_item_from_value(v: &json::Value) -> Option<DrawingItem> {
    let drawing = drawing_from_value(v)?;
    let mut item = DrawingItem::new(drawing);
    if let Some(st) = v.get("style") {
        item.style = parse_item_style(st);
    }
    if let Some(b) = v.get("visible").and_then(json::Value::as_bool) {
        item.visible = b;
    }
    if let Some(b) = v.get("locked").and_then(json::Value::as_bool) {
        item.locked = b;
    }
    if let Some(n) = v.get("name").and_then(json::Value::as_str) {
        if !n.is_empty() {
            item.name = Some(n.to_string());
        }
    }
    if let Some(g) = v.get("group").and_then(json::Value::as_i64) {
        if g >= 0 {
            item.group = Some(g as u32);
        }
    }
    if let Some(o) = v.get("opacity").and_then(json::Value::as_i64) {
        item.opacity = o.clamp(0, 100) as u8;
    }
    if let Some(k) = v.get("pitchfork").and_then(json::Value::as_str) {
        if let Some(p) = PitchforkKind::from_key(k) {
            item.pitchfork = p;
        }
    }
    if let Some(x) = v.get("extend").and_then(json::Value::as_str) {
        if let Some(e) = ExtendMode::from_key(x) {
            item.extend = e;
        }
    }
    if let Some(fs) = v.get("fib").and_then(json::Value::as_array) {
        let mut levels = Vec::new();
        for l in fs {
            if let Some(m) = l.get("r").and_then(json::Value::as_i64) {
                let color = l
                    .get("c")
                    .and_then(json::Value::as_str)
                    .and_then(parse_hex_rgb);
                levels.push(FibLevel {
                    milli: m.clamp(-10_000, 10_000) as i32,
                    color,
                });
            }
        }
        item.fib_levels = levels;
    }
    if let Some(k) = v.get("font").and_then(json::Value::as_str) {
        if let Some(t) = TextSize::from_key(k) {
            item.text_size = t;
        }
    }
    if let Some(k) = v.get("align").and_then(json::Value::as_str) {
        if let Some(t) = TextAlign::from_key(k) {
            item.text_align = t;
        }
    }
    Some(item)
}

fn drawing_to_json(d: &Drawing) -> String {
    match d {
        Drawing::HorizontalLine { price_ticks } => {
            format!("{{\"kind\":\"hline\",\"price\":{price_ticks}}}")
        }
        Drawing::HorizontalRay { origin } => {
            format!("{{\"kind\":\"hray\",\"a\":{}}}", anchor_json(origin))
        }
        Drawing::VerticalLine { ts_millis } => {
            format!("{{\"kind\":\"vline\",\"t\":{ts_millis}}}")
        }
        Drawing::CrossLine { anchor } => {
            format!("{{\"kind\":\"cross\",\"a\":{}}}", anchor_json(anchor))
        }
        Drawing::Trendline { a, b } => two_anchor_json("trend", a, b),
        Drawing::Ray { origin, through } => two_anchor_json("ray", origin, through),
        Drawing::ExtendedLine { a, b } => two_anchor_json("ext", a, b),
        Drawing::Arrow { a, b } => two_anchor_json("arrow", a, b),
        Drawing::Rectangle { a, b } => two_anchor_json("rect", a, b),
        Drawing::Ellipse { a, b } => two_anchor_json("ellipse", a, b),
        Drawing::Triangle { a, b, c } => format!(
            "{{\"kind\":\"triangle\",\"a\":{},\"b\":{},\"c\":{}}}",
            anchor_json(a),
            anchor_json(b),
            anchor_json(c)
        ),
        Drawing::FibRetracement { a, b } => two_anchor_json("fib", a, b),
        Drawing::RangeMeasure { a, b } => two_anchor_json("range", a, b),
        Drawing::Position {
            long,
            entry,
            target,
        } => format!(
            "{{\"kind\":\"position\",\"long\":{},\"a\":{},\"b\":{}}}",
            long,
            anchor_json(entry),
            anchor_json(target)
        ),
        Drawing::ParallelChannel { a, b, c, kind } => {
            let variant = if *kind == ChannelKind::Parallel {
                String::new()
            } else {
                format!(",\"variant\":\"{}\"", kind.key())
            };
            format!(
                "{{\"kind\":\"channel\",\"a\":{},\"b\":{},\"c\":{}{variant}}}",
                anchor_json(a),
                anchor_json(b),
                anchor_json(c)
            )
        }
        Drawing::Pitchfork { a, b, c } => format!(
            "{{\"kind\":\"pitchfork\",\"a\":{},\"b\":{},\"c\":{}}}",
            anchor_json(a),
            anchor_json(b),
            anchor_json(c)
        ),
        Drawing::FibExtension { a, b, c } => format!(
            "{{\"kind\":\"fibext\",\"a\":{},\"b\":{},\"c\":{}}}",
            anchor_json(a),
            anchor_json(b),
            anchor_json(c)
        ),
        Drawing::FibTime { a } => format!("{{\"kind\":\"fibtime\",\"a\":{}}}", anchor_json(a)),
        Drawing::Text { at, text } => format!(
            "{{\"kind\":\"text\",\"a\":{},\"text\":\"{}\"}}",
            anchor_json(at),
            json::escape(text)
        ),
        Drawing::Callout { at, text } => format!(
            "{{\"kind\":\"callout\",\"a\":{},\"text\":\"{}\"}}",
            anchor_json(at),
            json::escape(text)
        ),
        Drawing::RegressionChannel { a, b } => two_anchor_json("regression", a, b),
        Drawing::FibTrendExtension { a, b, c } => format!(
            "{{\"kind\":\"fibtrend\",\"a\":{},\"b\":{},\"c\":{}}}",
            anchor_json(a),
            anchor_json(b),
            anchor_json(c)
        ),
        Drawing::ElliottImpulse { points } => points_json("elliott_impulse", points),
        Drawing::ElliottCorrection { points } => points_json("elliott_correction", points),
        Drawing::Xabcd { points } => points_json("xabcd", points),
        Drawing::Abcd { points } => points_json("abcd", points),
        Drawing::GannFan { a, b } => two_anchor_json("gannfan", a, b),
        Drawing::Brush { points } => points_json("brush", points),
        Drawing::Highlighter { points } => points_json("highlighter", points),
        Drawing::Sticker { at, icon } => format!(
            "{{\"kind\":\"sticker\",\"a\":{},\"icon\":\"{}\"}}",
            anchor_json(at),
            icon.key()
        ),
    }
}

/// Titik pola dari `"pts":[…]`: jumlah efektif harus tepat `expect`.
fn points_from_value(v: &json::Value, expect: usize) -> Option<Vec<Anchor>> {
    let arr = v.get("pts")?.as_array()?;
    if arr.len() < expect {
        return None;
    }
    let pts: Vec<Anchor> = arr
        .iter()
        .take(expect)
        .filter_map(anchor_from_value)
        .collect();
    if pts.len() == expect {
        Some(pts)
    } else {
        None
    }
}

/// Titik garis bebas (brush/highlighter): minimal 2, maksimum `MAX_STROKE_POINTS`.
fn stroke_points_from_value(v: &json::Value) -> Option<Vec<Anchor>> {
    let arr = v.get("pts")?.as_array()?;
    let pts: Vec<Anchor> = arr
        .iter()
        .take(MAX_STROKE_POINTS)
        .filter_map(anchor_from_value)
        .collect();
    if pts.len() >= 2 {
        Some(pts)
    } else {
        None
    }
}

fn anchor_from_value(v: &json::Value) -> Option<Anchor> {
    Some(Anchor {
        ts_millis: v.get("t")?.as_i64()?,
        price_ticks: v.get("p")?.as_i64()?,
    })
}

fn drawing_from_value(v: &json::Value) -> Option<Drawing> {
    let a = || anchor_from_value(v.get("a")?);
    let b = || anchor_from_value(v.get("b")?);
    Some(match v.get("kind")?.as_str()? {
        "hline" => Drawing::HorizontalLine {
            price_ticks: v.get("price")?.as_i64()?,
        },
        "hray" => Drawing::HorizontalRay { origin: a()? },
        "vline" => Drawing::VerticalLine {
            ts_millis: v.get("t")?.as_i64()?,
        },
        "cross" => Drawing::CrossLine { anchor: a()? },
        "trend" => Drawing::Trendline { a: a()?, b: b()? },
        "ray" => Drawing::Ray {
            origin: a()?,
            through: b()?,
        },
        "ext" => Drawing::ExtendedLine { a: a()?, b: b()? },
        "arrow" => Drawing::Arrow { a: a()?, b: b()? },
        "rect" => Drawing::Rectangle { a: a()?, b: b()? },
        "ellipse" => Drawing::Ellipse { a: a()?, b: b()? },
        "triangle" => Drawing::Triangle {
            a: a()?,
            b: b()?,
            c: anchor_from_value(v.get("c")?)?,
        },
        "fib" => Drawing::FibRetracement { a: a()?, b: b()? },
        "range" => Drawing::RangeMeasure { a: a()?, b: b()? },
        "position" => Drawing::Position {
            long: v.get("long")?.as_bool()?,
            entry: a()?,
            target: b()?,
        },
        "channel" => Drawing::ParallelChannel {
            a: a()?,
            b: b()?,
            c: anchor_from_value(v.get("c")?)?,
            // Varian tak dikenal → paralel (berkas dari versi lebih baru).
            kind: v
                .get("variant")
                .and_then(json::Value::as_str)
                .and_then(ChannelKind::from_key)
                .unwrap_or(ChannelKind::Parallel),
        },
        "pitchfork" => Drawing::Pitchfork {
            a: a()?,
            b: b()?,
            c: anchor_from_value(v.get("c")?)?,
        },
        "fibext" => Drawing::FibExtension {
            a: a()?,
            b: b()?,
            c: anchor_from_value(v.get("c")?)?,
        },
        "fibtime" => Drawing::FibTime { a: a()? },
        "text" => Drawing::Text {
            at: a()?,
            text: v.get("text")?.as_str()?.to_string(),
        },
        "callout" => Drawing::Callout {
            at: a()?,
            text: v.get("text")?.as_str()?.to_string(),
        },
        "regression" => Drawing::RegressionChannel { a: a()?, b: b()? },
        "fibtrend" => Drawing::FibTrendExtension {
            a: a()?,
            b: b()?,
            c: anchor_from_value(v.get("c")?)?,
        },
        "elliott_impulse" => Drawing::ElliottImpulse {
            points: points_from_value(v, 5)?,
        },
        "elliott_correction" => Drawing::ElliottCorrection {
            points: points_from_value(v, 3)?,
        },
        "xabcd" => Drawing::Xabcd {
            points: points_from_value(v, 5)?,
        },
        "abcd" => Drawing::Abcd {
            points: points_from_value(v, 4)?,
        },
        "gannfan" => Drawing::GannFan { a: a()?, b: b()? },
        "brush" => Drawing::Brush {
            points: stroke_points_from_value(v)?,
        },
        "highlighter" => Drawing::Highlighter {
            points: stroke_points_from_value(v)?,
        },
        // Ikon tak dikenal → panah atas (berkas dari versi lebih baru).
        "sticker" => Drawing::Sticker {
            at: a()?,
            icon: v
                .get("icon")
                .and_then(json::Value::as_str)
                .and_then(StickerIcon::from_key)
                .unwrap_or_default(),
        },
        _ => return None,
    })
}

/// Parser input tanggal/waktu untuk "Go to date" (Alt+G).
/// Menerima epoch detik/milidetik (digit) atau `YYYY-MM-DD[ T]HH:MM[:SS]` (UTC).
pub fn parse_ts_input(text: &str) -> Result<i64, String> {
    let t = text.trim();
    if t.is_empty() {
        return Err("masukkan tanggal atau epoch".into());
    }
    if t.bytes().all(|b| b.is_ascii_digit()) {
        let v: i64 = t.parse().map_err(|_| format!("angka terlalu besar: {t}"))?;
        return if v.unsigned_abs() < 1_000_000_000_000 {
            v.checked_mul(1000)
                .ok_or_else(|| "epoch terlalu besar".to_string())
        } else {
            Ok(v)
        };
    }
    let (date_part, time_part) = match t.split_once([' ', 'T']) {
        Some((d, tm)) => (d, Some(tm)),
        None => (t, None),
    };
    let mut d = date_part.split('-');
    let y: i64 = d
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or("format: YYYY-MM-DD")?;
    let mo: i64 = d
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or("format: YYYY-MM-DD")?;
    let da: i64 = d
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or("format: YYYY-MM-DD")?;
    if d.next().is_some() {
        return Err("format: YYYY-MM-DD".into());
    }
    if !(1..=12).contains(&mo) || !(1..=31).contains(&da) {
        return Err("bulan 1–12, tanggal 1–31".into());
    }
    if !(-9999..=9999).contains(&y) {
        return Err("tahun di luar rentang -9999..9999".into());
    }
    let (mut hh, mut mi, mut ss) = (0i64, 0i64, 0i64);
    if let Some(tm) = time_part {
        let mut parts = tm.split(':');
        hh = parts
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or("format jam: HH:MM[:SS]")?;
        mi = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        ss = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        if parts.next().is_some()
            || !(0..24).contains(&hh)
            || !(0..60).contains(&mi)
            || !(0..60).contains(&ss)
        {
            return Err("jam 0–23, menit/detik 0–59".into());
        }
    }
    let days = days_from_civil(y, mo as u32, da as u32) as i128;
    let secs = days * 86_400 + hh as i128 * 3600 + mi as i128 * 60 + ss as i128;
    let ms = secs * 1000;
    if ms < i64::MIN as i128 || ms > i64::MAX as i128 {
        return Err("tanggal di luar rentang".into());
    }
    Ok(ms as i64)
}

/// days-from-civil (algoritma Hinnant, UTC, integer murni).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) as i64 + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Tambah garis horizontal pada harga tertentu (tool garis orisinal).
pub fn horizontal_line(price_ticks: i64) -> Drawing {
    Drawing::HorizontalLine { price_ticks }
}

pub fn trendline(a: Anchor, b: Anchor) -> Drawing {
    Drawing::Trendline { a, b }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sokanlab_domain::Bar;

    fn bars(n: usize) -> Vec<Bar> {
        (0..n)
            .map(|i| Bar {
                ts_millis: 1_000 + i as i64 * 60_000,
                open_ticks: 100 + i as i64,
                high_ticks: 110 + i as i64,
                low_ticks: 90 + i as i64,
                close_ticks: 105 + i as i64,
                volume_milli: 10,
            })
            .collect()
    }

    #[test]
    fn quit_maps_to_closed() {
        assert_eq!(handle(Command::Quit), Event::Closed);
    }

    #[test]
    fn load_fits_price_and_range() {
        let mut s = ChartState::new();
        s.load(bars(50));
        assert_eq!(s.viewport.t_min, 1_000);
        assert!(s.viewport.p_min <= 90);
        assert!(s.viewport.p_max >= 159);
    }

    #[test]
    fn undo_redo_through_state() {
        let mut s = ChartState::new();
        s.add_drawing(horizontal_line(100));
        assert_eq!(s.drawings.items.len(), 1);
        s.apply(Command::Undo);
        assert_eq!(s.drawings.items.len(), 0);
        s.apply(Command::Redo);
        assert_eq!(s.drawings.items.len(), 1);
    }

    #[test]
    fn remove_selected_then_ignored() {
        let mut s = ChartState::new();
        s.add_drawing(horizontal_line(100));
        assert_eq!(s.apply(Command::RemoveSelected), Event::DrawingsChanged);
        assert_eq!(s.apply(Command::RemoveSelected), Event::Ignored);
    }

    #[test]
    fn reset_view_kembali_ke_seluruh_data() {
        let mut s = ChartState::new();
        s.load(bars(50));
        s.viewport = s.viewport.zoom_time(0.1, 100.0);
        s.viewport.t_min += 5_000;
        s.apply(Command::ResetView);
        assert_eq!(s.viewport.t_min, 1_000);
        assert_eq!(s.viewport.t_max, 1_000 + 49 * 60_000);
    }

    #[test]
    fn pan_bars_kanan_kiri() {
        let mut s = ChartState::new();
        s.load(bars(50));
        let t0 = s.viewport.t_min;
        s.pan_bars(-1); // arrow kiri → lihat lebih lama
        assert!(s.viewport.t_min < t0);
        let t1 = s.viewport.t_min;
        s.pan_bars(1);
        assert!(s.viewport.t_min > t1);
    }

    #[test]
    fn zoom_center_mengubah_rentang() {
        let mut s = ChartState::new();
        s.load(bars(50));
        let before = s.viewport.t_max - s.viewport.t_min;
        s.zoom_center(0.5);
        assert!(s.viewport.t_max - s.viewport.t_min < before);
        s.zoom_center(2.0);
        assert!(s.viewport.t_max - s.viewport.t_min > before / 2);
    }

    #[test]
    fn anchor_at_magnet_menempel_ohlc() {
        let mut s = ChartState::new();
        s.load(bars(50));
        s.viewport = s.viewport.resize(800.0, 400.0);
        s.tool.magnet = Magnet::Weak;
        let spacing = s.spacing();
        let x = sokanlab_chart::drawing::bar_center_x(10, s.visible_start(), spacing);
        let y = s.viewport.y_for_price(s.bars[10].close_ticks);
        let (a, snapped) = s.anchor_at((x, y), 6.0);
        assert!(snapped);
        assert_eq!(a.ts_millis, s.bars[10].ts_millis);
        assert_eq!(a.price_ticks, s.bars[10].close_ticks);
    }

    #[test]
    fn handle_hit_menemukan_anchor_terpilih() {
        let mut s = ChartState::new();
        s.load(bars(50));
        s.viewport = s.viewport.resize(800.0, 400.0);
        let d = Drawing::Trendline {
            a: Anchor {
                ts_millis: s.bars[5].ts_millis,
                price_ticks: s.bars[5].high_ticks,
            },
            b: Anchor {
                ts_millis: s.bars[20].ts_millis,
                price_ticks: s.bars[20].low_ticks,
            },
        };
        s.add_drawing(d.clone());
        let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        let layout = painter::split_plot(area, s.spacing());
        let p = painter::handle_points(&layout, &s.viewport, &d)[0].1;
        let local = (
            p.x as f64 - layout.price.min.x as f64,
            p.y as f64 - layout.price.min.y as f64,
        );
        let hit = s.handle_hit(&layout, local, 8.0);
        assert_eq!(hit, Some((0, 0)));
        assert!(s
            .handle_hit(&layout, (local.0 + 500.0, local.1), 8.0)
            .is_none());
    }

    #[test]
    fn layout_json_has_schema_and_counts() {
        let mut s = ChartState::new();
        s.load(bars(3));
        s.add_drawing(horizontal_line(100));
        s.watchlist.push("BTC/USDT".into());
        let j = s.layout_json();
        assert!(j.contains("\"schema\":7"));
        assert!(j.contains("\"kind\":\"hline\""));
        assert!(j.contains("BTC/USDT"));
    }

    #[test]
    fn layout_roundtrip_full() {
        let mut a = ChartState::new();
        a.load(bars(50));
        a.theme = theme::Theme::Light;
        a.watchlist = vec!["BTC/USDT".into(), "ETH\"USD".into()];
        a.add_drawing(horizontal_line(100));
        a.add_drawing(Drawing::VerticalLine { ts_millis: 3_000 });
        a.add_drawing(trendline(
            Anchor {
                ts_millis: 1_000,
                price_ticks: 90,
            },
            Anchor {
                ts_millis: 50_000,
                price_ticks: 140,
            },
        ));
        a.add_drawing(Drawing::Ray {
            origin: Anchor {
                ts_millis: 2_000,
                price_ticks: 100,
            },
            through: Anchor {
                ts_millis: 4_000,
                price_ticks: 120,
            },
        });
        a.add_drawing(Drawing::Rectangle {
            a: Anchor {
                ts_millis: 5_000,
                price_ticks: 80,
            },
            b: Anchor {
                ts_millis: 9_000,
                price_ticks: 160,
            },
        });
        // Semua jenis baru ikut dipulihkan.
        a.add_drawing(Drawing::HorizontalRay {
            origin: Anchor {
                ts_millis: 2_000,
                price_ticks: 95,
            },
        });
        a.add_drawing(Drawing::CrossLine {
            anchor: Anchor {
                ts_millis: 3_500,
                price_ticks: 130,
            },
        });
        a.add_drawing(Drawing::ExtendedLine {
            a: Anchor {
                ts_millis: 1_000,
                price_ticks: 110,
            },
            b: Anchor {
                ts_millis: 4_000,
                price_ticks: 100,
            },
        });
        a.add_drawing(Drawing::Arrow {
            a: Anchor {
                ts_millis: 2_000,
                price_ticks: 145,
            },
            b: Anchor {
                ts_millis: 5_500,
                price_ticks: 155,
            },
        });
        a.add_drawing(Drawing::Ellipse {
            a: Anchor {
                ts_millis: 1_500,
                price_ticks: 100,
            },
            b: Anchor {
                ts_millis: 4_500,
                price_ticks: 130,
            },
        });
        a.add_drawing(Drawing::Triangle {
            a: Anchor {
                ts_millis: 1_000,
                price_ticks: 95,
            },
            b: Anchor {
                ts_millis: 5_000,
                price_ticks: 95,
            },
            c: Anchor {
                ts_millis: 3_000,
                price_ticks: 125,
            },
        });
        a.add_drawing(Drawing::FibRetracement {
            a: Anchor {
                ts_millis: 2_000,
                price_ticks: 160,
            },
            b: Anchor {
                ts_millis: 6_000,
                price_ticks: 105,
            },
        });
        a.add_drawing(Drawing::RangeMeasure {
            a: Anchor {
                ts_millis: 1_000,
                price_ticks: 100,
            },
            b: Anchor {
                ts_millis: 3_000,
                price_ticks: 120,
            },
        });
        a.add_drawing(Drawing::Position {
            long: false,
            entry: Anchor {
                ts_millis: 2_500,
                price_ticks: 115,
            },
            target: Anchor {
                ts_millis: 5_000,
                price_ticks: 95,
            },
        });
        a.tool.magnet = Magnet::Strong;
        a.tool.stay = true;
        a.drawings_hidden = true;
        a.viewport.t_min = 12_345;
        a.viewport.t_max = 67_890;

        // Metadata objek (batch 2): gaya, kunci, nama, sembunyi per objek.
        a.drawings.items[0].locked = true;
        a.drawings.items[0].name = Some("kunci \"penting\"".into());
        a.drawings.items[1].style = Some(ItemStyle {
            color: [0x12, 0x34, 0x56],
            width: 3,
            dash: DashStyle::Solid,
        });
        a.drawings.items[2].visible = false;

        let mut b = ChartState::new();
        b.restore_layout_json(&a.layout_json()).unwrap();
        assert_eq!(b.theme, a.theme);
        assert_eq!(b.drawings.items, a.drawings.items);
        assert_eq!(b.watchlist, a.watchlist);
        assert_eq!(b.viewport.t_min, a.viewport.t_min);
        assert_eq!(b.viewport.t_max, a.viewport.t_max);
        assert_eq!(b.viewport.p_min, a.viewport.p_min);
        assert_eq!(b.viewport.p_max, a.viewport.p_max);
        assert_eq!(b.tool.magnet, Magnet::Strong);
        assert!(b.tool.stay);
        assert!(b.drawings_hidden);
        assert_eq!(b.tool.kind, tools::ToolKind::Select);
        // Undo/redo tidak ikut dipulihkan (riwayat edit bukan bagian layout).
        assert!(b.drawings.undo.is_empty() && b.drawings.redo.is_empty());
        // Label nama kustom bertahan (escape kutip diuji di sini).
        assert_eq!(b.drawings.items[0].label(), "kunci \"penting\"");
        // Serialisasi ulang identik byte-for-byte.
        assert_eq!(b.layout_json(), a.layout_json());
    }

    #[test]
    fn parse_ts_input_menerima_epoch_dan_tanggal() {
        assert_eq!(parse_ts_input("1700000000").unwrap(), 1_700_000_000_000);
        assert_eq!(parse_ts_input("1700000000000").unwrap(), 1_700_000_000_000);
        assert_eq!(parse_ts_input("2023-11-14").unwrap(), 1_699_920_000_000);
        assert_eq!(
            parse_ts_input("2023-11-14 13:30").unwrap(),
            1_699_920_000_000 + 13 * 3_600_000 + 30 * 60_000
        );
        assert_eq!(
            parse_ts_input("2023-11-14T00:00:05").unwrap(),
            1_699_920_000_000 + 5_000
        );
        // salah format → pesan jelas, bukan panic
        assert!(parse_ts_input("").is_err());
        assert!(parse_ts_input("besok").is_err());
        assert!(parse_ts_input("2023-13-01").is_err());
        assert!(parse_ts_input("2023-01-01 25:00").is_err());
        assert!(parse_ts_input("2023-01-01 10:99").is_err());
        assert!(parse_ts_input("99999999999999999999").is_err());
        // tahun ekstrem ditolak (bukan overflow)
        assert!(parse_ts_input("99999999999-01-01").is_err());
        assert!(parse_ts_input("9223372036854775807-06-15").is_err());
    }

    #[test]
    fn center_on_time_dan_reset_sumbu() {
        let mut s = ChartState::new();
        s.load(bars(50));
        let span = s.viewport.t_max - s.viewport.t_min;
        let ts = s.bars[25].ts_millis;
        s.center_on_time(ts);
        assert_eq!(s.viewport.t_max - s.viewport.t_min, span);
        assert_eq!(
            (s.viewport.t_min + s.viewport.t_max) / 2,
            ts,
            "titik tengah = waktu target"
        );
        // waktu ekstrem: saturating, bukan panic
        s.center_on_time(i64::MAX);
        s.center_on_time(i64::MIN);
        assert!(s.viewport.t_min < s.viewport.t_max);
        // reset sumbu harga: fit ke bar terlihat
        s.viewport.p_min = 0;
        s.viewport.p_max = 1;
        s.reset_price_scale();
        assert!(s.viewport.p_min < s.viewport.p_max);
        assert!(s.viewport.p_min <= 90);
        // reset sumbu waktu: kembali ke rentang penuh
        s.viewport.t_min += 5_000;
        s.viewport.t_max += 5_000;
        s.reset_time_scale();
        assert_eq!(s.viewport.t_min, 1_000);
        assert_eq!(s.viewport.t_max, 1_000 + 49 * 60_000);
    }

    #[test]
    fn layout_v1_tanpa_meta_mendapat_default() {
        // Berkas v1 (tanpa metadata) harus tetap pulih dengan nilai default.
        let j = r#"{"schema":1,"drawings":[{"kind":"hline","price":99},{"kind":"trend","a":{"t":1,"p":2},"b":{"t":3,"p":4}}]}"#;
        let mut s = ChartState::new();
        s.restore_layout_json(j).unwrap();
        assert_eq!(s.drawings.items.len(), 2);
        assert!(s.drawings.items[0].visible);
        assert!(!s.drawings.items[0].locked);
        assert!(s.drawings.items[0].style.is_none());
        assert!(s.drawings.items[0].name.is_none());
        // Metadata lengkap dipulihkan (termasuk batas lebar 1..=4).
        let j2 = r##"{"schema":2,"drawings":[{"kind":"hline","price":1,"style":{"color":"#ff8800","width":9},"visible":false,"locked":true,"name":"x"}]}"##;
        let mut s2 = ChartState::new();
        s2.restore_layout_json(j2).unwrap();
        let it = &s2.drawings.items[0];
        assert!(!it.visible);
        assert!(it.locked);
        assert_eq!(it.style.unwrap().color, [0xff, 0x88, 0x00]);
        assert_eq!(it.style.unwrap().width, 4); // di-clamp
        assert_eq!(it.name.as_deref(), Some("x"));
    }

    #[test]
    fn layout_v3_skala_overlay_dash_grup_roundtrip() {
        let mut a = ChartState::new();
        a.load(bars(30));
        a.set_price_scale(ScaleMode::Log);
        a.viewport.invert = true;
        a.set_overlay_settings(OverlaySettings {
            sma: Some(5),
            ema: Some(3),
            vwap: true,
        });
        a.set_panes(vec![
            PaneSetting {
                kind: PaneKind::Rsi,
                period: 7,
                slow: 0,
                signal: 0,
            },
            PaneSetting {
                kind: PaneKind::Macd,
                period: 5,
                slow: 13,
                signal: 4,
            },
        ]);
        a.add_drawing(Drawing::Text {
            at: Anchor {
                ts_millis: 2_000,
                price_ticks: 111,
            },
            text: "catatan \"A\"".into(),
        });
        a.selected = Some(0);
        assert!(a.toggle_group_selected());
        a.drawings.items[0].style = Some(ItemStyle {
            color: [0x11, 0x22, 0x33],
            width: 2,
            dash: DashStyle::Dotted,
        });
        let j = a.layout_json();
        assert!(j.contains("\"schema\":7"));
        assert!(j.contains("\"scale\":\"log\""));
        assert!(j.contains("\"invert\":true"));
        assert!(j.contains("\"dash\":\"dotted\""));
        assert!(j.contains("\"group\":1"));
        assert!(j.contains("catatan"));
        assert!(j.contains("\"panes\":[{\"kind\":\"rsi\",\"period\":7}"));
        let mut b = ChartState::new();
        b.load(bars(30));
        b.restore_layout_json(&j).unwrap();
        assert_eq!(b.viewport.scale, ScaleMode::Log);
        assert!(b.viewport.invert);
        assert_eq!(b.overlay_settings, a.overlay_settings);
        assert_eq!(b.overlay_lines.len(), 3, "sma+ema+vwap terhitung");
        assert_eq!(b.drawings.items, a.drawings.items);
        assert_eq!(b.pane_settings, a.pane_settings, "pane dipulihkan");
        assert_eq!(b.panes.len(), 2, "rsi+macd terhitung");
        assert_eq!(b.panes[0].kind, PaneKind::Rsi);
        assert_eq!(b.panes[0].lines[0].0.len(), b.bars.len());
        assert!(
            b.panes[0].lines[0]
                .0
                .iter()
                .all(|v| v.is_none() || (0..=100).contains(v.as_ref().unwrap())),
            "RSI dalam 0–100"
        );
        assert_eq!(b.panes[1].kind, PaneKind::Macd);
        assert_eq!(b.panes[1].lines.len(), 2);
        assert_eq!(b.panes[1].hist.len(), b.bars.len());
        assert_eq!(b.layout_json(), j, "roundtrip identik");
    }

    #[test]
    fn schema5_metadata_objek_roundtrip() {
        let mut a = ChartState::new();
        a.load(bars(30));
        a.add_drawing(Drawing::Pitchfork {
            a: Anchor {
                ts_millis: 1_000,
                price_ticks: 100,
            },
            b: Anchor {
                ts_millis: 5_000,
                price_ticks: 120,
            },
            c: Anchor {
                ts_millis: 9_000,
                price_ticks: 110,
            },
        });
        a.drawings.items[0].pitchfork = PitchforkKind::Schiff;
        a.drawings.items[0].opacity = 40;
        a.add_drawing(Drawing::FibRetracement {
            a: Anchor {
                ts_millis: 1_000,
                price_ticks: 100,
            },
            b: Anchor {
                ts_millis: 9_000,
                price_ticks: 140,
            },
        });
        let sel = a.selected.unwrap();
        a.drawings.items[sel].fib_levels = vec![
            FibLevel {
                milli: 500,
                color: Some([0xf2, 0xb0, 0x4d]),
            },
            FibLevel {
                milli: 425,
                color: None,
            },
        ];
        a.drawings.items[sel].style = Some(ItemStyle {
            color: [0x11, 0x22, 0x33],
            width: 2,
            dash: DashStyle::Solid,
        });
        assert!(a.set_default_style_from_selected());
        assert_eq!(a.default_style, a.drawings.items[sel].style);
        let j = a.layout_json();
        assert!(j.contains("\"schema\":7"));
        assert!(j.contains("\"pitchfork\":\"schiff\""));
        assert!(j.contains("\"opacity\":40"));
        assert!(j.contains("\"fib\":[{\"r\":500,\"c\":\"#f2b04d\"},{\"r\":425}]"));
        assert!(j.contains("\"default_style\":{\"color\":\"#112233\",\"width\":2}"));
        let mut b = ChartState::new();
        b.load(bars(30));
        b.restore_layout_json(&j).unwrap();
        assert_eq!(b.drawings.items, a.drawings.items, "metadata objek utuh");
        assert_eq!(b.default_style, a.default_style);
        assert_eq!(b.layout_json(), j, "roundtrip identik");
    }

    #[test]
    fn default_style_dipakai_objek_baru_dan_seragam_sejenis() {
        let mut s = ChartState::new();
        s.load(bars(30));
        let tl = |p0: i64, p1: i64| Drawing::Trendline {
            a: Anchor {
                ts_millis: 1_000,
                price_ticks: p0,
            },
            b: Anchor {
                ts_millis: 5_000,
                price_ticks: p1,
            },
        };
        s.add_drawing(tl(100, 120));
        s.add_drawing(Drawing::Rectangle {
            a: Anchor {
                ts_millis: 2_000,
                price_ticks: 100,
            },
            b: Anchor {
                ts_millis: 6_000,
                price_ticks: 130,
            },
        });
        let st = ItemStyle {
            color: [0x33, 0x99, 0xcc],
            width: 3,
            dash: DashStyle::Dashed,
        };
        s.selected = Some(0);
        s.drawings.items[0].style = Some(st);
        s.drawings.items[0].opacity = 55;
        assert!(s.set_default_style_from_selected());
        // Objek baru mewarisi gaya default (opasitas tetap penuh).
        s.add_drawing(tl(100, 140));
        let last = s.drawings.items.len() - 1;
        assert_eq!(s.drawings.items[last].style, Some(st));
        assert_eq!(s.drawings.items[last].opacity, 100);
        // Terapkan ke sejenis: dua trendline disamakan, kotak tidak tersentuh.
        s.selected = Some(0);
        assert_eq!(s.apply_style_to_same_kind(), 2);
        assert_eq!(s.drawings.items[1].style, None);
        assert_eq!(s.drawings.items[1].opacity, 100);
        assert_eq!(s.drawings.items[last].style, Some(st));
        assert_eq!(s.drawings.items[last].opacity, 55);
        // Tanpa seleksi tidak ada yang berubah.
        s.selected = None;
        assert_eq!(s.apply_style_to_same_kind(), 0);
        assert!(!s.set_default_style_from_selected());
    }

    #[test]
    fn berkas_lama_tanpa_metadata_baru_tetap_terbaca() {
        let mut s = ChartState::new();
        s.load(bars(10));
        let j = r##"{"schema":4,"drawings":[{"kind":"pitchfork","a":{"t":1,"p":2},"b":{"t":3,"p":4},"c":{"t":5,"p":6}},{"kind":"fib","a":{"t":1,"p":2},"b":{"t":3,"p":4}}]}"##;
        s.restore_layout_json(j).unwrap();
        assert_eq!(s.drawings.items.len(), 2);
        assert_eq!(s.drawings.items[0].pitchfork, PitchforkKind::Andrew);
        assert_eq!(s.drawings.items[0].opacity, 100);
        assert!(s.drawings.items[0].fib_levels.is_empty());
        assert!(s.drawings.items[1].fib_levels.is_empty());
        assert_eq!(s.default_style, None);
        // Nilai liar dijepit; kunci pitchfork tak dikenal diabaikan (Andrew).
        let j2 = r##"{"schema":5,"default_style":{"color":"#0a0b0c","width":9,"dash":"dashed"},"drawings":[{"kind":"pitchfork","a":{"t":1,"p":2},"b":{"t":3,"p":4},"c":{"t":5,"p":6},"pitchfork":"noise","opacity":250}]}"##;
        s.restore_layout_json(j2).unwrap();
        assert_eq!(s.drawings.items[0].pitchfork, PitchforkKind::Andrew);
        assert_eq!(
            s.drawings.items[0].opacity, 100,
            "opacity 250 dijepit ke 100"
        );
        assert_eq!(
            s.default_style,
            Some(ItemStyle {
                color: [0x0a, 0x0b, 0x0c],
                width: 4,
                dash: DashStyle::Dashed
            })
        );
    }

    #[test]
    fn set_panes_maks_tiga_dan_histogram_macd() {
        let mut s = ChartState::new();
        s.load(bars(60));
        s.set_panes(vec![
            PaneSetting {
                kind: PaneKind::Rsi,
                period: 14,
                slow: 0,
                signal: 0,
            },
            PaneSetting {
                kind: PaneKind::Macd,
                period: 3,
                slow: 6,
                signal: 3,
            },
            PaneSetting {
                kind: PaneKind::Rsi,
                period: 7,
                slow: 0,
                signal: 0,
            },
            PaneSetting {
                kind: PaneKind::Rsi,
                period: 5,
                slow: 0,
                signal: 0,
            },
        ]);
        assert_eq!(s.pane_settings.len(), 3, "dipotong ke maksimum 3 pane");
        assert_eq!(s.panes.len(), 3);
        assert_eq!(s.panes[1].label, "MACD 3 6 3");
        let (m, sig, hist) = (
            &s.panes[1].lines[0].0,
            &s.panes[1].lines[1].0,
            &s.panes[1].hist,
        );
        for i in 0..s.bars.len() {
            match (m[i], sig[i], hist[i]) {
                (Some(a), Some(b), Some(h)) => assert_eq!(h, a - b, "hist = macd − sinyal"),
                _ => assert!(hist[i].is_none(), "hist kosong saat warm-up"),
            }
        }
        // Setelan identik → cache tidak dihitung ulang (nilai tetap).
        let before = s.panes.clone();
        s.set_panes(s.pane_settings.clone());
        assert_eq!(s.panes, before);
    }

    #[test]
    fn layout_pane_tak_dikenal_dilewati() {
        let mut s = ChartState::new();
        s.load(bars(10));
        s.restore_layout_json(
            r#"{"schema":4,"panes":[{"kind":"volume"},{"kind":"rsi","period":9}]}"#,
        )
        .unwrap();
        assert_eq!(s.pane_settings.len(), 1, "entry tak dikenal dilewati");
        assert_eq!(s.pane_settings[0].period, 9);
        assert_eq!(s.panes.len(), 1);
    }

    #[test]
    fn layout_skala_tidak_dikenal_error() {
        let mut s = ChartState::new();
        assert!(s
            .restore_layout_json(
                r#"{"schema":3,"viewport":{"t_min":1,"t_max":2,"p_min":0,"p_max":1,"scale":"kubik"}}"#
            )
            .is_err());
    }

    #[test]
    fn skala_persen_basis_dari_bar_terlihat() {
        let mut s = ChartState::new();
        s.load(bars(20));
        assert_eq!(s.viewport.pct_base, 0);
        s.set_price_scale(ScaleMode::Percent);
        assert_eq!(s.viewport.pct_base, s.bars[0].close_ticks);
        s.set_price_scale(ScaleMode::Linear);
        assert_eq!(s.viewport.pct_base, 0);
        assert!(s.layout_json().contains("\"scale\":\"linear\""));
    }

    #[test]
    fn duplikat_z_dan_grup_lewat_state() {
        let mut s = ChartState::new();
        s.load(bars(20));
        s.add_drawing(horizontal_line(100));
        s.add_drawing(Drawing::VerticalLine { ts_millis: 1_000 });
        assert_eq!(s.selected, Some(1));
        assert!(s.move_selected_z(-1));
        assert_eq!(s.selected, Some(0));
        assert_eq!(
            s.drawings.items[0].drawing,
            Drawing::VerticalLine { ts_millis: 1_000 }
        );
        assert!(!s.move_selected_z(-1), "sudah paling belakang");
        assert!(s.duplicate_selected());
        assert_eq!(s.drawings.items.len(), 3);
        assert_eq!(s.selected, Some(2));
        assert_eq!(
            s.drawings.items[2].drawing,
            Drawing::VerticalLine {
                ts_millis: 1_000 + 120_000
            },
            "duplikat bergeser 2 bar"
        );
        assert!(s.toggle_group_selected());
        assert_eq!(s.selected_group(), Some(1));
        assert!(s.toggle_group_selected());
        assert_eq!(s.selected_group(), None);
    }

    #[test]
    fn teks_dan_callout_json_roundtrip() {
        let mut s = ChartState::new();
        s.add_drawing(Drawing::Text {
            at: Anchor {
                ts_millis: 1,
                price_ticks: 2,
            },
            text: "halo \"B\"".into(),
        });
        s.add_drawing(Drawing::Callout {
            at: Anchor {
                ts_millis: 3,
                price_ticks: 4,
            },
            text: "titik".into(),
        });
        let j = s.layout_json();
        let mut b = ChartState::new();
        b.restore_layout_json(&j).unwrap();
        assert_eq!(b.drawings.items, s.drawings.items);
    }

    #[test]
    fn layout_save_and_load_file_roundtrip() {
        let mut a = ChartState::new();
        a.load(bars(10));
        a.watchlist = vec!["SYNTH/USDT".into()];
        a.add_drawing(horizontal_line(123));
        let dir = std::env::temp_dir().join("sokanlab-layout-test");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("layout.json");
        a.save_layout(&path).unwrap();
        let mut b = ChartState::new();
        b.restore_layout(&path).unwrap();
        assert_eq!(b.drawings.items, a.drawings.items);
        assert_eq!(b.watchlist, a.watchlist);
        // Berkas hilang → error yang jelas, bukan panic.
        let missing = dir.join("tidak-ada.json");
        assert!(b.restore_layout(&missing).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn layout_old_format_numbers_tolerated() {
        // Format lama: "drawings" berupa angka; field lain tetap dipulihkan.
        let j = r#"{"schema":1,"theme":"light","viewport":{"t_min":1,"t_max":2,"p_min":0.0,"p_max":1.0},"watchlist":["A"],"drawings":3}"#;
        let mut s = ChartState::new();
        s.restore_layout_json(j).unwrap();
        assert_eq!(s.theme, theme::Theme::Light);
        assert_eq!(s.watchlist, vec!["A".to_string()]);
        assert!(s.drawings.items.is_empty());
        // Tanpa field tools → default tetap terpakai.
        assert_eq!(s.tool.magnet, Magnet::Off);
        assert!(!s.tool.stay);
    }

    #[test]
    fn layout_invalid_json_or_schema_leaves_state_untouched() {
        let mut s = ChartState::new();
        s.load(bars(5));
        s.watchlist = vec!["KEEP".into()];
        let before = s.layout_json();
        assert!(s.restore_layout_json("{rusak").is_err());
        assert!(s
            .restore_layout_json(r#"{"schema":9,"theme":"dark"}"#)
            .is_err());
        assert!(s
            .restore_layout_json(r#"{"schema":1,"theme":"neon"}"#)
            .is_err());
        assert!(s
            .restore_layout_json(r#"{"schema":1,"tools":{"magnet":"plasma"}}"#)
            .is_err());
        assert_eq!(s.layout_json(), before);
    }

    #[test]
    fn zoom_and_pan_change_viewport() {
        let mut s = ChartState::new();
        s.load(bars(50));
        let before = s.viewport.t_max - s.viewport.t_min;
        let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 400.0));
        s.zoom_at(0.5, egui::pos2(400.0, 200.0), area);
        assert!(s.viewport.t_max - s.viewport.t_min < before);
        let t0 = s.viewport.t_min;
        s.pan(egui::vec2(80.0, 0.0), area);
        assert!(s.viewport.t_min < t0);
    }

    // ---- Batch 6: multi-seleksi & operasi seleksi banyak ----

    fn vline(ts: i64) -> Drawing {
        Drawing::VerticalLine { ts_millis: ts }
    }

    #[test]
    fn toggle_seleksi_multi_dan_primary_pindah() {
        let mut s = ChartState::new();
        for i in 0..4 {
            s.add_drawing(vline(1_000 * (i + 1)));
        }
        assert_eq!(s.selected, Some(3), "objek terakhir langsung terpilih");
        s.toggle_in_selection(1);
        s.toggle_in_selection(2);
        assert_eq!(s.selection(), vec![1, 2, 3]);
        assert_eq!(s.selected, Some(3), "primer tidak berubah");
        // Lepas primer → anggota teratas tersisa jadi primer.
        s.toggle_in_selection(3);
        assert_eq!(s.selected, Some(2));
        assert_eq!(s.selection(), vec![1, 2]);
        // Ctrl+klik objek yang sudah terpilih (non-primer) = lepas.
        s.toggle_in_selection(1);
        assert_eq!(s.selection(), vec![2]);
        assert_eq!(s.selected, Some(2));
        // Lepas primer terakhir → seleksi kosong.
        s.toggle_in_selection(2);
        assert_eq!(s.selection(), Vec::<usize>::new());
        s.toggle_in_selection(0);
        assert!(s.is_selected(0));
        assert!(!s.is_selected(1));
    }

    #[test]
    fn set_selection_membuang_indeks_duplikat_dan_tak_valid() {
        let mut s = ChartState::new();
        for i in 0..3 {
            s.add_drawing(vline(1_000 * (i + 1)));
        }
        s.set_selection(&[1, 1, 0, 9]);
        assert_eq!(s.selection(), vec![0, 1]);
        assert_eq!(s.selected, Some(1), "primer = indeks terbesar (teratas)");
        s.set_selection(&[]);
        assert!(s.selection().is_empty());
        assert_eq!(s.selected, None);
    }

    #[test]
    fn hapus_banyak_sekali_undo_dan_indeks_seleksi_ikut_benar() {
        let mut s = ChartState::new();
        for i in 0..4 {
            s.add_drawing(vline(1_000 * (i + 1)));
        }
        let undo_before = s.drawings.undo.len();
        s.set_selection(&[1, 2]);
        assert_eq!(s.delete_selection(), 2);
        assert_eq!(s.drawings.undo.len(), undo_before + 1, "satu langkah undo");
        assert_eq!(s.drawings.items.len(), 2);
        assert!(s.selection().is_empty(), "seleksi dibersihkan");
        // Undo mengembalikan keduanya.
        assert_eq!(s.apply(Command::Undo), Event::DrawingsChanged);
        assert_eq!(s.drawings.items.len(), 4);
        assert!(s.selection().is_empty());
        // Command::RemoveSelected juga menghapus semua yang terpilih.
        s.set_selection(&[0, 3]);
        assert_eq!(s.apply(Command::RemoveSelected), Event::DrawingsChanged);
        assert_eq!(s.drawings.items.len(), 2);
        assert_eq!(s.apply(Command::RemoveSelected), Event::Ignored);
    }

    #[test]
    fn z_dan_duplikat_dan_grup_multi() {
        let mut s = ChartState::new();
        s.load(bars(5));
        for i in 0..3 {
            s.add_drawing(vline(1_000 * (i + 1)));
        }
        s.set_selection(&[0, 1]);
        assert!(s.move_selected_z(1), "dua objek bisa naik satu langkah");
        assert_eq!(s.selection(), vec![1, 2]);
        assert!(matches!(
            s.drawings.items[1].drawing,
            Drawing::VerticalLine { ts_millis: 1_000 }
        ));
        assert!(matches!(
            s.drawings.items[2].drawing,
            Drawing::VerticalLine { ts_millis: 2_000 }
        ));
        assert!(!s.move_selected_z(1), "sudah mentok di atas");
        // Duplikat banyak: satu langkah undo, seleksi pindah ke salinan.
        let undo_before = s.drawings.undo.len();
        s.set_selection(&[0, 1]);
        assert!(s.duplicate_selected());
        assert_eq!(s.drawings.items.len(), 5);
        assert_eq!(s.drawings.undo.len(), undo_before + 1);
        assert_eq!(s.selection(), vec![3, 4]);
        // Grup multi: satu id grup untuk keduanya.
        assert!(s.toggle_group_selected());
        let g = s.selected_group().expect("primer bergrup");
        assert_eq!(s.drawings.items[3].group, Some(g));
        assert_eq!(s.drawings.items[4].group, Some(g));
        assert_eq!(s.drawings.undo.len(), undo_before + 2);
        assert!(s.toggle_group_selected(), "lepas grup");
        assert!(s.drawings.items[3].group.is_none());
        assert!(s.drawings.items[4].group.is_none());
    }

    #[test]
    fn sync_gaya_multi_dari_primary() {
        let mut s = ChartState::new();
        for i in 0..3 {
            s.add_drawing(vline(1_000 * (i + 1)));
        }
        let st = ItemStyle {
            color: [0x33, 0x99, 0xcc],
            width: 3,
            dash: DashStyle::Dashed,
        };
        s.drawings.items[1].style = Some(st);
        s.drawings.items[1].opacity = 40;
        s.set_selection(&[0, 1, 2]);
        assert_eq!(s.selected, Some(2));
        // Sinkron dari primer (2) yang gaya default → semua jadi default.
        assert_eq!(s.sync_style_from_primary(), 2);
        assert_eq!(s.drawings.items[1].style, None);
        assert_eq!(s.drawings.items[1].opacity, 100);
        // Primer diubah → anggota lain ikut.
        s.select_only(1);
        s.drawings.items[1].style = Some(st);
        s.drawings.items[1].opacity = 40;
        s.set_selection(&[0, 1]);
        assert_eq!(s.sync_style_from_primary(), 1);
        assert_eq!(s.drawings.items[0].style, Some(st));
        assert_eq!(s.drawings.items[0].opacity, 40);
        // Seleksi tunggal: tidak ada yang disinkronkan.
        s.select_only(0);
        assert_eq!(s.sync_style_from_primary(), 0);
    }

    #[test]
    fn template_gaya_simpan_terapkan_hapus_multi() {
        let mut s = ChartState::new();
        for i in 0..3 {
            s.add_drawing(vline(1_000 * (i + 1)));
        }
        let st = ItemStyle {
            color: [0x33, 0x99, 0xcc],
            width: 3,
            dash: DashStyle::Dashed,
        };
        s.drawings.items[2].style = Some(st);
        s.drawings.items[2].opacity = 40;
        s.select_only(2);
        assert!(s
            .save_template("  biru putus  ")
            .unwrap()
            .contains("disimpan"));
        assert_eq!(s.templates.len(), 1);
        assert_eq!(s.templates[0].name, "biru putus");
        assert_eq!(s.templates[0].style, Some(st));
        assert_eq!(s.templates[0].opacity, 40);
        // Terapkan ke seluruh multi-seleksi (3 objek).
        s.set_selection(&[0, 1, 2]);
        assert_eq!(s.apply_template("biru putus"), 3);
        assert_eq!(s.drawings.items[0].style, Some(st));
        assert_eq!(s.drawings.items[1].opacity, 40);
        assert_eq!(s.apply_template("tidak ada"), 0);
        // Nama sama → perbarui di tempat, bukan duplikat.
        s.select_only(0);
        s.drawings.items[0].style = None;
        s.drawings.items[0].opacity = 100;
        assert!(s
            .save_template("biru putus")
            .unwrap()
            .contains("diperbarui"));
        assert_eq!(s.templates.len(), 1);
        assert_eq!(s.templates[0].style, None);
        assert_eq!(s.templates[0].opacity, 100);
        // Nama kosong & tanpa seleksi ditolak.
        assert!(s.save_template("   ").is_err());
        s.clear_selection();
        assert!(s.save_template("x").is_err());
        // Hapus: pilihan panel ikut dibersihkan.
        s.template_sel = Some("biru putus".into());
        assert!(s.remove_template("biru putus"));
        assert!(s.templates.is_empty());
        assert!(s.template_sel.is_none());
        assert!(!s.remove_template("biru putus"));
        // Nama panjang dipotong ke NAME_MAX.
        s.add_drawing(vline(9_000));
        assert!(s.save_template(&"n".repeat(80)).is_ok());
        assert_eq!(s.templates[0].name.chars().count(), StyleTemplate::NAME_MAX);
        // Katalog penuh: simpan ke-(MAX+1) ditolak.
        while s.templates.len() < StyleTemplate::MAX {
            let name = format!("t{}", s.templates.len());
            assert!(s.save_template(&name).is_ok());
        }
        assert!(s.save_template("kelebihan").is_err());
        assert_eq!(s.templates.len(), StyleTemplate::MAX);
    }

    #[test]
    fn schema6_channel_varian_extend_dan_template_roundtrip() {
        let mut a = ChartState::new();
        a.load(bars(30));
        let ch = |kind: ChannelKind| Drawing::ParallelChannel {
            a: Anchor {
                ts_millis: 1_000,
                price_ticks: 100,
            },
            b: Anchor {
                ts_millis: 9_000,
                price_ticks: 140,
            },
            c: Anchor {
                ts_millis: 3_000,
                price_ticks: 160,
            },
            kind,
        };
        a.add_drawing(ch(ChannelKind::Disjoint));
        a.drawings.items[0].extend = ExtendMode::Left;
        a.add_drawing(ch(ChannelKind::Flat));
        a.drawings.items[1].extend = ExtendMode::Both;
        a.add_drawing(ch(ChannelKind::Parallel));
        a.drawings.items[2].style = Some(ItemStyle {
            color: [0x11, 0x22, 0x33],
            width: 3,
            dash: DashStyle::Solid,
        });
        a.select_only(2);
        assert!(a.save_template("standar").is_ok());
        let j = a.layout_json();
        assert!(j.contains("\"schema\":7"));
        // Varian hanya ditulis bila bukan paralel (default).
        assert_eq!(j.matches("\"variant\"").count(), 2);
        assert!(j.contains("\"variant\":\"disjoint\""));
        assert!(j.contains("\"variant\":\"flat\""));
        // Perpanjangan hanya ditulis bila bukan "none".
        assert_eq!(j.matches("\"extend\"").count(), 2);
        assert!(j.contains("\"extend\":\"left\""));
        assert!(j.contains("\"extend\":\"both\""));
        assert!(j.contains("\"templates\":[{\"name\":\"standar\""));
        let mut b = ChartState::new();
        b.restore_layout_json(&j).expect("layout v6 valid");
        assert_eq!(b.drawings.items.len(), 3);
        match &b.drawings.items[0].drawing {
            Drawing::ParallelChannel { kind, .. } => assert_eq!(*kind, ChannelKind::Disjoint),
            other => panic!("bukan channel: {other:?}"),
        }
        match &b.drawings.items[1].drawing {
            Drawing::ParallelChannel { kind, .. } => assert_eq!(*kind, ChannelKind::Flat),
            other => panic!("bukan channel: {other:?}"),
        }
        assert_eq!(b.drawings.items[0].extend, ExtendMode::Left);
        assert_eq!(b.drawings.items[1].extend, ExtendMode::Both);
        assert_eq!(b.drawings.items[2].extend, ExtendMode::None);
        assert_eq!(b.templates, a.templates);
        assert!(b.template_sel.is_none());
        // Round-trip byte-identik.
        assert_eq!(b.layout_json(), j);
        // Nilai varian/extend asing diabaikan (bukan error).
        let mut c = ChartState::new();
        c.restore_layout_json(
            r#"{"schema":6,"drawings":[{"kind":"channel","a":{"t":1,"p":2},"b":{"t":3,"p":4},"c":{"t":5,"p":6},"variant":"regression","extend":"atas"}]}"#,
        )
        .unwrap();
        match &c.drawings.items[0].drawing {
            Drawing::ParallelChannel { kind, .. } => assert_eq!(*kind, ChannelKind::Parallel),
            other => panic!("bukan channel: {other:?}"),
        }
        assert_eq!(c.drawings.items[0].extend, ExtendMode::None);
        // Template rusak/nama ganda/kosong dilewati, katalog dibatasi MAX.
        let many: Vec<String> = (0..20).map(|i| format!("{{\"name\":\"t{i}\"}}")).collect();
        let js = format!(
            "{{\"schema\":6,\"templates\":[{},{{\"name\":\"\"}},{{\"name\":\"t0\"}},{{}}]}}",
            many.join(",")
        );
        let mut d = ChartState::new();
        d.restore_layout_json(&js).unwrap();
        assert_eq!(d.templates.len(), StyleTemplate::MAX);
        assert_eq!(d.templates[0].name, "t0");
        assert_eq!(d.templates[0].style, None);
        assert_eq!(d.templates[0].opacity, 100);
    }

    #[test]
    fn layout_v5_naik_ke_v6_hanya_ganti_schema() {
        let mut a = ChartState::new();
        a.load(bars(30));
        a.add_drawing(vline(1_000));
        a.add_drawing(Drawing::ParallelChannel {
            a: Anchor {
                ts_millis: 1_000,
                price_ticks: 100,
            },
            b: Anchor {
                ts_millis: 9_000,
                price_ticks: 140,
            },
            c: Anchor {
                ts_millis: 3_000,
                price_ticks: 160,
            },
            kind: ChannelKind::Parallel,
        });
        let j6 = a.layout_json();
        let j5 = j6.replace("\"schema\":7", "\"schema\":5");
        let mut b = ChartState::new();
        b.restore_layout_json(&j5).expect("berkas v5 tetap termuat");
        assert_eq!(b.layout_json(), j6);
    }
    #[test]
    fn batch8_alat_baru_roundtrip_byte_identik() {
        let mut a = ChartState::new();
        a.load(bars(40));
        let an = |t: i64, p: i64| Anchor {
            ts_millis: t,
            price_ticks: p,
        };
        a.add_drawing(Drawing::RegressionChannel {
            a: an(1_000, 100),
            b: an(30_000, 130),
        });
        a.drawings.items[0].extend = ExtendMode::Right;
        a.add_drawing(Drawing::FibTrendExtension {
            a: an(2_000, 120),
            b: an(12_000, 160),
            c: an(18_000, 140),
        });
        a.drawings.items[1].extend = ExtendMode::Left;
        a.add_drawing(Drawing::ElliottImpulse {
            points: (0..5).map(|i| an(1_000 + i * 2_000, 100 + i * 5)).collect(),
        });
        a.add_drawing(Drawing::ElliottCorrection {
            points: vec![an(1_000, 110), an(5_000, 90), an(9_000, 105)],
        });
        a.add_drawing(Drawing::Xabcd {
            points: (0..5).map(|i| an(2_000 + i * 3_000, 90 + i * 4)).collect(),
        });
        a.add_drawing(Drawing::Abcd {
            points: (0..4).map(|i| an(2_000 + i * 3_000, 90 + i * 4)).collect(),
        });
        a.add_drawing(Drawing::GannFan {
            a: an(3_000, 95),
            b: an(15_000, 150),
        });
        a.add_drawing(Drawing::Brush {
            points: vec![an(1_000, 100), an(4_000, 104), an(7_000, 99)],
        });
        a.add_drawing(Drawing::Highlighter {
            points: vec![an(1_000, 96), an(6_000, 101)],
        });
        a.add_drawing(Drawing::Sticker {
            at: an(9_000, 120),
            icon: StickerIcon::Star,
        });
        a.add_drawing(Drawing::Text {
            at: an(1_500, 130),
            text: "catatan".into(),
        });
        let n = a.drawings.items.len();
        a.drawings.items[n - 1].text_size = TextSize::L;
        a.drawings.items[n - 1].text_align = TextAlign::Center;
        let j = a.layout_json();
        assert!(j.contains("\"schema\":7"));
        for k in [
            "regression",
            "fibtrend",
            "elliott_impulse",
            "elliott_correction",
            "xabcd",
            "abcd",
            "gannfan",
            "brush",
            "highlighter",
            "sticker",
        ] {
            assert_eq!(
                j.matches(&format!("\"kind\":\"{k}\"")).count(),
                1,
                "kind {k}"
            );
        }
        assert!(j.contains("\"icon\":\"star\""));
        assert!(j.contains("\"font\":\"l\""));
        assert!(j.contains("\"align\":\"center\""));
        let mut b = ChartState::new();
        b.restore_layout_json(&j).expect("layout v7 valid");
        assert_eq!(b.drawings.items.len(), n);
        for i in 0..n {
            assert_eq!(b.drawings.items[i].drawing, a.drawings.items[i].drawing);
            assert_eq!(b.drawings.items[i].extend, a.drawings.items[i].extend);
        }
        assert_eq!(b.drawings.items[n - 1].text_size, TextSize::L);
        assert_eq!(b.drawings.items[n - 1].text_align, TextAlign::Center);
        // Round-trip byte-identik lalu simpan ulang (idempoten).
        assert_eq!(b.layout_json(), j);
        let mut c = ChartState::new();
        c.restore_layout_json(&b.layout_json()).unwrap();
        assert_eq!(c.layout_json(), j);
    }

    #[test]
    fn batch8_reader_menolak_data_rusak_dan_skema_lama() {
        let j = r#"{"schema":7,"drawings":[
            {"kind":"sticker","a":{"t":1,"p":2},"icon":"tidak-ada"},
            {"kind":"brush","pts":[{"t":1,"p":2}]},
            {"kind":"xabcd","pts":[{"t":1,"p":2},{"t":2,"p":3},{"t":3,"p":4}]},
            {"kind":"abcd","pts":[{"t":1,"p":2},{"t":2,"p":3},{"t":3,"p":4},{"t":4,"p":5},{"t":5,"p":6}]},
            {"kind":"elliott_impulse","pts":[{"t":1,"p":2},{"t":2,"p":3},{"t":3,"p":4},{"t":4,"p":5},{"t":5,"p":6},{"x":1}]},
            {"kind":"fibtrend","a":{"t":1,"p":2},"b":{"t":3,"p":4}}
        ]}"#;
        let mut s = ChartState::new();
        s.restore_layout_json(j).unwrap();
        assert_eq!(s.drawings.items.len(), 3, "entri rusak dilewati");
        match &s.drawings.items[0].drawing {
            Drawing::Sticker { icon, .. } => assert_eq!(*icon, StickerIcon::ArrowUp),
            other => panic!("bukan sticker: {other:?}"),
        }
        match &s.drawings.items[1].drawing {
            Drawing::Abcd { points } => assert_eq!(points.len(), 4, "dipotong ke 4"),
            other => panic!("bukan abcd: {other:?}"),
        }
        match &s.drawings.items[2].drawing {
            Drawing::ElliottImpulse { points } => assert_eq!(points.len(), 5),
            other => panic!("bukan elliott: {other:?}"),
        }
        // Titik garis bebas dipotong ke MAX_STROKE_POINTS.
        let pts: Vec<String> = (0..4100)
            .map(|i| format!("{{\"t\":{i},\"p\":{i}}}"))
            .collect();
        let j2 = format!(
            "{{\"schema\":7,\"drawings\":[{{\"kind\":\"highlighter\",\"pts\":[{}]}}]}}",
            pts.join(",")
        );
        let mut s2 = ChartState::new();
        s2.restore_layout_json(&j2).unwrap();
        match &s2.drawings.items[0].drawing {
            Drawing::Highlighter { points } => assert_eq!(points.len(), MAX_STROKE_POINTS),
            other => panic!("bukan highlighter: {other:?}"),
        }
        // Skema 6 (pra-batch 8) tetap termuat; meta teks kembali ke default.
        let mut s3 = ChartState::new();
        s3.restore_layout_json(
            r#"{"schema":6,"drawings":[{"kind":"text","a":{"t":1,"p":2},"text":"lama"}]}"#,
        )
        .unwrap();
        assert_eq!(s3.drawings.items[0].text_size, TextSize::M);
        assert_eq!(s3.drawings.items[0].text_align, TextAlign::Left);
        // Skema masa depan ditolak.
        assert!(ChartState::new()
            .restore_layout_json(r#"{"schema":8,"drawings":[]}"#)
            .is_err());
    }
    #[test]
    fn hover_pick_bercache_dan_invalidasi() {
        let mut s = ChartState::new();
        s.load(bars(30));
        let a = Anchor {
            ts_millis: s.bars[10].ts_millis,
            price_ticks: 100,
        };
        let b = Anchor {
            ts_millis: s.bars[20].ts_millis,
            price_ticks: 120,
        };
        s.add_drawing(Drawing::Trendline { a, b });
        let mx = s.viewport.x_for_time((a.ts_millis + b.ts_millis) / 2);
        let my = s.viewport.y_for_price(110);
        assert_eq!(s.hover_pick(Some((mx, my)), 6.0), Some(0));
        // Pointer tidak bergerak → hasil sama (dari cache).
        assert_eq!(s.hover_pick(Some((mx, my)), 6.0), Some(0));
        // Di luar kerangka garis → None, cache diperbarui.
        assert_eq!(s.hover_pick(Some((mx + 6.0, my + 120.0)), 6.0), None);
        assert_eq!(s.hover_pick(Some((mx, my)), 6.0), Some(0));
        // Objek dihapus → invalidasi (jumlah objek berubah).
        s.drawings.remove_many(&[0]);
        assert_eq!(s.hover_pick(Some((mx, my)), 6.0), None, "objek dihapus");
        // Undo mengembalikan objek → kedalaman undo berubah → dihitung ulang.
        assert_eq!(s.apply(Command::Undo), Event::DrawingsChanged);
        assert_eq!(s.hover_pick(Some((mx, my)), 6.0), Some(0), "objek kembali");
        // Pointer keluar plot → cache dibersihkan.
        assert_eq!(s.hover_pick(None, 6.0), None);
        // `load` menaikkan revisi dataset (kunci cache).
        let rev = s.bars_rev;
        s.load(bars(30));
        assert_eq!(s.bars_rev, rev + 1);
    }
}
