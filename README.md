<p align="center"><img src="icons/icon.png" width="96" alt="Harmonie icon"></p>

<h1 align="center">Harmonie</h1>

<p align="center">A clean Spotify theme inspired by Apple Music, with a small Rust app that installs it in one click through <a href="https://spicetify.app">Spicetify</a>.</p>

---

The theme is purely visual: it does not touch ads or Premium features.

## What the theme changes

- **Blurred art background** — the colors of the current cover fill the window and change with every track
- **Frosted glass** — translucent panels and rounded corners across the interface
- **Floating player** — the bottom player becomes a rounded glass card
- **Accent color** — rose, violet, blue, green or orange on playback, progress, shuffle and repeat
- **Dark and light modes**
- **Typography** — a fine system font with tighter titles

## The installer app

Built with Rust and [Tauri 2](https://tauri.app) (about 3 MB).

- Glass sidebar that collapses to icons (keys `1` `2` `3` switch pages)
- **Home** — Spicetify and theme status, live preview, install button
- **Appearance** — dark or light mode, accent color, live preview
- **About** — what the theme changes
- **Remove theme** restores the theme you had before

## Requirements

- Windows 10 or 11 with WebView2 (built into Windows 11)
- Spotify desktop
- [Spicetify](https://spicetify.app)

> Spicetify works best with Spotify installed from spotify.com. The Microsoft Store version may not be supported.

## Build

You need [Rust](https://rustup.rs) and the Visual Studio Build Tools ("Desktop development with C++").

```
cargo build --release
```

The app is written to `target/release/harmonie.exe`.

If linking fails with `LNK1104`, the folder path is too long. Build into a shorter folder:

```
set CARGO_TARGET_DIR=C:\build\harmonie
cargo build --release
```

## Install the theme by hand

Copy the `theme/` folder to `%APPDATA%\spicetify\Themes\Harmonie`, then run:

```
spicetify config current_theme Harmonie color_scheme dark inject_theme_js 1
spicetify apply
```

To go back:

```
spicetify config current_theme marketplace
spicetify apply
```

## Project layout

| Path | What it is |
| --- | --- |
| `theme/` | The Spicetify theme: `color.ini`, `user.css`, `theme.js` |
| `src/main.rs` | Rust backend: finds Spicetify, writes the theme, runs `spicetify apply` |
| `ui/index.html` | The app's interface |
| `icons/` | App icon |

## Disclaimer

Harmonie is an independent project. It is not affiliated with Spotify or Apple.
