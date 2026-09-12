import {
  CANVAS_W as W,
  CANVAS_H as H,
  HAIRS,
  EYES,
  ETHNICITIES,
  mergeCreator,
} from "./creator-data.js";

export {
  DND_RACES,
  BLIGHT_RACES,
  PRESENTS,
  ETHNICITIES,
  HAIRS,
  EYES,
  FACE_COUNT,
  FACE_NAMES,
  FEMME_FACES,
  BODY_TYPES,
  BUST_SIZES,
  HAIR_COLORS,
  EYE_COLORS,
  WEAR_SLOTS,
  BODY_PARTS,
  LOSS_SLOTS,
  CHROME_SLOTS,
  CHROME_STYLES,
  SCAR_KINDS,
  SCAR_PARTS,
  FINGERS,
  defaultCreator,
  mergeCreator,
} from "./creator-data.js";

export const AUGS = [
  ["eyes", "Cybereyes"],
  ["jack", "Neural jack"],
  ["arm", "Chrome arm"],
  ["dermal", "Dermal plates"],
  ["speed", "Speedware glow"],
];

const cache = new Map();

function loadImg(src) {
  if (cache.has(src)) return cache.get(src);
  const p = new Promise((resolve) => {
    const im = new Image();
    im.onload = () => resolve(im);
    im.onerror = () => resolve(null);
    im.src = src;
  });
  cache.set(src, p);
  return p;
}

export function creatorPath(race, present) {
  return `assets/creator/${race}-${present}.jpg`;
}

function bodyCandidates(race, present, view) {
  const base = `assets/creator/bodies/${race}-${present}-${view}`;
  return [`${base}.png`, `${base}.jpg`];
}

async function loadBodyPhoto(opts, view) {
  const present = opts.present || "femme";
  const type = opts.bodyType || "average";
  const bust = opts.bust || "m";
  if (present === "femme") {
    if (type === "average") {
      const bustImg = await loadFirst([
        `assets/creator/bodies/bust/${bust}-${view}.jpg`,
        `assets/creator/bodies/bust/${bust}-front.jpg`,
      ]);
      if (bustImg) return { img: bustImg, photobust: true };
    }
    const typeImg = await loadFirst([
      `assets/creator/bodies/types/${type}-${view}.jpg`,
      `assets/creator/bodies/types/${type}-front.jpg`,
    ]);
    if (typeImg) return { img: typeImg, photobust: false };
  }
  const raceImg = await loadFirst(bodyCandidates(opts.race, present, view));
  return { img: raceImg, photobust: false };
}

function faceCandidates(race, present, n) {
  return [
    `assets/creator/faces/${race}-${present}-${n}.png`,
    `assets/creator/faces/${race}-${present}-${n}.jpg`,
  ];
}

function hairCandidates(style, view) {
  return [
    `assets/creator/hair/${style}-${view}.png`,
    `assets/creator/hair/${style}-front.png`,
    `assets/creator/hair/${style}.png`,
  ];
}

function eyeCandidates(shape) {
  return [`assets/creator/eyes/${shape}.png`, `assets/creator/eyes/${shape}.jpg`];
}

function wearCandidates(slot, item, view) {
  return [
    `assets/creator/wear/${slot}-${item}-${view}.png`,
    `assets/creator/wear/${slot}-${item}-front.png`,
    `assets/creator/wear/${slot}-${item}.png`,
  ];
}

function chromeCandidates(part, style) {
  return [
    `assets/creator/chrome/${part}-${style}.png`,
    `assets/creator/chrome/${part}-chrome.png`,
    `assets/creator/chrome/${part}.png`,
  ];
}

async function loadFirst(urls) {
  for (const u of urls) {
    const im = await loadImg(u);
    if (im) return im;
  }
  return null;
}

function colorizeOnto(ctx, img, x, y, w, h, hex, mode = "multiply") {
  const c = document.createElement("canvas");
  c.width = Math.max(1, w);
  c.height = Math.max(1, h);
  const g = c.getContext("2d");
  g.drawImage(img, 0, 0, c.width, c.height);
  g.globalCompositeOperation = mode === "tint" ? "source-atop" : mode;
  g.globalAlpha = mode === "tint" ? 0.78 : 1;
  g.fillStyle = hex || "#888";
  g.fillRect(0, 0, c.width, c.height);
  g.globalAlpha = 1;
  if (mode !== "tint" && mode !== "source-atop") {
    g.globalCompositeOperation = "destination-in";
    g.drawImage(img, 0, 0, c.width, c.height);
  }
  ctx.drawImage(c, x, y);
}

function roundRectPath(ctx, x, y, w, h, r) {
  const rad = Math.max(0, Math.min(r, w / 2, h / 2));
  if (typeof ctx.roundRect === "function") {
    ctx.roundRect(x, y, w, h, rad);
    return;
  }
  ctx.moveTo(x + rad, y);
  ctx.arcTo(x + w, y, x + w, y + h, rad);
  ctx.arcTo(x + w, y + h, x, y + h, rad);
  ctx.arcTo(x, y + h, x, y, rad);
  ctx.arcTo(x, y, x + w, y, rad);
  ctx.closePath();
}

