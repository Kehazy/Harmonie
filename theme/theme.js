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
