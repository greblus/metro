use crate::i18n::tr;
use crate::{BPM_MAX, BPM_MIN, Meter, MetroParams, Shared, Sound};
use nice_plug::context::gui::GuiContext;
use nice_plug::editor::dpi::LogicalSize;
use nice_plug::prelude::*;
use egui::{
    self, Align2, Color32, CursorIcon, FontId, Painter, Pos2, Rect, Response, Sense, Stroke, Ui,
    Vec2,
};
use nice_plug_egui::{
    EguiEditor, EguiEditorState, EguiNiceSettings, NiceEguiApp, RepaintNotifier,
    create_egui_editor,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

pub const WIDTH: f32 = 560.0;
pub const HEIGHT: f32 = 520.0;

const BG: Color32 = Color32::from_rgb(0x24, 0x23, 0x28);
const CARD: Color32 = Color32::from_rgb(0x35, 0x34, 0x39);
const BTN: Color32 = Color32::from_rgb(0x3b, 0x3a, 0x40);
const BTN_HOVER: Color32 = Color32::from_rgb(0x48, 0x47, 0x4e);
const BTN_SEL: Color32 = Color32::from_rgb(0x6c, 0x6b, 0x70);
const TRACK: Color32 = Color32::from_rgb(0x55, 0x54, 0x5a);
const BLUE: Color32 = Color32::from_rgb(0x35, 0x84, 0xe4);
const BLUE_HOVER: Color32 = Color32::from_rgb(0x4a, 0x93, 0xec);
const ACCENT: Color32 = Color32::from_rgb(0xff, 0x9f, 0x43);
const TEXT: Color32 = Color32::from_rgb(0xf2, 0xf2, 0xf2);
const DIM: Color32 = Color32::from_rgb(0xa0, 0xa0, 0xa8);

#[derive(Default)]
struct UiState {
    taps: Vec<Instant>,
    menu_open: bool,
    /// Wersja samodzielna: ukrywa opcje, które mają sens tylko w DAW.
    standalone: bool,
    out: OutputUi,
}

/// Wybór wyjścia audio w wersji samodzielnej (pod PipeWire, patrz `audio_out`).
#[derive(Default)]
struct OutputUi {
    /// 0 = jeszcze nie wiadomo, 1 = można wybierać, 2 = niedostępne (np. backend JACK).
    available: Arc<AtomicU8>,
    /// `node.name` wybranego wyjścia; `None` = domyślne systemowe.
    choice: Option<String>,
    /// (`node.name`, czytelna nazwa)
    sinks: Vec<(String, String)>,
}

/// Po starcie: przywraca zapisane wyjście i sprawdza, czy wybór jest w ogóle możliwy.
#[cfg(feature = "standalone")]
fn out_start(out: &mut OutputUi) {
    out.choice = crate::audio_out::load_choice();
    out_refresh(out);
    let choice = out.choice.clone();
    let available = out.available.clone();
    std::thread::spawn(move || {
        let ok = crate::audio_out::apply(choice.as_deref());
        available.store(if ok { 1 } else { 2 }, Ordering::Relaxed);
    });
}
#[cfg(not(feature = "standalone"))]
fn out_start(_: &mut OutputUi) {}

/// Odświeża listę wyjść (przy otwarciu panelu opcji).
#[cfg(feature = "standalone")]
fn out_refresh(out: &mut OutputUi) {
    out.sinks = crate::audio_out::list_sinks()
        .into_iter()
        .map(|s| (s.name, s.description))
        .collect();
}
#[cfg(not(feature = "standalone"))]
fn out_refresh(_: &mut OutputUi) {}

/// Zapisuje wybór i przełącza strumień (w tle, żeby nie blokować okna).
#[cfg(feature = "standalone")]
fn out_select(out: &mut OutputUi, choice: Option<String>) {
    crate::audio_out::save_choice(choice.as_deref());
    out.choice = choice.clone();
    std::thread::spawn(move || {
        crate::audio_out::apply(choice.as_deref());
    });
}
#[cfg(not(feature = "standalone"))]
fn out_select(_: &mut OutputUi, _: Option<String>) {}

/// Stan okna (stały rozmiar, bez skalowania przez użytkownika).
pub fn initial_state() -> Arc<EguiEditorState> {
    EguiEditorState::from_size(LogicalSize::new(WIDTH, HEIGHT), 1.0)
}

pub struct MetroEditor {
    params: Arc<MetroParams>,
    shared: Arc<Shared>,
    st: UiState,
    /// Kontekst GUI wtyczki; istnieje tylko, gdy okno jest otwarte.
    gui: Option<GuiContext>,
}

pub fn create(
    state: Arc<EguiEditorState>,
    repaint: RepaintNotifier,
    params: Arc<MetroParams>,
    shared: Arc<Shared>,
) -> Option<EguiEditor<MetroEditor>> {
    let app = MetroEditor {
        params,
        shared,
        // METRO_SHOW_MENU=1 otwiera panel opcji od razu (np. do zrzutów ekranu)
        st: UiState {
            menu_open: std::env::var_os("METRO_SHOW_MENU").is_some(),
            ..Default::default()
        },
        gui: None,
    };
    create_egui_editor(state, repaint, EguiNiceSettings::new().with_tile("Metro"), app)
}

impl NiceEguiApp for MetroEditor {
    fn build(
        &mut self,
        egui_ctx: egui::Context,
        gui: GuiContext,
        _frame: &mut nice_plug_egui::Frame,
    ) -> Result<(), nice_plug_egui::baseview::HandlerError> {
        egui_ctx.set_visuals(egui::Visuals::dark());
        self.st.standalone = gui.plugin_api() == PluginApi::Standalone;
        if self.st.standalone {
            out_start(&mut self.st.out);
        }
        self.gui = Some(gui);
        Ok(())
    }

    fn editor_closed(&mut self) {
        self.gui = None;
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut nice_plug_egui::Frame) {
        let Some(gui) = self.gui.as_ref() else {
            return;
        };
        let setter = gui.param_setter();
        ui.painter().rect_filled(ui.max_rect(), 0.0, BG);
        draw(ui, &setter, &mut self.st, &self.params, &self.shared);
        // Wątek audio zgłasza zmiany przez RepaintNotifier, ale nie na każdej platformie timer
        // baseview go niezawodnie odpytuje. Dlatego okno samo prosi o kolejne klatki: często,
        // gdy metronom gra (wskaźnik miary) lub trzymany jest przycisk „+/−”, rzadko w spoczynku
        // (np. żeby zauważyć start transportu w DAW).
        let busy = self.shared.running.load(Ordering::Relaxed) || ui.input(|i| i.pointer.any_down());
        ui.ctx()
            .request_repaint_after(Duration::from_millis(if busy { 30 } else { 100 }));
    }
}

fn set<P: Param>(setter: &ParamSetter, p: &P, v: P::Plain) {
    setter.begin_set_parameter(p);
    setter.set_parameter(p, v);
    setter.end_set_parameter(p);
}

fn hover_fill(r: &Response, normal: Color32, hover: Color32) -> Color32 {
    if r.hovered() {
        r.ctx.set_cursor_icon(CursorIcon::PointingHand);
        hover
    } else {
        normal
    }
}

fn circle_button(ui: &mut Ui, id: &str, c: Pos2, r: f32) -> Response {
    let rect = Rect::from_center_size(c, Vec2::splat(r * 2.0));
    ui.interact(rect, ui.scope_id().with(id), Sense::click())
}

fn draw(
    ui: &mut Ui,
    setter: &ParamSetter,
    st: &mut UiState,
    params: &MetroParams,
    shared: &Shared,
) {
    let painter = ui.painter().clone();
    let full = ui.max_rect();
    let o = full.min;
    let w = full.width();
    let cx = o.x + w / 2.0;
    let p = |x: f32, y: f32| Pos2::new(o.x + x, o.y + y);

    // ---------- Metrum ----------
    let meters = Meter::variants().len();
    let spacing = 66.0;
    let start = cx - spacing * (meters as f32 - 1.0) / 2.0;
    let cur_meter = params.meter.value();
    for i in 0..meters {
        let m = Meter::from_index(i);
        let c = Pos2::new(start + i as f32 * spacing, o.y + 48.0);
        let r = circle_button(ui, &format!("meter{i}"), c, 26.0);
        let fill = if m == cur_meter { BTN_SEL } else { hover_fill(&r, BTN, BTN_HOVER) };
        painter.circle_filled(c, 26.0, fill);
        let f = FontId::proportional(16.0);
        painter.text(c - Vec2::new(0.0, 9.0), Align2::CENTER_CENTER, m.num().to_string(), f.clone(), TEXT);
        painter.text(c + Vec2::new(0.0, 9.0), Align2::CENTER_CENTER, m.den().to_string(), f, TEXT);
        if r.clicked() {
            set(setter, &params.meter, m);
        }
    }

    // ---------- Karta ----------
    let card = Rect::from_min_size(p(60.0, 98.0), Vec2::new(w - 120.0, 230.0));
    painter.rect_filled(card, 12.0, CARD);

    // synchronizacja z hostem ma sens tylko w DAW
    let sync = !st.standalone && params.sync.value();
    if st.menu_open {
        draw_settings(ui, &painter, card, setter, params, st.standalone, &mut st.out);
    } else {
        draw_tempo_card(ui, &painter, card, setter, params, shared, sync);
    }

    // ---------- Dolny rząd ----------
    let row_y = o.y + 420.0;

    // Stuknij (tap tempo)
    let tap_c = Pos2::new(cx - 112.0, row_y);
    let tap = circle_button(ui, "tap", tap_c, 28.0);
    painter.circle_filled(tap_c, 28.0, hover_fill(&tap, BTN, BTN_HOVER));
    painter.text(tap_c, Align2::CENTER_CENTER, tr("Tap", "Stuknij"), FontId::proportional(14.0), TEXT);
    if tap.clicked() && !sync {
        let now = Instant::now();
        if st.taps.last().is_some_and(|t| now - *t > Duration::from_secs(2)) {
            st.taps.clear();
        }
        st.taps.push(now);
        if st.taps.len() > 6 {
            st.taps.remove(0);
        }
        if st.taps.len() >= 2 {
            let span = (*st.taps.last().unwrap() - st.taps[0]).as_secs_f64();
            let avg = span / (st.taps.len() - 1) as f64;
            let bpm = (60.0 / avg).round() as i32;
            set(setter, &params.bpm, bpm.clamp(BPM_MIN, BPM_MAX));
        }
    }

    // Start / stop
    let running = shared.running.load(Ordering::Relaxed);
    let play_c = Pos2::new(cx, row_y);
    let play = circle_button(ui, "play", play_c, 46.0);
    painter.circle_stroke(play_c, 48.0, Stroke::new(2.0_f32, Color32::from_rgb(0x3a, 0x3a, 0x44)));
    let fill = if sync {
        Color32::from_rgb(0x2a, 0x5a, 0x9a)
    } else {
        hover_fill(&play, BLUE, BLUE_HOVER)
    };
    painter.circle_filled(play_c, 44.0, fill);
    if running {
        // pauza
        for dx in [-8.0, 8.0] {
            let r = Rect::from_center_size(play_c + Vec2::new(dx, 0.0), Vec2::new(8.0, 26.0));
            painter.rect_filled(r, 2.0, Color32::WHITE);
        }
    } else {
        let a = play_c + Vec2::new(-8.0, -13.0);
        let b = play_c + Vec2::new(-8.0, 13.0);
        let c = play_c + Vec2::new(14.0, 0.0);
        painter.add(egui::Shape::convex_polygon(vec![a, b, c], Color32::WHITE, Stroke::NONE));
    }
    if play.clicked() && !sync {
        set(setter, &params.playing, !params.playing.value());
    }
    if sync {
        painter.text(
            play_c + Vec2::new(0.0, 62.0),
            Align2::CENTER_CENTER,
            tr("controlled by host transport", "sterowane przez transport hosta"),
            FontId::proportional(12.0),
            DIM,
        );
    }

    // Menu
    let menu_c = Pos2::new(cx + 112.0, row_y);
    let menu = circle_button(ui, "menu", menu_c, 26.0);
    let fill = if st.menu_open { BTN_SEL } else { hover_fill(&menu, BTN, BTN_HOVER) };
    painter.circle_filled(menu_c, 26.0, fill);
    for dy in [-5.0, 0.0, 5.0] {
        let y = menu_c.y + dy;
        painter.line_segment(
            [Pos2::new(menu_c.x - 7.0, y), Pos2::new(menu_c.x + 7.0, y)],
            Stroke::new(2.0_f32, TEXT),
        );
    }
    if menu.clicked() {
        st.menu_open = !st.menu_open;
        if st.menu_open && st.standalone {
            out_refresh(&mut st.out);
        }
    }
}

fn draw_tempo_card(
    ui: &mut Ui,
    painter: &Painter,
    card: Rect,
    setter: &ParamSetter,
    params: &MetroParams,
    shared: &Shared,
    sync: bool,
) {
    let bpm = params.bpm.value();
    let num_c = Pos2::new(card.center().x - 20.0, card.min.y + 82.0);

    let shown = if sync {
        let t = shared.host_tempo_x100.load(Ordering::Relaxed) as f32 / 100.0;
        if t > 0.0 { format!("{}", t.round() as i32) } else { "—".into() }
    } else {
        bpm.to_string()
    };
    painter.text(num_c, Align2::CENTER_CENTER, shown, FontId::proportional(110.0), TEXT);
    if sync {
        painter.text(
            num_c + Vec2::new(0.0, -62.0),
            Align2::CENTER_CENTER,
            tr("HOST TEMPO", "TEMPO HOSTA"),
            FontId::proportional(11.0),
            DIM,
        );
    }

    // kółko myszy nad kartą zmienia tempo
    let card_resp = ui.interact(card, ui.scope_id().with("card"), Sense::hover());
    if card_resp.hovered() && !sync {
        let dy: f32 = ui.input(|i| {
            i.raw
                .events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::MouseWheel { delta, .. } => Some(delta.y),
                    _ => None,
                })
                .sum()
        });
        if dy != 0.0 {
            set(setter, &params.bpm, (bpm + dy.signum() as i32).clamp(BPM_MIN, BPM_MAX));
        }
    }

    // + / -
    let pm_x = card.center().x + 115.0;
    for (i, (label, delta)) in [("+", 1), ("−", -1)].into_iter().enumerate() {
        let c = Pos2::new(pm_x, card.min.y + 58.0 + i as f32 * 44.0);
        let r = circle_button(ui, &format!("pm{i}"), c, 18.0);
        painter.circle_filled(c, 18.0, hover_fill(&r, Color32::from_rgb(0x48, 0x47, 0x4d), BTN_SEL));
        painter.text(c, Align2::CENTER_CENTER, label, FontId::proportional(22.0), TEXT);
        if (r.clicked() || r.is_pointer_button_down_on() && r.ctx.input(|i| i.pointer.press_start_time())
            .is_some_and(|t| r.ctx.input(|i| i.time) - t > 0.4))
            && !sync
        {
            set(setter, &params.bpm, (bpm + delta).clamp(BPM_MIN, BPM_MAX));
        }
    }

    // Wskaźnik uderzeń
    let num = params.meter.value().num();
    let beat = shared.beat.load(Ordering::Relaxed);
    let accent = params.accent.value();
    let acc_idx = params.accent_index(num);
    let dot_y = card.min.y + 150.0;
    let gap = 24.0;
    let x0 = card.center().x - gap * (num as f32 - 1.0) / 2.0;
    for i in 0..num {
        let c = Pos2::new(x0 + i as f32 * gap, dot_y);
        // klik w kropkę ustawia na niej akcent (drugi klik w akcentowaną wyłącza akcent)
        let hit = ui.interact(
            Rect::from_center_size(c, Vec2::splat(gap)),
            ui.scope_id().with(("dot", i)),
            Sense::click(),
        );
        if hit.clicked() {
            if accent && i == acc_idx {
                set(setter, &params.accent, false);
            } else {
                set(setter, &params.accent_beat, i + 1);
                if !accent {
                    set(setter, &params.accent, true);
                }
            }
        }
        let is_acc = accent && i == acc_idx;
        let r = if is_acc { 6.5 } else { 4.5 };
        let col = if i == beat {
            if is_acc { ACCENT } else { BLUE }
        } else if is_acc {
            Color32::from_rgb(0x8a, 0x6a, 0x4a)
        } else if hit.hovered() {
            BTN_SEL
        } else {
            TRACK
        };
        if hit.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        painter.circle_filled(c, r, col);
    }

    // Suwak 20–260
    let track = Rect::from_min_max(
        Pos2::new(card.min.x + 34.0, card.min.y + 178.0),
        Pos2::new(card.max.x - 34.0, card.min.y + 182.0),
    );
    let norm = (bpm - BPM_MIN) as f32 / (BPM_MAX - BPM_MIN) as f32;
    let new = h_slider(ui, painter, "bpm_slider", track, norm, !sync, &params.bpm, setter);
    if let Some(n) = new {
        let v = BPM_MIN + (n * (BPM_MAX - BPM_MIN) as f32).round() as i32;
        setter.set_parameter(&params.bpm, v);
    }
    for t in [20, 80, 140, 200, 260] {
        let n = (t - BPM_MIN) as f32 / (BPM_MAX - BPM_MIN) as f32;
        let x = egui::lerp(track.x_range(), n);
        painter.line_segment(
            [Pos2::new(x, track.max.y + 4.0), Pos2::new(x, track.max.y + 8.0)],
            Stroke::new(1.0_f32, DIM),
        );
        painter.text(
            Pos2::new(x, track.max.y + 18.0),
            Align2::CENTER_CENTER,
            t.to_string(),
            FontId::proportional(13.0),
            DIM,
        );
    }
}

