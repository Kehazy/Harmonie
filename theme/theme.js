// Harmonie — uses the current track's cover art as a blurred background
(function harmonie() {
  const P = window.Spicetify && Spicetify.Player;
  // Wait only for the Player API; it is fine if nothing is playing yet.
  if (!P || typeof P.addEventListener !== "function") {
    setTimeout(harmonie, 300);
    return;
  }

  const root = document.documentElement;
  let lastUrl = null;

  function toHttps(uri) {
    if (!uri) return null;
    if (uri.startsWith("spotify:image:")) {
      return "https://i.scdn.co/image/" + uri.split(":")[2];
    }
    return uri;
  }

  function currentArt() {
    const data = P.data;
    const item = data && (data.item || data.track);
    const meta = item && item.metadata;
    const fromMeta = meta && (meta.image_xlarge_url || meta.image_large_url || meta.image_url);
    if (fromMeta) return toHttps(fromMeta);
    // Newer clients expose images on the album object instead of metadata.
    const images = item && item.album && item.album.images;
    if (images && images.length) {
      const best = images.reduce((a, b) => ((b.width || 0) > (a.width || 0) ? b : a));
      return toHttps(best.url);
    }
    return null;
  }

  function update() {
    const url = currentArt();
    if (url === lastUrl) return;
    lastUrl = url;
    if (url) {
      root.style.setProperty("--harmonie-art", `url("${url.replace(/"/g, "%22")}")`);
    } else {
      root.style.removeProperty("--harmonie-art");
    }
  }

  P.addEventListener("songchange", update);
  update();

  /* ----- Song colors: the accent follows the current cover ----- */
  const ACCENT_VARS = ["--spice-button", "--spice-button-active", "--spice-rgb-button", "--spice-notification"];

  function adaptiveOn() {
    return getComputedStyle(root).getPropertyValue("--harmonie-adaptive").trim() === "1";
  }

  function isLightTheme() {
    const rgb = getComputedStyle(root).getPropertyValue("--spice-rgb-main").split(",").map(Number);
    return rgb.length === 3 && rgb[0] * 0.299 + rgb[1] * 0.587 + rgb[2] * 0.114 > 150;
  }

  function hexToHsl(hex) {
    const n = parseInt(hex.replace("#", ""), 16);
    const r = ((n >> 16) & 255) / 255, g = ((n >> 8) & 255) / 255, b = (n & 255) / 255;
    const max = Math.max(r, g, b), min = Math.min(r, g, b);
    const l = (max + min) / 2;
    if (max === min) return [0, 0, l];
    const d = max - min;
    const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    const h = max === r ? (g - b) / d + (g < b ? 6 : 0) : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
    return [h * 60, s, l];
  }

  function hslToRgb(h, s, l) {
    const k = (n) => (n + h / 30) % 12;
    const a = s * Math.min(l, 1 - l);
    const f = (n) => l - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));
    return [f(0), f(8), f(4)].map((v) => Math.round(v * 255));
  }

  const toHex = (rgb) => "#" + rgb.map((v) => v.toString(16).padStart(2, "0")).join("");

  // Keep the color readable: vivid enough, and light or dark enough for the mode.
  // Returns null for grey covers, which have no real hue to follow.
  function tune(hex) {
    let [h, s, l] = hexToHsl(hex);
    if (s < 0.12) return null;
    s = Math.max(s, 0.45);
    const [lo, hi] = isLightTheme() ? [0.36, 0.5] : [0.55, 0.68];
    l = Math.min(Math.max(l, lo), hi);
    return { main: hslToRgb(h, s, l), active: hslToRgb(h, s, Math.min(l + 0.08, 0.8)) };
  }

  function clearAdaptive() {
    ACCENT_VARS.forEach((v) => root.style.removeProperty(v));
    root.classList.remove("harmonie-adaptive");
  }

  let colorRequest = 0;
  async function adapt() {
    if (!adaptiveOn() || typeof Spicetify.colorExtractor !== "function") return clearAdaptive();
    const item = P.data && (P.data.item || P.data.track);
    const uri = item && item.uri;
    if (!uri) return;
    const request = ++colorRequest;
    try {
      const colors = await Spicetify.colorExtractor(uri);
      if (request !== colorRequest) return; // a newer song won
      const hex = colors && (colors.VIBRANT_NON_ALARMING || colors.VIBRANT || colors.PROMINENT);
      const tuned = hex && tune(hex);
      // No usable color for this song: go back to the chosen accent.
      if (!tuned) return clearAdaptive();
      const { main, active } = tuned;
      root.style.setProperty("--spice-button", toHex(main));
      root.style.setProperty("--spice-button-active", toHex(active));
      root.style.setProperty("--spice-rgb-button", main.join(","));
      root.style.setProperty("--spice-notification", toHex(main));
      root.classList.add("harmonie-adaptive");
    } catch (_) {
      // Offline or not available: keep the chosen accent.
    }
  }

  P.addEventListener("songchange", adapt);
  adapt();

  /* ----- Harmonie wave: three sine waves along the top of the player ----- */
  const NS = "http://www.w3.org/2000/svg";

  // A sine path over 0..1200. The svg is twice the wave's width and slides by half,
  // so `periods` must be even for the loop to be seamless.
  function sinePath(periods, amplitude, phase) {
    const width = 1200;
    const mid = 11;
    let d = "";
    for (let x = 0; x <= width; x += 6) {
      const y = mid + amplitude * Math.sin((x / width) * periods * 2 * Math.PI + phase);
      d += (x === 0 ? "M" : "L") + x + "," + y.toFixed(2);
    }
    return d;
  }

  function buildWave() {
    const wave = document.createElement("div");
    wave.className = "harmonie-wave";
    wave.setAttribute("aria-hidden", "true");
    for (const [periods, amplitude, phase] of [[6, 6.5, 0], [4, 8, 1.3], [8, 4.5, 2.6]]) {
      const svg = document.createElementNS(NS, "svg");
      svg.setAttribute("viewBox", "0 0 1200 22");
      svg.setAttribute("preserveAspectRatio", "none");
      const path = document.createElementNS(NS, "path");
      path.setAttribute("d", sinePath(periods, amplitude, phase));
      svg.appendChild(path);
      wave.appendChild(svg);
    }
    return wave;
  }

  function ensureWave() {
    const bar = document.querySelector(".main-nowPlayingBar-container");
    if (bar && !bar.querySelector(":scope > .harmonie-wave")) bar.appendChild(buildWave());
  }

  function syncPlaying() {
    const playing = typeof P.isPlaying === "function" ? P.isPlaying() : !(P.data && P.data.isPaused);
    root.classList.toggle("harmonie-playing", !!playing);
  }

  syncPlaying();
  P.addEventListener("onplaypause", syncPlaying);
  P.addEventListener("songchange", syncPlaying);

  // Wait for the player to exist, then keep the wave in place if Spotify re-renders it.
  (function watchPlayer() {
    const nowPlaying = document.querySelector(".Root__now-playing-bar");
    if (!nowPlaying) {
      setTimeout(watchPlayer, 500);
      return;
    }
    ensureWave();
    new MutationObserver(ensureWave).observe(nowPlaying, { childList: true, subtree: true });
  })();
})();
