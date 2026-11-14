//! Syntezowane próbki kliknięć. Generowane raz w `initialize()` dla danej częstotliwości próbkowania.

use crate::i18n::tr;
use nice_plug::prelude::Enum;
use std::f32::consts::TAU;

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sound {
    Click,
    Wood,
    Beep,
    Cowbell,
    HiHat,
}

impl Sound {
    /// Nazwa w bieżącym języku (GUI i host).
    pub fn name(self) -> &'static str {
        match self {
            Sound::Click => tr("Click", "Klik"),
            Sound::Wood => tr("Wood", "Drewno"),
            Sound::Beep => "Beep",
            Sound::Cowbell => tr("Cowbell", "Dzwonek"),
            Sound::HiHat => "Hi-hat",
        }
    }
}

pub const SOUND_COUNT: usize = 5;

/// `[dźwięk][0 = zwykły, 1 = akcent]`
pub type SampleBank = Vec<[Vec<f32>; 2]>;

pub fn build_bank(sr: f32) -> SampleBank {
    (0..SOUND_COUNT)
        .map(|i| {
            let s = Sound::from_index(i);
            [render(s, false, sr), render(s, true, sr)]
        })
        .collect()
}

fn render(sound: Sound, accent: bool, sr: f32) -> Vec<f32> {
    let len_s = match sound {
        Sound::Click => 0.03,
        Sound::Wood => 0.09,
        Sound::Beep => 0.07,
        Sound::Cowbell => 0.25,
        Sound::HiHat => 0.12,
    };
    let n = (len_s * sr) as usize;
    let amp = if accent { 1.0 } else { 0.7 };
    let mut out = Vec::with_capacity(n);
    let mut rng: u32 = 0x1234_5678;
    let mut prev_noise = 0.0f32;
    let mut lp = 0.0f32;

    for i in 0..n {
        let t = i as f32 / sr;
        // krótki attack, żeby nie trzaskało
        let attack = (t / 0.0008).min(1.0);
        // wygaszanie na końcu bufora
        let tail = ((n - i) as f32 / (0.004 * sr)).min(1.0);

        let v = match sound {
            Sound::Click => {
                let f = if accent { 2500.0 } else { 1600.0 };
                (TAU * f * t).sin() * (-t / 0.006).exp()
            }
            Sound::Wood => {
                let f = if accent { 1250.0 } else { 900.0 };
                let body = (TAU * f * t).sin() + 0.35 * (TAU * f * 2.63 * t).sin();
                0.75 * body * (-t / 0.016).exp()
            }
            Sound::Beep => {
                let f = if accent { 1760.0 } else { 880.0 };
                let env = (1.0 - t / len_s).max(0.0).powf(0.3);
                0.45 * (3.0 * (TAU * f * t).sin()).tanh() * env
            }
            Sound::Cowbell => {
                let k = if accent { 1.25 } else { 1.0 };
                let sq = |f: f32| if (TAU * f * k * t).sin() >= 0.0 { 1.0 } else { -1.0 };
                let raw = sq(540.0) + sq(800.0);
                // prosty filtr dolnoprzepustowy łagodzący prostokąty
                lp += 0.35 * (raw - lp);
                0.3 * lp * (-t / if accent { 0.07 } else { 0.05 }).exp()
            }
            Sound::HiHat => {
                rng ^= rng << 13;
                rng ^= rng >> 17;
                rng ^= rng << 5;
                let noise = (rng as f32 / u32::MAX as f32) * 2.0 - 1.0;
                let hp = noise - prev_noise; // filtr górnoprzepustowy
                prev_noise = noise;
                0.6 * hp * (-t / if accent { 0.035 } else { 0.018 }).exp()
            }
        };
        out.push(v * amp * attack * tail);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn peaks_in_range() {
        for (i, pair) in build_bank(48000.0).iter().enumerate() {
            for (a, b) in pair.iter().enumerate() {
                let peak = b.iter().fold(0.0f32, |m, v| m.max(v.abs()));
                println!("sound {i} accent {a}: len {} peak {peak:.3}", b.len());
                assert!(peak > 0.05 && peak <= 1.0);
            }
        }
    }
}
