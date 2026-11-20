// Harmonie — uses the current track's cover art as a blurred background
(function harmonie() {
  const P = window.Spicetify && Spicetify.Player;
  if (!P || !P.addEventListener || !P.data) {
    setTimeout(harmonie, 300);
    return;
  }

  function toHttps(uri) {
    if (!uri) return null;
    if (uri.startsWith("spotify:image:")) {
      return "https://i.scdn.co/image/" + uri.split(":")[2];
    }
    return uri;
  }

  function update() {
    const item = (P.data && (P.data.item || P.data.track)) || null;
    const meta = item && item.metadata;
    const url = toHttps(
      meta && (meta.image_xlarge_url || meta.image_large_url || meta.image_url)
    );
    const root = document.documentElement;
    if (url) {
      root.style.setProperty("--harmonie-art", `url("${url}")`);
    } else {
      root.style.removeProperty("--harmonie-art");
    }
  }

  P.addEventListener("songchange", update);
  update();
})();