/// Poziomy suwak. Zwraca nową wartość znormalizowaną podczas przeciągania.
/// Wywołuje begin/end gesture na parametrze.
#[allow(clippy::too_many_arguments)]
fn h_slider<P: Param>(
    ui: &mut Ui,
    painter: &Painter,
    id: &str,
    track: Rect,
    norm: f32,
    enabled: bool,
    param: &P,
    setter: &ParamSetter,
) -> Option<f32> {
    let hit = track.expand2(Vec2::new(10.0, 12.0));
    let sense = if enabled { Sense::click_and_drag() } else { Sense::hover() };
    let r = ui.interact(hit, ui.scope_id().with(id), sense);

    let knob_x = egui::lerp(track.x_range(), norm.clamp(0.0, 1.0));
    painter.rect_filled(track, 2.0, TRACK);
    let filled = Rect::from_min_max(track.min, Pos2::new(knob_x, track.max.y));
    painter.rect_filled(filled, 2.0, if enabled { BLUE } else { DIM });
    let knob_c = Pos2::new(knob_x, track.center().y);
    let knob_col = if r.hovered() || r.dragged() {
        Color32::WHITE
    } else {
        Color32::from_rgb(0xd8, 0xd8, 0xdc)
    };
    painter.circle_filled(knob_c, 10.0, knob_col);

    let pos_norm = |r: &Response| {
        r.interact_pointer_pos()
            .map(|p| ((p.x - track.min.x) / track.width()).clamp(0.0, 1.0))
    };

    if !enabled {
        return None;
    }
    if r.drag_started() {
        setter.begin_set_parameter(param);
    }
    let mut out = None;
    if r.drag_started() || r.dragged() {
        out = pos_norm(&r);
    }
    if r.drag_stopped() {
        setter.end_set_parameter(param);
    }
    if r.clicked() {
        if let Some(n) = pos_norm(&r) {
            setter.begin_set_parameter(param);
            setter.set_parameter_normalized(param, n);
            setter.end_set_parameter(param);
        }
    }
    out
}

