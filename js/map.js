function catalogSrc(kind, ref) {
  const id = String(ref || "").replace(/[^a-z0-9_-]/gi, "");
  if (!id) return "";
  if (kind === "beast" || kind === "srdnpc") return `assets/bestiary/${id}.jpg`;
  if (kind === "shard") return `assets/datashard/${id}.jpg`;
  if (kind === "npc") return `assets/npcs/${id}.jpg`;
  if (kind === "god") return `assets/gods/${id}.jpg`;
  if (kind === "lore") return `assets/lore/${id}.jpg`;
  return "";
}

function portableSrc(src, spec = {}) {
  const raw = String(src || "");
  if (raw.startsWith("data:")) return raw;
  if (raw.startsWith("blob:")) return catalogSrc(spec.kind, spec.ref);
  const asset = raw.match(/(?:^|\/)(assets\/[^\s"'?]+(?:\?[^"'#]*)?)/i);
  if (asset) return asset[1];
  if (raw.startsWith("assets/")) return raw;
  return catalogSrc(spec.kind, spec.ref) || "";
}

function copyToken(t) {
  const row = { ...t };
  row.src = portableSrc(row.src, row);
  return row;
}

function copyDrawing(d) {
  return { ...d, points: d.points ? d.points.map((p) => ({ ...p })) : undefined };
}

export function createMapTable(hooks) {
  const state = {
    maps: new Map(),
    activeId: null,
    view: { x: 0, y: 0, scale: 1 },
    grid: { on: true, size: 70 },
    drag: null,
    applying: false,
    tool: "pan",
    selected: null,
    aim: null,
  };

  const TOKEN_MIN = 16;
  const TOKEN_MAX = 640;
  let markTimer = 0;
  let pendingRemote = null;

  const HINTS = {
    pan: "Drag to pan. Scroll the wheel to zoom. Drop pictures to place them. Drag a token to move it. Drag its gold corner to resize, or Alt+scroll on it.",
    draw: "Draw freehand on the map.",
    circle: "Drag to draw a circle.",
    rect: "Drag to draw a rectangle.",
    erase: "Click a token or a mark to delete it.",
  };

  const board = () => document.getElementById("map-board");
  const stage = () => document.getElementById("map-stage");
  const world = () => document.getElementById("map-world");
  const img = () => document.getElementById("map-image");
  const grid = () => document.getElementById("map-grid-canvas");
  const drawEl = () => document.getElementById("map-draw-canvas");
  const tokensEl = () => document.getElementById("map-tokens");
  const select = () => document.getElementById("map-select");
  const hint = () => document.getElementById("map-hint");

  function uid(prefix) {
    return prefix + Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
  }

  function active() {
    return state.activeId ? state.maps.get(state.activeId) : null;
  }

  function marksOf(rec) {
    if (!rec.tokens) rec.tokens = [];
    if (!rec.drawings) rec.drawings = [];
    return rec;
  }

  function ink() {
    const rgb = getComputedStyle(document.documentElement).getPropertyValue("--gold-rgb").trim() || "226, 179, 74";
    return `rgb(${rgb})`;
  }

  function letterToken(name) {
    const c = document.createElement("canvas");
    c.width = 128;
    c.height = 128;
    const ctx = c.getContext("2d");
    ctx.fillStyle = "#1a120c";
    ctx.fillRect(0, 0, 128, 128);
    ctx.strokeStyle = ink();
    ctx.lineWidth = 6;
    ctx.strokeRect(6, 6, 116, 116);
    ctx.fillStyle = ink();
    ctx.font = "700 64px sans-serif";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(String(name || "?").slice(0, 1).toUpperCase(), 64, 70);
    return c.toDataURL("image/png");
  }

  function snapshot() {
    const rec = active();
    if (!rec) return null;
    marksOf(rec);
    return {
      id: rec.id,
      name: rec.name,
      mime: rec.mime,
      w: rec.w,
      h: rec.h,
      view: { ...state.view },
      grid: { ...state.grid },
      tokens: rec.tokens.map(copyToken),
      drawings: rec.drawings.map(copyDrawing),
    };
  }

  function emitMarks() {
    window.clearTimeout(markTimer);
    markTimer = 0;
    if (state.applying) return;
    const snap = snapshot();
    if (!snap) return;
    if (hooks.canControl?.()) hooks.onChange?.(snap);
    else hooks.onMarks?.(snap);
  }

  function emitMarksSoon() {
    window.clearTimeout(markTimer);
    markTimer = window.setTimeout(emitMarks, 80);
  }

  function clampTokenSize(n) {
    const v = Number(n);
    if (!Number.isFinite(v)) return Math.max(TOKEN_MIN, Number(state.grid.size) || 70);
    return Math.max(TOKEN_MIN, Math.min(TOKEN_MAX, v));
  }

  function applyTransform() {
    const w = world();
    if (!w) return;
    const { x, y, scale } = state.view;
    w.style.transform = `translate(${x}px, ${y}px) scale(${scale})`;
    w.style.setProperty("--map-scale", String(scale));
    drawGrid();
    paintDrawings();
  }

  function syncHint() {
    const el = hint();
    if (el) el.textContent = HINTS[state.tool] || HINTS.pan;
    const st = stage();
    if (st) st.dataset.tool = state.tool;
    for (const btn of document.querySelectorAll("[data-map-tool]")) {
      btn.classList.toggle("on", btn.dataset.mapTool === state.tool);
    }
  }

  function setTool(tool) {
    state.tool = tool || "pan";
    state.drag = null;
    if (tool !== "pan") state.selected = null;
    paintTokens();
    syncHint();
  }

  function drawGrid() {
    const canvas = grid();
    const rec = active();
    if (!canvas || !rec || typeof canvas.getContext !== "function") return;
    const size = Math.max(8, Number(state.grid.size) || 70);
    if (canvas.width !== rec.w) canvas.width = rec.w;
    if (canvas.height !== rec.h) canvas.height = rec.h;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, rec.w, rec.h);
    if (!state.grid.on) {
      canvas.hidden = true;
      return;
    }
    canvas.hidden = false;
    ctx.strokeStyle = "rgba(252, 238, 10, 0.28)";
    ctx.lineWidth = Math.max(1, rec.w / 1200);
    ctx.beginPath();
    for (let x = 0; x <= rec.w; x += size) {
      ctx.moveTo(x, 0);
      ctx.lineTo(x, rec.h);
    }
    for (let y = 0; y <= rec.h; y += size) {
      ctx.moveTo(0, y);
      ctx.lineTo(rec.w, y);
    }
    ctx.stroke();
  }

  function strokeWidth(rec) {
    return Math.max(2, (rec.w || 1000) / 280);
  }

  function drawShape(ctx, d, color) {
    ctx.strokeStyle = d.color || color;
    ctx.lineWidth = d.width || 4;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    if (d.type === "path" && d.points?.length) {
      ctx.beginPath();
      ctx.moveTo(d.points[0].x, d.points[0].y);
      for (let i = 1; i < d.points.length; i++) ctx.lineTo(d.points[i].x, d.points[i].y);
      ctx.stroke();
    } else if (d.type === "circle") {
      ctx.beginPath();
      ctx.arc(d.x, d.y, Math.max(1, d.r || 0), 0, Math.PI * 2);
      ctx.stroke();
    } else if (d.type === "rect") {
      ctx.strokeRect(d.x, d.y, d.w || 0, d.h || 0);
    }
  }

  function paintDrawings() {
    const canvas = drawEl();
    const rec = active();
    if (!canvas || !rec || typeof canvas.getContext !== "function") return;
    marksOf(rec);
    if (canvas.width !== rec.w) canvas.width = rec.w;
    if (canvas.height !== rec.h) canvas.height = rec.h;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, rec.w, rec.h);
    const color = ink();
    for (const d of rec.drawings) drawShape(ctx, d, color);
    if (state.drag?.preview) drawShape(ctx, state.drag.preview, color);
  }

  function paintTokens() {
    const root = tokensEl();
    const rec = active();
    if (!root) return;
    if (!rec) {
      root.innerHTML = "";
      return;
    }
    marksOf(rec);
    const worldEl = world();
    if (worldEl && root.parentElement === worldEl) worldEl.appendChild(root);
    for (const t of rec.tokens) t.size = clampTokenSize(t.size);
    const ids = rec.tokens.map((t) => t.id).join("|");
    if (root.dataset.ids !== ids) {
      root.dataset.ids = ids;
      root.innerHTML = rec.tokens
        .map((t) => {
          const on = t.id === state.selected ? " on" : "";
          const src = portableSrc(t.src, t) || letterToken(t.name);
          const sheetId = t.sheetId || (t.kind === "char" ? t.ref : "") || "";
          const sheet = sheetId ? ` data-sheet-id="${escape(sheetId)}" data-sheet-owner="${escape(t.ownerId || "")}"` : "";
          const aim = state.aim && sheetId ? " aimable" : "";
          const vitals = hooks?.tokenStatus?.(t) || "";
          const vitalsCls = vitals === "dead" ? " is-dead" : vitals === "downed" ? " is-down" : "";
          const vitalsTitle = vitals === "dead" ? "Dead. " : vitals === "downed" ? "Downed. " : "";
          return `<button type="button" class="map-token${on}${aim}${vitalsCls}" data-token="${escape(t.id)}"${sheet} title="${escape(vitalsTitle + (t.name || "Token"))} — drag the gold corner or Alt+scroll to resize" style="left:${t.x - t.size / 2}px;top:${t.y - t.size / 2}px;width:${t.size}px;height:${t.size}px"><img src="${escape(src)}" alt="" draggable="false" /><span class="map-token-handle" data-token-handle="${escape(t.id)}" aria-hidden="true"></span></button>`;
        })
        .join("");
      return;
    }
    for (const t of rec.tokens) {
      const el = root.querySelector(`[data-token="${CSS.escape(t.id)}"]`);
      if (!el) continue;
      el.classList.toggle("on", t.id === state.selected);
      el.classList.toggle("aimable", Boolean(state.aim && (t.sheetId || (t.kind === "char" && t.ref))));
      const vitals = hooks?.tokenStatus?.(t) || "";
      el.classList.toggle("is-down", vitals === "downed");
      el.classList.toggle("is-dead", vitals === "dead");
      const vitalsTitle = vitals === "dead" ? "Dead. " : vitals === "downed" ? "Downed. " : "";
      el.title = `${vitalsTitle}${t.name || "Token"} — drag the gold corner or Alt+scroll to resize`;
      if (t.sheetId) el.dataset.sheetId = t.sheetId;
      el.style.left = t.x - t.size / 2 + "px";
      el.style.top = t.y - t.size / 2 + "px";
      el.style.width = t.size + "px";
      el.style.height = t.size + "px";
      const im = el.querySelector("img");
      const src = portableSrc(t.src, t) || letterToken(t.name);
      if (im && im.getAttribute("src") !== src) im.src = src;
      if (!el.querySelector("[data-token-handle]")) {
        el.insertAdjacentHTML("beforeend", `<span class="map-token-handle" data-token-handle="${escape(t.id)}" aria-hidden="true"></span>`);
      }
    }
  }

  function fillSelect() {
    const el = select();
    if (!el) return;
    const ids = [...state.maps.keys()];
    el.innerHTML = ids.length
      ? ids
          .map((id) => {
            const rec = state.maps.get(id);
            const sel = id === state.activeId ? " selected" : "";
            return `<option value="${id}"${sel}>${escape(rec.name)}</option>`;
          })
          .join("")
      : `<option value="">No maps</option>`;
  }

  function escape(value) {
    return String(value).replace(/[&<>"']/g, (ch) => ({
      "&": "&amp;",
      "<": "&lt;",
      ">": "&gt;",
      '"': "&quot;",
      "'": "&#39;",
    }[ch]));
  }

  function openBoard() {
    const el = board();
    if (!el) return;
    el.hidden = false;
    document.querySelector(".app")?.classList.add("map-open");
    syncHint();
  }

  function markHasMap(on) {
    board()?.classList.toggle("has-map", Boolean(on));
  }

  function closeBoard(broadcast = true) {
    const el = board();
    if (el) el.hidden = true;
    document.querySelector(".app")?.classList.remove("map-open");
    markHasMap(false);
    state.activeId = null;
    state.selected = null;
    if (broadcast) pendingRemote = null;
    const image = img();
    if (image) image.removeAttribute("src");
    paintTokens();
    if (broadcast) hooks.onChange?.(null);
  }

  function fit() {
    const rec = active();
    const st = stage();
    if (!rec || !st) return;
    const box = st.getBoundingClientRect();
    const scale = Math.min(box.width / rec.w, box.height / rec.h) * 0.96;
    state.view.scale = Math.max(0.08, Math.min(8, scale));
    state.view.x = (box.width - rec.w * state.view.scale) / 2;
    state.view.y = (box.height - rec.h * state.view.scale) / 2;
    applyTransform();
  }

  function show(id, opts = {}) {
    const rec = state.maps.get(id);
    if (!rec) return;
    state.activeId = id;
    if (opts.view) state.view = { ...state.view, ...opts.view };
    if (opts.grid) state.grid = { ...state.grid, ...opts.grid };
    if (opts.tokens) rec.tokens = opts.tokens.map((t) => ({ ...t }));
    if (opts.drawings) rec.drawings = opts.drawings.map((d) => ({ ...d }));
    openBoard();
    markHasMap(Boolean(rec.url));
    const image = img();
    const worldEl = world();
    if (image && rec.url) {
      image.onload = () => {
        rec.w = image.naturalWidth || rec.w;
        rec.h = image.naturalHeight || rec.h;
        if (worldEl) {
          worldEl.style.width = rec.w + "px";
          worldEl.style.height = rec.h + "px";
        }
        if (!opts.keepView) fit();
        else applyTransform();
        paintTokens();
        paintDrawings();
        fillSelect();
        if (!state.applying && !opts.silent) hooks.onChange?.(snapshot());
      };
      image.src = rec.url;
    } else {
      applyTransform();
      paintTokens();
      paintDrawings();
    }
    fillSelect();
  }

  function mergePending(rec) {
    const pending = pendingRemote;
    if (!pending || pending.id !== rec.id) return rec;
    if (pending.tokens) rec.tokens = pending.tokens.map(copyToken);
    if (pending.drawings) rec.drawings = pending.drawings.map(copyDrawing);
    return rec;
  }

  function addMap(rec) {
    const prev = state.maps.get(rec.id);
    if (prev?.url && prev.url !== rec.url && String(prev.url).startsWith("blob:")) {
      URL.revokeObjectURL(prev.url);
    }
    rec.tokens = rec.tokens || prev?.tokens || [];
    rec.drawings = rec.drawings || prev?.drawings || [];
    mergePending(rec);
    state.maps.set(rec.id, rec);
    fillSelect();
    return rec;
  }

  function forget(id) {
    const rec = state.maps.get(id);
    if (rec?.url && String(rec.url).startsWith("blob:")) URL.revokeObjectURL(rec.url);
    state.maps.delete(id);
    if (state.activeId === id) closeBoard(true);
    fillSelect();
  }

  function clearEphemeral() {
    for (const [id, rec] of state.maps) {
      if (rec.ephemeral) forget(id);
    }
  }

  function clientToWorld(clientX, clientY) {
    const st = stage();
    if (!st) return { x: 0, y: 0 };
    const box = st.getBoundingClientRect();
    const mx = clientX - box.left;
    const my = clientY - box.top;
    return {
      x: (mx - state.view.x) / state.view.scale,
      y: (my - state.view.y) / state.view.scale,
    };
  }

  function addToken(spec, x, y) {
    const rec = active();
    if (!rec) return null;
    marksOf(rec);
    const size = clampTokenSize(Math.max(40, Number(state.grid.size) || 70));
    const src = spec.src || letterToken(spec.name);
    rec.tokens.push({
      id: uid("t"),
      src: portableSrc(src, spec) || src,
      name: spec.name || "",
      kind: spec.kind || "token",
      ref: spec.ref || "",
      sheetId: spec.sheetId || (spec.kind === "char" ? spec.ref : "") || "",
      ownerId: spec.ownerId || "",
      x,
      y,
      size,
    });
    paintTokens();
    emitMarks();
    const tok = rec.tokens[rec.tokens.length - 1];
    hooks.onToken?.(tok, spec);
    return tok;
  }

  function imageFileToToken(file) {
    return new Promise((resolve, reject) => {
      const url = URL.createObjectURL(file);
      const im = new Image();
      im.onload = () => {
        let w = im.width;
        let h = im.height;
        const long = Math.max(w, h);
        if (long > 256) {
          const s = 256 / long;
          w = Math.max(1, Math.round(w * s));
          h = Math.max(1, Math.round(h * s));
        }
        const c = document.createElement("canvas");
        c.width = w;
        c.height = h;
        c.getContext("2d").drawImage(im, 0, 0, w, h);
        URL.revokeObjectURL(url);
        resolve(c.toDataURL("image/jpeg", 0.82));
      };
      im.onerror = () => {
        URL.revokeObjectURL(url);
        reject(new Error("Could not read that picture."));
      };
      im.src = url;
    });
  }

  function removeToken(id) {
    const rec = active();
    if (!rec) return;
    marksOf(rec);
    rec.tokens = rec.tokens.filter((t) => t.id !== id);
    if (state.selected === id) state.selected = null;
    paintTokens();
    emitMarks();
  }

  function hitDrawing(x, y) {
    const rec = active();
    if (!rec) return null;
    marksOf(rec);
    const tol = Math.max(8, strokeWidth(rec) * 2);
    for (let i = rec.drawings.length - 1; i >= 0; i--) {
      const d = rec.drawings[i];
      if (d.type === "circle") {
        const dist = Math.hypot(x - d.x, y - d.y);
        if (Math.abs(dist - (d.r || 0)) <= tol || dist <= Math.min(tol, d.r || 0)) return d;
      } else if (d.type === "rect") {
        const left = Math.min(d.x, d.x + d.w);
        const right = Math.max(d.x, d.x + d.w);
        const top = Math.min(d.y, d.y + d.h);
        const bottom = Math.max(d.y, d.y + d.h);
        const near =
          (x >= left - tol && x <= right + tol && Math.abs(y - top) <= tol) ||
          (x >= left - tol && x <= right + tol && Math.abs(y - bottom) <= tol) ||
          (y >= top - tol && y <= bottom + tol && Math.abs(x - left) <= tol) ||
          (y >= top - tol && y <= bottom + tol && Math.abs(x - right) <= tol);
        if (near) return d;
      } else if (d.type === "path" && d.points?.length) {
        for (let p = 1; p < d.points.length; p++) {
          const a = d.points[p - 1];
          const b = d.points[p];
          const dx = b.x - a.x;
          const dy = b.y - a.y;
          const len2 = dx * dx + dy * dy || 1;
          let t = ((x - a.x) * dx + (y - a.y) * dy) / len2;
          t = Math.max(0, Math.min(1, t));
          const px = a.x + t * dx;
          const py = a.y + t * dy;
          if (Math.hypot(x - px, y - py) <= tol) return d;
        }
      }
    }
    return null;
  }

  function eraseAt(x, y) {
    const rec = active();
    if (!rec) return;
    marksOf(rec);
    const tok = rec.tokens.find((t) => Math.hypot(x - t.x, y - t.y) <= t.size / 2);
    if (tok) {
      removeToken(tok.id);
      return;
    }
    const d = hitDrawing(x, y);
    if (d) {
      rec.drawings = rec.drawings.filter((row) => row.id !== d.id);
      paintDrawings();
      emitMarks();
    }
  }

  function clearMarks() {
    const rec = active();
    if (!rec) return;
    rec.tokens = [];
    rec.drawings = [];
    state.selected = null;
    paintTokens();
    paintDrawings();
    emitMarks();
  }

  function applyMarks(edit) {
    if (!edit) return false;
    let rec = edit.id ? state.maps.get(edit.id) : active();
    if (!rec) {
      pendingRemote = {
        id: edit.id,
        view: pendingRemote?.view || null,
        grid: pendingRemote?.grid || null,
        tokens: Array.isArray(edit.tokens) ? edit.tokens.map(copyToken) : pendingRemote?.tokens || null,
        drawings: Array.isArray(edit.drawings) ? edit.drawings.map(copyDrawing) : pendingRemote?.drawings || null,
      };
      return false;
    }
    state.applying = true;
    if (Array.isArray(edit.tokens)) rec.tokens = edit.tokens.map(copyToken);
    if (Array.isArray(edit.drawings)) rec.drawings = edit.drawings.map(copyDrawing);
    if (rec.id === state.activeId) {
      paintTokens();
      paintDrawings();
    }
    state.applying = false;
    return true;
  }

  function hydrate(id) {
    const rec = id ? state.maps.get(id) : null;
    if (!rec) return false;
    mergePending(rec);
    const pending = pendingRemote;
    const want = !state.activeId || state.activeId === rec.id || pending?.id === rec.id;
    if (!want) return false;
    const already = state.activeId === rec.id && Boolean(img()?.getAttribute("src"));
    if (already) {
      if (pending?.id === rec.id && pending.view) state.view = { ...state.view, ...pending.view };
      if (pending?.id === rec.id && pending.grid) state.grid = { ...state.grid, ...pending.grid };
      applyTransform();
      paintTokens();
      paintDrawings();
      fillSelect();
      markHasMap(Boolean(rec.url));
      return true;
    }
    show(rec.id, {
      keepView: true,
      silent: true,
      view: pending?.id === rec.id ? pending.view || undefined : undefined,
      grid: pending?.id === rec.id ? pending.grid || undefined : undefined,
      tokens: rec.tokens,
      drawings: rec.drawings,
    });
    return true;
  }

  function bind() {
    const st = stage();
    if (!st || st.dataset.ready) return;
    st.dataset.ready = "1";
    st.addEventListener("dragover", (e) => {
      e.preventDefault();
      e.dataTransfer.dropEffect = "copy";
    });
    st.addEventListener("drop", async (e) => {
      e.preventDefault();
      const files = [...(e.dataTransfer.files || [])].filter((f) => /^image\//.test(f.type || ""));
      if (!active()) {
        if (files[0]) hooks.onImport?.(files[0]);
        return;
      }
      const pt = clientToWorld(e.clientX, e.clientY);
      if (files.length) {
        for (let i = 0; i < files.length; i++) {
          const file = files[i];
          try {
            const src = await imageFileToToken(file);
            addToken(
              {
                kind: "token",
                name: String(file.name || "Token").replace(/\.[^.]+$/, ""),
                src,
              },
              pt.x + i * 16,
              pt.y + i * 16
            );
          } catch {
            /* skip unread pictures */
          }
        }
        return;
      }
      const raw = e.dataTransfer.getData("application/x-hearth-token") || e.dataTransfer.getData("text/plain");
      if (!raw) return;
      let spec;
      try {
        spec = JSON.parse(raw);
      } catch {
        return;
      }
      if (!spec || typeof spec !== "object") return;
      addToken(spec, pt.x, pt.y);
    });
    st.addEventListener("pointerdown", (e) => {
      if (!active()) return;
      const rec = active();
      marksOf(rec);
      const pt = clientToWorld(e.clientX, e.clientY);
      const handle = e.target.closest("[data-token-handle]");
      const tokEl = e.target.closest(".map-token");
      if (state.aim && tokEl && e.button === 0) {
        const tok = rec.tokens.find((t) => t.id === tokEl.dataset.token);
        if (tok && (tok.sheetId || (tok.kind === "char" && tok.ref))) {
          e.preventDefault();
          e.stopPropagation();
          const fn = state.aim;
          state.aim = null;
          const stg = stage();
          if (stg) stg.classList.remove("aiming");
          paintTokens();
          fn(tok);
          return;
        }
      }
      if (state.tool === "erase") {
        eraseAt(pt.x, pt.y);
        return;
      }
      if (state.tool === "pan" && (handle || tokEl)) {
        const id = handle?.dataset.tokenHandle || tokEl.dataset.token;
        const tok = rec.tokens.find((t) => t.id === id);
        if (!tok) return;
        state.selected = tok.id;
        paintTokens();
        st.setPointerCapture(e.pointerId);
        if (handle || e.altKey) {
          state.drag = { kind: "resize", id: tok.id };
          return;
        }
        state.drag = { kind: "token", id: tok.id, dx: pt.x - tok.x, dy: pt.y - tok.y };
        return;
      }
      if (state.tool === "pan") {
        state.selected = null;
        paintTokens();
        st.setPointerCapture(e.pointerId);
        state.drag = { kind: "pan", x: e.clientX, y: e.clientY, vx: state.view.x, vy: state.view.y };
        return;
      }
      if (state.tool === "draw") {
        st.setPointerCapture(e.pointerId);
        const stroke = {
          id: uid("d"),
          type: "path",
          color: ink(),
          width: strokeWidth(rec),
          points: [{ x: pt.x, y: pt.y }],
        };
        state.drag = { kind: "draw", preview: stroke };
        paintDrawings();
        return;
      }
      if (state.tool === "circle") {
        st.setPointerCapture(e.pointerId);
        state.drag = {
          kind: "circle",
          preview: { id: uid("d"), type: "circle", color: ink(), width: strokeWidth(rec), x: pt.x, y: pt.y, r: 0 },
        };
        paintDrawings();
        return;
      }
      if (state.tool === "rect") {
        st.setPointerCapture(e.pointerId);
        state.drag = {
          kind: "rect",
          ox: pt.x,
          oy: pt.y,
          preview: { id: uid("d"), type: "rect", color: ink(), width: strokeWidth(rec), x: pt.x, y: pt.y, w: 0, h: 0 },
        };
        paintDrawings();
      }
    });
    st.addEventListener("pointermove", (e) => {
      if (!state.drag) return;
      const rec = active();
      if (!rec) return;
      const pt = clientToWorld(e.clientX, e.clientY);
      if (state.drag.kind === "pan") {
        state.view.x = state.drag.vx + (e.clientX - state.drag.x);
        state.view.y = state.drag.vy + (e.clientY - state.drag.y);
        applyTransform();
        if (hooks.canControl?.()) hooks.onView?.(snapshot());
        return;
      }
      if (state.drag.kind === "token") {
        const tok = rec.tokens.find((t) => t.id === state.drag.id);
        if (!tok) return;
        tok.x = pt.x - state.drag.dx;
        tok.y = pt.y - state.drag.dy;
        paintTokens();
        emitMarksSoon();
        return;
      }
      if (state.drag.kind === "resize") {
        const tok = rec.tokens.find((t) => t.id === state.drag.id);
        if (!tok) return;
        tok.size = clampTokenSize(Math.hypot(pt.x - tok.x, pt.y - tok.y) * 2);
        paintTokens();
        emitMarksSoon();
        return;
      }
      if (state.drag.kind === "draw" && state.drag.preview) {
        const last = state.drag.preview.points[state.drag.preview.points.length - 1];
        if (!last || Math.hypot(pt.x - last.x, pt.y - last.y) >= 1.5) state.drag.preview.points.push({ x: pt.x, y: pt.y });
        paintDrawings();
        return;
      }
      if (state.drag.kind === "circle" && state.drag.preview) {
        state.drag.preview.r = Math.hypot(pt.x - state.drag.preview.x, pt.y - state.drag.preview.y);
        paintDrawings();
        return;
      }
      if (state.drag.kind === "rect" && state.drag.preview) {
        state.drag.preview.x = Math.min(state.drag.ox, pt.x);
        state.drag.preview.y = Math.min(state.drag.oy, pt.y);
        state.drag.preview.w = Math.abs(pt.x - state.drag.ox);
        state.drag.preview.h = Math.abs(pt.y - state.drag.oy);
        paintDrawings();
      }
    });
    const endDrag = () => {
      if (!state.drag) return;
      const rec = active();
      const drag = state.drag;
      state.drag = null;
      if (!rec) return;
      marksOf(rec);
      if (drag.kind === "pan") {
        if (hooks.canControl?.()) hooks.onChange?.(snapshot());
        return;
      }
      if (drag.kind === "token" || drag.kind === "resize") {
        emitMarks();
        return;
      }
      if (drag.preview) {
        const p = drag.preview;
        const empty =
          (p.type === "circle" && (p.r || 0) < 2) ||
          (p.type === "rect" && Math.abs(p.w || 0) < 2 && Math.abs(p.h || 0) < 2) ||
          (p.type === "path" && (p.points?.length || 0) < 2);
        if (!empty) rec.drawings.push(p);
        paintDrawings();
        emitMarks();
      }
    };
    st.addEventListener("pointerup", endDrag);
    st.addEventListener("pointercancel", endDrag);
    tokensEl()?.addEventListener(
      "error",
      (e) => {
        const im = e.target;
        if (!(im instanceof HTMLImageElement)) return;
        if (im.dataset.fallback) return;
        im.dataset.fallback = "1";
        const rec = active();
        const id = im.closest("[data-token]")?.dataset.token;
        const tok = rec?.tokens.find((t) => t.id === id);
        const fallback = letterToken(tok?.name || "?");
        if (tok) tok.src = fallback;
        im.src = fallback;
      },
      true
    );
    st.addEventListener(
      "wheel",
      (e) => {
        if (!active()) return;
        e.preventDefault();
        e.stopPropagation();
        const rec = active();
        const tokEl = e.altKey && state.tool === "pan" ? e.target.closest(".map-token") : null;
        if (tokEl && rec) {
          const tok = rec.tokens.find((t) => t.id === tokEl.dataset.token);
          if (tok) {
            state.selected = tok.id;
            const factor = e.deltaY < 0 ? 1.1 : 0.9;
            tok.size = clampTokenSize(tok.size * factor);
            paintTokens();
            emitMarksSoon();
            return;
          }
        }
        const box = st.getBoundingClientRect();
        const mx = e.clientX - box.left;
        const my = e.clientY - box.top;
        const old = Math.max(0.01, state.view.scale);
        const steps = e.deltaMode === 1 ? e.deltaY : e.deltaY / 100;
        const factor = Math.exp(-steps * 0.12);
        const next = Math.max(0.08, Math.min(8, old * factor));
        if (next === old) return;
        const zx = (mx - state.view.x) / old;
        const zy = (my - state.view.y) / old;
        state.view.scale = next;
        state.view.x = mx - zx * next;
        state.view.y = my - zy * next;
        applyTransform();
        if (hooks.canControl?.()) hooks.onView?.(snapshot());
      },
      { passive: false }
    );
    document.addEventListener("keydown", (e) => {
      if (state.aim) return;
      if (e.key !== "Delete" && e.key !== "Backspace") return;
      const tag = (e.target && e.target.tagName) || "";
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
      if (!state.selected || !active()) return;
      e.preventDefault();
      removeToken(state.selected);
    });
    for (const btn of document.querySelectorAll("[data-map-tool]")) {
      btn.addEventListener("click", () => setTool(btn.dataset.mapTool));
    }
    document.getElementById("map-clear")?.addEventListener("click", () => clearMarks());
    syncHint();
  }

  function applyRemote(meta) {
    if (!meta) {
      pendingRemote = null;
      closeBoard(false);
      return;
    }
    pendingRemote = {
      id: meta.id,
      name: meta.name,
      mime: meta.mime,
      w: meta.w,
      h: meta.h,
      view: meta.view ? { ...meta.view } : null,
      grid: meta.grid ? { ...meta.grid } : null,
      tokens: Array.isArray(meta.tokens) ? meta.tokens.map(copyToken) : null,
      drawings: Array.isArray(meta.drawings) ? meta.drawings.map(copyDrawing) : null,
    };
    state.applying = true;
    try {
      const rec = state.maps.get(meta.id);
      if (rec) {
        mergePending(rec);
        if (pendingRemote.view) state.view = { ...state.view, ...pendingRemote.view };
        if (pendingRemote.grid) state.grid = { ...state.grid, ...pendingRemote.grid };
        const showing = state.activeId === rec.id && Boolean(img()?.getAttribute("src"));
        if (showing) {
          applyTransform();
          paintTokens();
          paintDrawings();
          fillSelect();
          markHasMap(Boolean(rec.url));
        } else {
          show(rec.id, {
            keepView: true,
            silent: true,
            view: pendingRemote.view || undefined,
            grid: pendingRemote.grid || undefined,
            tokens: rec.tokens,
            drawings: rec.drawings,
          });
        }
      } else {
        state.activeId = meta.id;
        openBoard();
        markHasMap(false);
      }
    } finally {
      state.applying = false;
    }
  }

  return {
    state,
    addMap,
    addToken,
    dropAt(spec, clientX, clientY) {
      if (!active() || !spec) return;
      const pt = clientToWorld(clientX, clientY);
      addToken(spec, pt.x, pt.y);
    },
    show,
    fit,
    forget,
    clearEphemeral,
    clearMarks,
    setTool,
    snapshot,
    applyRemote,
    applyMarks,
    hydrate,
    bind,
    fillSelect,
    openBoard,
    closeBoard,
    toggleGrid() {
      state.grid.on = !state.grid.on;
      drawGrid();
      if (hooks.canControl?.()) hooks.onChange?.(snapshot());
    },
    tokens() {
      const rec = active();
      return rec ? rec.tokens.map((t) => ({ ...t })) : [];
    },
    refreshTokens() {
      paintTokens();
    },
    setAim(fn) {
      state.aim = typeof fn === "function" ? fn : null;
      const st = stage();
      if (st) st.classList.toggle("aiming", Boolean(state.aim));
      if (state.aim) openBoard();
      paintTokens();
    },
    nudgeMarks() {
      emitMarks();
    },
    project(fromId, toId, kind) {
      const rec = active();
      const root = tokensEl();
      if (!rec || !root || !toId) return;
      const to = rec.tokens.find((t) => t.id === toId);
      if (!to) return;
      const from = fromId ? rec.tokens.find((t) => t.id === fromId) : null;
      if (from && from.id !== to.id) {
        const dx = to.x - from.x;
        const dy = to.y - from.y;
        const len = Math.max(8, Math.hypot(dx, dy));
        const ang = Math.atan2(dy, dx);
        const beam = document.createElement("div");
        beam.className = "map-beam map-beam-" + (kind || "hit");
        beam.style.left = from.x + "px";
        beam.style.top = from.y + "px";
        beam.style.width = len + "px";
        beam.style.transform = `rotate(${ang}rad)`;
        root.appendChild(beam);
        window.setTimeout(() => beam.remove(), 700);
      }
      const el = root.querySelector(`[data-token="${CSS.escape(to.id)}"]`);
      if (el) {
        el.classList.remove("struck", "struck-heal", "struck-hack", "struck-hit");
        void el.offsetWidth;
        el.classList.add("struck", "struck-" + (kind || "hit"));
        window.setTimeout(() => el.classList.remove("struck", "struck-heal", "struck-hack", "struck-hit"), 900);
      }
    },
  };
}

export function imageSize(url) {
  return new Promise((resolve) => {
    const im = new Image();
    im.onload = () => resolve({ w: im.naturalWidth, h: im.naturalHeight });
    im.onerror = () => resolve({ w: 1024, h: 768 });
    im.src = url;
  });
}

export function tokenPayload(el) {
  if (!el) return null;
  let node = el.closest("[data-token-kind]");
  if (!node) {
    const host = el.closest("[data-god-id], [data-lore-id], [data-beast], [data-shard], [data-npc], .beast-row, .char-chip, .beast-block");
    node = host?.querySelector("img[data-token-kind]") || host?.querySelector("[data-token-kind]");
  }
  if (!node) return null;
  const img = node.matches("img") ? node : node.querySelector("img:not([hidden])");
  const kind = node.dataset.tokenKind || "token";
  const ref = node.dataset.tokenRef || "";
  const raw = node.dataset.tokenSrc || (img && (img.getAttribute("src") || img.src)) || "";
  return {
    kind,
    ref,
    sheetId: node.dataset.tokenSheet || "",
    ownerId: node.dataset.tokenOwner || "",
    name: node.dataset.tokenName || "",
    src: portableSrc(raw, { kind, ref }) || raw,
  };
}
