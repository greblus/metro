//! `cargo xtask bundle metro --release` – buduje paczki VST3/CLAP do target/bundled
//! `cargo xtask install`                – buduje i instaluje do ~/.vst3 i ~/.clap

use std::fs;
use std::path::{Path, PathBuf};

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &to)?;
        } else {
            fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}

fn install() -> nih_plug_xtask::Result<()> {
    let args = ["bundle", "metro", "--release"].map(String::from);
    nih_plug_xtask::main_with_args("cargo xtask", args)?;

    let bundled = Path::new("target/bundled");
    let home = PathBuf::from(std::env::var("HOME")?);

    let vst3 = home.join(".vst3/Metro.vst3");
    if vst3.exists() {
        fs::remove_dir_all(&vst3)?;
    }
    copy_dir(&bundled.join("Metro.vst3"), &vst3)?;
    println!("Zainstalowano {}", vst3.display());

    let clap_dir = home.join(".clap");
    fs::create_dir_all(&clap_dir)?;
    let clap = clap_dir.join("Metro.clap");
    fs::copy(bundled.join("Metro.clap"), &clap)?;
    println!("Zainstalowano {}", clap.display());

    println!("Uruchom ponownie DAW (w razie potrzeby przeskanuj wtyczki).");
    Ok(())
}

fn main() -> nih_plug_xtask::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("install") {
        install()
    } else {
        nih_plug_xtask::main()
    }
}
