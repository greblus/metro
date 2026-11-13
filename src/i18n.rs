//! Minimalna obsługa języków: polski albo angielski, wybierany z locale.
//!
//! Kolejność: `METRO_LANG` (nadpisanie, np. `pl` / `en`), potem `LC_ALL`, `LC_MESSAGES`, `LANG`.
//! Locale zaczynające się od `pl` daje polski, każde inne angielski.

use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Pl,
}

fn detect() -> Lang {
    for key in ["METRO_LANG", "LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(key) {
            let v = v.trim().to_ascii_lowercase();
            if !v.is_empty() {
                return if v.starts_with("pl") { Lang::Pl } else { Lang::En };
            }
        }
    }
    Lang::En
}

pub fn lang() -> Lang {
    static LANG: OnceLock<Lang> = OnceLock::new();
    *LANG.get_or_init(detect)
}

/// Zwraca tekst w bieżącym języku.
pub fn tr(en: &'static str, pl: &'static str) -> &'static str {
    match lang() {
        Lang::En => en,
        Lang::Pl => pl,
    }
}
