// Harmonie: installs a clean, Apple Music–inspired theme on Spotify through Spicetify.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use tauri::Manager;
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
    "https://github.com/Kehazy/harmonie",
];

/// Background pattern drawn between the blurred art and the glass panels.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug, Default)]
#[serde(rename_all = "lowercase")]
enum Pattern {
    None,
    #[default]
    Waves,
    Dots,
    Lines,
    Rings,
}

impl Pattern {
    const ALL: [Pattern; 5] = [Pattern::None, Pattern::Waves, Pattern::Dots, Pattern::Lines, Pattern::Rings];

    fn name(self) -> &'static str {
        match self {
            Pattern::None => "none",
            Pattern::Waves => "waves",
            Pattern::Dots => "dots",
            Pattern::Lines => "lines",
            Pattern::Rings => "rings",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        Pattern::ALL.into_iter().find(|p| p.name() == name)
    }
}

/// Look options written into the theme's CSS variables.
#[derive(Serialize, Deserialize, Clone, Copy)]
struct Options {
    /// Blurred album art strength, 0 to 80 (percent opacity).
    art: u8,
    /// Panel corner radius in pixels, 0 to 24.
    radius: u8,
    /// Floating player card, or docked to the window edge.
    floating: bool,
    /// Background pattern. Older app versions do not send it.
    #[serde(default)]
    pattern: Pattern,
}

impl Default for Options {
    fn default() -> Self {
        Options { art: 42, radius: 14, floating: true, pattern: Pattern::Waves }
    }
}

const OPTIONS_START: &str = "/* ---------- Options ----------";
const OPTIONS_END: &str = "/* ---------- End of options ---------- */";

impl Options {
    fn clamped(self) -> Self {
        Options { art: self.art.min(80), radius: self.radius.min(24), ..self }
    }

    fn css_block(self) -> String {
        let (gap, player_radius) = if self.floating {
            (10, self.radius as u32 + 6)
        } else {
            (0, 0)
        };
        let lines = [
            OPTIONS_START.to_string(),
            "   Written by the Harmonie app. You can also edit these values by hand. */".into(),
            ":root {".into(),
            format!("  --harmonie-art-opacity: {:.2};", self.art as f32 / 100.0),
            "  --harmonie-art-blur: 110px;".into(),
            format!("  --harmonie-radius: {}px;", self.radius),
            format!("  --harmonie-player-gap: {gap}px;"),
            format!("  --harmonie-player-radius: {player_radius}px;"),
            format!(
                "  --harmonie-pattern: var(--harmonie-pattern-{}); /* waves, dots, lines, rings or none */",
                self.pattern.name()
            ),
            // With no pattern the layer would be a solid tint, so hide it.
            format!(
                "  --harmonie-pattern-opacity: {};",
                if self.pattern == Pattern::None { "0" } else { "0.1" }
            ),
            "}".into(),
            OPTIONS_END.to_string(),
        ];
        lines.join("\n")
    }

    /// The theme's CSS with this options block in place of the default one.
    fn apply_to(self, css: &str) -> String {
        match (css.find(OPTIONS_START), css.find(OPTIONS_END)) {
            (Some(a), Some(b)) if b > a => {
                format!("{}{}{}", &css[..a], self.css_block(), &css[b + OPTIONS_END.len()..])
            }
            _ => css.to_string(),
        }
    }

    /// Reads the options back from an installed user.css.
    fn parse(css: &str) -> Self {
        let var = |name: &str| -> Option<f32> {
            let i = css.find(&format!("--harmonie-{name}:"))?;
            let rest = &css[i..];
            let value = rest[rest.find(':')? + 1..rest.find(';')?].trim();
            value.trim_end_matches("px").parse().ok()
        };
        let pattern = css
            .find("--harmonie-pattern: var(--harmonie-pattern-")
            .map(|i| &css[i + "--harmonie-pattern: var(--harmonie-pattern-".len()..])
            .and_then(|rest| rest.split(')').next())
            .and_then(Pattern::from_name);
        let d = Options::default();
        Options {
            art: var("art-opacity").map_or(d.art, |v| (v * 100.0).round() as u8),
            radius: var("radius").map_or(d.radius, |v| v as u8),
            floating: var("player-gap").map_or(d.floating, |v| v > 0.0),
            pattern: pattern.unwrap_or(d.pattern),
        }
        .clamped()
    }
}

#[derive(Serialize)]
struct Status {
    version: &'static str,
    spicetify: Option<String>,
    installed: bool,
    active: bool,
    scheme: String,
    accent: String,
    current_theme: String,
    options: Options,
    /// Page to open first, from HARMONIE_PAGE (used to take screenshots).
    start_page: Option<String>,
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

/// Turns Spicetify's output into one sentence a person can act on, followed by
/// the last meaningful line of the raw output.
fn friendly(err: &str) -> String {
    let lower = err.to_ascii_lowercase();
    let hint = if lower.contains("cannot detect spotify")
        || lower.contains("cannot find spotify")
        || lower.contains("spotify_path")
    {
        Some("Spotify was not found. Install Spotify and open it once, then try again.")
    } else if lower.contains("access is denied")
        || lower.contains("permission denied")
        || lower.contains("used by another process")
    {
        Some("Windows blocked Spotify's files. Quit Spotify completely and try again.")
    } else if lower.contains("not a valid zip") || lower.contains("failed to extract") {
        Some("Spicetify could not read Spotify's files. Open and close Spotify once, then try again.")
    } else if lower.contains("outdated") || lower.contains("backup") {
        Some("Spotify was updated and Spicetify could not catch up. Run \"spicetify restore backup apply\" in a terminal.")
    } else {
        None
    };
    let detail = err
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("spicetify v") && !l.ends_with("failed:"))
        .last()
        .unwrap_or("")
        .to_string();
    match hint {
        Some(h) if detail.is_empty() => h.to_string(),
        Some(h) => format!("{h}\n{detail}"),
        None if detail.is_empty() => err.trim().to_string(),
        None => detail,
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
        options: Options::default(),
        start_page: env::var("HARMONIE_PAGE").ok(),
        store_spotify: false,
    };
    if let Ok(dir) = config_dir() {
        let css = dir.join("Themes").join(THEME).join("user.css");
        s.installed = css.exists();
        if let Ok(text) = fs::read_to_string(&css) {
            s.options = Options::parse(&text);
        }
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
async fn install(scheme: String, accent: String, options: Option<Options>) -> Result<String, String> {
    let options = options.unwrap_or_default().clamped();
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

    let user_css = options.apply_to(USER_CSS);
    for (name, content) in [
        ("color.ini", COLOR_INI),
        ("user.css", user_css.as_str()),
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
    ])
    .map_err(|e| friendly(&e))?;
    apply().map_err(|e| friendly(&e))
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
    ])
    .map_err(|e| friendly(&e))?;
    let log = apply().map_err(|e| friendly(&e))?;

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

