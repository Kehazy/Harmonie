<p align="center"><img src="icons/icon.png" width="104" alt="Harmonie icon"></p>

<h1 align="center">Harmonie</h1>

<p align="center">
  A clean, Apple Music-inspired theme for Spotify.<br>
  Install it in one click with a tiny Rust app, or straight from the Spicetify Marketplace.
</p>

<p align="center">
  <a href="https://github.com/Kehazy/harmonie/releases/latest"><img alt="Download" src="https://img.shields.io/github/v/release/Kehazy/harmonie?label=download&color=e5445c"></a>
  <a href="https://github.com/Kehazy/harmonie/releases"><img alt="Downloads" src="https://img.shields.io/github/downloads/Kehazy/harmonie/total?color=3b82d6"></a>
  <a href="https://github.com/Kehazy/harmonie/stargazers"><img alt="Stars" src="https://img.shields.io/github/stars/Kehazy/harmonie?style=flat&color=e08a2e"></a>
  <a href="LICENSE"><img alt="MIT license" src="https://img.shields.io/github/license/Kehazy/harmonie?color=2fa36b"></a>
  <a href="https://spicetify.app"><img alt="Spicetify theme" src="https://img.shields.io/badge/spicetify-theme-8e6be8"></a>
</p>

<p align="center"><img src="assets/app-home.png" width="860" alt="The Harmonie app, home page"></p>

---

## Features

- **Blurred album art** — the colors of the current cover fill the window and change with every track
- **Apple Music-style lyrics** — big bold lines over the blurred cover, the current line in white
- **Harmonie wave** — three glowing sine waves ripple along the player while music plays, and settle on pause
- **Frosted glass** — translucent panels, pill search bar, soft shadows on covers
- **Background patterns** — waves, dots, lines or rings, softly showing through the glass (or none)
- **Floating player** — the bottom player becomes a rounded glass card (or stays docked, your choice)
- **5 accent colors** — rose, violet, blue, green, orange, in dark and light (10 schemes)
- **Song colors** — optional: the accent takes the color of each song's cover, like Apple Music
- **Your own look** — tune the album art strength, corners, player style and pattern with a live preview
- **Presets** — Harmonie, Midnight, Sunset, Studio, Daylight and Vivid in one click
- **Extensions** — turn on all 10 of Spicetify's own extensions (Full App Display, Shuffle+, Loopy Loop, Popup Lyrics, Trashbin…) plus Volume Percentage (ohitstom) and Section Marker (Aimarekin) in one click, with search; community ones are downloaded from their author's repository and keep their own license
- **Small touches** — accent edge on hovered tracks, accent keyboard focus, pages that fade in, rounded tooltips
- **Survives Spotify updates** — the app repairs Spicetify's backup when Spotify updates
- **Handles Spicetify for you** — installs it if missing, updates it if outdated, repairs it if applying fails, and opens a terminal with the fix as a last resort
- **One-click install and undo** — the app restores the theme you had before
- **Tiny and fast** — the installer is a 3 MB native Rust app, no Electron

Harmonie only changes how Spotify looks. It does not touch ads or Premium features.

## Install

### Option 1: the Harmonie app (Windows)

1. Download **[Harmonie.exe](https://github.com/Kehazy/harmonie/releases/latest)** and run it. No setup needed.
2. You don't need to install Spicetify yourself: Harmonie installs it, or updates it, when you install the theme.
3. Pick a preset, or a mode, color and pattern in **Appearance**, then click **Apply to Spotify** (or press **Ctrl + Enter**).

> Windows SmartScreen may warn about an unknown publisher because the app is not code-signed yet. Click **More info**, then **Run anyway**.

### Option 2: Spicetify Marketplace (Windows, macOS, Linux)

Open the **Marketplace** tab in Spotify, go to **Themes**, search for **Harmonie** and click **Install**.

### Option 3: by hand

Copy the `theme/` folder to your Spicetify `Themes` folder as `Harmonie` (run `spicetify path userdata` to find it), then:

```
spicetify config current_theme Harmonie color_scheme dark inject_theme_js 1
spicetify apply
```

Available schemes: `dark`, `light`, `dark-violet`, `light-violet`, `dark-blue`, `light-blue`, `dark-green`, `light-green`, `dark-orange`, `light-orange`.

To change the look by hand, edit the **Options** block at the top of `user.css`:

```css
--harmonie-art-opacity: 0.42;   /* 0 hides the blurred album art */
--harmonie-radius: 14px;        /* panel corners */
--harmonie-player-gap: 10px;    /* 0 docks the player to the window edge */
--harmonie-player-radius: 20px;
--harmonie-pattern: var(--harmonie-pattern-waves); /* waves, dots, lines, rings or none */
--harmonie-pattern-opacity: 0.1;  /* set to 0 with "none" */
--harmonie-adaptive: 0;           /* 1: accent follows the song's cover */
```

## Screenshots

| Appearance | About |
| --- | --- |
| <img src="assets/app-appearance.png" alt="Appearance page"> | <img src="assets/app-about.png" alt="About page"> |

## Troubleshooting

**The theme does not show after a Spotify update.** Spotify updates replace Spicetify's changes. Open Harmonie and click **Update theme**: v2 refreshes the Spicetify backup automatically. By hand: `spicetify restore backup apply`.

**I use Spotify from the Microsoft Store.** Its own Start menu tile opens Spotify without the theme. In Harmonie, click **Create shortcut**: it puts a "Spotify (Harmonie)" shortcut on your desktop that starts Spotify through `spicetify auto`. Spicetify works best with the version from [spotify.com](https://www.spotify.com/download).

**I want my old look back.** Click **Remove theme** in the app. By hand: `spicetify config current_theme marketplace` then `spicetify apply`.

## Build from source

You need [Rust](https://rustup.rs) and the Visual Studio Build Tools ("Desktop development with C++").

```
cargo build --release
```

The app is written to `target/release/harmonie.exe`. If linking fails with `LNK1104`, the folder path is too long: set `CARGO_TARGET_DIR` to a short folder such as `C:\build\harmonie`.

| Path | What it is |
| --- | --- |
| `theme/` | The Spicetify theme: `color.ini`, `user.css`, `theme.js` |
| `src/main.rs` | Rust backend: finds Spicetify, writes the theme, runs `spicetify apply` |
| `ui/index.html` | The app's interface |
| `manifest.json` | Spicetify Marketplace listing |

## Contributing

Bug reports, screenshots and ideas are welcome in [Issues](https://github.com/Kehazy/harmonie/issues). Spotify changes its class names often; if something looks off after an update, a screenshot plus your Spotify version helps fix it fast.

If you like Harmonie, starring the repo helps other people find it.

## License

[MIT](LICENSE). Harmonie is an independent project, not affiliated with Spotify or Apple.
