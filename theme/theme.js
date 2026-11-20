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
})();