function hexRgb(hex) {
  const h = String(hex || "#888").replace("#", "");
  const n = parseInt(h.length === 3 ? h.split("").map((c) => c + c).join("") : h, 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

function rgbStr(rgb, a = 1) {
  return `rgba(${rgb[0]|0},${rgb[1]|0},${rgb[2]|0},${a})`;
}

function mixRgb(a, b, t) {
  return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
}

function shade(rgb, t) {
  return t < 0
    ? [rgb[0] * (1 + t), rgb[1] * (1 + t), rgb[2] * (1 + t)]
    : mixRgb(rgb, [255, 255, 255], t);
}

const RACE_TINT = {
  orc: [72, 110, 70],
  tiefling: [140, 62, 58],
  vampire: [210, 198, 196],
  dragonborn: [70, 118, 96],
  succubus: [150, 88, 110],
  incubus: [150, 88, 110],
  dwarf: null,
  elf: null,
  gnome: null,
  halfling: null,
  human: null,
};

function skinOf(opts) {
  const eth = ETHNICITIES.find((e) => e.id === opts.ethnicity) || ETHNICITIES[3];
  let rgb = eth.skin.slice();
  const tint = RACE_TINT[opts.race];
  if (tint) rgb = mixRgb(rgb, tint, 0.55);
  const s = Number(opts.skin) || 0;
  if (s < 0) rgb = mixRgb(rgb, [255, 228, 200], -s);
  if (s > 0) rgb = mixRgb(rgb, [40, 22, 12], s);
  return rgb;
}

function layout(view) {
  if (view === "profile") {
    return {
      head: { x: 0.42, y: 0.02, w: 0.28, h: 0.16 },
      neck: { x: 0.48, y: 0.175, w: 0.12, h: 0.04 },
      chest: { x: 0.4, y: 0.21, w: 0.26, h: 0.16 },
      waist: { x: 0.42, y: 0.36, w: 0.22, h: 0.08 },
      hips: { x: 0.4, y: 0.44, w: 0.26, h: 0.08 },
      lArm: { x: 0.36, y: 0.24, w: 0.1, h: 0.28 },
      rArm: { x: 0.58, y: 0.24, w: 0.14, h: 0.28 },
      lLeg: { x: 0.42, y: 0.52, w: 0.1, h: 0.42 },
      rLeg: { x: 0.52, y: 0.52, w: 0.12, h: 0.42 },
      lHand: { x: 0.34, y: 0.48, w: 0.08, h: 0.07 },
      rHand: { x: 0.62, y: 0.48, w: 0.1, h: 0.07 },
      lFoot: { x: 0.4, y: 0.9, w: 0.12, h: 0.06 },
      rFoot: { x: 0.52, y: 0.9, w: 0.16, h: 0.06 },
      profile: true,
    };
  }
  return {
    head: { x: 0.31, y: 0.02, w: 0.38, h: 0.16 },
    neck: { x: 0.42, y: 0.175, w: 0.16, h: 0.04 },
    chest: { x: 0.3, y: 0.21, w: 0.4, h: 0.16 },
    waist: { x: 0.34, y: 0.36, w: 0.32, h: 0.08 },
    hips: { x: 0.3, y: 0.44, w: 0.4, h: 0.08 },
    lArm: { x: 0.12, y: 0.23, w: 0.2, h: 0.28 },
    rArm: { x: 0.68, y: 0.23, w: 0.2, h: 0.28 },
    lLeg: { x: 0.3, y: 0.52, w: 0.18, h: 0.4 },
    rLeg: { x: 0.52, y: 0.52, w: 0.18, h: 0.4 },
    lHand: { x: 0.08, y: 0.48, w: 0.14, h: 0.08 },
    rHand: { x: 0.78, y: 0.48, w: 0.14, h: 0.08 },
    lFoot: { x: 0.28, y: 0.9, w: 0.18, h: 0.07 },
    rFoot: { x: 0.54, y: 0.9, w: 0.18, h: 0.07 },
    profile: false,
  };
}

function box(r) {
  return { x: r.x * W, y: r.y * H, w: r.w * W, h: r.h * H };
}

function ellipse(ctx, x, y, rx, ry) {
  ctx.beginPath();
  ctx.ellipse(x, y, rx, ry, 0, 0, Math.PI * 2);
  ctx.fill();
}

function limb(ctx, x, y, w, h, rgb, round = 0.5) {
  ctx.fillStyle = rgbStr(shade(rgb, -0.08));
  const r = Math.min(w, h) * round;
  ctx.beginPath();
  roundRectPath(ctx, x, y, w, h, r);
  ctx.fill();
  ctx.fillStyle = rgbStr(shade(rgb, 0.12), 0.35);
  ctx.beginPath();
  roundRectPath(ctx, x + w * 0.15, y + 2, w * 0.35, h - 4, r * 0.5);
  ctx.fill();
}

function drawMannequin(ctx, L, skin, shape, view) {
  const s = (id) => Math.max(0.55, Math.min(1.55, Number(shape[id]) || 1));
  const head = box(L.head);
  const neck = box(L.neck);
  const chest = box(L.chest);
  const waist = box(L.waist);
  const hips = box(L.hips);
  const lArm = box(L.lArm);
  const rArm = box(L.rArm);
  const lLeg = box(L.lLeg);
  const rLeg = box(L.rLeg);
  const lHand = box(L.lHand);
  const rHand = box(L.rHand);
  const lFoot = box(L.lFoot);
  const rFoot = box(L.rFoot);

  const hx = 1 + (s("height") - 1) * 0.35;
  ctx.save();
  ctx.translate(W / 2, 0);
  ctx.scale(1, hx);
  ctx.translate(-W / 2, 0);

  const widen = (b, k) => {
    const dw = b.w * (k - 1);
    return { x: b.x - dw / 2, y: b.y, w: b.w + dw, h: b.h };
  };

  const ch = widen(chest, s("chest") * (0.7 + s("shoulders") * 0.3));
  const wa = widen(waist, s("waist"));
  const hi = widen(hips, s("hips"));
  const be = widen({ x: waist.x, y: waist.y + waist.h * 0.4, w: waist.w, h: waist.h }, s("belly"));

  ctx.fillStyle = rgbStr(shade(skin, -0.12));
  ctx.beginPath();
  ctx.moveTo(ch.x, ch.y + 8);
  ctx.lineTo(ch.x + ch.w, ch.y + 8);
  ctx.lineTo(wa.x + wa.w, wa.y);
  ctx.lineTo(hi.x + hi.w, hi.y + hi.h);
  ctx.lineTo(hi.x, hi.y + hi.h);
  ctx.lineTo(wa.x, wa.y);
  ctx.closePath();
  ctx.fill();
  ctx.fillStyle = rgbStr(shade(skin, 0.1), 0.3);
  ctx.fillRect(ch.x + ch.w * 0.3, ch.y + 10, ch.w * 0.2, ch.h * 0.7);
  ellipse(ctx, be.x + be.w / 2, be.y + be.h / 2, be.w * 0.42, be.h * 0.7);

  const armW = (b, k) => ({ ...b, w: b.w * k, x: b.x + (b === lArm ? b.w * (1 - k) : 0) });
  const la = armW(lArm, s("lUpperArm"));
  const ra = armW(rArm, s("rUpperArm"));
  limb(ctx, la.x, la.y, la.w, la.h * 0.55, skin, 0.4);
  limb(ctx, ra.x, ra.y, ra.w, ra.h * 0.55, skin, 0.4);
  limb(ctx, la.x + la.w * 0.08, la.y + la.h * 0.5, la.w * s("lForearm") * 0.85, la.h * 0.5, shade(skin, -0.04), 0.45);
  limb(ctx, ra.x + ra.w * 0.08, ra.y + ra.h * 0.5, ra.w * s("rForearm") * 0.85, ra.h * 0.5, shade(skin, -0.04), 0.45);

  limb(ctx, lLeg.x, lLeg.y, lLeg.w * s("lThigh"), lLeg.h * 0.5, skin, 0.35);
  limb(ctx, rLeg.x + rLeg.w * (1 - s("rThigh")), rLeg.y, rLeg.w * s("rThigh"), rLeg.h * 0.5, skin, 0.35);
  limb(ctx, lLeg.x + 4, lLeg.y + lLeg.h * 0.48, lLeg.w * s("lCalf") * 0.85, lLeg.h * 0.48, shade(skin, -0.06), 0.4);
  limb(ctx, rLeg.x + 6, rLeg.y + rLeg.h * 0.48, rLeg.w * s("rCalf") * 0.85, rLeg.h * 0.48, shade(skin, -0.06), 0.4);

  limb(ctx, lFoot.x, lFoot.y, lFoot.w * s("lFoot"), lFoot.h, shade(skin, -0.1), 0.4);
  limb(ctx, rFoot.x, rFoot.y, rFoot.w * s("rFoot"), rFoot.h, shade(skin, -0.1), 0.4);

  const hs = s("head");
  ctx.fillStyle = rgbStr(skin);
  ellipse(ctx, head.x + head.w / 2, head.y + head.h / 2, (head.w / 2) * hs, (head.h / 2) * hs);
  limb(ctx, neck.x, neck.y, neck.w * s("neck"), neck.h, shade(skin, -0.05), 0.4);

  const hw = s("lHand");
  const hwR = s("rHand");
  ctx.fillStyle = rgbStr(skin);
  ellipse(ctx, lHand.x + lHand.w / 2, lHand.y + lHand.h / 2, lHand.w * 0.4 * hw, lHand.h * 0.4);
  ellipse(ctx, rHand.x + rHand.w / 2, rHand.y + rHand.h / 2, rHand.w * 0.4 * hwR, rHand.h * 0.4);

  ctx.restore();
  return { head, lHand, rHand, lArm, rArm, lLeg, rLeg, chest, hips, neck, L };
}

function warpCanvas(src, shape) {
  const out = document.createElement("canvas");
  out.width = src.width;
  out.height = src.height;
  const ctx = out.getContext("2d");
  const s = (id) => Math.max(0.62, Math.min(1.45, Number(shape[id]) || 1));
  const bands = [
    { y0: 0.0, y1: 0.18, w: s("head") },
    { y0: 0.18, y1: 0.22, w: s("neck") },
    { y0: 0.22, y1: 0.3, w: s("shoulders") },
    { y0: 0.3, y1: 0.4, w: s("chest") },
    { y0: 0.4, y1: 0.47, w: s("waist") * 0.5 + s("belly") * 0.5 },
    { y0: 0.47, y1: 0.55, w: s("hips") },
    { y0: 0.55, y1: 0.75, w: (s("lThigh") + s("rThigh")) / 2 },
    { y0: 0.75, y1: 0.92, w: (s("lCalf") + s("rCalf")) / 2 },
    { y0: 0.92, y1: 1, w: (s("lFoot") + s("rFoot")) / 2 },
  ];
  const hy = s("height");
  for (const b of bands) {
    const sy = b.y0 * src.height;
    const sh = Math.max(1, (b.y1 - b.y0) * src.height);
    const dw = src.width * b.w;
    const dx = (src.width - dw) / 2;
    const dy = sy * hy + (src.height * (1 - hy)) * 0.08;
    const dh = sh * hy;
    ctx.drawImage(src, 0, sy, src.width, sh, dx, dy, dw, dh);
  }
  return out;
}

function drawHair(ctx, head, hair, view) {
  const spec = HAIRS[hair.style] || HAIRS[0];
  const rgb = hexRgb(hair.color);
  const cx = head.x + head.w / 2;
  const cy = head.y + head.h * 0.42;
  const rx = head.w * 0.52 + spec.side * 40;
  const ry = head.h * 0.55;
  ctx.save();
  ctx.fillStyle = rgbStr(shade(rgb, -0.1));
  if (spec.mohawk > 0) {
    ctx.beginPath();
    ctx.moveTo(cx - 8 - spec.mohawk * 10, head.y + head.h * 0.55);
    ctx.quadraticCurveTo(cx, head.y - spec.len * 80 - spec.puff * 30, cx + 8 + spec.mohawk * 10, head.y + head.h * 0.55);
    ctx.fill();
  } else if (spec.afro > 0.4) {
    ellipse(ctx, cx, cy - 8, rx * (1.15 + spec.afro * 0.4), ry * (1.1 + spec.afro * 0.5));
  } else {
    ctx.beginPath();
    ctx.ellipse(cx, cy - 4, rx * (0.95 + spec.puff * 0.3), ry * (0.85 + spec.puff * 0.25), 0, Math.PI, 0);
    ctx.fill();
    if (spec.len > 0.12) {
      const fall = spec.len * head.h * 3.2;
      const spread = rx * (0.7 + spec.side * 2);
      ctx.beginPath();
      ctx.moveTo(cx - spread, cy);
      ctx.quadraticCurveTo(cx - spread * 1.1, cy + fall * 0.5, cx - spread * 0.6, cy + fall);
      ctx.lineTo(cx + spread * 0.6, cy + fall);
      ctx.quadraticCurveTo(cx + spread * 1.1, cy + fall * 0.5, cx + spread, cy);
      ctx.fill();
    }
  }
  if (spec.bang > 0.15) {
    ctx.fillStyle = rgbStr(rgb);
    const by = head.y + head.h * (0.28 + (1 - spec.bang) * 0.08);
    ctx.beginPath();
    ctx.moveTo(head.x + 4, head.y + head.h * 0.22);
    if (spec.bang > 0.85) {
      ctx.lineTo(head.x + head.w - 4, head.y + head.h * 0.22);
      ctx.lineTo(head.x + head.w - 8, by + 8);
      ctx.lineTo(head.x + 8, by + 8);
    } else {
      const part = head.x + head.w * spec.part;
      ctx.quadraticCurveTo(part, by + 18, head.x + head.w - 4, head.y + head.h * 0.22);
      ctx.quadraticCurveTo(part, head.y + 4, head.x + 4, head.y + head.h * 0.22);
    }
    ctx.fill();
  }
  if (spec.spike > 0.2) {
    ctx.fillStyle = rgbStr(shade(rgb, 0.05));
    const n = 5 + Math.round(spec.spike * 6);
    for (let i = 0; i < n; i++) {
      const a = -0.9 + (i / (n - 1)) * 1.8;
      const x = cx + Math.sin(a) * rx * 0.7;
      ctx.beginPath();
      ctx.moveTo(x - 4, cy - ry * 0.2);
      ctx.lineTo(x, cy - ry - spec.spike * 28);
      ctx.lineTo(x + 4, cy - ry * 0.2);
      ctx.fill();
    }
  }
  if (spec.bun > 0.4) {
    const high = spec.bun > 0.9;
    const by = high ? head.y + 6 : head.y + head.h * 0.12;
    if (spec.bun >= 2) {
      ellipse(ctx, cx - rx * 0.55, by + 10, 16, 14);
      ellipse(ctx, cx + rx * 0.55, by + 10, 16, 14);
    } else {
      ellipse(ctx, cx, by, 22 + spec.puff * 8, 18);
    }
  }
  if (spec.tail > 0) {
    const ty = spec.tail > 0.85 ? head.y + 8 : head.y + head.h * 0.35;
    const fall = 40 + spec.len * 90;
    const drawTail = (x) => {
      ctx.beginPath();
      ctx.moveTo(x - 8, ty);
      ctx.quadraticCurveTo(x - 6, ty + fall * 0.5, x - 4, ty + fall);
      ctx.lineTo(x + 4, ty + fall);
      ctx.quadraticCurveTo(x + 6, ty + fall * 0.5, x + 8, ty);
      ctx.fill();
    };
    ctx.fillStyle = rgbStr(rgb);
    if (spec.tail >= 2) {
      drawTail(cx - rx * 0.7);
      drawTail(cx + rx * 0.7);
    } else {
      drawTail(view === "profile" ? head.x : cx);
    }
  }
  if (spec.braid > 0) {
    ctx.strokeStyle = rgbStr(shade(rgb, -0.2));
    ctx.lineWidth = 5;
    const count = spec.braid >= 2 ? 2 : 1;
    for (let i = 0; i < count; i++) {
      const x = count === 1 ? cx : cx + (i === 0 ? -rx * 0.55 : rx * 0.55);
      ctx.beginPath();
      ctx.moveTo(x, head.y + head.h * 0.3);
      for (let t = 0; t < 8; t++) {
        ctx.lineTo(x + (t % 2 ? 5 : -5), head.y + head.h * 0.3 + t * 14);
      }
      ctx.stroke();
    }
  }
  if (spec.rows > 0) {
    ctx.strokeStyle = rgbStr(shade(rgb, -0.15));
    ctx.lineWidth = 3;
    for (let i = 0; i < spec.rows; i++) {
      const x = head.x + 10 + (i / spec.rows) * (head.w - 20);
      ctx.beginPath();
      ctx.moveTo(x, head.y + 12);
      ctx.quadraticCurveTo(x + 4, head.y + head.h, x, head.y + head.h + spec.len * 80);
      ctx.stroke();
    }
  }
  if (spec.locs > 0.3) {
    ctx.strokeStyle = rgbStr(shade(rgb, -0.05));
    ctx.lineWidth = 4;
    const n = 8 + Math.round(spec.locs * 8);
    for (let i = 0; i < n; i++) {
      const x = head.x + 8 + (i / n) * (head.w - 16);
      ctx.beginPath();
      ctx.moveTo(x, head.y + 10);
      ctx.lineTo(x + (i % 2 ? 6 : -6), head.y + head.h + spec.len * 90);
      ctx.stroke();
    }
  }
  ctx.fillStyle = rgbStr(shade(rgb, 0.25), 0.28);
  ellipse(ctx, cx - rx * 0.25, cy - ry * 0.3, rx * 0.2, ry * 0.12);
  ctx.restore();
}

function drawEye(ctx, x, y, spec, color, lost, chrome, flip) {
  const w = 13 * spec.w;
  const h = 12 * spec.h;
  const tilt = spec.tilt * (flip ? -1 : 1);
  ctx.save();
  ctx.translate(x, y);
  ctx.rotate(tilt);
  if (lost) {
    ctx.strokeStyle = "rgba(40,20,16,0.85)";
    ctx.lineWidth = 1.4;
    ctx.beginPath();
    ctx.moveTo(-w, 0);
    ctx.lineTo(w, 0);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(-w * 0.4, -3);
    ctx.lineTo(w * 0.5, 4);
    ctx.stroke();
    ctx.restore();
    return;
  }
  ctx.fillStyle = chrome ? rgbStr(hexRgb(color)) : "#f2efe8";
  ctx.beginPath();
  ctx.ellipse(0, 0, w, h, 0, 0, Math.PI * 2);
  ctx.fill();
  if (chrome) {
    ctx.strokeStyle = "rgba(0,240,255,0.7)";
    ctx.lineWidth = 1;
    ctx.stroke();
    ctx.fillStyle = "#111";
    ellipse(ctx, 0, 0, w * 0.28, h * 0.28);
    ctx.restore();
    return;
  }
  const iris = hexRgb(color);
  ctx.fillStyle = rgbStr(iris);
  const iw = spec.slit ? w * 0.28 : w * 0.55;
  const ih = spec.slit === 2 ? h * 0.85 : spec.slit ? h * 0.9 : h * 0.7;
  ellipse(ctx, 0, 1, iw, ih);
  ctx.fillStyle = "#111";
  if (spec.slit === 1) ctx.fillRect(-1.2, -ih * 0.7, 2.4, ih * 1.4);
  else if (spec.slit === 2) ctx.fillRect(-iw * 0.9, -1.1, iw * 1.8, 2.2);
  else ellipse(ctx, 0, 1, 3.2, 3.2);
  ctx.fillStyle = "rgba(255,255,255,0.7)";
  ellipse(ctx, -2.4, -1.6, 1.6, 1.2);
  if (spec.glow) {
    ctx.shadowColor = rgbStr(iris, 0.9);
    ctx.shadowBlur = 8;
    ctx.fillStyle = rgbStr(iris, 0.35);
    ellipse(ctx, 0, 0, w, h);
    ctx.shadowBlur = 0;
  }
  ctx.fillStyle = "rgba(20,12,10,0.55)";
  ctx.beginPath();
  ctx.ellipse(0, -h * (0.35 + spec.lid), w * 1.05, h * (0.7 + spec.lid), 0, Math.PI, 0, true);
  ctx.fill();
  ctx.restore();
}

function metalFill(ctx, style, color) {
  const rgb = hexRgb(color);
  if (style === "neon") return rgbStr(rgb);
  if (style === "brass") return rgbStr(mixRgb(rgb, [180, 140, 60], 0.5));
  if (style === "carbon") return rgbStr(mixRgb(rgb, [20, 20, 22], 0.7));
  if (style === "ceramic") return rgbStr(mixRgb(rgb, [230, 228, 220], 0.6));
  if (style === "ivory") return rgbStr(mixRgb(rgb, [220, 210, 190], 0.7));
  if (style === "bone") return rgbStr(mixRgb(rgb, [200, 190, 170], 0.45));
  if (style === "steel") return rgbStr(mixRgb(rgb, [30, 34, 40], 0.65));
  return rgbStr(mixRgb(rgb, [180, 190, 200], 0.4));
}

function drawChromePart(ctx, b, slot, view) {
  if (!slot?.on) return;
  const x = b.x, y = b.y, w = b.w, h = b.h;
  ctx.save();
  ctx.fillStyle = metalFill(ctx, slot.style, slot.color);
  ctx.beginPath();
  roundRectPath(ctx, x, y, w, h, Math.min(w, h) * 0.2);
  ctx.fill();
  ctx.strokeStyle = "rgba(255,255,255,0.35)";
  ctx.lineWidth = 1;
  ctx.strokeRect(x + 3, y + 3, w - 6, h - 6);
  ctx.strokeStyle = "rgba(0,0,0,0.35)";
  for (let i = 1; i < 4; i++) {
    ctx.beginPath();
    ctx.moveTo(x + 4, y + (h * i) / 4);
    ctx.lineTo(x + w - 4, y + (h * i) / 4);
    ctx.stroke();
  }
  const rgb = hexRgb(slot.color);
  if (slot.style === "neon") {
    ctx.strokeStyle = rgbStr(rgb, 0.9);
    ctx.shadowColor = rgbStr(rgb);
    ctx.shadowBlur = 8;
    ctx.strokeRect(x + 5, y + 5, w - 10, h - 10);
    ctx.shadowBlur = 0;
  }
  ctx.fillStyle = "rgba(255,255,255,0.25)";
  ctx.fillRect(x + w * 0.15, y + 4, w * 0.12, h - 8);
  ctx.restore();
}

function drawFingers(ctx, hand, side, opts, view) {
  const loss = opts.loss || {};
  const chrome = opts.chrome || {};
  const names = side === "l"
    ? ["lThumb", "lIndex", "lMiddle", "lRing", "lPinky"]
    : ["rThumb", "rIndex", "rMiddle", "rRing", "rPinky"];
  const skin = skinOf(opts);
  const cx = hand.x + hand.w / 2;
  const cy = hand.y + hand.h * 0.35;
  const dir = side === "l" ? -1 : 1;
  names.forEach((id, i) => {
    const ang = (i - 2) * 0.18;
    const len = 10 + (i === 2 ? 4 : i === 0 ? -2 : 0);
    const x0 = cx + dir * (i - 2) * 5;
    const y0 = cy;
    const x1 = x0 + Math.sin(ang) * len * dir;
    const y1 = y0 + Math.cos(ang) * len + 8;
    if (loss[id] || (side === "l" && loss.lArm) || (side === "r" && loss.rArm)) {
      ctx.strokeStyle = "rgba(90,40,36,0.9)";
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(x0, y0);
      ctx.lineTo(x0 + dir * 3, y0 + 4);
      ctx.stroke();
      return;
    }
    const ch = chrome[id];
    ctx.strokeStyle = ch?.on ? metalFill(ctx, ch.style, ch.color) : rgbStr(shade(skin, -0.08));
    ctx.lineWidth = 3.2;
    ctx.lineCap = "round";
    ctx.beginPath();
    ctx.moveTo(x0, y0);
    ctx.lineTo(x1, y1);
    ctx.stroke();
    if (ch?.on) {
      ctx.strokeStyle = "rgba(255,255,255,0.4)";
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(x0, y0);
      ctx.lineTo(x1, y1);
      ctx.stroke();
    }
  });
}

function drawScars(ctx, L, opts) {
  const scars = opts.scars || {};
  const map = {
    face: L.head,
    neck: L.neck,
    chest: L.chest,
    belly: { x: L.waist.x, y: L.waist.y, w: L.waist.w, h: L.waist.h },
    lArm: L.lArm,
    rArm: L.rArm,
    lHand: L.lHand,
    rHand: L.rHand,
    lLeg: L.lLeg,
    rLeg: L.rLeg,
    back: L.chest,
  };
  for (const [part, kind] of Object.entries(scars)) {
    if (!kind || kind === "none") continue;
    const r = map[part];
    if (!r) continue;
    const b = box(r);
    ctx.save();
    ctx.strokeStyle = "rgba(90,40,36,0.8)";
    ctx.fillStyle = "rgba(90,40,36,0.35)";
    ctx.lineWidth = 1.2;
    const cx = b.x + b.w / 2;
    const cy = b.y + b.h / 2;
    if (kind === "cut" || kind === "surgical") {
      ctx.beginPath();
      ctx.moveTo(b.x + 4, cy - 8);
      ctx.lineTo(b.x + b.w - 4, cy + 8);
      ctx.stroke();
      if (kind === "suture" || kind === "surgical") {
        for (let i = 0; i < 6; i++) {
          const t = i / 5;
          const x = b.x + 4 + t * (b.w - 8);
          const y = cy - 8 + t * 16;
          ctx.beginPath();
          ctx.moveTo(x - 3, y - 3);
          ctx.lineTo(x + 3, y + 3);
          ctx.stroke();
        }
      }
    } else if (kind === "suture") {
      ctx.beginPath();
      ctx.moveTo(b.x + 6, b.y + 8);
      ctx.lineTo(b.x + b.w - 8, b.y + b.h - 8);
      ctx.stroke();
    } else if (kind === "claw") {
      for (let i = 0; i < 3; i++) {
        ctx.beginPath();
        ctx.moveTo(b.x + 8 + i * 7, b.y + 6);
        ctx.lineTo(b.x + 20 + i * 7, b.y + b.h - 8);
        ctx.stroke();
      }
    } else if (kind === "burn") {
      ctx.fillStyle = "rgba(70,40,32,0.45)";
      ellipse(ctx, cx, cy, b.w * 0.28, b.h * 0.22);
    } else if (kind === "pox") {
      for (let i = 0; i < 9; i++) {
        ellipse(ctx, b.x + 8 + (i * 13) % (b.w - 12), b.y + 8 + (i * 11) % (b.h - 12), 2.2, 2.2);
      }
    } else if (kind === "shrapnel") {
      ctx.fillStyle = "rgba(50,40,40,0.7)";
      for (let i = 0; i < 7; i++) {
        ctx.fillRect(b.x + 6 + i * 7, cy - 4 + (i % 3) * 3, 4, 2);
      }
    } else if (kind === "brand") {
      ctx.strokeRect(cx - 8, cy - 8, 16, 16);
    } else if (kind === "keloid") {
      ctx.fillStyle = "rgba(120,70,70,0.55)";
      ctx.beginPath();
      ctx.moveTo(b.x + 8, cy);
      ctx.quadraticCurveTo(cx, cy - 6, b.x + b.w - 8, cy + 4);
      ctx.quadraticCurveTo(cx, cy + 8, b.x + 8, cy);
      ctx.fill();
    }
    ctx.restore();
  }
}

function drawWear(ctx, L, wear, view) {
  const poly = (pts, fill, stroke) => {
    ctx.beginPath();
    ctx.moveTo(pts[0][0], pts[0][1]);
    for (let i = 1; i < pts.length; i++) ctx.lineTo(pts[i][0], pts[i][1]);
    ctx.closePath();
    ctx.fillStyle = fill;
    ctx.fill();
    if (stroke) {
      ctx.strokeStyle = stroke;
      ctx.lineWidth = 1;
      ctx.stroke();
    }
  };
  const c = box(L.chest);
  const wai = box(L.waist);
  const hi = box(L.hips);
  const ll = box(L.lLeg);
  const rl = box(L.rLeg);
  const lf = box(L.lFoot);
  const rf = box(L.rFoot);
  const la = box(L.lArm);
  const ra = box(L.rArm);
  const hd = box(L.head);

  if (wear.bottomOn && wear.bottom && wear.bottom !== "none") {
    const col = rgbStr(hexRgb(wear.bottomColor));
    const dk = rgbStr(shade(hexRgb(wear.bottomColor), -0.25));
    if (wear.bottom === "skirt" || wear.bottom === "kilt" || wear.bottom === "dress") {
      poly([[hi.x - 8, hi.y], [hi.x + hi.w + 8, hi.y], [rl.x + rl.w + 18, ll.y + ll.h * (wear.bottom === "dress" ? 0.7 : 0.35)], [ll.x - 18, ll.y + ll.h * (wear.bottom === "dress" ? 0.7 : 0.35)]], col, dk);
    } else {
      const short = wear.bottom === "shorts";
      poly([[hi.x, hi.y], [hi.x + hi.w, hi.y], [rl.x + rl.w, short ? rl.y + rl.h * 0.25 : rl.y + rl.h * 0.85], [rl.x, short ? rl.y + rl.h * 0.25 : rl.y + rl.h * 0.85], [ll.x + ll.w, short ? ll.y + ll.h * 0.25 : ll.y + ll.h * 0.85], [ll.x, short ? ll.y + ll.h * 0.25 : ll.y + ll.h * 0.85]], col, dk);
    }
  }
  if (wear.topOn && wear.top && wear.top !== "none") {
    const col = rgbStr(hexRgb(wear.topColor));
    const dk = rgbStr(shade(hexRgb(wear.topColor), -0.3));
    const low = wear.top === "tank" || wear.top === "harness" ? c.y + 6 : c.y - 4;
    poly([[c.x + 6, low], [c.x + c.w - 6, low], [wai.x + wai.w + 4, wai.y + wai.h], [wai.x - 4, wai.y + wai.h]], col, dk);
    if (wear.top === "mail" || wear.top === "armor") {
      ctx.strokeStyle = "rgba(180,190,200,0.5)";
      for (let y = c.y + 8; y < wai.y + wai.h; y += 6) {
        ctx.beginPath();
        ctx.moveTo(c.x + 10, y);
        ctx.lineTo(c.x + c.w - 10, y);
        ctx.stroke();
      }
    }
    if (wear.top === "hoodie" || wear.top === "jacket" || wear.top === "sweater") {
      ctx.fillStyle = col;
      ctx.fillRect(la.x + 2, la.y + 4, la.w - 4, la.h * 0.7);
      ctx.fillRect(ra.x + 2, ra.y + 4, ra.w - 4, ra.h * 0.7);
    }
    if (wear.top === "robe" || wear.bottom === "dress") {
      ctx.fillStyle = col;
      ctx.fillRect(c.x + 4, c.y, c.w - 8, hi.y + hi.h - c.y);
    }
  }
  if (wear.outerOn && wear.outer && wear.outer !== "none") {
    const col = rgbStr(hexRgb(wear.outerColor), 0.92);
    const dk = rgbStr(shade(hexRgb(wear.outerColor), -0.3));
    if (wear.outer === "cape" || wear.outer === "cloak") {
      poly([[hd.x + hd.w * 0.2, hd.y + hd.h * 0.7], [hd.x + hd.w * 0.8, hd.y + hd.h * 0.7], [c.x + c.w + 30, H - 40], [c.x - 30, H - 40]], col, dk);
    } else {
      poly([[c.x - 10, c.y], [c.x + c.w + 10, c.y], [hi.x + hi.w + 16, hi.y + hi.h + 20], [hi.x - 16, hi.y + hi.h + 20]], col, dk);
    }
  }
  if (wear.shoesOn && wear.shoes && wear.shoes !== "none") {
    const col = rgbStr(hexRgb(wear.shoesColor));
    ctx.fillStyle = col;
    const tall = wear.shoes === "boots" || wear.shoes === "sabatons" ? 22 : 10;
    ctx.beginPath();
    roundRectPath(ctx, lf.x - 2, lf.y - tall + lf.h, lf.w + 6, tall + 4, 6);
    ctx.fill();
    ctx.beginPath();
    roundRectPath(ctx, rf.x - 2, rf.y - tall + rf.h, rf.w + 6, tall + 4, 6);
    ctx.fill();
  }
  if (wear.glovesOn && wear.gloves && wear.gloves !== "none") {
    ctx.fillStyle = rgbStr(hexRgb(wear.glovesColor));
    const lh = box(L.lHand);
    const rh = box(L.rHand);
    ellipse(ctx, lh.x + lh.w / 2, lh.y + lh.h / 2, lh.w * 0.42, lh.h * 0.42);
    ellipse(ctx, rh.x + rh.w / 2, rh.y + rh.h / 2, rh.w * 0.42, rh.h * 0.42);
  }
}

function drawHeadwear(ctx, L, wear) {
  if (!wear.headOn || !wear.head || wear.head === "none") return;
  const hd = box(L.head);
  const col = rgbStr(hexRgb(wear.headColor));
  ctx.fillStyle = col;
  if (wear.head === "hood") {
    ctx.beginPath();
    ctx.ellipse(hd.x + hd.w / 2, hd.y + hd.h * 0.35, hd.w * 0.62, hd.h * 0.62, 0, Math.PI, 0);
    ctx.fill();
  } else if (wear.head === "helm") {
    ctx.beginPath();
    ctx.ellipse(hd.x + hd.w / 2, hd.y + hd.h * 0.4, hd.w * 0.55, hd.h * 0.5, 0, Math.PI, 0);
    ctx.fill();
    ctx.fillRect(hd.x + 8, hd.y + hd.h * 0.4, hd.w - 16, 10);
  } else if (wear.head === "circlet") {
    ctx.strokeStyle = col;
    ctx.lineWidth = 3;
    ctx.beginPath();
    ctx.ellipse(hd.x + hd.w / 2, hd.y + hd.h * 0.22, hd.w * 0.42, 5, 0, 0, Math.PI * 2);
    ctx.stroke();
  } else if (wear.head === "goggles") {
    ctx.fillStyle = "rgba(0, 40, 50, 0.7)";
    ctx.fillRect(hd.x + 18, hd.y + hd.h * 0.42, 28, 14);
    ctx.fillRect(hd.x + hd.w - 46, hd.y + hd.h * 0.42, 28, 14);
  } else {
    ctx.beginPath();
    ctx.ellipse(hd.x + hd.w / 2, hd.y + 12, hd.w * 0.48, 10, 0, 0, Math.PI * 2);
    ctx.fill();
  }
}

function maskLoss(ctx, L, loss) {
  const cut = (b, fromTop) => {
    const r = box(b);
    ctx.clearRect(r.x - 2, fromTop ? r.y : r.y + r.h * 0.15, r.w + 4, fromTop ? r.h + 8 : r.h);
  };
  ctx.save();
  ctx.globalCompositeOperation = "destination-out";
  if (loss.lArm) cut(L.lArm, false);
  if (loss.rArm) cut(L.rArm, false);
  if (loss.lLeg) cut(L.lLeg, false);
  if (loss.rLeg) cut(L.rLeg, false);
  ctx.restore();
  const stump = (b, t) => {
    const r = box(b);
    ctx.fillStyle = "rgba(150, 90, 80, 0.95)";
    ctx.beginPath();
    ctx.ellipse(r.x + r.w / 2, r.y + r.h * t, r.w * 0.28, 7, 0, 0, Math.PI * 2);
    ctx.fill();
    ctx.strokeStyle = "rgba(80,30,28,0.8)";
    ctx.stroke();
  };
  if (loss.lArm) stump(L.lArm, 0.18);
  if (loss.rArm) stump(L.rArm, 0.18);
  if (loss.lLeg) stump(L.lLeg, 0.12);
  if (loss.rLeg) stump(L.rLeg, 0.12);
}

function drawEars(ctx, head, opts, view) {
  const skin = skinOf(opts);
  const loss = opts.loss || {};
  const chrome = opts.chrome || {};
  const ear = (x, y, missing, chromeSlot, elf) => {
    if (missing) {
      ctx.fillStyle = rgbStr(shade(skin, -0.15));
      ellipse(ctx, x, y, 4, 5);
      ctx.strokeStyle = "rgba(90,40,36,0.8)";
      ctx.beginPath();
      ctx.arc(x, y, 5, 0.2, 2.8);
      ctx.stroke();
      return;
    }
    if (chromeSlot?.on) {
      ctx.fillStyle = metalFill(ctx, chromeSlot.style, chromeSlot.color);
      ellipse(ctx, x, y, 7, 10);
      return;
    }
    ctx.fillStyle = rgbStr(skin);
    if (elf || opts.race === "elf" || opts.race === "tiefling") {
      ctx.beginPath();
      ctx.moveTo(x, y + 8);
      ctx.lineTo(x + (x < W / 2 ? -16 : 16), y - 14);
      ctx.lineTo(x + (x < W / 2 ? 4 : -4), y + 4);
      ctx.fill();
    } else {
      ellipse(ctx, x, y, 7, 10);
    }
  };
  if (view === "profile") {
    ear(head.x + 6, head.y + head.h * 0.5, loss.rEar, chrome.rEar, false);
  } else {
    ear(head.x - 2, head.y + head.h * 0.5, loss.lEar, chrome.lEar, false);
    ear(head.x + head.w + 2, head.y + head.h * 0.5, loss.rEar, chrome.rEar, false);
  }
}

async function pickFace(opts) {
  const n = Math.max(0, Math.min(19, Number(opts.face) || 0));
  const race = opts.race || "human";
  const present = opts.present || "femme";
  return (
    (await loadFirst([
      `assets/creator/faces/pack-${present}-${n}.jpg`,
      `assets/creator/faces/pack-femme-${n}.jpg`,
    ])) ||
    (await loadFirst(faceCandidates(race, present, n))) ||
    (await loadFirst(faceCandidates("human", present, n))) ||
    (await loadImg(creatorPath(race, present))) ||
    (await loadImg(creatorPath("human", present))) ||
    (await loadImg(creatorPath("human", "femme")))
  );
}

function drawFacePlate(ctx, head, img, opts) {
  if (!img) return;
  const n = Math.max(0, Math.min(19, Number(opts.face) || 0));
  const hs = Math.max(0.7, Math.min(1.4, Number(opts.shape?.head) || 1));
  const dw = head.w * hs * 1.15;
  const dh = head.h * hs * 1.55;
  const dx = head.x + head.w / 2 - dw / 2;
  const dy = head.y - 4;
  ctx.save();
  ctx.beginPath();
  ctx.ellipse(head.x + head.w / 2, head.y + head.h * 0.52, dw * 0.48, dh * 0.48, 0, 0, Math.PI * 2);
  ctx.clip();
  const zoom = 1 + (n % 5) * 0.045;
  const ox = ((n * 7) % 11 - 5) / 90;
  const oy = ((n * 5) % 9 - 4) / 110;
  const sw = img.width / zoom;
  const sh = (img.height * 0.72) / zoom;
  const sx = (img.width - sw) * (0.5 + ox);
  const sy = Math.max(0, (img.height * 0.72 - sh) * (0.35 + oy));
  ctx.drawImage(img, sx, sy, sw, sh, dx, dy, dw, dh);
  ctx.restore();
  if (opts.race === "orc") {
    ctx.fillStyle = rgbStr(shade(skinOf(opts), 0.15));
    ctx.beginPath();
    ctx.moveTo(head.x + head.w * 0.32, head.y + head.h * 0.72);
    ctx.lineTo(head.x + head.w * 0.28, head.y + head.h * 0.88);
    ctx.lineTo(head.x + head.w * 0.36, head.y + head.h * 0.74);
    ctx.moveTo(head.x + head.w * 0.68, head.y + head.h * 0.72);
    ctx.lineTo(head.x + head.w * 0.72, head.y + head.h * 0.88);
    ctx.lineTo(head.x + head.w * 0.64, head.y + head.h * 0.74);
    ctx.fill();
  }
  if (opts.race === "tiefling" || opts.race === "succubus" || opts.race === "incubus") {
    ctx.fillStyle = rgbStr(shade(skinOf(opts), -0.2));
    const horn = (x, dir) => {
      ctx.beginPath();
      ctx.moveTo(x, head.y + head.h * 0.18);
      ctx.lineTo(x + dir * 10, head.y - 8);
      ctx.lineTo(x + dir * 4, head.y + head.h * 0.16);
      ctx.fill();
    };
    horn(head.x + head.w * 0.22, -1);
    horn(head.x + head.w * 0.78, 1);
  }
}

function composeFallback(raw) {
  const opts = mergeCreator(raw);
  const canvas = document.createElement("canvas");
  canvas.width = W;
  canvas.height = H;
  const ctx = canvas.getContext("2d");
  ctx.fillStyle = "#140010";
  ctx.fillRect(0, 0, W, H);
  try {
    const L = layout(opts.view === "profile" ? "profile" : "front");
    drawMannequin(ctx, L, skinOf(opts), opts.shape || {}, opts.view);
    drawHair(ctx, box(L.head), opts.hair || { style: 0, color: "#1c1410" }, opts.view);
  } catch {
    ctx.fillStyle = "#ff2d6a";
    ctx.fillRect(W * 0.3, H * 0.15, W * 0.4, H * 0.6);
  }
  return canvas.toDataURL("image/jpeg", 0.82);
}

export async function composePortrait(raw) {
  try {
    return await composePortraitRaw(raw);
  } catch (err) {
    console.warn("portrait", err);
    return composeFallback(raw);
  }
}

async function composePortraitRaw(raw) {
  const opts = mergeCreator(raw);
  const view = opts.view === "profile" ? "profile" : "front";
  const canvas = document.createElement("canvas");
  canvas.width = W;
  canvas.height = H;
  const ctx = canvas.getContext("2d");
  ctx.fillStyle = "#1a1a1c";
  ctx.fillRect(0, 0, W, H);

  const L = layout(view);
  const skin = skinOf(opts);
  const body = await loadBodyPhoto(opts, view);
  const bodyImg = body.img;
  const layer = document.createElement("canvas");
  layer.width = W;
  layer.height = H;
  const lx = layer.getContext("2d");
  lx.fillStyle = "#140010";
  lx.fillRect(0, 0, W, H);
  const shape = { ...(opts.shape || {}) };
  if (body.photobust) shape.chest = 1;
  drawMannequin(lx, L, skin, shape, view);
  if (bodyImg) lx.drawImage(bodyImg, 0, 0, W, H);
  const warped = warpCanvas(layer, shape);
  maskLoss(warped.getContext("2d"), L, opts.loss || {});
  ctx.drawImage(warped, 0, 0);

  const head = box(L.head);
  const faceImg = await pickFace(opts);
  if (faceImg) drawFacePlate(ctx, head, faceImg, opts);
  drawEars(ctx, head, opts, view);

  drawScars(ctx, L, opts);

  const chrome = opts.chrome || {};
  const loss = opts.loss || {};
  const mapping = [
    ["chest", L.chest],
    ["neck", L.neck],
    ["lUpperArm", L.lArm],
    ["rUpperArm", L.rArm],
    ["lForearm", L.lArm],
    ["rForearm", L.rArm],
    ["lThigh", L.lLeg],
    ["rThigh", L.rLeg],
    ["lCalf", L.lLeg],
    ["rCalf", L.rLeg],
    ["lFoot", L.lFoot],
    ["rFoot", L.rFoot],
    ["lHand", L.lHand],
    ["rHand", L.rHand],
    ["spine", L.chest],
    ["lShoulder", L.lArm],
    ["rShoulder", L.rArm],
    ["skull", L.head],
    ["jaw", L.head],
  ];
  const firstOn = (...ids) => ids.map((id) => chrome[id]).find((slot) => slot?.on);
  async function paintChrome(id, b, slot) {
    if (!slot?.on || !b) return;
    const cimg = await loadFirst(chromeCandidates(id, slot.style));
    if (cimg) colorizeOnto(ctx, cimg, b.x - 4, b.y - 4, b.w + 8, b.h + 8, slot.color, "tint");
    else drawChromePart(ctx, b, slot, view);
  }
  const lArmSlot = firstOn("lUpperArm", "lForearm", "lHand");
  const rArmSlot = firstOn("rUpperArm", "rForearm", "rHand");
  const lLegSlot = firstOn("lThigh", "lCalf", "lFoot");
  const rLegSlot = firstOn("rThigh", "rCalf", "rFoot");
  if (lArmSlot) await paintChrome("lUpperArm", box(L.lArm), lArmSlot);
  if (rArmSlot) await paintChrome("rUpperArm", box(L.rArm), rArmSlot);
  if (lLegSlot) await paintChrome("lThigh", box(L.lLeg), lLegSlot);
  if (rLegSlot) await paintChrome("rThigh", box(L.rLeg), rLegSlot);
  const grouped = new Set(["lUpperArm", "lForearm", "lHand", "rUpperArm", "rForearm", "rHand", "lThigh", "lCalf", "lFoot", "rThigh", "rCalf", "rFoot"]);
  for (const [id, region] of mapping) {
    if (grouped.has(id) || !chrome[id]?.on) continue;
    let b = box(region);
    if (id === "jaw") b = { x: b.x + b.w * 0.2, y: b.y + b.h * 0.62, w: b.w * 0.6, h: b.h * 0.32 };
    if (id === "skull") b = { x: b.x + 8, y: b.y, w: b.w - 16, h: b.h * 0.4 };
    if (id === "spine") b = { x: b.x + b.w * 0.42, y: b.y, w: b.w * 0.16, h: b.h * 1.4 };
    await paintChrome(id, b, chrome[id]);
  }

  if (!loss.lArm) drawFingers(ctx, box(L.lHand), "l", opts, view);
  if (!loss.rArm) drawFingers(ctx, box(L.rHand), "r", opts, view);

  const wear = opts.wear || {};
  const worn = { top: false, bottom: false, outer: false, shoes: false, gloves: false, head: false };
  for (const slot of ["bottom", "top", "outer", "shoes", "gloves"]) {
    if (!wear[slot + "On"] || !wear[slot] || wear[slot] === "none") continue;
    const wimg = await loadFirst(wearCandidates(slot, wear[slot], view));
    if (wimg) {
      colorizeOnto(ctx, wimg, 0, 0, W, H, wear[slot + "Color"], "tint");
      worn[slot] = true;
    }
  }
  const fallbackWear = { ...wear, headOn: false };
  for (const slot of ["bottom", "top", "outer", "shoes", "gloves"]) {
    if (worn[slot]) fallbackWear[slot + "On"] = false;
  }
  drawWear(ctx, L, fallbackWear, view);

  const eyeSpec = EYES[opts.eyes?.shape] || EYES[0];
  const eyeY = head.y + head.h * 0.48;
  const spread = opts.eyes?.shape === 7 ? 0.28 : opts.eyes?.shape === 8 ? 0.18 : 0.22;
  if (view === "profile") {
    drawEye(ctx, head.x + head.w * 0.72, eyeY, eyeSpec, opts.eyes.color, loss.rEye, chrome.rEye?.on, false);
  } else {
    drawEye(ctx, head.x + head.w * (0.5 - spread), eyeY, eyeSpec, opts.eyes.color, loss.lEye, chrome.lEye?.on, true);
    drawEye(ctx, head.x + head.w * (0.5 + spread), eyeY, eyeSpec, opts.eyes.colorR || opts.eyes.color, loss.rEye, chrome.rEye?.on, false);
  }

  const hairImg = await loadFirst(hairCandidates(opts.hair?.style ?? 0, view));
  if (hairImg) {
    const hb = box(L.head);
    colorizeOnto(ctx, hairImg, hb.x - hb.w * 0.45, hb.y - hb.h * 0.55, hb.w * 1.9, hb.h * 2.4, opts.hair?.color || "#1c1410");
  } else {
    drawHair(ctx, head, opts.hair || { style: 0, color: "#111" }, view);
  }
  if (wear.headOn && wear.head && wear.head !== "none") {
    const wimg = await loadFirst(wearCandidates("head", wear.head, view));
    if (wimg) colorizeOnto(ctx, wimg, 0, 0, W, H, wear.headColor, "tint");
    else drawHeadwear(ctx, L, wear);
  }

  return canvas.toDataURL("image/jpeg", 0.86);
}

export async function composePortraitHead(raw) {
  const url = await composePortrait(raw);
  return new Promise((resolve) => {
    const im = new Image();
    im.onload = () => {
      const c = document.createElement("canvas");
      c.width = 384;
      c.height = 512;
      const cx = c.getContext("2d");
      cx.drawImage(im, 0, 0, 384, 420, 0, 0, 384, 512);
      resolve(c.toDataURL("image/jpeg", 0.86));
    };
    im.onerror = () => resolve(url);
    im.src = url;
  });
}