fn toggle(ui: &mut Ui, painter: &Painter, id: &str, label: &str, y: f32, card: Rect, on: bool) -> bool {
    let x_l = card.min.x + 26.0;
    painter.text(Pos2::new(x_l, y), Align2::LEFT_CENTER, label, FontId::proportional(15.0), TEXT);
    let sw = Rect::from_center_size(Pos2::new(card.max.x - 50.0, y), Vec2::new(44.0, 24.0));
    // klikalny tylko przełącznik (w wierszu akcentu są jeszcze inne przyciski)
    let r = ui.interact(sw.expand(6.0), ui.scope_id().with(id), Sense::click());
    let bg = if on { BLUE } else { hover_fill(&r, TRACK, BTN_SEL) };
    painter.rect_filled(sw, 12.0, bg);
    let kx = if on { sw.max.x - 12.0 } else { sw.min.x + 12.0 };
    painter.circle_filled(Pos2::new(kx, y), 9.0, Color32::WHITE);
    r.clicked()
}

fn draw_settings(
    ui: &mut Ui,
    painter: &Painter,
    card: Rect,
    setter: &ParamSetter,
    params: &MetroParams,
    standalone: bool,
    out: &mut OutputUi,
) {
    let x_l = card.min.x + 26.0;
    painter.text(
        Pos2::new(x_l, card.min.y + 22.0),
        Align2::LEFT_CENTER,
        tr("Sound", "Dźwięk"),
        FontId::proportional(13.0),
        DIM,
    );

    // przyciski dźwięków
    let n = Sound::variants().len();
    let avail = card.width() - 52.0;
    let gap = 6.0;
    let cw = (avail - gap * (n as f32 - 1.0)) / n as f32;
    let cur = Sound::from_index(params.sound.value() as usize);
    for i in 0..n {
        let s = Sound::from_index(i);
        let r = Rect::from_min_size(
            Pos2::new(x_l + i as f32 * (cw + gap), card.min.y + 36.0),
            Vec2::new(cw, 30.0),
        );
        let resp = ui.interact(r, ui.scope_id().with(("snd", i)), Sense::click());
        let fill = if s == cur { BLUE } else { hover_fill(&resp, BTN, BTN_HOVER) };
        painter.rect_filled(r, 15.0, fill);
        painter.text(r.center(), Align2::CENTER_CENTER, s.name(), FontId::proportional(14.0), TEXT);
        if resp.clicked() {
            set(setter, &params.sound, i as i32);
        }
    }

    let y0 = card.min.y + 92.0;
    // Akcent: [wył.] [1] [2] ... [N]
    let accent = params.accent.value();
    painter.text(Pos2::new(x_l, y0), Align2::LEFT_CENTER, tr("Accent", "Akcent"), FontId::proportional(15.0), TEXT);
    let off_r = Rect::from_center_size(Pos2::new(x_l + 92.0, y0), Vec2::new(46.0, 24.0));
    let off = ui.interact(off_r, ui.scope_id().with("acc_off"), Sense::click());
    let fill = if !accent { BLUE } else { hover_fill(&off, BTN, BTN_HOVER) };
    painter.rect_filled(off_r, 12.0, fill);
    painter.text(off_r.center(), Align2::CENTER_CENTER, tr("off", "wył."), FontId::proportional(13.0), TEXT);
    if off.clicked() && accent {
        set(setter, &params.accent, false);
    }
    let num = params.meter.value().num();
    let acc_idx = params.accent_index(num);
    for i in 0..num {
        let c = Pos2::new(x_l + 140.0 + i as f32 * 30.0, y0);
        let r = circle_button(ui, &format!("accb{i}"), c, 12.0);
        let fill = if i == acc_idx {
            if accent { ACCENT } else { BTN_SEL }
        } else {
            hover_fill(&r, BTN, BTN_HOVER)
        };
        painter.circle_filled(c, 12.0, fill);
        painter.text(c, Align2::CENTER_CENTER, (i + 1).to_string(), FontId::proportional(13.0), TEXT);
        if r.clicked() {
            set(setter, &params.accent_beat, i + 1);
            if !accent {
                set(setter, &params.accent, true);
            }
        }
    }
    if standalone {
        if out.available.load(Ordering::Relaxed) == 1 {
            output_row(ui, painter, card, y0 + 34.0, out);
        }
        // opcje zależne od DAW nie mają tu zastosowania
        painter.text(
            Pos2::new(x_l, y0 + 70.0),
            Align2::LEFT_CENTER,
            tr(
                "Host sync and track mixing are available in the plugin version.",
                "Synchronizacja i miksowanie ze ścieżką są dostępne w wersji wtyczki.",
            ),
            FontId::proportional(13.0),
            DIM,
        );
    } else {
        if toggle(ui, painter, "sync", tr("Sync to host transport", "Synchronizuj z transportem hosta"), y0 + 34.0, card, params.sync.value()) {
            set(setter, &params.sync, !params.sync.value());
        }
        if toggle(ui, painter, "thru", tr("Mix with track audio", "Miksuj z dźwiękiem ścieżki"), y0 + 68.0, card, params.pass_input.value()) {
            set(setter, &params.pass_input, !params.pass_input.value());
        }
    }

    // głośność
    let y = y0 + 108.0;
    painter.text(Pos2::new(x_l, y), Align2::LEFT_CENTER, tr("Volume", "Głośność"), FontId::proportional(15.0), TEXT);
    let track = Rect::from_min_max(Pos2::new(x_l + 90.0, y - 2.0), Pos2::new(card.max.x - 90.0, y + 2.0));
    let norm = params.gain.unmodulated_normalized_value();
    if let Some(nv) = h_slider(ui, painter, "gain", track, norm, true, &params.gain, setter) {
        setter.set_parameter_normalized(&params.gain, nv);
    }
    painter.text(
        Pos2::new(card.max.x - 26.0, y),
        Align2::RIGHT_CENTER,
        params.gain.to_string(),
        FontId::proportional(13.0),
        DIM,
    );
}

