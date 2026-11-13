use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;

mod editor;
mod sounds;

pub use sounds::Sound;

pub const BPM_MIN: i32 = 20;
pub const BPM_MAX: i32 = 260;

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Meter {
    #[name = "2/4"]
    M2_4,
    #[name = "3/4"]
    M3_4,
    #[name = "4/4"]
    M4_4,
    #[name = "5/4"]
    M5_4,
    #[name = "6/8"]
    M6_8,
    #[name = "7/8"]
    M7_8,
}

impl Meter {
    pub fn num(self) -> i32 {
        match self {
            Meter::M2_4 => 2,
            Meter::M3_4 => 3,
            Meter::M4_4 => 4,
            Meter::M5_4 => 5,
            Meter::M6_8 => 6,
            Meter::M7_8 => 7,
        }
    }
    pub fn den(self) -> i32 {
        match self {
            Meter::M6_8 | Meter::M7_8 => 8,
            _ => 4,
        }
    }
}

#[derive(Params)]
pub struct MetroParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    #[id = "bpm"]
    pub bpm: IntParam,
    #[id = "meter"]
    pub meter: EnumParam<Meter>,
    #[id = "sound"]
    pub sound: EnumParam<Sound>,
    #[id = "accent"]
    pub accent: BoolParam,
    /// Na którą miarę (1..=7) pada akcent; przy krótszym metrum przycinane do ostatniej.
    #[id = "accbeat"]
    pub accent_beat: IntParam,
    #[id = "play"]
    pub playing: BoolParam,
    #[id = "sync"]
    pub sync: BoolParam,
    #[id = "thru"]
    pub pass_input: BoolParam,
    #[id = "gain"]
    pub gain: FloatParam,
}

impl MetroParams {
    /// Indeks (od 0) akcentowanej miary dla metrum o `num` miarach.
    pub fn accent_index(&self, num: i32) -> i32 {
        self.accent_beat.value().min(num) - 1
    }
}

impl Default for MetroParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(editor::WIDTH, editor::HEIGHT),
            bpm: IntParam::new("Tempo", 100, IntRange::Linear { min: BPM_MIN, max: BPM_MAX })
                .with_unit(" BPM"),
            meter: EnumParam::new("Metrum", Meter::M4_4),
            sound: EnumParam::new("Dźwięk", Sound::Wood),
            accent: BoolParam::new("Akcent", true),
            accent_beat: IntParam::new("Akcent na mierze", 1, IntRange::Linear { min: 1, max: 7 }),
            playing: BoolParam::new("Start", false),
            sync: BoolParam::new("Synchronizacja z hostem", false),
            pass_input: BoolParam::new("Przepuść wejście", true),
            gain: FloatParam::new(
                "Głośność",
                util::db_to_gain(-6.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-40.0),
                    max: util::db_to_gain(6.0),
                    factor: FloatRange::gain_skew_factor(-40.0, 6.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(30.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
        }
    }
}

/// Stan współdzielony z GUI (wskaźnik bieżącej miary).
#[derive(Default)]
pub struct Shared {
    /// Numer bieżącego uderzenia w takcie, -1 gdy stoi.
    pub beat: AtomicI32,
    pub running: AtomicBool,
    /// Tempo hosta * 100 (gdy synchronizacja).
    pub host_tempo_x100: AtomicI32,
}

pub struct Metro {
    params: Arc<MetroParams>,
    shared: Arc<Shared>,
    sr: f32,
    bank: sounds::SampleBank,

    // tryb swobodny
    phase: f64,
    beat: i32,
    was_playing: bool,
    // tryb synchronizacji
    last_idx: Option<i64>,

    // aktywny głos
    voice: Option<(usize, usize)>, // (indeks dźwięku, akcent?)
    voice_pos: usize,
}

impl Default for Metro {
    fn default() -> Self {
        let shared = Arc::new(Shared::default());
        shared.beat.store(-1, Ordering::Relaxed);
        Self {
            params: Arc::new(MetroParams::default()),
            shared,
            sr: 48000.0,
            bank: Vec::new(),
            phase: 0.0,
            beat: -1,
            was_playing: false,
            last_idx: None,
            voice: None,
            voice_pos: 0,
        }
    }
}

impl Metro {
    fn trigger(&mut self, accent: bool) {
        let sound = self.params.sound.value() as usize;
        let acc = (accent && self.params.accent.value()) as usize;
        self.voice = Some((sound, acc));
        self.voice_pos = 0;
    }

