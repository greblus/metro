//! Wybór wyjścia audio w wersji samodzielnej (tylko pod PipeWire).
//!
//! Standalone gra przez ALSA „default”, czyli jako zwykły strumień PipeWire. Wyjście zmieniamy tak
//! samo jak systemowe ustawienia dźwięku: ustawiając `target.object` strumienia (`pw-metadata`),
//! co WirePlumber stosuje od razu, bez restartu. Listę wyjść i własny strumień bierzemy z `pw-dump`.
//! Wybór jest zapisywany w `~/.config/metro/standalone.conf`.

use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sink {
    /// `node.name`, np. `alsa_output.usb-…analog-stereo`
    pub name: String,
    /// Czytelna nazwa, np. „iRig USB Analog Stereo”
    pub description: String,
}

fn pw_dump() -> Option<serde_json::Value> {
    let out = Command::new("pw-dump").output().ok()?;
    if !out.status.success() {
        return None;
    }
    serde_json::from_slice(&out.stdout).ok()
}

fn props(o: &serde_json::Value) -> Option<&serde_json::Map<String, serde_json::Value>> {
    o.get("info")?.get("props")?.as_object()
}

fn prop_str<'a>(p: &'a serde_json::Map<String, serde_json::Value>, k: &str) -> Option<&'a str> {
    p.get(k)?.as_str()
}

/// Wszystkie wyjścia audio (urządzenia) widoczne w PipeWire.
pub fn list_sinks() -> Vec<Sink> {
    let Some(dump) = pw_dump() else { return Vec::new() };
    let mut sinks: Vec<Sink> = dump
        .as_array()
        .into_iter()
        .flatten()
        .filter(|o| o.get("type").and_then(|t| t.as_str()) == Some("PipeWire:Interface:Node"))
        .filter_map(props)
        .filter(|p| prop_str(p, "media.class") == Some("Audio/Sink"))
        .filter_map(|p| {
            let name = prop_str(p, "node.name")?.to_string();
            let description = prop_str(p, "node.description")
                .or_else(|| prop_str(p, "node.nick"))
                .unwrap_or(&name)
                .to_string();
            Some(Sink { name, description })
        })
        .collect();
    sinks.sort_by(|a, b| a.description.cmp(&b.description));
    sinks
}

/// Id węzła PipeWire naszego strumienia wyjściowego ALSA, jeśli już istnieje.
fn own_stream_id() -> Option<u64> {
    let dump = pw_dump()?;
    let objs = dump.as_array()?;
    let pid = std::process::id().to_string();
    let is_ours = |p: &serde_json::Map<String, serde_json::Value>| match p.get("application.process.id") {
        Some(serde_json::Value::String(s)) => *s == pid,
        Some(serde_json::Value::Number(n)) => n.to_string() == pid,
        _ => false,
    };
    let clients: Vec<u64> = objs
        .iter()
        .filter(|o| o.get("type").and_then(|t| t.as_str()) == Some("PipeWire:Interface:Client"))
        .filter(|o| props(o).is_some_and(is_ours))
        .filter_map(|o| o.get("id")?.as_u64())
        .collect();
    objs.iter()
        .filter(|o| o.get("type").and_then(|t| t.as_str()) == Some("PipeWire:Interface:Node"))
        .find(|o| {
            props(o).is_some_and(|p| {
                prop_str(p, "media.class") == Some("Stream/Output/Audio")
                    && p.get("client.id").and_then(|c| c.as_u64()).is_some_and(|c| clients.contains(&c))
            })
        })
        .and_then(|o| o.get("id")?.as_u64())
}

/// Kieruje nasz strumień na wskazane wyjście (`None` = domyślne systemowe).
/// Czeka do kilku sekund, aż strumień powstanie (tuż po starcie programu).
pub fn apply(target: Option<&str>) -> bool {
    for _ in 0..20 {
        if let Some(id) = own_stream_id() {
            let id = id.to_string();
            let status = match target {
                Some(name) => Command::new("pw-metadata").args([&id, "target.object", name]).output(),
                None => Command::new("pw-metadata").args(["-d", &id, "target.object"]).output(),
            };
            return status.is_ok_and(|s| s.status.success());
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    false
}

fn config_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("metro").join("standalone.conf"))
}

/// Zapisane wyjście (`None` = domyślne systemowe).
pub fn load_choice() -> Option<String> {
    let text = std::fs::read_to_string(config_path()?).ok()?;
    text.lines()
        .find_map(|l| l.strip_prefix("output="))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

pub fn save_choice(target: Option<&str>) {
    let Some(path) = config_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, format!("output={}\n", target.unwrap_or("")));
}
