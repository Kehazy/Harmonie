// Harmonie: installs a clean, Apple Music–inspired theme on Spotify through Spicetify.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const THEME: &str = "Harmonie";
const COLOR_INI: &str = include_str!("../theme/color.ini");
const USER_CSS: &str = include_str!("../theme/user.css");
const THEME_JS: &str = include_str!("../theme/theme.js");
const PREVIOUS_FILE: &str = ".harmonie-previous";
const ACCENT_FILE: &str = ".accent";

/// Available accent colors: (id, color, hover color)
const ACCENTS: &[(&str, &str, &str)] = &[
    ("rose", "E5445C", "EE6176"),
    ("violet", "8E6BE8", "A389EE"),
    ("blue", "3B82D6", "5B9AE0"),
    ("green", "2FA36B", "48B97F"),
    ("orange", "E08A2E", "E9A050"),
];

#[derive(Serialize)]
struct Status {
    spicetify: Option<String>,
    installed: bool,
    active: bool,
    scheme: String,
    accent: String,
    current_theme: String,
}

/// Path to spicetify.exe (default install location, otherwise PATH).
fn spicetify_exe() -> Option<PathBuf> {
    if let Ok(local) = env::var("LOCALAPPDATA") {
        let p = Path::new(&local).join("spicetify").join("spicetify.exe");
        if p.exists() {
            return Some(p);
        }
    }
    let probe = command("spicetify").arg("-v").output().ok()?;
    probe.status.success().then(|| PathBuf::from("spicetify"))
}

/// Spicetify's config folder (the one containing config-xpui.ini).
fn config_dir() -> Result<PathBuf, String> {
    for var in ["APPDATA", "LOCALAPPDATA"] {
        if let Ok(base) = env::var(var) {
            let dir = Path::new(&base).join("spicetify");
            if dir.join("config-xpui.ini").exists() {
                return Ok(dir);
            }
        }
    }
    Err("Spicetify config not found. Run Spicetify once first.".into())
}

fn command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: no flashing console
    cmd
}

/// Strips ANSI color codes from Spicetify's output.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for n in chars.by_ref() {
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn spicetify(args: &[&str]) -> Result<String, String> {
    let exe = spicetify_exe().ok_or("Spicetify is not installed.")?;
    let out = command(exe)
        .args(args)
        .output()
        .map_err(|e| format!("Could not start Spicetify: {e}"))?;
    let text = strip_ansi(&format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    ));
    if out.status.success() {
        Ok(text.trim().to_string())
    } else {
        Err(format!("spicetify {} failed:\n{}", args.join(" "), text.trim()))
    }
}

fn read_setting(dir: &Path, key: &str) -> String {
    fs::read_to_string(dir.join("config-xpui.ini"))
        .unwrap_or_default()
        .lines()
        .find_map(|line| {
            let (k, v) = line.split_once('=')?;
            (k.trim() == key).then(|| v.trim().to_string())
        })
        .unwrap_or_default()
}

#[tauri::command]
async fn status() -> Status {
    let version = spicetify_exe().and(spicetify(&["-v"]).ok());
    let (installed, current_theme, scheme, accent) = match config_dir() {
        Ok(dir) => {
            let theme_dir = dir.join("Themes").join(THEME);
            (
                theme_dir.join("user.css").exists(),
                read_setting(&dir, "current_theme"),
                read_setting(&dir, "color_scheme"),
                fs::read_to_string(theme_dir.join(ACCENT_FILE))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| "rose".into()),
            )
        }
        Err(_) => (false, String::new(), String::new(), "rose".into()),
    };
    Status {
        spicetify: version,
        installed,
        active: current_theme == THEME,
        scheme,
        accent,
        current_theme,
    }
}

#[tauri::command]
async fn install(scheme: String, accent: String) -> Result<String, String> {
    if scheme != "dark" && scheme != "light" {
        return Err("Unknown mode.".into());
    }
    let &(_, color, hover) = ACCENTS
        .iter()
        .find(|(id, _, _)| *id == accent)
        .ok_or("Unknown color.")?;
    let color_ini = COLOR_INI
        .replace("FA2D48", color)
        .replace("FF4F66", hover)
        .replace("E0243D", hover);
    let dir = config_dir()?;
    let theme_dir = dir.join("Themes").join(THEME);
    fs::create_dir_all(&theme_dir).map_err(|e| e.to_string())?;

    // Remember the current theme so it can be restored later.
    let current = read_setting(&dir, "current_theme");
    if current != THEME {
        let previous = format!("{current}\n{}", read_setting(&dir, "color_scheme"));
        fs::write(dir.join(PREVIOUS_FILE), previous).map_err(|e| e.to_string())?;
    }

    for (name, content) in [
        ("color.ini", color_ini.as_str()),
        ("user.css", USER_CSS),
        ("theme.js", THEME_JS),
        (ACCENT_FILE, accent.as_str()),
    ] {
        fs::write(theme_dir.join(name), content).map_err(|e| e.to_string())?;
    }

    spicetify(&[
        "config",
        "current_theme",
        THEME,
        "color_scheme",
        &scheme,
        "inject_css",
        "1",
        "inject_theme_js",
        "1",
        "replace_colors",
        "1",
    ])?;
    spicetify(&["apply"])
}

#[tauri::command]
async fn uninstall() -> Result<String, String> {
    let dir = config_dir()?;
    let saved = fs::read_to_string(dir.join(PREVIOUS_FILE)).unwrap_or_default();
    let mut lines = saved.lines();
    let prev_theme = lines.next().unwrap_or("").trim().to_string();
    let prev_scheme = lines.next().unwrap_or("").trim().to_string();
    let prev_theme = if prev_theme.is_empty() || prev_theme == THEME {
        "marketplace".to_string()
    } else {
        prev_theme
    };

    spicetify(&[
        "config",
        "current_theme",
        &prev_theme,
        "color_scheme",
        &prev_scheme,
    ])?;
    let log = spicetify(&["apply"])?;

    let _ = fs::remove_dir_all(dir.join("Themes").join(THEME));
    let _ = fs::remove_file(dir.join(PREVIOUS_FILE));
    Ok(log)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![status, install, uninstall])
        .run(tauri::generate_context!())
        .expect("failed to start Harmonie");
}
