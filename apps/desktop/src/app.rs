// SokanLab app shell (eframe) — Apache-2.0. Stateful chart + panel backtest.
// Interaksi: toolbar alat (gaya TradingView), hotkey, magnet, drag-edit anchor.
pub(crate) mod backtest_panel;
pub(crate) mod headless;
pub(crate) mod state;
pub use state::AppState;

use egui::{Panel, Ui};
use sokanlab_chart::drawing::{
    constrain_angle, pick, Anchor, ChannelKind, DashStyle, Drawing, ExtendMode, FibLevel,
    ItemStyle, Magnet, PitchforkKind, Projector, StickerIcon, StyleTemplate, TextAlign, TextSize,
    FIB_LEVELS_MILLI, MAX_STROKE_POINTS,
};
use sokanlab_chart::{ScaleMode, Viewport};
use sokanlab_ui::painter::{
    draw_anchor_handles, draw_axes, draw_candles, draw_crosshair, draw_drawings, draw_grid,
    draw_indicator_panes, draw_last_price, draw_overlays, draw_preview, draw_rubber_band,
    draw_volume, fit_spacing, plot_width, split_plot_panes, ITEM_PALETTE,
};
use sokanlab_ui::tools::{Action, ToolKind};
use sokanlab_ui::{theme, ChartState, Command, Gesture, PaneKind, PaneSetting};
use std::sync::{Arc, Mutex};

pub struct SokanApp {
    pub state: Arc<Mutex<AppState>>,
    /// Diagnostik Fase 5: instrumen ringan frame-time + waktu frame pertama.
    started: std::time::Instant,
    frames: u64,
    window_ms: f64,
    window_frames: u32,
    /// Mode bench eksplisit (--bench-frames N): repaint terus, cetak statistik, keluar.
    pub bench: Option<FrameBench>,
    /// Dialog pintasan keyboard (F1).
    pub show_shortcuts: bool,
    /// Dialog "pergi ke tanggal" (Alt+G): teks input + pesan error terakhir.
    pub show_goto: bool,
    pub goto_text: String,
    pub goto_error: Option<String>,
}

/// Statistik frame untuk `--bench-frames`: kumpulkan durasi, lalu ringkas.
pub struct FrameBench {
    left: u64,
    durs: Vec<f64>,
}

impl FrameBench {
    pub fn new(n: u64) -> Self {
        Self {
            left: n,
            durs: Vec::with_capacity(n as usize),
        }
    }

    fn record(&mut self, dt_ms: f64) -> bool {
        self.durs.push(dt_ms);
        self.left = self.left.saturating_sub(1);
        if self.left == 0 {
            let mut v = self.durs.clone();
            v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let avg = v.iter().sum::<f64>() / v.len() as f64;
            let p50 = v[v.len() / 2];
            let p95 = v[(v.len() * 95 / 100).min(v.len() - 1)];
            println!(
                "[sokanlab] bench-frames n={} avg={:.1} p50={:.1} p95={:.1} ms",
                v.len(),
                avg,
                p50,
                p95
            );
            return true;
        }
        false
    }
}

impl SokanApp {
    /// Dipakai main: state (termasuk dataset pra-muat) diserahkan ke app.
    pub fn new_with(cc: &eframe::CreationContext<'_>, state: AppState) -> Self {
        theme::apply(&cc.egui_ctx, theme::Theme::Dark);
        Self {
            state: Arc::new(Mutex::new(state)),
            started: std::time::Instant::now(),
            frames: 0,
            window_ms: 0.0,
            window_frames: 0,
            bench: None,
            show_shortcuts: false,
            show_goto: false,
            goto_text: String::new(),
            goto_error: None,
        }
    }

    /// Catat durasi frame; cetak frame-pertama dan rata-rata tiap 120 frame.
    fn note_frame(&mut self, dt_ms: f64) {
        self.frames += 1;
        if self.frames == 1 {
            let ms = self.started.elapsed().as_millis();
            log::info!("frame-pertama {ms} ms");
            println!("[sokanlab] frame-pertama {ms} ms");
        }
        self.window_ms += dt_ms;
        self.window_frames += 1;
        if self.window_frames == 120 {
            let avg = self.window_ms / 120.0;
            log::info!("frame #{}: rata-rata {:.1} ms/frame", self.frames, avg);
            println!(
                "[sokanlab] frame #{} rata-rata {:.1} ms/frame",
                self.frames, avg
            );
            self.window_ms = 0.0;
            self.window_frames = 0;
        }
        if let Some(b) = self.bench.as_mut() {
            if b.record(dt_ms) {
                std::process::exit(0);
            }
        }
    }

    /// Proses event keyboard → aksi. Tidak aktif saat fokus di kolom teks.
    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let mut actions: Vec<Action> = Vec::new();
        ctx.input(|i| {
            for e in &i.events {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } = e
                {
                    if let Some(a) = sokanlab_ui::tools::action_for(*key, *modifiers) {
                        actions.push(a);
                    }
                }
            }
        });
        if actions.is_empty() {
            return;
        }
        let mut open_help = false;
        let mut open_goto = false;
        {
            let mut s = self.state.lock().unwrap();
            for a in actions {
                match a {
                    Action::OpenShortcuts => open_help = true,
                    Action::OpenGoToDate => open_goto = true,
                    other => apply_action(&mut s, other),
                }
            }
        }
        if open_help {
            self.show_shortcuts = true;
        }
        if open_goto {
            self.show_goto = true;
            self.goto_error = None;
        }
    }

    /// Dialog "Pergi ke tanggal" (Alt+G): terima tanggal UTC atau epoch.
    fn goto_window(&mut self, ctx: &egui::Context) {
        if !self.show_goto {
            return;
        }
        let mut open = self.show_goto;
        let mut submit = false;
        let mut close = false;
        egui::Window::new("Pergi ke tanggal (Alt+G)")
            .open(&mut open)
            .collapsible(false)
            .default_width(340.0)
            .show(ctx, |ui| {
                ui.label(
                    "Format: 2023-11-14, 2023-11-14 13:30, atau epoch (detik/milidetik). UTC.",
                );
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.goto_text).hint_text("2023-11-14 13:30"),
                );
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    submit = true;
                }
                if let Some(err) = &self.goto_error {
                    ui.colored_label(theme::bear_color(), err);
                }
                ui.horizontal(|ui| {
                    if ui.button("Pergi").clicked() {
                        submit = true;
                    }
                    if ui.button("Batal").clicked() {
                        close = true;
                    }
                });
            });
        if submit || close {
            if submit {
                match sokanlab_ui::parse_ts_input(&self.goto_text) {
                    Ok(ts) => {
                        let mut s = self.state.lock().unwrap();
                        s.chart.center_on_time(ts);
                        s.notice = Some(format!("tampilan dipusatkan ke {ts} ms"));
                        self.goto_error = None;
                        close = true;
                    }
                    Err(e) => self.goto_error = Some(e),
                }
            }
            if close {
                open = false;
                self.goto_error = None;
            }
        }
        self.show_goto = open;
    }

    fn shortcuts_window(&mut self, ctx: &egui::Context) {
        if !self.show_shortcuts {
            return;
        }
        let mut open = self.show_shortcuts;
        egui::Window::new("Pintasan keyboard")
            .open(&mut open)
            .collapsible(false)
            .default_width(430.0)
            .show(ctx, |ui| {
                egui::Grid::new("shortcut-grid")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        for (k, v) in SHORTCUTS {
                            ui.label(*k);
                            ui.label(*v);
                            ui.end_row();
                        }
                    });
                ui.separator();
                ui.small(
                    "Pintasan yang sama dengan TradingView dipakai bila fiturnya ada di \
                     SokanLab (Alt+T/H/V/C/F, Ctrl+Z/Y, Alt+R, Ctrl+Alt+H, panah, \
                     Ctrl+panah, Ctrl+S). Angka 1–9 = alat (khusus SokanLab).",
                );
            });
        self.show_shortcuts = open;
    }
}

/// Tabel pintasan (dipakai dialog + dokumentasi).
pub const SHORTCUTS: &[(&str, &str)] = &[
    ("Esc", "Batalkan penempatan / lepas pilihan"),
    ("Del / Backspace", "Hapus semua objek terpilih"),
    ("Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z", "Undo / redo"),
    ("Ctrl+S", "Simpan layout ke disk"),
    (
        "Ctrl+klik",
        "Tambah/lepas objek dari seleksi (multi-seleksi)",
    ),
    (
        "Ctrl+seret (area kosong)",
        "Kotak seleksi: pilih semua objek yang tersentuh",
    ),
    ("Alt+R", "Reset tampilan (seluruh data)"),
    ("Alt+G", "Pergi ke tanggal/waktu (dialog)"),
    (
        "Dobel klik sumbu",
        "Sumbu harga: fit ulang harga · sumbu waktu: rentang penuh",
    ),
    (
        "Alt+T / Alt+H / Alt+V / Ctrl+Alt+H",
        "Trend / garis harga / garis waktu / sembunyi semua",
    ),
    ("Alt+C / Alt+F", "Crossline / Fibonacci retracement"),
    ("← / →", "Geser 1 bar (Ctrl: 10 bar)"),
    ("Ctrl+↑ / Ctrl+↓", "Zoom in / out"),
    ("Scroll", "Zoom di posisi kursor (Shift+scroll: geser)"),
    ("Drag kiri (kosong)", "Pan chart; tombol tengah juga pan"),
    ("Drag handle", "Pindahkan anchor objek terpilih"),
    ("Drag badan objek", "Geser seluruh objek"),
    (
        "Shift + seret (menggambar)",
        "Kunci horizontal / vertikal / 45°",
    ),
    (
        "1–9",
        "Pilih alat (khusus SokanLab; TV memakai angka untuk timeframe)",
    ),
    ("Alt+S", "Snapshot chart → data/snapshot-<epoch>.png"),
    (
        "Drag sumbu",
        "Sumbu harga: skala vertikal · sumbu waktu: skala horizontal",
    ),
    ("F1", "Dialog pintasan ini"),
];

/// Terapkan aksi hotkey ke state (tanpa UI).
fn apply_action(s: &mut AppState, a: Action) {
    match a {
        Action::Tool(k) => s.chart.tool.select(k),
        Action::Undo => {
            s.chart.apply(Command::Undo);
        }
        Action::Redo => {
            s.chart.apply(Command::Redo);
        }
        Action::DeleteSelected => {
            s.chart.apply(Command::RemoveSelected);
        }
        Action::Cancel => {
            if !s.chart.tool.cancel() {
                s.chart.clear_selection();
            }
        }
        Action::ResetView => {
            s.chart.apply(Command::ResetView);
        }
        Action::SaveLayout => match s.save_layout() {
            Ok(()) => s.notice = Some(format!("layout tersimpan: {}", s.layout_path)),
            Err(e) => s.notice = Some(format!("gagal simpan layout: {e}")),
        },
        Action::ToggleHideAll => {
            let hidden = s.chart.toggle_hide_all();
            s.notice = Some(if hidden {
                "drawing disembunyikan (Ctrl+Alt+H)".into()
            } else {
                "drawing ditampilkan".into()
            });
        }
        Action::ZoomIn => s.chart.zoom_center(0.9),
        Action::ZoomOut => s.chart.zoom_center(1.1),
        Action::PanBars(n) => s.chart.pan_bars(n),
        Action::OpenShortcuts | Action::OpenGoToDate => {}
        Action::Snapshot => match save_snapshot(&s.chart) {
            Ok(p) => s.notice = Some(format!("snapshot: {}", p.display())),
            Err(e) => s.notice = Some(format!("snapshot gagal: {e}")),
        },
    }
}

/// Ukuran gambar snapshot dari area chart terakhir (fallback 1200×700).
fn snapshot_size(chart: &ChartState) -> (usize, usize) {
    let r = chart.area_rect;
    let w = if r.width() >= 200.0 {
        r.width()
    } else {
        1200.0
    };
    let h = if r.height() >= 150.0 {
        r.height()
    } else {
        700.0
    };
    ((w as usize).clamp(320, 3840), (h as usize).clamp(200, 2160))
}