/// Wiersz „Wyjście” z listą wyjść PipeWire (tylko w wersji samodzielnej).
fn output_row(ui: &mut Ui, painter: &Painter, card: Rect, y: f32, out: &mut OutputUi) {
    let x_l = card.min.x + 26.0;
    painter.text(Pos2::new(x_l, y), Align2::LEFT_CENTER, tr("Output", "Wyjście"), FontId::proportional(15.0), TEXT);
    let default_label = tr("System default", "Domyślne systemowe");
    let current = match &out.choice {
        None => default_label.to_string(),
        Some(name) => out
            .sinks
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, d)| d.clone())
            .unwrap_or_else(|| name.clone()),
    };
    let rect = Rect::from_min_max(Pos2::new(x_l + 90.0, y - 12.0), Pos2::new(card.max.x - 26.0, y + 12.0));
    let mut picked: Option<Option<String>> = None;
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        egui::ComboBox::from_id_salt("metro_output")
            .width(rect.width())
            .selected_text(current)
            .show_ui(ui, |ui| {
                if ui.selectable_label(out.choice.is_none(), default_label).clicked() {
                    picked = Some(None);
                }
                for (name, desc) in &out.sinks {
                    if ui.selectable_label(out.choice.as_deref() == Some(name), desc).clicked() {
                        picked = Some(Some(name.clone()));
                    }
                }
            });
    });
    if let Some(choice) = picked {
        if choice != out.choice {
            out_select(out, choice);
        }
    }
}
