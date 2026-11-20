// Harmonie: installs a clean, Apple Music–inspired theme on Spotify through Spicetify.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const THEME: &str = "Harmonie";
const COLOR_INI: &str = include_str!("../theme/color.ini");
const USER_CSS: &str = include_str!("../theme/user.css");
const THEME_JS: &str = include_str!("../theme/theme.js");
const PREVIOUS_FILE: &str = ".harmonie-previous";

/// Accent ids. "rose" is the plain `dark` / `light` scheme; the others are `dark-<id>` / `light-<id>`.
const ACCENTS: &[&str] = &["rose", "violet", "blue", "green", "orange"];

/// Links the app is allowed to open in the browser.
const LINKS: &[&str] = &[
    "https://spicetify.app",
    "https://github.com/DevStudioss/harmonie",
];

#[derive(Serialize)]
struct Status {
    version: &'static str,
    spicetify: Option<String>,
    installed: bool,
    active: bool,
    scheme: String,
    accent: String,
    current_theme: String,
    /// Spotify from the Microsoft Store, which Spicetify does not fully support.
    store_spotify: bool,
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
    let mut child = command(exe)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not start Spicetify: {e}"))?;
    // Spicetify asks "[y/N]" for the Microsoft Store version of Spotify; answer yes.
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(b"y\ny\ny\n");
    }
    let out = child
        .wait_with_output()
        .map_err(|e| format!("Spicetify did not finish: {e}"))?;
    let text = strip_ansi(&format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    ));
    let text = text.trim().to_string();
    // Spicetify sometimes exits with 0 but prints an error line.
    let failed = !out.status.success()
        || text
            .lines()
            .any(|l| matches!(l.split_whitespace().next(), Some("error" | "fatal")));
    if failed {
        Err(format!("spicetify {} failed:\n{}", args.join(" "), text))
    } else {
        Ok(text)
    }
}

/// After a Spotify update, `apply` exits with 0 but only prints that the
/// preprocessed data is outdated, without applying anything.
fn needs_new_backup(log: &str) -> bool {
    let log = log.to_ascii_lowercase();
    log.contains("outdated") || log.contains("restore backup") || log.contains("backup apply")
}

/// Applies the config to Spotify, refreshing Spicetify's backup when Spotify
/// was updated. Extraction sometimes fails once right after copying, so retry.
fn apply() -> Result<String, String> {
    let first = match spicetify(&["apply"]) {
        Ok(log) if !needs_new_backup(&log) => return Ok(log),
        Ok(log) => log,
        Err(e) => e,
    };
    let mut last = first;
    for args in [
        &["restore", "backup", "apply"][..],
        &["backup", "apply"][..],
        &["backup", "apply"][..],
    ] {
        match spicetify(args) {
            Ok(log) if !needs_new_backup(&log) => return Ok(log),
            Ok(log) => last = log,
            Err(e) => last = e,
        }
    }
    Err(last)
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

/// Splits a scheme like `light-blue` into ("light", "blue").
fn split_scheme(scheme: &str) -> (String, String) {
    let (mode, accent) = scheme.split_once('-').unwrap_or((scheme, "rose"));
    let mode = if mode == "light" { "light" } else { "dark" };
    let accent = if ACCENTS.contains(&accent) { accent } else { "rose" };
    (mode.into(), accent.into())
}

#[tauri::command]
async fn status() -> Status {
    let version = spicetify(&["-v"]).ok().filter(|v| !v.is_empty());
    let mut s = Status {
        version: env!("CARGO_PKG_VERSION"),
        spicetify: version,
        installed: false,
        active: false,
        scheme: "dark".into(),
        accent: "rose".into(),
        current_theme: String::new(),
        store_spotify: false,
    };
    if let Ok(dir) = config_dir() {
        s.installed = dir.join("Themes").join(THEME).join("user.css").exists();
        s.current_theme = read_setting(&dir, "current_theme");
        s.active = s.current_theme == THEME;
        if s.active {
            let (mode, accent) = split_scheme(&read_setting(&dir, "color_scheme"));
            s.scheme = mode;
            s.accent = accent;
        }
        s.store_spotify = read_setting(&dir, "spotify_path").contains("WindowsApps");
    }
    s
}

#[tauri::command]
async fn install(scheme: String, accent: String) -> Result<String, String> {
    if scheme != "dark" && scheme != "light" {
        return Err("Unknown mode.".into());
    }
    if !ACCENTS.contains(&accent.as_str()) {
        return Err("Unknown color.".into());
    }
    let color_scheme = if accent == "rose" {
        scheme
    } else {
        format!("{scheme}-{accent}")
    };

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
        ("color.ini", COLOR_INI),
        ("user.css", USER_CSS),
        ("theme.js", THEME_JS),
    ] {
        fs::write(theme_dir.join(name), content).map_err(|e| e.to_string())?;
    }
    // Leftover from 1.0, which stored the accent in its own file.
    let _ = fs::remove_file(theme_dir.join(".accent"));

    spicetify(&[
        "config",
        "current_theme",
        THEME,
        "color_scheme",
        &color_scheme,
        "inject_css",
        "1",
        "inject_theme_js",
        "1",
        "replace_colors",
        "1",
    ])?;
    apply()
}