/// Simpan snapshot PNG chart saat ini ke `data/snapshot-<epoch>.png`.
fn save_snapshot(chart: &ChartState) -> Result<std::path::PathBuf, String> {
    let (w, h) = snapshot_size(chart);
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = std::path::PathBuf::from(format!("data/snapshot-{secs}.png"));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    crate::app::headless::render_chart_png(&path, chart, w, h).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Proyektor viewport yang sama dengan chart → hit-test & render konsisten.
struct Vp(Viewport);

impl Projector for Vp {
    fn x(&self, ts_millis: i64) -> f64 {
        self.0.x_for_time(ts_millis)
    }
    fn y(&self, price_ticks: i64) -> f64 {
        self.0.y_for_price(price_ticks)
    }
}

/// Anchor lain (untuk constrain sudut saat menggeser satu anchor).
/// Anchor mentah (tanpa magnet) dari posisi lokal — dipakai alat garis bebas
/// supaya goresan mengikuti kursor apa adanya, bukan menempel ke OHLC.
fn raw_anchor(v: &Viewport, local: (f64, f64)) -> Anchor {
    Anchor {
        ts_millis: v.time_at_x(local.0),
        price_ticks: v.price_at_y(local.1),
    }
}

fn other_anchor(d: &Drawing, current: usize) -> Option<Anchor> {
    let anchors = d.anchors();
    if anchors.len() < 2 {
        return None;
    }
    Some(anchors[(current + 1) % anchors.len()])
}

fn chart_panel(ui: &mut Ui, chart: &mut ChartState) {
    let area = ui.available_rect_before_wrap();
    let visible =
        sokanlab_chart::visible_indices(&chart.bars, chart.viewport.t_min, chart.viewport.t_max);
    // Lebar plot = area minus strip sumbu kanan; spacing & peta X memakai ini.
    let spacing = fit_spacing(
        plot_width(area) as f64,
        (visible.end - visible.start).max(1),
    );
    let layout = split_plot_panes(area, spacing, chart.panes.len());
    chart.plot_rect = layout.price;
    chart.area_rect = area;
    // Peta Y harus mengikuti tinggi pane harga (bukan tinggi jendela), agar candle
    // tidak pernah jatuh ke pane volume/pane indikator dan anchor zoom konsisten.
    chart.viewport = chart
        .viewport
        .resize(layout.price.width() as f64, layout.price.height() as f64);
    let painter = ui.painter_at(area);

    let response = ui.interact(
        area,
        ui.scope_id().with("plot"),
        egui::Sense::click_and_drag(),
    );

    let ox = layout.price.min.x as f64;
    let oy = layout.price.min.y as f64;
    let to_local = |p: egui::Pos2| ((p.x as f64 - ox), (p.y as f64 - oy));

    // --- input snapshot ---
    let (
        primary_pressed,
        primary_released,
        primary_down,
        pointer,
        middle_down,
        shift,
        ctrl,
        scroll,
    ) = ui.input(|i| {
        (
            i.pointer.primary_pressed(),
            i.pointer.primary_released(),
            i.pointer.primary_down(),
            i.pointer.interact_pos(),
            i.pointer.middle_down(),
            i.modifiers.shift,
            // Ctrl (Windows/Linux) atau Cmd (macOS): padanan multi-seleksi TV.
            i.modifiers.ctrl || i.modifiers.command,
            i.smooth_scroll_delta.y,
        )
    });
    let hovering = response.contains_pointer();
    let draw_mode = chart.tool.kind.is_draw_mode();

    // --- hover + magnet untuk pratinjau ---
    if draw_mode {
        match response.hover_pos() {
            Some(p) => {
                if chart.tool.kind.is_stroke() {
                    // Garis bebas: pratinjau mengikuti kursor mentah.
                    chart.tool.hover = Some(raw_anchor(&chart.viewport, to_local(p)));
                    chart.tool.hover_snapped = false;
                } else {
                    let (a, snapped) = chart.anchor_at(to_local(p), 6.0);
                    chart.tool.hover = Some(a);
                    chart.tool.hover_snapped = snapped;
                }
            }
            None => chart.tool.hover = None,
        }
    } else {
        chart.tool.hover = None;
        chart.tool.hover_snapped = false;
    }

    // --- scroll: zoom (default) / geser horizontal (Shift) ---
    if hovering && scroll.abs() > 0.01 {
        if shift {
            chart.viewport = chart.viewport.pan_px(scroll as f64, 0.0);
        } else if let Some(p) = response.hover_pos() {
            chart.zoom_at(if scroll > 0.0 { 0.9 } else { 1.1 }, p, area);
        }
    }

    // --- tombol tengah: pan kapan pun ---
    if middle_down {
        if let Some(p) = pointer {
            if let Some(prev) = chart.drag {
                chart.pan(egui::vec2(p.x - prev.x, p.y - prev.y), area);
            }
            chart.drag = Some(p);
        }
    } else {
        chart.drag = None;
        chart.panning = false;
    }

    // --- gestur tombol kiri ---
    match chart.gesture.clone() {
        Gesture::Idle => {
            if primary_pressed {
                if let Some(p) = pointer.filter(|_| hovering) {
                    let lp = to_local(p);
                    let near_price_axis = layout.axis_y.contains(p);
                    let near_time_axis = layout.axis_x.contains(p);
                    if !draw_mode && near_price_axis {
                        chart.gesture = Gesture::ScalePrice { last: p };
                    } else if !draw_mode && near_time_axis {
                        chart.gesture = Gesture::ScaleTime { last: p };
                    } else if draw_mode {
                        let (a, _) = if chart.tool.kind.is_stroke() {
                            (raw_anchor(&chart.viewport, lp), false)
                        } else {
                            chart.anchor_at(lp, 6.0)
                        };
                        if chart.tool.kind.is_stroke() {
                            // Mulai goresan: titik pertama = posisi tekan.
                            chart.tool.pending.clear();
                            chart.tool.pending.push(a);
                        }
                        chart.gesture = Gesture::PlaceDrag {
                            start_local: lp,
                            start_anchor: a,
                            moved: false,
                        };
                    } else if let Some((idx, anchor)) = chart.handle_hit(&layout, lp, 8.0) {
                        let orig = chart.drawings.items[idx].drawing.clone();
                        chart.drawings.begin_edit();
                        chart.gesture = Gesture::MoveAnchor { idx, anchor, orig };
                    } else if let Some(idx) = pick(
                        &chart.drawings.items,
                        &chart.bars,
                        &Vp(chart.viewport),
                        lp,
                        6.0,
                    ) {
                        if ctrl {
                            // Ctrl+klik: keputusan "tambah/lepas seleksi" vs
                            // "klon lalu geser" (TV) ditunda sampai tahu apakah
                            // pointer benar-benar diseret.
                            chart.gesture = Gesture::Press {
                                start: p,
                                on_drawing: Some(idx),
                                toggle: true,
                            };
                        } else {
                            // Objek yang sudah bagian dari multi-seleksi tidak
                            // dikolapskan saat ditekan (drag = geser semua).
                            if !chart.is_selected(idx) {
                                chart.select_only(idx);
                            }
                            chart.gesture = Gesture::Press {
                                start: p,
                                on_drawing: Some(idx),
                                toggle: false,
                            };
                        }
                    } else if ctrl {
                        // Ctrl+seret di area kosong: kotak seleksi (TV).
                        chart.gesture = Gesture::Rubber {
                            origin: p,
                            current: p,
                        };
                    } else {
                        chart.gesture = Gesture::Press {
                            start: p,
                            on_drawing: None,
                            toggle: false,
                        };
                    }
                }
            }
        }
        Gesture::Press {
            start,
            on_drawing,
            toggle,
        } => {
            if primary_down {
                if let Some(p) = pointer {
                    if p.distance(start) > 4.0 {
                        match on_drawing {
                            Some(idx) => {
                                // Multi-seleksi: geser semua objek terpilih.
                                // Selain itu: anggota grup (terlihat & tidak
                                // terkunci) ikut bergeser. Ctrl+seret = klon
                                // dulu, lalu geser klonnya (padanan TV).
                                let ids = if ctrl {
                                    chart.duplicate_for_drag(idx)
                                } else {
                                    let sel = chart.selection();
                                    if sel.len() > 1 && sel.contains(&idx) {
                                        sel
                                    } else {
                                        chart.drawings.group_members(idx)
                                    }
                                };
                                let targets: Vec<(usize, Drawing)> = ids
                                    .into_iter()
                                    .filter(|i| {
                                        chart
                                            .drawings
                                            .items
                                            .get(*i)
                                            .map(|it| it.visible && !it.locked)
                                            .unwrap_or(false)
                                    })
                                    .map(|i| (i, chart.drawings.items[i].drawing.clone()))
                                    .collect();
                                chart.drawings.begin_edit();
                                chart.gesture = Gesture::MoveBody {
                                    targets,
                                    start_local: to_local(start),
                                };
                            }
                            None => chart.gesture = Gesture::Pan { last: p },
                        }
                    }
                }
            } else if primary_released {
                if let Some(p) = pointer {
                    if toggle {
                        // Ctrl+klik tanpa seret: tambah/lepas dari seleksi.
                        if let Some(idx) = on_drawing {
                            chart.toggle_in_selection(idx);
                        }
                    } else {
                        match pick(
                            &chart.drawings.items,
                            &chart.bars,
                            &Vp(chart.viewport),
                            to_local(p),
                            6.0,
                        ) {
                            Some(idx) => chart.select_only(idx),
                            None => chart.clear_selection(),
                        }
                    }
                }
                chart.gesture = Gesture::Idle;
            }
        }
        Gesture::Rubber { origin, current } => {
            if primary_down {
                if let Some(p) = pointer {
                    chart.gesture = Gesture::Rubber { origin, current: p };
                }
            } else if primary_released {
                let o = to_local(origin);
                let c = to_local(current);
                let hits = if chart.drawings_hidden {
                    Vec::new()
                } else {
                    sokanlab_chart::drawing::rubber_select(
                        &chart.drawings.items,
                        &Vp(chart.viewport),
                        (o.0, o.1, c.0, c.1),
                    )
                };
                chart.set_selection(&hits);
                chart.gesture = Gesture::Idle;
            }
        }
        Gesture::Pan { last } => {
            if primary_down {
                if let Some(p) = pointer {
                    chart.pan(egui::vec2(p.x - last.x, p.y - last.y), area);
                    chart.gesture = Gesture::Pan { last: p };
                }
            } else if primary_released {
                chart.gesture = Gesture::Idle;
            }
        }
        Gesture::MoveAnchor { idx, anchor, orig } => {
            if primary_down {
                if let Some(p) = pointer {
                    let lp = to_local(p);
                    let mut a = chart.anchor_at(lp, 6.0).0;
                    if shift {
                        if let Some(other) = other_anchor(&orig, anchor) {
                            a = constrain_angle(other, lp, &chart.viewport);
                        }
                    }
                    let mut d = orig.clone();
                    d.set_anchor(anchor, a);
                    chart.drawings.set(idx, d);
                }
            } else if primary_released {
                chart.gesture = Gesture::Idle;
            }
        }
        Gesture::MoveBody {
            targets,
            start_local,
        } => {
            if primary_down {
                if let Some(p) = pointer {
                    let lp = to_local(p);
                    let dts =
                        chart.viewport.time_at_x(lp.0) - chart.viewport.time_at_x(start_local.0);
                    let dp =
                        chart.viewport.price_at_y(lp.1) - chart.viewport.price_at_y(start_local.1);
                    for (i, orig) in &targets {
                        chart.drawings.set(*i, orig.translated(dts, dp));
                    }
                }
            } else if primary_released {
                chart.gesture = Gesture::Idle;
            }
        }
        Gesture::ScalePrice { last } => {
            if primary_down {
                if let Some(p) = pointer {
                    chart.scale_price_drag((p.y - last.y) as f64);
                    chart.gesture = Gesture::ScalePrice { last: p };
                }
            } else if primary_released {
                chart.gesture = Gesture::Idle;
            }
        }
        Gesture::ScaleTime { last } => {
            if primary_down {
                if let Some(p) = pointer {
                    chart.scale_time_drag((p.x - last.x) as f64);
                    chart.gesture = Gesture::ScaleTime { last: p };
                }
            } else if primary_released {
                chart.gesture = Gesture::Idle;
            }
        }
        Gesture::PlaceDrag {
            start_local,
            start_anchor,
            moved,
        } => {
            let mut moved = moved;
            if primary_down {
                if let Some(p) = pointer {
                    let lp = to_local(p);
                    if (lp.0 - start_local.0).hypot(lp.1 - start_local.1) > 4.0 {
                        moved = true;
                    }
                    if chart.tool.kind.is_stroke() {
                        // Sampel titik saat seret: jarak ≥ 3 px dari sampel
                        // terakhir (batas MAX_STROKE_POINTS sebagai pengaman).
                        let raw = raw_anchor(&chart.viewport, lp);
                        let far = chart.tool.pending.last().is_none_or(|l| {
                            let dx = chart.viewport.x_for_time(l.ts_millis)
                                - chart.viewport.x_for_time(raw.ts_millis);
                            let dy = chart.viewport.y_for_price(l.price_ticks)
                                - chart.viewport.y_for_price(raw.price_ticks);
                            dx.hypot(dy) >= 3.0
                        });
                        if far && chart.tool.pending.len() < MAX_STROKE_POINTS {
                            chart.tool.pending.push(raw);
                        }
                    }
                    chart.gesture = Gesture::PlaceDrag {
                        start_local,
                        start_anchor,
                        moved,
                    };
                }
            } else if primary_released {
                let release_local = pointer.map(to_local).unwrap_or(start_local);
                if chart.tool.kind.is_stroke() {
                    // Garis bebas: selesai = titik tersampel (minimal 2).
                    let pts = std::mem::take(&mut chart.tool.pending);
                    if let Some(d) = chart.tool.kind.build_stroke(&pts) {
                        chart.add_drawing(d);
                    }
                    if !chart.tool.stay {
                        chart.tool.select(ToolKind::Select);
                    }
                } else {
                    let mut b = chart.anchor_at(release_local, 6.0).0;
                    if shift {
                        b = constrain_angle(start_anchor, release_local, &chart.viewport);
                    }
                    let done = if moved
                        && chart.tool.pending.is_empty()
                        && chart.tool.kind.needed() == 2
                    {
                        // seret sekali (press-drag-release) → dua anchor sekaligus
                        let _ = chart.tool.click(start_anchor);
                        chart.tool.click(b)
                    } else {
                        chart.tool.click(b)
                    };
                    if let Some(d) = done {
                        chart.add_drawing(d);
                    }
                }
                chart.gesture = Gesture::Idle;
            }
        }
    }

    // Hit-test hover ber-cache (objek regresi memindai rentang bar: mahal bila
    // diulang tiap frame walau pointer diam). Dipakai bersama oleh kursor dan
    // render supaya `pick` dijalankan sekali per perubahan, bukan dua kali.
    let hover_local =
        if !draw_mode && matches!(&chart.gesture, Gesture::Idle) && !chart.drawings_hidden {
            response.hover_pos().map(to_local)
        } else {
            None
        };
    let hover_idx = chart.hover_pick(hover_local, 6.0);

    // --- kursor ---
    let over_drawing = if hovering && !chart.drawings_hidden {
        if hover_local.is_some() {
            hover_idx.is_some()
        } else {
            // Di luar jangkauan cache (mode gambar/gesture aktif): perilaku lama.
            pick(
                &chart.drawings.items,
                &chart.bars,
                &Vp(chart.viewport),
                response.hover_pos().map(to_local).unwrap_or((-1e9, -1e9)),
                6.0,
            )
            .is_some()
        }
    } else {
        false
    };
    let cursor = if middle_down || matches!(&chart.gesture, Gesture::Pan { .. }) {
        egui::CursorIcon::Grabbing
    } else if draw_mode && hovering {
        egui::CursorIcon::Crosshair
    } else if matches!(
        &chart.gesture,
        Gesture::MoveAnchor { .. } | Gesture::MoveBody { .. }
    ) {
        egui::CursorIcon::Grabbing
    } else if over_drawing {
        egui::CursorIcon::Grab
    } else {
        egui::CursorIcon::Default
    };
    ui.ctx().set_cursor_icon(cursor);

    // --- render ---
    draw_grid(&painter, &layout, 5);
    draw_axes(&painter, &layout, &chart.viewport);
    let start = chart.visible_start();
    draw_candles(&painter, &layout, &chart.viewport, &chart.bars, start);
    draw_volume(
        &painter,
        &layout,
        &chart.bars,
        chart.viewport.t_min,
        chart.viewport.t_max,
        start,
    );
    draw_overlays(
        &painter,
        &layout,
        &chart.viewport,
        &chart.bars,
        start,
        &chart.overlay_lines,
    );
    draw_indicator_panes(
        &painter,
        &layout,
        &chart.viewport,
        &chart.bars,
        start,
        &chart.panes,
    );
    draw_drawings(
        &painter,
        &layout,
        &chart.viewport,
        &chart.bars,
        &chart.drawings.items,
        &chart.selection(),
        hover_idx,
        chart.drawings_hidden,
    );
    if let Gesture::Rubber { origin, current } = &chart.gesture {
        draw_rubber_band(
            &painter,
            egui::Rect::from_two_pos(*origin, *current),
            theme::EMBER,
        );
    }
    if let Some(idx) = chart.selected {
        if !chart.drawings_hidden {
            if let Some(it) = chart.drawings.items.get(idx) {
                if it.visible && !it.locked {
                    draw_anchor_handles(&painter, &layout, &chart.viewport, &it.drawing);
                }
            }
        }
    }
    if draw_mode {
        if let Some(a) = chart.tool.hover {
            if let Some(preview) = chart.tool.preview(a) {
                draw_preview(&painter, &layout, &chart.viewport, &preview);
            }
        }
    }
    draw_last_price(&painter, &layout, &chart.viewport, chart.last_close());
    let cross = response.hover_pos();
    if draw_crosshair(&painter, &layout, &chart.viewport, cross).is_some() {
        chart.crosshair = cross;
    } else {
        chart.crosshair = None;
    }

    // --- dobel klik sumbu (padanan TV: reset skala harga / waktu) ---
    if response.double_clicked() {
        if let Some(p) = response.interact_pointer_pos() {
            let near_price_axis = layout.axis_y.contains(p);
            let near_time_axis = layout.axis_x.contains(p);
            if near_price_axis {
                chart.reset_price_scale();
            } else if near_time_axis {
                chart.reset_time_scale();
            }
        }
    }

    // --- menu klik kanan (gaya TV: aksi objek + tampilan) ---
    response.context_menu(|ui| {
        let has_selected = chart.selected.is_some();
        if ui
            .add_enabled(has_selected, egui::Button::new("Hapus drawing terpilih"))
            .clicked()
        {
            chart.apply(Command::RemoveSelected);
            ui.close();
        }
        if ui
            .add_enabled(
                !chart.drawings.items.is_empty(),
                egui::Button::new("Hapus semua drawing"),
            )
            .clicked()
        {
            chart.drawings.clear();
            chart.clear_selection();
            ui.close();
        }
        let sel = chart.selection();
        if !sel.is_empty() {
            let all_locked = sel.iter().all(|i| chart.drawings.items[*i].locked);
            if ui
                .button(if all_locked {
                    "Buka kunci objek"
                } else {
                    "Kunci objek"
                })
                .clicked()
            {
                chart.drawings.snapshot();
                for i in &sel {
                    if let Some(it) = chart.drawings.items.get_mut(*i) {
                        it.locked = !all_locked;
                    }
                }
                ui.close();
            }
            let all_vis = sel.iter().all(|i| chart.drawings.items[*i].visible);
            if ui
                .button(if all_vis {
                    "Sembunyikan objek ini"
                } else {
                    "Tampilkan objek ini"
                })
                .clicked()
            {
                chart.drawings.snapshot();
                for i in &sel {
                    if let Some(it) = chart.drawings.items.get_mut(*i) {
                        it.visible = !all_vis;
                    }
                }
                ui.close();
            }
        }
        if chart.selected.is_some() {
            ui.separator();
            if ui.button("Bawa ke depan (z)").clicked() {
                chart.move_selected_z(1);
                ui.close();
            }
            if ui.button("Kirim ke belakang (z)").clicked() {
                chart.move_selected_z(-1);
                ui.close();
            }
            if ui.button("Duplikat objek").clicked() {
                chart.duplicate_selected();
                ui.close();
            }
            let grp = if chart.selected_group().is_some() {
                "Lepas grup"
            } else {
                "Jadikan grup"
            };
            if ui.button(grp).clicked() {
                chart.toggle_group_selected();
                ui.close();
            }
        }
        ui.separator();
        let hide_label = if chart.drawings_hidden {
            "Tampilkan drawing (Ctrl+Alt+H)"
        } else {
            "Sembunyikan drawing (Ctrl+Alt+H)"
        };
        if ui.button(hide_label).clicked() {
            chart.toggle_hide_all();
            ui.close();
        }
        ui.separator();
        let next = chart.tool.magnet.next();
        if ui
            .button(format!(
                "Magnet: {} → {}",
                chart.tool.magnet.label(),
                next.label()
            ))
            .clicked()
        {
            chart.tool.magnet = next;
            ui.close();
        }
        if ui.button("Reset tampilan (Alt+R)").clicked() {
            chart.apply(Command::ResetView);
            ui.close();
        }
    });
}

fn tool_strip(ui: &mut Ui, s: &mut AppState, open_help: &mut bool) {
    ui.add_space(2.0);
    ui.spacing_mut().item_spacing.y = 2.0;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for kind in ToolKind::ALL {
                let selected = s.chart.tool.kind == *kind;
                let hint = kind.hotkey_hint();
                let tip = if hint.is_empty() {
                    kind.label().to_string()
                } else {
                    format!("{}  ({hint})", kind.label())
                };
                let btn = ui.add_sized(
                    [42.0, 22.0],
                    egui::Button::selectable(selected, kind.short()),
                );
                if btn.on_hover_text(tip).clicked() {
                    s.chart.tool.select(*kind);
                }
            }
            ui.add_space(4.0);
            // magnet (Off → Weak → Strong)
            let m = s.chart.tool.magnet;
            let code = match m {
                Magnet::Off => "M:O",
                Magnet::Weak => "M:W",
                Magnet::Strong => "M:S",
            };
            if ui
                .add_sized(
                    [42.0, 22.0],
                    egui::Button::selectable(m != Magnet::Off, code),
                )
                .on_hover_text(format!(
                    "{} — klik untuk ganti (Off → Weak → Strong)",
                    m.label()
                ))
                .clicked()
            {
                s.chart.tool.magnet = m.next();
            }
            if ui
                .add_sized(
                    [42.0, 22.0],
                    egui::Button::selectable(s.chart.tool.stay, "STAY"),
                )
                .on_hover_text("Tetap di mode gambar setelah satu objek (stay in drawing mode)")
                .clicked()
            {
                s.chart.tool.stay = !s.chart.tool.stay;
            }
            if ui
                .add_sized(
                    [42.0, 22.0],
                    egui::Button::selectable(s.chart.drawings_hidden, "EYE"),
                )
                .on_hover_text("Sembunyikan semua drawing (Ctrl+Alt+H)")
                .clicked()
            {
                s.chart.toggle_hide_all();
            }
            ui.add_space(4.0);
            if ui
                .add_sized([42.0, 22.0], egui::Button::new("?"))
                .on_hover_text("Pintasan keyboard (F1)")
                .clicked()
            {
                *open_help = true;
            }
        });
}

