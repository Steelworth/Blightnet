const KINDS = [
  ["sky", "Sky"],
  ["food", "Food"],
  ["fuel", "Fuel"],
  ["water", "Water"],
  ["air", "Air"],
  ["street", "Street"],
  ["net", "NET"],
  ["chrome", "Chrome"],
  ["war", "War"],
  ["road", "Road"],
  ["weather", "Weather"],
  ["sleep", "Sleep"],
  ["death", "Death"],
];

const ART_V = "1";

function bySort(a, b) {
  const d = (Number(a.sort) || 0) - (Number(b.sort) || 0);
  if (d) return d;
  return String(a.name || "").localeCompare(String(b.name || ""), undefined, { sensitivity: "base" });
}

function esc(value) {
  return String(value ?? "").replace(/[&<>"']/g, (ch) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[ch])
  );
}

function artUrl(id, bust) {
  const q = bust ? `v=${ART_V}&r=${bust}` : `v=${ART_V}`;
  return `assets/lore/${esc(id)}.jpg?${q}`;
}

function tokenAttrs(m, src) {
  return `draggable="true" data-lore-art="${esc(m.id)}" data-lore-id="${esc(m.id)}" data-token-kind="lore" data-token-ref="${esc(m.id)}" data-token-name="${esc(m.name)}" data-token-src="${src}"`;
}

function kindLabel(id) {
  return (KINDS.find(([k]) => k === id) || [id, id])[1];
}

function blockHTML(m) {
  if (!m) return `<div class="beast-empty">Pick a file.</div>`;
  const art = artUrl(m.id);
  const paras = String(m.text || "")
    .split(/\n\n+/)
    .filter(Boolean)
    .map((p) => `<p class="beast-blurb">${esc(p)}</p>`)
    .join("");
  const hooks = (m.hooks || [])
    .map((h) => `<p class="beast-trait">${esc(h)}</p>`)
    .join("");
  return `
    <article class="beast-block">
      <p class="kit-drag-hint">Drag the painting onto a loaded map for a token. This is street fact, not a person.</p>
      <figure class="beast-art">
        <img src="${art}" alt="" decoding="async" ${tokenAttrs(m, art)} />
      </figure>
      <h3>${esc(m.name)}</h3>
      <p class="beast-kind">${esc(kindLabel(m.kind))} · Night City 2045</p>
      ${m.blurb ? `<p class="beast-blurb">${esc(m.blurb)}</p>` : ""}
      ${paras}
      ${hooks ? `<h4>At the table</h4>${hooks}` : ""}
    </article>`;
}

export function createLore() {
  const state = { all: [], ready: false, id: "", q: "", kind: "all" };
  const $ = (part) => document.getElementById(`lore-${part}`);

  async function ensure() {
    if (state.ready) return;
    const res = await fetch("data/lore.json", { cache: "no-store" });
    if (!res.ok) throw new Error("lore missing");
    state.all = (await res.json()).slice().sort(bySort);
    state.ready = true;
    if (!state.id && state.all[0]) state.id = state.all[0].id;
  }

  function item(id) {
    return state.all.find((row) => row.id === id) || null;
  }

  function filtered() {
    const q = state.q.trim().toLowerCase();
    return state.all
      .filter((m) => {
        if (state.kind !== "all" && m.kind !== state.kind) return false;
        if (!q) return true;
        const hay = `${m.name} ${m.kind} ${m.blurb || ""} ${m.text || ""} ${(m.hooks || []).join(" ")}`.toLowerCase();
        return hay.includes(q);
      })
      .slice()
      .sort(bySort);
  }

  function current() {
    return state.all.find((m) => m.id === state.id) || filtered()[0] || null;
  }

  function paint() {
    const list = $("list");
    const page = $("page");
    const count = $("count");
    if (!list || !page) return;
    const rows = filtered();
    if (count) count.textContent = `${rows.length} of ${state.all.length}`;
    if (!rows.some((m) => m.id === state.id) && rows[0]) state.id = rows[0].id;
    list.innerHTML = rows.length
      ? rows
          .map((m) => {
            const on = m.id === state.id ? " on" : "";
            const src = artUrl(m.id);
            return `<button type="button" class="beast-row${on}" data-lore="${esc(m.id)}" draggable="true" data-lore-id="${esc(m.id)}"><img class="beast-thumb" src="${src}" alt="" decoding="async" loading="lazy" ${tokenAttrs(m, src)} /><span><b>${esc(m.name)}</b><small>${esc(kindLabel(m.kind))}</small></span></button>`;
          })
          .join("")
      : `<p class="beast-empty">No file matches.</p>`;
    page.innerHTML = blockHTML(current());
  }

  function openBoard() {
    const el = $("board");
    if (!el) return;
    el.hidden = false;
    document.querySelector(".app")?.classList.add("lore-open");
    document.getElementById("lore-toggle")?.classList.add("on");
    ensure()
      .then(paint)
      .catch(() => {
        const page = $("page");
        if (page) page.innerHTML = `<div class="beast-empty">The street file would not load.</div>`;
      });
  }

  function closeBoard() {
    const el = $("board");
    if (el) el.hidden = true;
    document.querySelector(".app")?.classList.remove("lore-open");
    document.getElementById("lore-toggle")?.classList.remove("on");
  }

  function toggle() {
    const el = $("board");
    if (!el || el.hidden) openBoard();
    else closeBoard();
  }

  function bind() {
    $("search")?.addEventListener("input", (e) => {
      state.q = e.target.value || "";
      paint();
    });
    $("kind")?.addEventListener("change", (e) => {
      state.kind = e.target.value || "all";
      paint();
    });
    $("list")?.addEventListener("click", (e) => {
      const btn = e.target.closest("[data-lore]");
      if (!btn) return;
      state.id = btn.dataset.lore;
      paint();
    });
    $("close")?.addEventListener("click", closeBoard);
  }

  return { openBoard, closeBoard, toggle, bind, ensure, item, KINDS };
}
