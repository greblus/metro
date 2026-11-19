use metro::Metro;
use nice_plug::prelude::*;
use std::path::Path;

/// Czy działa serwer PipeWire bieżącego użytkownika (gniazdo `pipewire-0`).
fn pipewire_running() -> bool {
    std::env::var_os("PIPEWIRE_REMOTE").is_some()
        || std::env::var_os("XDG_RUNTIME_DIR")
            .is_some_and(|dir| Path::new(&dir).join("pipewire-0").exists())
}

/// Czy użytkownik sam wybrał backend (`-b x`, `-bx`, `--backend x`, `--backend=x`).
fn backend_given(args: &[String]) -> bool {
    args.iter().skip(1).any(|a| {
        a == "-b" || a == "--backend" || a.starts_with("--backend=") || (a.starts_with("-b") && a.len() > 2)
    })
}

fn main() {
    let mut args: Vec<String> = std::env::args().collect();

    // Backend JACK w nice-plug podłącza wyjścia do `system:playback_1/2`, co pod PipeWire oznacza
    // pierwszą kartę w kolejności JACK-a, a nie wyjście wybrane w systemie. Przy kilku kartach
    // (np. interfejs USB na słuchawki + głośniki laptopa) klik trafiał w złe miejsce. ALSA
    // „default” idzie przez PipeWire na domyślne wyjście systemu i podąża za jego zmianą, tak
    // jak inne aplikacje. `-b jack` nadal działa, jeśli ktoś woli JACK.
    if !backend_given(&args) && pipewire_running() {
        eprintln!(
            "PipeWire detected: playing through the system default output (ALSA). \
             Use '-b jack' for JACK."
        );
        args.insert(1, "--backend".into());
        args.insert(2, "alsa".into());
    }

    nice_export_standalone_with_args::<Metro, _>(args);
}