/// Warna level fib berikutnya saat swatch diklik: auto → 8 warna palet → auto.
fn next_fib_color(cur: Option<[u8; 3]>) -> Option<[u8; 3]> {
    match cur {
        None => Some(ITEM_PALETTE[0].0),
        Some(c) => match ITEM_PALETTE.iter().position(|(rgb, _)| *rgb == c) {
            Some(i) if i + 1 < ITEM_PALETTE.len() => Some(ITEM_PALETTE[i + 1].0),
            _ => None,
        },
    }
}

fn side_panel(ui: &mut Ui, s: &mut AppState) {
    let mut notice: Option<String> = None;
    ui.heading("SokanLab");
    ui.small("backtest offline · data lokal");
    ui.separator();

    let chart = &mut s.chart;
    let n = chart.bars.len();
    ui.label(format!("dataset: {n} bar"));
    if n > 0 {
        ui.label(format!(
            "rentang: {}–{}",
            chart.bars[0].ts_millis,
            chart.bars[n - 1].ts_millis
        ));
        if let Some(last) = chart.last_close() {
            ui.label(format!("harga terakhir: {last}"));
        }
    } else {
        ui.small("belum ada data: sokanlab-desktop import --file <csv>");
    }
    ui.separator();

    if ui.button("Pas ke data").clicked() {
        chart.apply(Command::FitToData);
    }
    ui.horizontal(|ui| {
        if ui.button("Batal (undo)").clicked() {
            chart.apply(Command::Undo);
        }
        if ui.button("Ulangi (redo)").clicked() {
            chart.apply(Command::Redo);
        }
    });
    if ui.button("Hapus terpilih").clicked() {
        chart.apply(Command::RemoveSelected);
    }
    let mut want_snapshot = false;
    ui.horizontal(|ui| {
        ui.label("Skala");
        for mode in [ScaleMode::Linear, ScaleMode::Log, ScaleMode::Percent] {
            if ui
                .selectable_label(chart.viewport.scale == mode, mode.label())
                .clicked()
            {
                chart.set_price_scale(mode);
            }
        }
        let mut inv = chart.viewport.invert;
        if ui.checkbox(&mut inv, "Invert").changed() {
            chart.toggle_invert();
        }
    });
    ui.horizontal(|ui| {
        if ui
            .button("Snapshot PNG (Alt+S)")
            .on_hover_text("Simpan tampilan chart ke data/snapshot-<epoch>.png")
            .clicked()
        {
            want_snapshot = true;
        }
        ui.label("Resolusi");
        for s in [1u32, 2, 4] {
            if ui
                .selectable_label(chart.snapshot_scale == s, format!("{s}×"))
                .on_hover_text("Skala piksel render snapshot (dibatasi ~8K)")
                .clicked()
            {
                chart.snapshot_scale = s;
            }
        }
        if ui
            .checkbox(&mut chart.drawings_hidden, "Sembunyikan drawing")
            .changed()
        {
            // checkbox sudah mengubah nilainya; tidak ada aksi tambahan
        }
    });
    if want_snapshot {
        match save_snapshot(chart) {
            Ok(p) => notice = Some(format!("snapshot: {}", p.display())),
            Err(e) => notice = Some(format!("snapshot gagal: {e}")),
        }
    }

    // Indikator overlay (batch 3): SMA/EMA/VWAP di atas candle.
    ui.horizontal(|ui| {
        ui.label("Overlay");
        let mut ov = chart.overlay_settings;
        let mut changed = false;
        let mut on = ov.sma.is_some();
        if ui.checkbox(&mut on, "SMA").changed() {
            ov.sma = if on { Some(50) } else { None };
            changed = true;
        }
        if let Some(p) = ov.sma.as_mut() {
            let mut period = *p as i64;
            if ui
                .add(egui::DragValue::new(&mut period).range(2..=400))
                .changed()
            {
                *p = period as usize;
                changed = true;
            }
        }
        let mut on = ov.ema.is_some();
        if ui.checkbox(&mut on, "EMA").changed() {
            ov.ema = if on { Some(21) } else { None };
            changed = true;
        }
        if let Some(p) = ov.ema.as_mut() {
            let mut period = *p as i64;
            if ui
                .add(egui::DragValue::new(&mut period).range(2..=400))
                .changed()
            {
                *p = period as usize;
                changed = true;
            }
        }
        if ui.checkbox(&mut ov.vwap, "VWAP").changed() {
            changed = true;
        }
        if changed {
            chart.set_overlay_settings(ov);
        }
    });

    // Pane indikator (batch 4): RSI/MACD di bawah volume (maks 3 pane).
    ui.horizontal(|ui| {
        ui.label("Pane");
        let mut set = chart.pane_settings.clone();
        let mut changed = false;
        let mut on = set.iter().any(|p| p.kind == PaneKind::Rsi);
        if ui.checkbox(&mut on, "RSI").changed() {
            if on {
                set.push(PaneSetting {
                    kind: PaneKind::Rsi,
                    period: 14,
                    slow: 0,
                    signal: 0,
                });
            } else {
                set.retain(|p| p.kind != PaneKind::Rsi);
            }
            changed = true;
        }
        if let Some(p) = set.iter_mut().find(|p| p.kind == PaneKind::Rsi) {
            let mut period = p.period as i64;
            if ui
                .add(egui::DragValue::new(&mut period).range(2..=100))
                .changed()
            {
                p.period = period as usize;
                changed = true;
            }
        }
        let mut on = set.iter().any(|p| p.kind == PaneKind::Macd);
        if ui.checkbox(&mut on, "MACD").changed() {
            if on {
                set.push(PaneSetting {
                    kind: PaneKind::Macd,
                    period: 12,
                    slow: 26,
                    signal: 9,
                });
            } else {
                set.retain(|p| p.kind != PaneKind::Macd);
            }
            changed = true;
        }
        if let Some(p) = set.iter_mut().find(|p| p.kind == PaneKind::Macd) {
            let mut fast = p.period as i64;
            if ui
                .add(egui::DragValue::new(&mut fast).range(1..=100))
                .changed()
            {
                p.period = fast as usize;
                changed = true;
            }
            let mut slow = p.slow as i64;
            if ui
                .add(egui::DragValue::new(&mut slow).range(2..=200))
                .changed()
            {
                p.slow = slow as usize;
                changed = true;
            }
            let mut signal = p.signal as i64;
            if ui
                .add(egui::DragValue::new(&mut signal).range(1..=100))
                .changed()
            {
                p.signal = signal as usize;
                changed = true;
            }
        }
        if changed {
            chart.set_panes(set);
        }
    });
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Tema");
        if ui
            .selectable_label(chart.theme == theme::Theme::Dark, "Gelap")
            .clicked()
        {
            chart.theme = theme::Theme::Dark;
        }
        if ui
            .selectable_label(chart.theme == theme::Theme::Light, "Terang")
            .clicked()
        {
            chart.theme = theme::Theme::Light;
        }
    });
    ui.separator();

    // Daftar objek (padanan "object tree" TV): pilih, tampil, kunci, hapus.
    ui.label(format!("drawing ({})", chart.drawings.items.len()));
    let multi_n = chart.selection().len();
    if multi_n > 1 {
        ui.small(format!(
            "{multi_n} objek terpilih — Ctrl+klik menambah/melepas, Ctrl+seret di chart = kotak seleksi"
        ));
    }
    let ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
    let sel_now = chart.selection();
    let mut remove: Option<usize> = None;
    let mut pick_row: Option<usize> = None;
    egui::ScrollArea::vertical()
        .id_salt("objek")
        .max_height(150.0)
        .show(ui, |ui| {
            for (i, it) in chart.drawings.items.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    let eye = if it.visible { "V" } else { "-" };
                    if ui
                        .small_button(eye)
                        .on_hover_text("Tampilkan/sembunyikan objek ini")
                        .clicked()
                    {
                        it.visible = !it.visible;
                    }
                    let lock = if it.locked { "L" } else { "U" };
                    if ui
                        .small_button(lock)
                        .on_hover_text("Kunci/buka kunci objek (objek terkunci tidak bisa digeser)")
                        .clicked()
                    {
                        it.locked = !it.locked;
                    }
                    if it.group.is_some() {
                        ui.small("G");
                    }
                    if ui
                        .selectable_label(sel_now.contains(&i), format!("{}. {}", i, it.label()))
                        .on_hover_text("Klik: pilih · Ctrl+klik: tambah/lepas dari seleksi")
                        .clicked()
                    {
                        pick_row = Some(i);
                    }
                    if ui.small_button("×").on_hover_text("Hapus objek").clicked() {
                        remove = Some(i);
                    }
                });
            }
        });
    if let Some(i) = pick_row {
        if ctrl {
            // Ctrl+klik di daftar objek: tambah/lepas dari seleksi.
            chart.toggle_in_selection(i);
        } else {
            chart.select_only(i);
        }
    }
    if let Some(i) = remove {
        if chart.drawings.remove(i) {
            chart.selection_after_removal(i);
        }
    }

    // Properti objek terpilih (batch 2): nama, warna, ketebalan.
    // Batch 3: pola garis, isi teks, z-order/duplikat/grup.
    let mut delete_selected = false;
    let mut z_forward = false;
    let mut z_back = false;
    let mut dup = false;
    let mut grp = false;
    let mut set_default_style = false;
    let mut apply_kind = false;
    // Batch 6: perubahan gaya pada multi-seleksi disamakan ke anggota lain.
    let mut sync_style = false;
    if let Some(sel) = chart.selected {
        if sel < chart.drawings.items.len() {
            let multi_n = chart.selection().len();
            ui.separator();
            if multi_n > 1 {
                ui.label(format!("Properti objek (primer dari {multi_n} terpilih)"));
                ui.small("Warna/tebal/pola/opasitas ikut diterapkan ke semua yang terpilih");
            } else {
                ui.label("Properti objek");
            }
            let it = &mut chart.drawings.items[sel];
            let mut name = it.name.clone().unwrap_or_default();
            ui.horizontal(|ui| {
                ui.label("Nama");
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut name)
                            .hint_text(it.drawing.kind_label())
                            .desired_width(150.0),
                    )
                    .changed()
                {
                    let trimmed = name.trim();
                    it.name = if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    };
                }
            });
            ui.horizontal(|ui| {
                ui.label("Warna");
                for (rgb, label) in ITEM_PALETTE {
                    let color = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                    let chosen = it.style.map(|st| st.color == rgb).unwrap_or(false);
                    let btn = egui::Button::new("")
                        .fill(color)
                        .min_size(egui::vec2(16.0, 16.0));
                    let resp = if chosen {
                        ui.add(btn.stroke(egui::Stroke::new(2.0, egui::Color32::WHITE)))
                    } else {
                        ui.add(btn)
                    };
                    if resp.on_hover_text(label).clicked() {
                        let width = it.style.map(|st| st.width).unwrap_or(2);
                        let dash = it.style.map(|st| st.dash).unwrap_or(DashStyle::Solid);
                        it.style = Some(ItemStyle {
                            color: rgb,
                            width,
                            dash,
                        });
                        sync_style = true;
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Tebal");
                for w in 1u8..=4 {
                    let chosen = it.style.map(|st| st.width == w).unwrap_or(false);
                    if ui.selectable_label(chosen, format!("{w}")).clicked() {
                        let color = it.style.map(|st| st.color).unwrap_or([0x5b, 0x65, 0x70]);
                        let dash = it.style.map(|st| st.dash).unwrap_or(DashStyle::Solid);
                        it.style = Some(ItemStyle {
                            color,
                            width: w,
                            dash,
                        });
                        sync_style = true;
                    }
                }
                if ui
                    .add_enabled(it.style.is_some(), egui::Button::new("Default"))
                    .on_hover_text("Kembali ke gaya default alat")
                    .clicked()
                {
                    it.style = None;
                    sync_style = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Garis");
                for (d, label) in [
                    (DashStyle::Solid, "Solid"),
                    (DashStyle::Dashed, "Putus"),
                    (DashStyle::Dotted, "Titik"),
                ] {
                    let chosen = it
                        .style
                        .map(|st| st.dash == d)
                        .unwrap_or(d == DashStyle::Solid);
                    if ui.selectable_label(chosen, label).clicked() {
                        let color = it.style.map(|st| st.color).unwrap_or([0x5b, 0x65, 0x70]);
                        let width = it.style.map(|st| st.width).unwrap_or(2);
                        it.style = Some(ItemStyle {
                            color,
                            width,
                            dash: d,
                        });
                        sync_style = true;
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Opasitas");
                if ui
                    .add(
                        egui::Slider::new(&mut it.opacity, 0..=100)
                            .suffix("%")
                            .fixed_decimals(0),
                    )
                    .on_hover_text("0% = tembus pandang penuh, 100% = penuh")
                    .changed()
                {
                    sync_style = true;
                }
            });
            if matches!(it.drawing, Drawing::Pitchfork { .. }) {
                ui.horizontal(|ui| {
                    ui.label("Varian");
                    egui::ComboBox::from_id_salt("varian-pitchfork")
                        .selected_text(it.pitchfork.label())
                        .show_ui(ui, |ui| {
                            for k in PitchforkKind::all() {
                                ui.selectable_value(&mut it.pitchfork, k, k.label());
                            }
                        });
                });
            }
            if let Drawing::ParallelChannel { kind, .. } = &mut it.drawing {
                ui.horizontal(|ui| {
                    ui.label("Varian");
                    egui::ComboBox::from_id_salt("varian-channel")
                        .selected_text(kind.label())
                        .show_ui(ui, |ui| {
                            for k in ChannelKind::all() {
                                ui.selectable_value(kind, k, k.label());
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Extend");
                    let (mut l, mut r) = (it.extend.left(), it.extend.right());
                    if ui.checkbox(&mut l, "kiri").changed()
                        || ui.checkbox(&mut r, "kanan").changed()
                    {
                        it.extend = ExtendMode::from_checks(l, r);
                    }
                });
            }
            if matches!(it.drawing, Drawing::RegressionChannel { .. }) {
                ui.horizontal(|ui| {
                    ui.label("Extend");
                    let (mut l, mut r) = (it.extend.left(), it.extend.right());
                    if ui.checkbox(&mut l, "kiri").changed()
                        || ui.checkbox(&mut r, "kanan").changed()
                    {
                        it.extend = ExtendMode::from_checks(l, r);
                    }
                });
            }
            if matches!(it.drawing, Drawing::FibTrendExtension { .. }) {
                ui.horizontal(|ui| {
                    ui.label("Level");
                    let mut l = it.extend.left();
                    if ui
                        .checkbox(&mut l, "perpanjang kiri")
                        .on_hover_text("Default: level digambar dari titik ketiga ke kanan")
                        .changed()
                    {
                        it.extend = ExtendMode::from_checks(l, false);
                    }
                });
            }
            if let Drawing::Sticker { icon, .. } = &mut it.drawing {
                ui.horizontal(|ui| {
                    ui.label("Ikon");
                    egui::ComboBox::from_id_salt("ikon-sticker")
                        .selected_text(icon.label())
                        .show_ui(ui, |ui| {
                            for k in StickerIcon::all() {
                                ui.selectable_value(icon, k, k.label());
                            }
                        });
                });
            }
            if matches!(it.drawing, Drawing::FibRetracement { .. }) {
                if it.fib_levels.is_empty() {
                    if ui
                        .button("Ubah level fib")
                        .on_hover_text("Aktifkan level kustom, mulai dari level bawaan")
                        .clicked()
                    {
                        it.fib_levels = FIB_LEVELS_MILLI
                            .iter()
                            .map(|m| FibLevel {
                                milli: *m,
                                color: None,
                            })
                            .collect();
                    }
                } else {
                    ui.label("Level fib kustom");
                    let mut remove_level: Option<usize> = None;
                    for i in 0..it.fib_levels.len() {
                        ui.horizontal(|ui| {
                            let mut milli = it.fib_levels[i].milli;
                            let resp = ui.add(
                                egui::DragValue::new(&mut milli)
                                    .speed(1)
                                    .range(-2000..=3000)
                                    .custom_formatter(|n, _| format!("{:.1}%", n / 10.0))
                                    .custom_parser(|s| {
                                        s.trim_end_matches('%')
                                            .trim()
                                            .parse::<f64>()
                                            .ok()
                                            .map(|v| v * 10.0)
                                    }),
                            );
                            if resp.changed() {
                                it.fib_levels[i].milli = milli;
                            }
                            let rgb = it.fib_levels[i].color.unwrap_or([0x5b, 0x65, 0x70]);
                            let swatch = egui::Button::new("")
                                .fill(egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]))
                                .min_size(egui::vec2(16.0, 16.0));
                            let tip = match it.fib_levels[i].color {
                                Some(_) => "Warna level: klik untuk warna berikutnya (sampai auto)",
                                None => "Warna otomatis — klik untuk memilih warna",
                            };
                            if ui.add(swatch).on_hover_text(tip).clicked() {
                                it.fib_levels[i].color = next_fib_color(it.fib_levels[i].color);
                            }
                            if ui.small_button("x").on_hover_text("Hapus level").clicked() {
                                remove_level = Some(i);
                            }
                        });
                    }
                    if let Some(i) = remove_level {
                        it.fib_levels.remove(i);
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .button("+ Level")
                            .on_hover_text("Tambah level pada 50%")
                            .clicked()
                        {
                            let mut m = 500;
                            while it.fib_levels.iter().any(|l| l.milli == m) && m < 1100 {
                                m += 1;
                            }
                            it.fib_levels.push(FibLevel {
                                milli: m,
                                color: None,
                            });
                            it.fib_levels.sort_by_key(|l| l.milli);
                        }
                        if ui
                            .button("Kembali bawaan")
                            .on_hover_text("Pakai lagi tujuh level standar")
                            .clicked()
                        {
                            it.fib_levels.clear();
                        }
                    });
                }
            }
            ui.horizontal(|ui| {
                if ui
                    .button("Jadikan default")
                    .on_hover_text("Objek baru memakai warna/tebal/pola objek ini")
                    .clicked()
                {
                    set_default_style = true;
                }
                if ui
                    .button("Terapkan ke sejenis")
                    .on_hover_text("Samakan warna/tebal/pola ke semua objek jenis ini")
                    .clicked()
                {
                    apply_kind = true;
                }
            });
            if let Drawing::Text { text, .. } | Drawing::Callout { text, .. } = &mut it.drawing {
                ui.horizontal(|ui| {
                    ui.label("Teks");
                    ui.add(
                        egui::TextEdit::singleline(text)
                            .hint_text("isi catatan")
                            .desired_width(150.0),
                    );
                });
            }
            if matches!(it.drawing, Drawing::Text { .. } | Drawing::Callout { .. }) {
                ui.horizontal(|ui| {
                    ui.label("Ukuran");
                    egui::ComboBox::from_id_salt("ukuran-teks")
                        .selected_text(it.text_size.label())
                        .show_ui(ui, |ui| {
                            for k in TextSize::all() {
                                ui.selectable_value(&mut it.text_size, k, k.label());
                            }
                        });
                    ui.label("Perataan");
                    egui::ComboBox::from_id_salt("align-teks")
                        .selected_text(it.text_align.label())
                        .show_ui(ui, |ui| {
                            for k in TextAlign::all() {
                                ui.selectable_value(&mut it.text_align, k, k.label());
                            }
                        });
                });
            }
            ui.horizontal(|ui| {
                ui.checkbox(&mut it.visible, "Tampil");
                ui.checkbox(&mut it.locked, "Kunci");
                let label = if multi_n > 1 {
                    format!("Hapus {multi_n} objek")
                } else {
                    "Hapus objek".to_string()
                };
                if ui
                    .button(label)
                    .on_hover_text("Menghapus semua objek yang terpilih (satu langkah undo)")
                    .clicked()
                {
                    delete_selected = true;
                }
            });
            ui.horizontal(|ui| {
                if ui
                    .button("Ke depan")
                    .on_hover_text("Naikkan z-order")
                    .clicked()
                {
                    z_forward = true;
                }
                if ui
                    .button("Ke belakang")
                    .on_hover_text("Turunkan z-order")
                    .clicked()
                {
                    z_back = true;
                }
                if ui.button("Duplikat").clicked() {
                    dup = true;
                }
                let grp_label = match it.group {
                    Some(g) => format!("Lepas grup G{g}"),
                    None => "Jadikan grup".to_string(),
                };
                if ui
                    .button(grp_label)
                    .on_hover_text("Drag badan objek bergrup menggeser semua anggotanya")
                    .clicked()
                {
                    grp = true;
                }
            });
        }
    }
    if delete_selected {
        chart.delete_selection();
    }
    if sync_style {
        chart.sync_style_from_primary();
    }
    if z_forward {
        chart.move_selected_z(1);
    }
    if z_back {
        chart.move_selected_z(-1);
    }
    if dup {
        chart.duplicate_selected();
    }
    if set_default_style {
        notice = Some(if chart.set_default_style_from_selected() {
            "Gaya objek jadi default objek baru".into()
        } else {
            "Atur warna/tebal/pola dulu sebelum jadikan default".into()
        });
    }
    if apply_kind {
        let n = chart.apply_style_to_same_kind();
        notice = Some(format!("{n} objek sejenis disamakan"));
    }
    if grp {
        chart.toggle_group_selected();
    }

    // Batch 7: katalog template gaya bernama (simpan dari objek primer,
    // terapkan ke seluruh objek terpilih, hapus dari katalog).
    ui.separator();
    ui.label("Template gaya");
    let mut tpl_pick: Option<String> = None;
    let (mut tpl_save, mut tpl_apply, mut tpl_del) = (false, false, false);
    ui.horizontal(|ui| {
        let sel_text = chart
            .template_sel
            .clone()
            .unwrap_or_else(|| "— pilih —".to_string());
        egui::ComboBox::from_id_salt("template-gaya")
            .selected_text(sel_text)
            .show_ui(ui, |ui| {
                for t in &chart.templates {
                    let chosen = chart.template_sel.as_deref() == Some(t.name.as_str());
                    if ui.selectable_label(chosen, &t.name).clicked() {
                        tpl_pick = Some(t.name.clone());
                    }
                }
            });
        if ui
            .add_enabled(chart.template_sel.is_some(), egui::Button::new("Terapkan"))
            .on_hover_text("Terapkan warna/tebal/pola/opasitas ke semua objek terpilih")
            .clicked()
        {
            tpl_apply = true;
        }
        if ui
            .add_enabled(chart.template_sel.is_some(), egui::Button::new("Hapus"))
            .on_hover_text("Hapus template dari katalog")
            .clicked()
        {
            tpl_del = true;
        }
    });
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut chart.template_name)
                .hint_text("nama template")
                .desired_width(110.0),
        );
        let can_save = chart.selected.is_some() && !chart.template_name.trim().is_empty();
        if ui
            .add_enabled(can_save, egui::Button::new("Simpan"))
            .on_hover_text("Simpan gaya objek primer sebagai template (nama sama = perbarui)")
            .clicked()
        {
            tpl_save = true;
        }
        if chart.selected.is_none() {
            ui.small("pilih objek dulu");
        }
    });
    if let Some(name) = tpl_pick {
        chart.template_sel = Some(name);
    }
    if tpl_save {
        let name = chart.template_name.clone();
        match chart.save_template(&name) {
            Ok(msg) => {
                chart.template_sel =
                    Some(name.trim().chars().take(StyleTemplate::NAME_MAX).collect());
                notice = Some(msg);
            }
            Err(e) => notice = Some(e),
        }
    }
    if tpl_apply {
        let name = chart.template_sel.clone();
        let n = match name {
            Some(name) => chart.apply_template(&name),
            None => 0,
        };
        notice = Some(match n {
            0 => "belum ada objek terpilih / template belum dipilih".into(),
            n => format!("template diterapkan ke {n} objek"),
        });
    }
    if tpl_del {
        if let Some(name) = chart.template_sel.clone() {
            notice = Some(if chart.remove_template(&name) {
                format!("template \"{name}\" dihapus")
            } else {
                "template tidak ditemukan".into()
            });
        }
    }

    if let Some(n) = notice {
        s.notice = Some(n);
    }
}

