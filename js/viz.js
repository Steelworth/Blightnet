export function createVisualizer({ canvas, mixer }) {
  const ctx =
    canvas.getContext("2d", { alpha: true, desynchronized: true }) ||
    canvas.getContext("2d");
  const W = 480;
  const H = 64;
  const n = 24;
  canvas.width = W;
  canvas.height = H;
  const bins = new Uint8Array(128);
  const smooth = new Float32Array(n);
  const gold = "rgba(255,196,92,0.55)";
  const cyan = "rgba(0,240,255,0.7)";
  const yellow = "rgba(252,238,10,0.7)";
  let blight = document.documentElement?.dataset?.theme === "blight";
  let themeAge = 0;
  let idle = 0;
  let lastSig = -1;

  function tick() {
    requestAnimationFrame(tick);
    if (document.hidden) return;
    if (canvas.offsetParent === null) return;

    if (++themeAge > 45) {
      themeAge = 0;
      blight = document.documentElement?.dataset?.theme === "blight";
    }

    const live = Boolean(mixer.livePlaying) || (mixer.playing && mixer.playing.size > 0);
    if (live && mixer.analyser) mixer.analyser.getByteFrequencyData(bins);

    let sig = 0;
    const span = 4;
    for (let i = 0; i < n; i++) {
      let s = 0;
      if (live) {
        const a = i * span;
        s = (bins[a] + bins[a + 1] + bins[a + 2] + bins[a + 3]) || 0;
      }
      const v = live ? s / (1020) : 0.08;
      const next = smooth[i] + (v - smooth[i]) * 0.35;
      smooth[i] = next;
      sig += next;
    }

    if (live) {
      idle = 0;
    } else {
      const rounded = (sig * 200) | 0;
      if (rounded === lastSig) {
        if (++idle > 2) return;
      } else {
        idle = 0;
        lastSig = rounded;
      }
    }

    ctx.clearRect(0, 0, W, H);
    const barW = W / n;
    const split = blight ? 8 : -1;
    for (let i = 0; i < n; i++) {
      const bh = Math.max(2, smooth[i] * H * 0.92);
      ctx.fillStyle = blight ? (i < split ? cyan : yellow) : gold;
      ctx.fillRect((i * barW) | 0, (H - bh) | 0, (barW - 1) | 0, bh | 0);
    }
  }

  requestAnimationFrame(tick);
}