/// Opens Spotify. The Microsoft Store version only loads the theme when started
/// through `spicetify auto`.
#[tauri::command]
async fn open_spotify() -> Result<(), String> {
    let store = config_dir()
        .map(|d| read_setting(&d, "spotify_path").contains("WindowsApps"))
        .unwrap_or(false);
    let mut cmd = if store {
        let mut c = command(spicetify_exe().ok_or("Spicetify is not installed.")?);
        c.arg("auto");
        c
    } else {
        let mut c = command("explorer");
        c.arg("spotify:");
        c
    };
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not open Spotify: {e}"))
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
        // A second launch focuses the open window instead of starting a second app
        // that could apply or remove the theme at the same time.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        // Reopen at the size and position the window had last time.
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            status,
            install,
            uninstall,
            create_shortcut,
            open_spotify,
            open_link
        ])
        .run(tauri::generate_context!())
        .expect("failed to start Harmonie");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_round_trip() {
        let defaults = Options::parse(USER_CSS);
        assert_eq!((defaults.art, defaults.radius, defaults.floating), (42, 14, true));
        assert_eq!(defaults.pattern, Pattern::Waves);

        let o = Options { art: 20, radius: 6, floating: false, pattern: Pattern::Rings };
        let css = o.apply_to(USER_CSS);
        let back = Options::parse(&css);
        assert_eq!((back.art, back.radius, back.floating), (20, 6, false));
        assert_eq!(back.pattern, Pattern::Rings);
        assert!(css.contains("--harmonie-player-gap: 0px;"));
        // Only the options block changes.
        assert_eq!(css.matches(OPTIONS_START).count(), 1);
        assert!(css.contains(".harmonie-wave"));
    }

    #[test]
    fn options_are_clamped() {
        let o = Options { art: 200, radius: 99, floating: true, pattern: Pattern::Dots }.clamped();
        assert_eq!((o.art, o.radius), (80, 24));
        assert_eq!(o.pattern, Pattern::Dots);
    }

    #[test]
    fn every_pattern_exists_in_the_theme() {
        for p in Pattern::ALL {
            let css = Options { pattern: p, ..Options::default() }.apply_to(USER_CSS);
            assert!(USER_CSS.contains(&format!("--harmonie-pattern-{}:", p.name())), "{}", p.name());
            assert_eq!(Options::parse(&css).pattern, p);
            let hidden = css.contains("--harmonie-pattern-opacity: 0;");
            assert_eq!(hidden, p == Pattern::None, "{}", p.name());
        }
    }

    #[test]
    fn old_apps_without_pattern_still_work() {
        let o: Options = serde_json::from_str(r#"{"art":30,"radius":10,"floating":true}"#).unwrap();
        assert_eq!(o.pattern, Pattern::Waves);
    }

    #[test]
    fn schemes_split_into_mode_and_accent() {
        assert_eq!(split_scheme("dark"), ("dark".into(), "rose".into()));
        assert_eq!(split_scheme("light-blue"), ("light".into(), "blue".into()));
        assert_eq!(split_scheme("weird-thing"), ("dark".into(), "rose".into()));
        assert_eq!(split_scheme(""), ("dark".into(), "rose".into()));
    }

    #[test]
    fn color_ini_has_every_scheme() {
        for mode in ["dark", "light"] {
            for accent in ACCENTS {
                let section = if *accent == "rose" {
                    format!("[{mode}]")
                } else {
                    format!("[{mode}-{accent}]")
                };
                assert!(COLOR_INI.contains(&section), "missing {section}");
            }
        }
    }

    #[test]
    fn detects_outdated_backup() {
        assert!(needs_new_backup(
            "info Preprocessed Spotify data is outdated. Please run \"spicetify restore backup apply\""
        ));
        assert!(!needs_new_backup("success Applied additional modifications"));
    }

    #[test]
    fn strips_ansi_colors() {
        assert_eq!(strip_ansi("\u{1b}[32m success \u{1b}[0m done"), " success  done");
    }

    #[test]
    fn friendly_errors() {
        let e = "spicetify apply failed:\nspicetify v2.46.7\nerror Failed to extract backup\nfatal zip: not a valid zip file";
        let msg = friendly(e);
        assert!(msg.starts_with("Spicetify could not read Spotify's files."), "{msg}");
        assert!(msg.ends_with("fatal zip: not a valid zip file"), "{msg}");
        assert_eq!(friendly("something odd"), "something odd");
    }
}