impl eframe::App for SokanApp {
    fn ui(&mut self, root: &mut Ui, _frame: &mut eframe::Frame) {
        let t0 = std::time::Instant::now();
        let ctx = root.ctx().clone();
        if self.bench.is_some() {
            // Mode bench: paksa repaint terus agar frame-time terukur.
            ctx.request_repaint();
        }
        self.handle_shortcuts(&ctx);
        let theme_now = {
            let s = self.state.lock().unwrap();
            s.chart.theme
        };
        theme::apply(&ctx, theme_now);

        let mut open_help = false;

        Panel::top("toolbar").show(&mut *root, |ui| {
            ui.horizontal(|ui| {
                ui.heading("SokanLab");
                ui.separator();
                let mut s = self.state.lock().unwrap();
                if ui.button("Pas ke data").clicked() {
                    s.chart.apply(Command::FitToData);
                }
                if ui.button("Undo").clicked() {
                    s.chart.apply(Command::Undo);
                }
                if ui.button("Redo").clicked() {
                    s.chart.apply(Command::Redo);
                }
                ui.separator();
                let tool = s.chart.tool.kind.label();
                ui.label(format!("alat: {tool}"));
                if s.chart.tool.magnet != Magnet::Off {
                    ui.label(format!("· {}", s.chart.tool.magnet.label()));
                }
                ui.separator();
                ui.label(format!("bar: {}", s.bars.len()));
                if s.runner.is_running() {
                    ui.spinner();
                    ui.small("worker backtest jalan");
                }
                if s.bt.dirty && s.result.is_some() {
                    ui.colored_label(theme::EMBER, "hasil basi (parameter berubah)");
                }
            });
        });

        Panel::left("tools")
            .exact_size(50.0)
            .resizable(false)
            .show(&mut *root, |ui| {
                let mut s = self.state.lock().unwrap();
                tool_strip(ui, &mut s, &mut open_help);
            });

        Panel::left("side")
            .resizable(true)
            .default_size(300.0)
            .show(&mut *root, |ui| {
                let mut s = self.state.lock().unwrap();
                side_panel(ui, &mut s);
                backtest_panel::pump(ui, &mut s);
                backtest_panel::backtest_form(ui, &mut s);
                backtest_panel::backtest_result(ui, &mut s);
            });

        egui::CentralPanel::default().show(root, |ui| {
            let mut s = self.state.lock().unwrap();
            if s.chart.bars.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("Belum ada data. Jalankan: sokanlab-desktop import --file <csv>");
                });
            } else {
                chart_panel(ui, &mut s.chart);
            }
        });

        if open_help {
            self.show_shortcuts = true;
        }
        self.shortcuts_window(&ctx);
        self.goto_window(&ctx);

        self.note_frame(t0.elapsed().as_secs_f64() * 1000.0);
    }

    fn on_exit(&mut self) {
        let s = self.state.lock().unwrap();
        match s.save_layout() {
            Ok(()) => {
                log::info!("layout tersimpan: {}", s.layout_path);
                println!("layout tersimpan: {}", s.layout_path);
            }
            Err(e) => {
                log::warn!("gagal simpan layout: {e}");
                eprintln!("[sokanlab] gagal simpan layout: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sokanlab_domain::Bar;

    fn bars(n: usize) -> Vec<Bar> {
        (0..n)
            .map(|i| Bar {
                ts_millis: 1_700_000_000_000 + i as i64 * 60_000,
                open_ticks: 100 + (i as i64 % 9),
                high_ticks: 108 + (i as i64 % 9),
                low_ticks: 92 + (i as i64 % 9),
                close_ticks: 104 + (i as i64 % 9),
                volume_milli: 10,
            })
            .collect()
    }

    /// Satu pass egui headless dengan event sintetis (interaksi nyata dalam-proses).
    fn pass(
        ctx: &egui::Context,
        chart: &mut ChartState,
        events: Vec<egui::Event>,
        size: egui::Vec2,
    ) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            events,
            ..Default::default()
        };
        let mut full = ctx.run_pass(input, |ctx| {
            ctx.root_ui(|ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    chart_panel(ui, chart);
                });
            });
        });
        full.textures_delta.clear();
        drop(full);
    }

    /// Klik lengkap (tekan + lepas) pada satu titik.
    fn klik(ctx: &egui::Context, chart: &mut ChartState, p: egui::Pos2, size: egui::Vec2) {
        pass(ctx, chart, vec![move_to(p)], size);
        pass(ctx, chart, vec![press(p)], size);
        pass(ctx, chart, vec![release(p)], size);
    }

    /// Klik dengan Ctrl/Cmd ditahan (multi-seleksi). egui 0.37 mengubah
    /// `InputState.modifiers` lewat `Event::ModifiersChanged`, bukan RawInput.
    fn klik_ctrl(ctx: &egui::Context, chart: &mut ChartState, p: egui::Pos2, size: egui::Vec2) {
        let md = egui::Modifiers::CTRL;
        let btn = |pressed: bool| egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: md,
        };
        pass(
            ctx,
            chart,
            vec![egui::Event::ModifiersChanged(md), move_to(p)],
            size,
        );
        pass(ctx, chart, vec![btn(true)], size);
        pass(ctx, chart, vec![btn(false)], size);
        pass(
            ctx,
            chart,
            vec![egui::Event::ModifiersChanged(egui::Modifiers::default())],
            size,
        );
    }

    /// Seret tombol kiri dari `from` ke `to` (beberapa langkah agar > ambang 4 px).
    fn seret_mod(
        ctx: &egui::Context,
        chart: &mut ChartState,
        from: egui::Pos2,
        to: egui::Pos2,
        size: egui::Vec2,
        md: egui::Modifiers,
    ) {
        let btn = |p: egui::Pos2, pressed: bool| egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: md,
        };
        pass(
            ctx,
            chart,
            vec![egui::Event::ModifiersChanged(md), move_to(from)],
            size,
        );
        pass(ctx, chart, vec![btn(from, true)], size);
        let mid = egui::pos2((from.x + to.x) / 2.0, (from.y + to.y) / 2.0);
        pass(ctx, chart, vec![move_to(mid)], size);
        pass(ctx, chart, vec![move_to(to)], size);
        pass(ctx, chart, vec![btn(to, false)], size);
        pass(
            ctx,
            chart,
            vec![egui::Event::ModifiersChanged(egui::Modifiers::default())],
            size,
        );
    }

    fn seret(
        ctx: &egui::Context,
        chart: &mut ChartState,
        from: egui::Pos2,
        to: egui::Pos2,
        size: egui::Vec2,
    ) {
        seret_mod(ctx, chart, from, to, size, egui::Modifiers::default());
    }

    fn seret_ctrl(
        ctx: &egui::Context,
        chart: &mut ChartState,
        from: egui::Pos2,
        to: egui::Pos2,
        size: egui::Vec2,
    ) {
        seret_mod(ctx, chart, from, to, size, egui::Modifiers::CTRL);
    }

    fn move_to(p: egui::Pos2) -> egui::Event {
        egui::Event::PointerMoved(p)
    }

    fn press(p: egui::Pos2) -> egui::Event {
        egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::default(),
        }
    }

    fn release(p: egui::Pos2) -> egui::Event {
        egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::default(),
        }
    }

    fn setup() -> (egui::Context, ChartState, egui::Vec2) {
        let ctx = egui::Context::default();
        let mut chart = ChartState::new();
        chart.load(bars(60));
        let size = egui::vec2(800.0, 600.0);
        // pass awal: daftarkan widget
        pass(&ctx, &mut chart, vec![], size);
        (ctx, chart, size)
    }

    #[test]
    fn klik_dua_titik_membangun_trendline() {
        let (ctx, mut chart, size) = setup();
        chart.tool.select(ToolKind::Trend);
        let p0 = egui::pos2(150.0, 200.0);
        let p1 = egui::pos2(400.0, 320.0);
        // klik pertama
        pass(&ctx, &mut chart, vec![move_to(p0)], size);
        pass(&ctx, &mut chart, vec![press(p0)], size);
        pass(&ctx, &mut chart, vec![release(p0)], size);
        assert_eq!(chart.tool.pending.len(), 1);
        assert!(chart.drawings.items.is_empty());
        // klik kedua → objek selesai
        pass(&ctx, &mut chart, vec![move_to(p1)], size);
        pass(&ctx, &mut chart, vec![press(p1)], size);
        pass(&ctx, &mut chart, vec![release(p1)], size);
        assert_eq!(chart.drawings.items.len(), 1);
        match &chart.drawings.items[0].drawing {
            Drawing::Trendline { a, b } => {
                assert!(a.ts_millis != b.ts_millis);
                assert!(a.price_ticks != b.price_ticks);
            }
            other => panic!("jenis salah: {other:?}"),
        }
        // kembali ke Select karena stay=false
        assert_eq!(chart.tool.kind, ToolKind::Select);
        assert_eq!(chart.selected, Some(0));
    }

    #[test]
    fn seret_sekali_membuat_rectangle() {
        let (ctx, mut chart, size) = setup();
        chart.tool.select(ToolKind::Rectangle);
        let p0 = egui::pos2(200.0, 180.0);
        let p1 = egui::pos2(500.0, 300.0);
        pass(&ctx, &mut chart, vec![move_to(p0)], size);
        pass(&ctx, &mut chart, vec![press(p0)], size);
        pass(&ctx, &mut chart, vec![move_to(p1)], size);
        pass(&ctx, &mut chart, vec![move_to(p1)], size);
        pass(&ctx, &mut chart, vec![release(p1)], size);
        assert_eq!(chart.drawings.items.len(), 1);
        assert!(matches!(
            chart.drawings.items[0].drawing,
            Drawing::Rectangle { .. }
        ));
    }

    #[test]
    fn klik_pada_objek_menyeleksi_dan_drag_memindahkannya() {
        let (ctx, mut chart, size) = setup();
        let a = Anchor {
            ts_millis: chart.bars[10].ts_millis,
            price_ticks: 100,
        };
        let b = Anchor {
            ts_millis: chart.bars[30].ts_millis,
            price_ticks: 106,
        };
        chart.add_drawing(Drawing::Trendline { a, b });
        chart.selected = None;
        // titik tengah segmen di layar
        let rect = chart.plot_rect;
        let mid = egui::pos2(
            rect.min.x + chart.viewport.x_for_time((a.ts_millis + b.ts_millis) / 2) as f32,
            rect.min.y + chart.viewport.y_for_price(103) as f32,
        );
        pass(&ctx, &mut chart, vec![move_to(mid)], size);
        pass(&ctx, &mut chart, vec![press(mid)], size);
        pass(&ctx, &mut chart, vec![release(mid)], size);
        assert_eq!(chart.selected, Some(0), "klik badan objek harus menyeleksi");
        // drag badan objek ke kanan → anchor bergeser
        let before = chart.drawings.items[0].drawing.clone();
        pass(&ctx, &mut chart, vec![press(mid)], size);
        let moved = egui::pos2(mid.x + 60.0, mid.y + 20.0);
        pass(&ctx, &mut chart, vec![move_to(moved)], size);
        pass(&ctx, &mut chart, vec![move_to(moved)], size);
        pass(&ctx, &mut chart, vec![release(moved)], size);
        let after = chart.drawings.items[0].drawing.clone();
        assert_ne!(before, after, "drag badan harus menggeser objek");
        assert_eq!(chart.drawings.items.len(), 1);
    }

    /// Dua trendline lewat klik alat (posisi layar pasti) untuk tes multi-seleksi.
    fn dua_trendline(ctx: &egui::Context, chart: &mut ChartState, size: egui::Vec2) {
        chart.tool.select(ToolKind::Trend);
        klik(ctx, chart, egui::pos2(150.0, 200.0), size);
        klik(ctx, chart, egui::pos2(300.0, 260.0), size);
        chart.tool.select(ToolKind::Trend);
        klik(ctx, chart, egui::pos2(450.0, 180.0), size);
        klik(ctx, chart, egui::pos2(600.0, 240.0), size);
        assert_eq!(chart.drawings.items.len(), 2, "dua trendline terbentuk");
    }

    #[test]
    fn ctrl_klik_multi_seleksi_dan_seret_memindah_semuanya() {
        let (ctx, mut chart, size) = setup();
        dua_trendline(&ctx, &mut chart, size);
        assert_eq!(chart.selected, Some(1), "objek terakhir langsung terpilih");
        // Ctrl+klik badan objek pertama → dua objek terpilih.
        klik_ctrl(&ctx, &mut chart, egui::pos2(225.0, 230.0), size);
        assert_eq!(chart.selection(), vec![0, 1]);
        assert_eq!(chart.selected, Some(1), "primer tetap objek teratas");
        assert!(matches!(chart.gesture, Gesture::Idle));
        // Seret badan objek pertama (tanpa Ctrl): seleksi tidak kolaps,
        // kedua objek ikut bergeser dengan satu langkah undo.
        let before: Vec<Drawing> = chart
            .drawings
            .items
            .iter()
            .map(|it| it.drawing.clone())
            .collect();
        let undo_before = chart.drawings.undo.len();
        seret(
            &ctx,
            &mut chart,
            egui::pos2(225.0, 230.0),
            egui::pos2(255.0, 262.0),
            size,
        );
        assert_eq!(chart.selection(), vec![0, 1], "seleksi bertahan saat drag");
        assert_eq!(
            chart.drawings.undo.len(),
            undo_before + 1,
            "satu langkah undo untuk drag bersama"
        );
        for (i, b) in before.iter().enumerate() {
            assert_ne!(chart.drawings.items[i].drawing, *b, "objek {i} bergeser");
        }
        // Ctrl+klik salah satu objek yang terpilih → lepas dari seleksi
        // (klik di tengah objek 0 = anggota non-primer; primer tetap).
        let rect = chart.plot_rect;
        let (a, b) = match &chart.drawings.items[0].drawing {
            Drawing::Trendline { a, b } => (*a, *b),
            other => panic!("bukan trendline: {other:?}"),
        };
        let mid = egui::pos2(
            rect.min.x + chart.viewport.x_for_time((a.ts_millis + b.ts_millis) / 2) as f32,
            rect.min.y
                + chart
                    .viewport
                    .y_for_price((a.price_ticks + b.price_ticks) / 2) as f32,
        );
        klik_ctrl(&ctx, &mut chart, mid, size);
        assert_eq!(chart.selection(), vec![1], "objek 0 dilepas dari seleksi");
    }

    #[test]
    fn kotak_seleksi_ctrl_seret_di_area_kosong() {
        let (ctx, mut chart, size) = setup();
        dua_trendline(&ctx, &mut chart, size);
        chart.clear_selection();
        // Ctrl+seret kotak dari area kosong sampai melewati kedua objek.
        seret_ctrl(
            &ctx,
            &mut chart,
            egui::pos2(100.0, 320.0),
            egui::pos2(660.0, 150.0),
            size,
        );
        assert_eq!(chart.selection(), vec![0, 1], "kotak memilih keduanya");
        assert!(matches!(chart.gesture, Gesture::Idle), "gestur selesai");
        // Kotak kecil di area kosong: seleksi dibersihkan.
        seret_ctrl(
            &ctx,
            &mut chart,
            egui::pos2(700.0, 400.0),
            egui::pos2(740.0, 440.0),
            size,
        );
        assert!(chart.selection().is_empty(), "kotak kosong membersihkan");
        // Ctrl+klik di area kosong juga tidak menyeleksi apa pun.
        klik_ctrl(&ctx, &mut chart, egui::pos2(700.0, 420.0), size);
        assert!(chart.selection().is_empty());
    }

    #[test]
    fn multi_seleksi_hapus_undo_dan_z_order() {
        let (ctx, mut chart, size) = setup();
        // Tiga objek: dua trendline + satu garis horizontal.
        dua_trendline(&ctx, &mut chart, size);
        chart.add_drawing(Drawing::HorizontalLine { price_ticks: 100 });
        assert_eq!(chart.drawings.items.len(), 3);
        // Hapus semua terpilih (jalur Command::RemoveSelected = hotkey Del).
        chart.set_selection(&[0, 2]);
        assert_eq!(
            chart.apply(Command::RemoveSelected),
            sokanlab_ui::Event::DrawingsChanged
        );
        assert_eq!(chart.drawings.items.len(), 1, "dua terhapus sekaligus");
        assert!(chart.selection().is_empty());
        assert_eq!(
            chart.apply(Command::RemoveSelected),
            sokanlab_ui::Event::Ignored
        );
        // Undo mengembalikan dua objek sekaligus; seleksi tetap bersih.
        assert_eq!(
            chart.apply(Command::Undo),
            sokanlab_ui::Event::DrawingsChanged
        );
        assert_eq!(chart.drawings.items.len(), 3);
        assert!(chart.selection().is_empty(), "undo membersihkan seleksi");
        // Z-order seleksi banyak: [0, 1] naik satu langkah → [1, 2].
        chart.set_selection(&[0, 1]);
        assert!(chart.move_selected_z(1), "dua objek naik satu langkah");
        assert_eq!(chart.selection(), vec![1, 2]);
        assert!(!chart.move_selected_z(1), "anggota teratas sudah mentok");
        // Hapus per baris di daftar objek: indeks seleksi bergeser turun.
        assert!(chart.drawings.remove(0));
        chart.selection_after_removal(0);
        assert_eq!(chart.selection(), vec![0]);
    }

    #[test]
    fn esc_membatalkan_penempatan_lewat_hotkey() {
        let (ctx, mut chart, size) = setup();
        chart.tool.select(ToolKind::Trend);
        let p0 = egui::pos2(150.0, 200.0);
        pass(&ctx, &mut chart, vec![move_to(p0)], size);
        pass(&ctx, &mut chart, vec![press(p0)], size);
        pass(&ctx, &mut chart, vec![release(p0)], size);
        assert_eq!(chart.tool.pending.len(), 1);
        // Hotkey Esc melalui peta aksi yang sama dengan aplikasi.
        let action = sokanlab_ui::tools::action_for(egui::Key::Escape, egui::Modifiers::default());
        assert_eq!(action, Some(Action::Cancel));
        let mut s = AppState::new();
        s.chart = chart.clone();
        apply_action(&mut s, Action::Cancel);
        assert!(s.chart.tool.pending.is_empty());
        assert_eq!(s.chart.tool.kind, ToolKind::Trend);
        apply_action(&mut s, Action::Cancel);
        assert_eq!(s.chart.tool.kind, ToolKind::Select);
    }

    #[test]
    fn hotkey_real_switch_alat_dan_undo() {
        let mut s = AppState::new();
        s.chart = {
            let mut c = ChartState::new();
            c.load(bars(20));
            c
        };
        let alt = egui::Modifiers {
            alt: true,
            ..Default::default()
        };
        let a = sokanlab_ui::tools::action_for(egui::Key::T, alt).unwrap();
        apply_action(&mut s, a);
        assert_eq!(s.chart.tool.kind, ToolKind::Trend);
        s.chart.add_drawing(Drawing::Trendline {
            a: Anchor {
                ts_millis: 1,
                price_ticks: 1,
            },
            b: Anchor {
                ts_millis: 2,
                price_ticks: 2,
            },
        });
        let ctrl = egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        let undo = sokanlab_ui::tools::action_for(egui::Key::Z, ctrl).unwrap();
        apply_action(&mut s, undo);
        assert!(s.chart.drawings.items.is_empty());
    }

    #[test]
    fn objek_terkunci_dan_tersembunyi_tidak_terpilih_lewat_klik() {
        let (ctx, mut chart, size) = setup();
        let a = Anchor {
            ts_millis: chart.bars[10].ts_millis,
            price_ticks: 100,
        };
        let b = Anchor {
            ts_millis: chart.bars[30].ts_millis,
            price_ticks: 106,
        };
        chart.add_drawing(Drawing::Trendline { a, b });
        chart.selected = None;
        let rect = chart.plot_rect;
        let mid = egui::pos2(
            rect.min.x + chart.viewport.x_for_time((a.ts_millis + b.ts_millis) / 2) as f32,
            rect.min.y + chart.viewport.y_for_price(103) as f32,
        );
        // Kunci objek → klik tidak menyeleksi
        chart.drawings.items[0].locked = true;
        pass(&ctx, &mut chart, vec![move_to(mid)], size);
        pass(&ctx, &mut chart, vec![press(mid)], size);
        pass(&ctx, &mut chart, vec![release(mid)], size);
        assert_eq!(chart.selected, None, "objek terkunci tidak bisa dipilih");
        // Buka kunci → bisa dipilih lagi
        chart.drawings.items[0].locked = false;
        pass(&ctx, &mut chart, vec![move_to(mid)], size);
        pass(&ctx, &mut chart, vec![press(mid)], size);
        pass(&ctx, &mut chart, vec![release(mid)], size);
        assert_eq!(chart.selected, Some(0));
        // Sembunyikan objek → klik tidak menyeleksi
        chart.drawings.items[0].visible = false;
        chart.selected = None;
        pass(&ctx, &mut chart, vec![move_to(mid)], size);
        pass(&ctx, &mut chart, vec![press(mid)], size);
        pass(&ctx, &mut chart, vec![release(mid)], size);
        assert_eq!(chart.selected, None, "objek tersembunyi tidak bisa dipilih");
    }

    #[test]
    fn side_panel_dan_tool_strip_merender_tanpa_panic() {
        // Panel kiri (daftar objek + properti) dan strip alat dirender headless
        // dengan objek terpilih (gaya/nama/kunci) — memastikan UI batch 2 hidup.
        let ctx = egui::Context::default();
        let mut s = AppState::new();
        s.chart.load(bars(30));
        s.chart
            .add_drawing(Drawing::HorizontalLine { price_ticks: 100 });
        s.chart.add_drawing(Drawing::ParallelChannel {
            a: Anchor {
                ts_millis: 1_000,
                price_ticks: 90,
            },
            b: Anchor {
                ts_millis: 500_000,
                price_ticks: 110,
            },
            c: Anchor {
                ts_millis: 1_000,
                price_ticks: 95,
            },
            kind: ChannelKind::Parallel,
        });
        s.chart.selected = Some(1);
        s.chart.drawings.items[0].visible = false;
        s.chart.drawings.items[1].locked = true;
        s.chart.drawings.items[1].name = Some("ch uji".into());
        s.chart.drawings.items[1].style = Some(ItemStyle {
            color: [0x7f, 0xb2, 0xd9],
            width: 2,
            dash: DashStyle::Solid,
        });
        // Batch 3: teks/callout, objek bergrup + dash, overlay aktif.
        s.chart.add_drawing(Drawing::Text {
            at: Anchor {
                ts_millis: 2_000,
                price_ticks: 101,
            },
            text: "halo".into(),
        });
        s.chart.add_drawing(Drawing::Callout {
            at: Anchor {
                ts_millis: 3_000,
                price_ticks: 99,
            },
            text: "titik".into(),
        });
        s.chart.set_selection(&[2, 3]);
        assert!(s.chart.toggle_group_selected());
        s.chart.drawings.items[3].style = Some(ItemStyle {
            color: [0xf2, 0xb0, 0x4d],
            width: 3,
            dash: DashStyle::Dotted,
        });
        // Batch 7: varian channel (disjoint + flat) dengan perpanjangan, dan
        // katalog template terpilih (kombo + tombol Simpan/Terapkan/Hapus).
        for kind in [ChannelKind::Disjoint, ChannelKind::Flat] {
            s.chart.add_drawing(Drawing::ParallelChannel {
                a: Anchor {
                    ts_millis: 2_000,
                    price_ticks: 95,
                },
                b: Anchor {
                    ts_millis: 400_000,
                    price_ticks: 115,
                },
                c: Anchor {
                    ts_millis: 5_000,
                    price_ticks: 105,
                },
                kind,
            });
        }
        let n_ch = s.chart.drawings.items.len();
        s.chart.drawings.items[n_ch - 2].extend = ExtendMode::Left;
        s.chart.drawings.items[n_ch - 1].extend = ExtendMode::Both;
        s.chart.select_only(1);
        s.chart.template_name = "biru".into();
        assert!(s.chart.save_template("biru").is_ok());
        s.chart.template_sel = Some("biru".into());
        s.chart.set_overlay_settings(sokanlab_ui::OverlaySettings {
            sma: Some(5),
            ema: Some(3),
            vwap: true,
        });
        let mut help = false;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(420.0, 720.0),
            )),
            ..Default::default()
        };
        let mut full = ctx.run_pass(input, |ctx| {
            ctx.root_ui(|ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    side_panel(ui, &mut s);
                    tool_strip(ui, &mut s, &mut help);
                });
            });
        });
        full.textures_delta.clear();
        drop(full);
        assert_eq!(s.chart.drawings.items.len(), 6);
        assert_eq!(s.chart.drawings.items[1].label(), "ch uji");
        assert_eq!(
            s.chart.drawings.items[4].drawing.kind_label(),
            "Disjoint Channel"
        );
        assert_eq!(
            s.chart.drawings.items[5].drawing.kind_label(),
            "Flat Top/Bottom Channel"
        );
        let g = s.chart.drawings.items[2].group;
        assert!(g.is_some(), "grup terpasang");
        assert_eq!(s.chart.drawings.items[3].group, g);
        assert_eq!(s.chart.overlay_lines.len(), 3, "overlay terhitung");
        assert_eq!(s.chart.templates.len(), 1, "template tersimpan dari panel");
    }

    #[test]
    fn panel_properti_batch5_level_fib_varian_dan_gaya_default() {
        let ctx = egui::Context::default();
        let mut s = AppState::new();
        s.chart.load(bars(30));
        let pf = Drawing::Pitchfork {
            a: Anchor {
                ts_millis: 1_000,
                price_ticks: 100,
            },
            b: Anchor {
                ts_millis: 300_000,
                price_ticks: 120,
            },
            c: Anchor {
                ts_millis: 600_000,
                price_ticks: 110,
            },
        };
        s.chart.add_drawing(pf.clone());
        s.chart.drawings.items[0].pitchfork = PitchforkKind::ModifiedSchiff;
        s.chart.drawings.items[0].opacity = 35;
        s.chart.add_drawing(Drawing::FibRetracement {
            a: Anchor {
                ts_millis: 1_000,
                price_ticks: 100,
            },
            b: Anchor {
                ts_millis: 900_000,
                price_ticks: 140,
            },
        });
        s.chart.drawings.items[1].fib_levels = vec![
            FibLevel {
                milli: 500,
                color: Some([0xf2, 0xb0, 0x4d]),
            },
            FibLevel {
                milli: 425,
                color: None,
            },
            FibLevel {
                milli: 618,
                color: None,
            },
        ];
        // Render panel untuk kedua jenis objek: varian pitchfork + editor level fib.
        for sel in [0usize, 1usize] {
            s.chart.selected = Some(sel);
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(420.0, 720.0),
                )),
                ..Default::default()
            };
            let mut full = ctx.run_pass(input, |ctx| {
                ctx.root_ui(|ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        side_panel(ui, &mut s);
                    });
                });
            });
            full.textures_delta.clear();
            drop(full);
        }
        assert_eq!(s.chart.drawings.items[1].fib_levels.len(), 3, "level utuh");
        assert_eq!(s.chart.drawings.items[0].opacity, 35);
        // Jalur fungsional tombol "Jadikan default" (dipanggil setara klik):
        s.chart.selected = Some(0);
        let st = ItemStyle {
            color: [0x33, 0x99, 0xcc],
            width: 3,
            dash: DashStyle::Dashed,
        };
        s.chart.drawings.items[0].style = Some(st);
        assert!(s.chart.set_default_style_from_selected());
        s.chart.add_drawing(pf.clone());
        assert_eq!(
            s.chart.drawings.items[2].style,
            Some(st),
            "objek baru mewarisi gaya default"
        );
        // Tombol "Terapkan ke sejenis": dua pitchfork disamakan (degree sama).
        s.chart.selected = Some(0);
        assert_eq!(s.chart.apply_style_to_same_kind(), 2);
        assert_eq!(s.chart.drawings.items[2].style, Some(st));
        // Siklus warna level fib: auto → palet → ... → auto.
        let mut c = next_fib_color(None);
        assert_eq!(c, Some(ITEM_PALETTE[0].0), "auto → warna palet pertama");
        for _ in 0..(ITEM_PALETTE.len() - 1) {
            c = next_fib_color(c);
        }
        assert_eq!(c, Some(ITEM_PALETTE[ITEM_PALETTE.len() - 1].0));
        assert_eq!(next_fib_color(c), None, "kembali ke warna otomatis");
    }

    #[test]
    fn dobel_klik_di_sumbu_harga_me_refit_skala() {
        let (ctx, mut chart, size) = setup();
        chart.viewport.p_min = -10_000;
        chart.viewport.p_max = 10_000;
        let p = egui::pos2(chart.plot_rect.max.x + 10.0, chart.plot_rect.center().y);
        pass(&ctx, &mut chart, vec![move_to(p)], size);
        pass(&ctx, &mut chart, vec![press(p)], size);
        pass(&ctx, &mut chart, vec![release(p)], size);
        pass(&ctx, &mut chart, vec![press(p)], size);
        pass(&ctx, &mut chart, vec![release(p)], size);
        let span = chart.viewport.p_max - chart.viewport.p_min;
        assert!(span < 1_000, "skala harga di-fit ulang, span={span}");
    }

    #[test]
    fn drag_sumbu_mengubah_skala_harga_dan_waktu() {
        let (ctx, mut chart, size) = setup();
        // Sumbu harga: drag ke bawah = rentang harga membesar.
        // Batch 4: strip sumbu kanan berada di luar pane harga (plot_rect).
        let axis_x = chart.plot_rect.max.x + 10.0;
        let y0 = chart.plot_rect.min.y + 40.0;
        let p0 = egui::pos2(axis_x, y0);
        let span0 = chart.viewport.p_max - chart.viewport.p_min;
        pass(&ctx, &mut chart, vec![move_to(p0)], size);
        pass(&ctx, &mut chart, vec![press(p0)], size);
        let p1 = egui::pos2(axis_x, y0 + 60.0);
        pass(&ctx, &mut chart, vec![move_to(p1)], size);
        pass(&ctx, &mut chart, vec![move_to(p1)], size);
        pass(&ctx, &mut chart, vec![release(p1)], size);
        let span1 = chart.viewport.p_max - chart.viewport.p_min;
        assert!(span1 > span0, "drag bawah = zoom out ({span0} -> {span1})");
        assert!(matches!(&chart.gesture, Gesture::Idle));
        // Sumbu waktu: drag ke kanan = rentang waktu mengecil.
        let t_span0 = chart.viewport.t_max - chart.viewport.t_min;
        let t0 = egui::pos2(300.0, size.y - 8.0);
        pass(&ctx, &mut chart, vec![move_to(t0)], size);
        pass(&ctx, &mut chart, vec![press(t0)], size);
        let t1 = egui::pos2(380.0, size.y - 8.0);
        pass(&ctx, &mut chart, vec![move_to(t1)], size);
        pass(&ctx, &mut chart, vec![move_to(t1)], size);
        pass(&ctx, &mut chart, vec![release(t1)], size);
        let t_span1 = chart.viewport.t_max - chart.viewport.t_min;
        assert!(
            t_span1 < t_span0,
            "drag kanan = zoom in ({t_span0} -> {t_span1})"
        );
    }

    #[test]
    fn drag_badan_objek_bergrup_menggeser_keduanya() {
        let (ctx, mut chart, size) = setup();
        chart.add_drawing(Drawing::HorizontalLine { price_ticks: 100 });
        chart.add_drawing(Drawing::HorizontalLine { price_ticks: 104 });
        chart.selected = Some(0);
        assert!(chart.toggle_group_selected());
        let g = chart.selected_group().unwrap();
        chart.drawings.items[1].group = Some(g);
        chart.selected = Some(0);
        let rect = chart.plot_rect;
        let mid = egui::pos2(
            rect.min.x + 200.0,
            rect.min.y + chart.viewport.y_for_price(100) as f32,
        );
        pass(&ctx, &mut chart, vec![move_to(mid)], size);
        pass(&ctx, &mut chart, vec![press(mid)], size);
        let moved = egui::pos2(mid.x + 40.0, mid.y + 24.0);
        pass(&ctx, &mut chart, vec![move_to(moved)], size);
        pass(&ctx, &mut chart, vec![move_to(moved)], size);
        pass(&ctx, &mut chart, vec![release(moved)], size);
        let price_of = |it: &sokanlab_chart::drawing::DrawingItem| match &it.drawing {
            Drawing::HorizontalLine { price_ticks } => *price_ticks,
            other => panic!("jenis salah: {other:?}"),
        };
        let h0 = price_of(&chart.drawings.items[0]);
        let h1 = price_of(&chart.drawings.items[1]);
        assert!(h0 < 100, "objek 0 ikut bergeser turun: {h0}");
        assert_eq!(h1 - h0, 4, "jarak antar anggota grup tetap");
    }

    #[test]
    fn pane_indikator_aktif_dan_sumbu_terpisah() {
        let (ctx, mut chart, size) = setup();
        chart.set_panes(vec![
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
        pass(
            &ctx,
            &mut chart,
            vec![move_to(egui::pos2(200.0, 200.0))],
            size,
        );
        assert_eq!(chart.panes.len(), 2, "RSI + MACD dihitung");
        // Strip sumbu kanan & bawah bukan bagian area plot (batch 4).
        assert!(
            chart.plot_rect.max.x < chart.area_rect.max.x - 40.0,
            "strip sumbu kanan terpisah"
        );
        assert!(
            chart.plot_rect.max.y < chart.area_rect.max.y - 100.0,
            "volume + 2 pane menyusutkan pane harga"
        );
        // Klik di pane indikator bukan drag skala harga.
        let p_pane = egui::pos2(chart.plot_rect.center().x, chart.area_rect.max.y - 40.0);
        pass(&ctx, &mut chart, vec![move_to(p_pane), press(p_pane)], size);
        assert!(
            !matches!(&chart.gesture, Gesture::ScalePrice { .. }),
            "pane indikator bukan sumbu harga"
        );
        pass(&ctx, &mut chart, vec![release(p_pane)], size);
        // Klik di strip sumbu kanan = drag skala harga.
        let p_axis = egui::pos2(chart.plot_rect.max.x + 10.0, chart.plot_rect.center().y);
        pass(&ctx, &mut chart, vec![move_to(p_axis), press(p_axis)], size);
        assert!(
            matches!(&chart.gesture, Gesture::ScalePrice { .. }),
            "strip kanan = sumbu harga"
        );
        pass(&ctx, &mut chart, vec![release(p_axis)], size);
    }

    #[test]
    fn snapshot_png_tertulis_dan_valid() {
        let (ctx, mut chart, size) = setup();
        let _ = (ctx, size);
        chart.add_drawing(Drawing::Text {
            at: Anchor {
                ts_millis: chart.bars[5].ts_millis,
                price_ticks: 103,
            },
            text: "snap".into(),
        });
        chart.set_overlay_settings(sokanlab_ui::OverlaySettings {
            sma: Some(5),
            ema: None,
            vwap: true,
        });
        // Batch 4: snapshot menyertakan pane indikator + sumbu.
        chart.set_panes(vec![
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
        let dir = std::env::temp_dir().join("sokanlab-snapshot-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("snap.png");
        let tris = crate::app::headless::render_chart_png(&path, &chart, 640, 400).unwrap();
        assert!(tris > 100, "triangle ter-tessellasi: {tris}");
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.len() > 1_000, "png terlalu kecil: {}", bytes.len());
        assert_eq!(
            &bytes[..8],
            &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a],
            "header PNG"
        );
        // IHDR: lebar/tinggi 1× sesuai permintaan.
        assert_eq!(u32::from_be_bytes(bytes[16..20].try_into().unwrap()), 640);
        assert_eq!(u32::from_be_bytes(bytes[20..24].try_into().unwrap()), 400);
        // Skala 2× = raster ulang 2× piksel (bukan upscale file).
        chart.snapshot_scale = 2;
        let path2 = dir.join("snap2x.png");
        crate::app::headless::render_chart_png(&path2, &chart, 640, 400).unwrap();
        let b2 = std::fs::read(&path2).unwrap();
        assert_eq!(u32::from_be_bytes(b2[16..20].try_into().unwrap()), 1280);
        assert_eq!(u32::from_be_bytes(b2[20..24].try_into().unwrap()), 800);
        let _ = std::fs::remove_dir_all(&dir);
    }
    #[test]
    fn seret_membuat_goresan_brush() {
        let (ctx, mut chart, size) = setup();
        chart.tool.select(ToolKind::Brush);
        let p0 = egui::pos2(200.0, 200.0);
        let p1 = egui::pos2(320.0, 240.0);
        let p2 = egui::pos2(430.0, 180.0);
        pass(&ctx, &mut chart, vec![move_to(p0)], size);
        pass(&ctx, &mut chart, vec![press(p0)], size);
        pass(&ctx, &mut chart, vec![move_to(p1)], size);
        pass(&ctx, &mut chart, vec![move_to(p2)], size);
        pass(&ctx, &mut chart, vec![release(p2)], size);
        assert_eq!(chart.drawings.items.len(), 1, "satu goresan");
        match &chart.drawings.items[0].drawing {
            Drawing::Brush { points } => {
                assert!(points.len() >= 3, "titik tersampel: {}", points.len());
                assert!(points[0].ts_millis < points[1].ts_millis);
            }
            other => panic!("jenis salah: {other:?}"),
        }
        assert_eq!(chart.tool.kind, ToolKind::Select, "kembali ke Select");
        assert!(chart.tool.pending.is_empty());
        // Highlighter lewat jalur yang sama.
        chart.tool.select(ToolKind::Highlighter);
        pass(&ctx, &mut chart, vec![move_to(p0)], size);
        pass(&ctx, &mut chart, vec![press(p0)], size);
        pass(&ctx, &mut chart, vec![move_to(p2)], size);
        pass(&ctx, &mut chart, vec![release(p2)], size);
        assert!(matches!(
            chart.drawings.items[1].drawing,
            Drawing::Highlighter { .. }
        ));
    }

    #[test]
    fn ctrl_seret_pada_objek_mengklon_lalu_menggeser_klon() {
        let (ctx, mut chart, size) = setup();
        let a = Anchor {
            ts_millis: chart.bars[10].ts_millis,
            price_ticks: 100,
        };
        let b = Anchor {
            ts_millis: chart.bars[30].ts_millis,
            price_ticks: 106,
        };
        chart.add_drawing(Drawing::Trendline { a, b });
        let rect = chart.plot_rect;
        let mid = egui::pos2(
            rect.min.x + chart.viewport.x_for_time((a.ts_millis + b.ts_millis) / 2) as f32,
            rect.min.y + chart.viewport.y_for_price(103) as f32,
        );
        let before = chart.drawings.items[0].drawing.clone();
        seret_ctrl(
            &ctx,
            &mut chart,
            mid,
            egui::pos2(mid.x + 80.0, mid.y + 30.0),
            size,
        );
        assert_eq!(chart.drawings.items.len(), 2, "Ctrl+seret = klon");
        assert_eq!(chart.drawings.items[0].drawing, before, "asli tidak geser");
        assert_ne!(chart.drawings.items[1].drawing, before, "klon bergeser");
        assert!(chart.drawings.items[1].group.is_none(), "klon tanpa grup");
        assert_eq!(chart.selected, Some(1), "klon jadi objek primer");
    }

    #[test]
    fn klik_ctrl_tanpa_seret_tetap_toggle_seleksi() {
        let (ctx, mut chart, size) = setup();
        let a = Anchor {
            ts_millis: chart.bars[10].ts_millis,
            price_ticks: 100,
        };
        let b = Anchor {
            ts_millis: chart.bars[30].ts_millis,
            price_ticks: 106,
        };
        chart.add_drawing(Drawing::Trendline { a, b });
        chart.add_drawing(Drawing::HorizontalLine { price_ticks: 102 });
        chart.select_only(1);
        let rect = chart.plot_rect;
        let mid = egui::pos2(
            rect.min.x + chart.viewport.x_for_time((a.ts_millis + b.ts_millis) / 2) as f32,
            rect.min.y + chart.viewport.y_for_price(103) as f32,
        );
        klik_ctrl(&ctx, &mut chart, mid, size);
        assert_eq!(chart.drawings.items.len(), 2, "tidak ada klon tanpa seret");
        assert_eq!(chart.selection(), vec![0, 1], "Masuk seleksi");
        klik_ctrl(&ctx, &mut chart, mid, size);
        assert_eq!(chart.selection(), vec![1], "keluar dari seleksi");
    }

    #[test]
    fn panel_menampilkan_properti_alat_batch8() {
        let ctx = egui::Context::default();
        let mut s = AppState::new();
        s.chart = {
            let mut c = ChartState::new();
            c.load(bars(40));
            c
        };
        let mk = |x: i64| Anchor {
            ts_millis: s.chart.bars[x as usize].ts_millis,
            price_ticks: 100 + x,
        };
        let items = vec![
            Drawing::RegressionChannel {
                a: mk(5),
                b: mk(30),
            },
            Drawing::FibTrendExtension {
                a: mk(2),
                b: mk(10),
                c: mk(16),
            },
            Drawing::ElliottImpulse {
                points: vec![mk(1), mk(4), mk(7), mk(10), mk(13)],
            },
            Drawing::ElliottCorrection {
                points: vec![mk(1), mk(4), mk(7)],
            },
            Drawing::Xabcd {
                points: vec![mk(1), mk(4), mk(7), mk(10), mk(13)],
            },
            Drawing::Abcd {
                points: vec![mk(1), mk(4), mk(7), mk(10)],
            },
            Drawing::GannFan { a: mk(3), b: mk(9) },
            Drawing::Brush {
                points: vec![mk(1), mk(3), mk(6)],
            },
            Drawing::Highlighter {
                points: vec![mk(1), mk(3), mk(6)],
            },
            Drawing::Sticker {
                at: mk(8),
                icon: StickerIcon::Star,
            },
            Drawing::Text {
                at: mk(12),
                text: "catatan".into(),
            },
        ];
        for d in items {
            s.chart.add_drawing(d);
        }
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(420.0, 720.0),
            )),
            ..Default::default()
        };
        // Render panel + strip untuk tiap objek terpilih (semua jalur properti).
        let mut help = false;
        for i in 0..s.chart.drawings.items.len() {
            s.chart.select_only(i);
            let mut full = ctx.run_pass(input.clone(), |ctx| {
                ctx.root_ui(|ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        side_panel(ui, &mut s);
                        tool_strip(ui, &mut s, &mut help);
                    });
                });
            });
            full.textures_delta.clear();
            drop(full);
        }
        // Properti teks: ukuran & perataan bisa diubah (meta item).
        if let Some(it) = s.chart.drawings.items.last_mut() {
            it.text_size = TextSize::L;
            it.text_align = TextAlign::Center;
        }
        let j = s.chart.layout_json();
        assert!(j.contains("\"font\":\"l\""), "font tersimpan: {j}");
        assert!(j.contains("\"align\":\"center\""));
        assert_eq!(s.chart.drawings.items.len(), 11);
    }
}