#[tauri::command]
async fn uninstall() -> Result<String, String> {
    let dir = config_dir()?;
    let saved = fs::read_to_string(dir.join(PREVIOUS_FILE)).unwrap_or_default();
    let mut lines = saved.lines();
    let mut prev_theme = lines.next().unwrap_or("").trim().to_string();
    let mut prev_scheme = lines.next().unwrap_or("").trim().to_string();

    // Fall back to Spotify's own look if the previous theme is gone.
    if prev_theme == THEME || !dir.join("Themes").join(&prev_theme).is_dir() {
        prev_theme.clear();
        prev_scheme.clear();
    }

    spicetify(&[
        "config",
        "current_theme",
        &prev_theme,
        "color_scheme",
        &prev_scheme,
    ])?;
    let log = apply()?;

    let _ = fs::remove_dir_all(dir.join("Themes").join(THEME));
    let _ = fs::remove_file(dir.join(PREVIOUS_FILE));
    Ok(log)
}

/// Creates a desktop shortcut that starts Spotify through `spicetify auto`.
/// The Microsoft Store version loses the theme when started from its own tile.
#[tauri::command]
async fn create_shortcut() -> Result<String, String> {
    let exe = spicetify_exe().ok_or("Spicetify is not installed.")?;
    let exe = if exe.is_absolute() {
        exe
    } else {
        PathBuf::from(&env::var("LOCALAPPDATA").unwrap_or_default())
            .join("spicetify")
            .join("spicetify.exe")
    };
    let icon = config_dir()
        .map(|d| read_setting(&d, "spotify_path"))
        .map(|p| Path::new(&p).join("Spotify.exe"))
        .ok()
        .filter(|p| p.exists())
        .map(|p| format!("{},0", p.display()))
        .unwrap_or_default();
    // Paths go in through environment variables, never into the script text.
    let script = concat!(
        "$d = [Environment]::GetFolderPath('Desktop'); ",
        "$s = (New-Object -ComObject WScript.Shell).CreateShortcut((Join-Path $d 'Spotify (Harmonie).lnk')); ",
        "$s.TargetPath = $env:HARMONIE_TARGET; $s.Arguments = 'auto'; $s.WindowStyle = 7; ",
        "$s.Description = 'Spotify with the Harmonie theme'; ",
        "if ($env:HARMONIE_ICON) { $s.IconLocation = $env:HARMONIE_ICON }; $s.Save()"
    );
    let out = command("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("HARMONIE_TARGET", &exe)
        .env("HARMONIE_ICON", &icon)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok("Shortcut created on your desktop.".into())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// Opens one of the known project links in the default browser.
#[tauri::command]
fn open_link(url: String) -> Result<(), String> {
    let allowed = LINKS
        .iter()
        .any(|base| url == *base || url.starts_with(&format!("{base}/")));
    if !allowed {
        return Err("This link is not allowed.".into());
    }
    command("explorer")
        .arg(&url)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![status, install, uninstall, create_shortcut, open_link])
        .run(tauri::generate_context!())
        .expect("failed to start Harmonie");
}