    fn next_voice_sample(&mut self) -> f32 {
        if let Some((s, a)) = self.voice {
            let buf = &self.bank[s][a];
            if self.voice_pos < buf.len() {
                let v = buf[self.voice_pos];
                self.voice_pos += 1;
                return v;
            }
            self.voice = None;
        }
        0.0
    }
}

impl Plugin for Metro {
    const NAME: &'static str = "Metro";
    const VENDOR: &'static str = "greblus";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.shared.clone())
    }

    fn initialize(
        &mut self,
        _layout: &AudioIOLayout,
        cfg: &BufferConfig,
        _ctx: &mut impl InitContext<Self>,
    ) -> bool {
        self.sr = cfg.sample_rate;
        self.bank = sounds::build_bank(self.sr);
        true
    }

    fn reset(&mut self) {
        self.voice = None;
        self.last_idx = None;
        self.was_playing = false;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let meter = self.params.meter.value();
        let num = meter.num();
        let acc_idx = self.params.accent_index(num);
        let pass = self.params.pass_input.value();
        let sync = self.params.sync.value();
        let sr = self.sr as f64;

        // Przygotowanie trybu
        let transport = context.transport();
        let mut sync_state: Option<(f64, f64, f64)> = None; // (u, inc, offset taktu)
        let running;

        if sync {
            self.was_playing = false;
            match (transport.playing, transport.tempo, transport.pos_beats()) {
                (true, Some(tempo), Some(pos_q)) => {
                    let upq = meter.den() as f64 / 4.0; // jednostek metrum na ćwierćnutę
                    let offset = transport.bar_start_pos_beats().unwrap_or(0.0) * upq;
                    sync_state = Some((pos_q * upq, tempo / 60.0 / sr * upq, offset));
                    self.shared
                        .host_tempo_x100
                        .store((tempo * 100.0) as i32, Ordering::Relaxed);
                    running = true;
                }
                _ => {
                    if let Some(t) = transport.tempo {
                        self.shared
                            .host_tempo_x100
                            .store((t * 100.0) as i32, Ordering::Relaxed);
                    }
                    self.last_idx = None;
                    running = false;
                }
            }
        } else {
            self.last_idx = None;
            let playing = self.params.playing.value();
            if playing && !self.was_playing {
                self.phase = 1.0; // pierwszy klik od razu
                self.beat = -1;
            }
            self.was_playing = playing;
            running = playing;
        }

        let inc_free = self.params.bpm.value() as f64 / 60.0 / sr;

        for (i, channel_samples) in buffer.iter_samples().enumerate() {
            if running {
                if let Some((u0, inc, offset)) = sync_state {
                    let u = u0 + i as f64 * inc;
                    let idx = (u + 1e-9).floor() as i64;
                    if self.last_idx != Some(idx) {
                        // tylko jeśli faktycznie przeszliśmy przez granicę (nie przy skoku w środek)
                        if u - idx as f64 <= inc + 1e-9 {
                            let in_bar = (idx - offset.round() as i64).rem_euclid(num as i64);
                            self.beat = in_bar as i32;
                            self.trigger(in_bar as i32 == acc_idx);
                        }
                        self.last_idx = Some(idx);
                    }
                } else {
                    if self.phase >= 1.0 {
                        self.phase -= 1.0;
                        self.beat = (self.beat + 1).rem_euclid(num);
                        self.trigger(self.beat == acc_idx);
                    }
                    self.phase += inc_free;
                }
            }

            let gain = self.params.gain.smoothed.next();
            let click = self.next_voice_sample() * gain;
            for s in channel_samples {
                *s = if pass { *s + click } else { click };
            }
        }

        if !running {
            self.beat = -1;
        }
        self.shared.beat.store(self.beat, Ordering::Relaxed);
        self.shared.running.store(running, Ordering::Relaxed);

        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for Metro {
    const CLAP_ID: &'static str = "pl.greblus.metro";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Metronom z akcentem i wyborem dźwięku");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] =
        &[ClapFeature::AudioEffect, ClapFeature::Utility, ClapFeature::Stereo, ClapFeature::Mono];
}

impl Vst3Plugin for Metro {
    const VST3_CLASS_ID: [u8; 16] = *b"GreblusMetroVst3";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_clap!(Metro);
nih_export_vst3!(Metro);
