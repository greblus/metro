use crate::{Meter, MetroParams, Shared, Sound, BPM_MAX, BPM_MIN};
use nih_plug::prelude::*;
use nih_plug_egui::egui::{
    self, Align2, CentralPanel, Color32, Context, CursorIcon, FontId, Frame, Painter, Pos2, Rect,
    Response, Sense, Stroke, Ui, Vec2,
};
use nih_plug_egui::create_egui_editor;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const WIDTH: u32 = 560;
pub const HEIGHT: u32 = 520;

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
}

pub fn create(params: Arc<MetroParams>, shared: Arc<Shared>) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        // METRO_SHOW_MENU=1 otwiera panel opcji od razu (np. do zrzutów ekranu)
        UiState {
            menu_open: std::env::var_os("METRO_SHOW_MENU").is_some(),
            ..Default::default()
        },
        |ctx, _| {
            let mut style = (*ctx.style()).clone();
            style.visuals = egui::Visuals::dark();
            ctx.set_style(style);
        },
        move |ctx, setter, st| {
            CentralPanel::default()
                .frame(Frame::NONE.fill(BG))
                .show(ctx, |ui| draw(ui, ctx, setter, st, &params, &shared));
            // wskaźnik uderzeń musi się odświeżać
            ctx.request_repaint_after(Duration::from_millis(16));
        },
    )
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
    ui.interact(rect, ui.id().with(id), Sense::click())
}

fn draw(
    ui: &mut Ui,
    ctx: &Context,
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

    if st.menu_open {
        draw_settings(ui, &painter, card, setter, params);
    } else {
        draw_tempo_card(ui, &painter, card, setter, params, shared);
    }

    // ---------- Dolny rząd ----------
    let row_y = o.y + 420.0;
    let sync = params.sync.value();

    // Stuknij (tap tempo)
    let tap_c = Pos2::new(cx - 112.0, row_y);
    let tap = circle_button(ui, "tap", tap_c, 28.0);
    painter.circle_filled(tap_c, 28.0, hover_fill(&tap, BTN, BTN_HOVER));
    painter.text(tap_c, Align2::CENTER_CENTER, "Stuknij", FontId::proportional(14.0), TEXT);
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
            "sterowane przez transport hosta",
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
    }

    let _ = ctx;
}

fn draw_tempo_card(
    ui: &mut Ui,
    painter: &Painter,
    card: Rect,
    setter: &ParamSetter,
    params: &MetroParams,
    shared: &Shared,
) {
    let sync = params.sync.value();
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
            "TEMPO HOSTA",
            FontId::proportional(11.0),
            DIM,
        );
    }

    // kółko myszy nad kartą zmienia tempo
    let card_resp = ui.interact(card, ui.id().with("card"), Sense::hover());
    if card_resp.hovered() && !sync {
        let dy = ui.input(|i| i.raw_scroll_delta.y);
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
            ui.id().with(("dot", i)),
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
    let r = ui.interact(hit, ui.id().with(id), sense);

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
    let r = ui.interact(sw.expand(6.0), ui.id().with(id), Sense::click());
    let bg = if on { BLUE } else { hover_fill(&r, TRACK, BTN_SEL) };
    painter.rect_filled(sw, 12.0, bg);
    let kx = if on { sw.max.x - 12.0 } else { sw.min.x + 12.0 };
    painter.circle_filled(Pos2::new(kx, y), 9.0, Color32::WHITE);
    r.clicked()
}

fn draw_settings(ui: &mut Ui, painter: &Painter, card: Rect, setter: &ParamSetter, params: &MetroParams) {
    let x_l = card.min.x + 26.0;
    painter.text(
        Pos2::new(x_l, card.min.y + 22.0),
        Align2::LEFT_CENTER,
        "Dźwięk",
        FontId::proportional(13.0),
        DIM,
    );

    // przyciski dźwięków
    let n = Sound::variants().len();
    let avail = card.width() - 52.0;
    let gap = 6.0;
    let cw = (avail - gap * (n as f32 - 1.0)) / n as f32;
    let cur = params.sound.value();
    for i in 0..n {
        let s = Sound::from_index(i);
        let r = Rect::from_min_size(
            Pos2::new(x_l + i as f32 * (cw + gap), card.min.y + 36.0),
            Vec2::new(cw, 30.0),
        );
        let resp = ui.interact(r, ui.id().with(("snd", i)), Sense::click());
        let fill = if s == cur { BLUE } else { hover_fill(&resp, BTN, BTN_HOVER) };
        painter.rect_filled(r, 15.0, fill);
        painter.text(r.center(), Align2::CENTER_CENTER, Sound::variants()[i], FontId::proportional(14.0), TEXT);
        if resp.clicked() {
            set(setter, &params.sound, s);
        }
    }

    let y0 = card.min.y + 92.0;
    // Akcent: [wył.] [1] [2] ... [N]
    let accent = params.accent.value();
    painter.text(Pos2::new(x_l, y0), Align2::LEFT_CENTER, "Akcent", FontId::proportional(15.0), TEXT);
    let off_r = Rect::from_center_size(Pos2::new(x_l + 92.0, y0), Vec2::new(46.0, 24.0));
    let off = ui.interact(off_r, ui.id().with("acc_off"), Sense::click());
    let fill = if !accent { BLUE } else { hover_fill(&off, BTN, BTN_HOVER) };
    painter.rect_filled(off_r, 12.0, fill);
    painter.text(off_r.center(), Align2::CENTER_CENTER, "wył.", FontId::proportional(13.0), TEXT);
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
    if toggle(ui, painter, "sync", "Synchronizuj z transportem hosta", y0 + 34.0, card, params.sync.value()) {
        set(setter, &params.sync, !params.sync.value());
    }
    if toggle(ui, painter, "thru", "Przepuszczaj sygnał wejściowy", y0 + 68.0, card, params.pass_input.value()) {
        set(setter, &params.pass_input, !params.pass_input.value());
    }

    // głośność
    let y = y0 + 108.0;
    painter.text(Pos2::new(x_l, y), Align2::LEFT_CENTER, "Głośność", FontId::proportional(15.0), TEXT);
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
