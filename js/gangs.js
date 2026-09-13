function esc(value) {
  return String(value ?? "").replace(/[&<>"']/g, (ch) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[ch])
  );
}

const ART_V = "1";

export const GANG_KINDS = [
  ["booster", "Booster"],
  ["poser", "Poser"],
  ["cult", "Cult"],
  ["party", "Party"],
  ["guardian", "Guardian"],
  ["net", "Net"],
  ["nomad", "Nomad / raider"],
  ["crime", "Street crime"],
  ["historical", "Historical"],
];

function byName(a, b) {
  return String(a.name || "").localeCompare(String(b.name || ""), undefined, { sensitivity: "base" });
}

function artUrl(id) {
  return `assets/gangs/${esc(id)}.jpg?v=${ART_V}`;
}

function kindLabel(id) {
  return (GANG_KINDS.find(([k]) => k === id) || [id, id])[1];
}

function tokenAttrs(g, src) {
  return `draggable="true" data-gang-art="${esc(g.id)}" data-gang-id="${esc(g.id)}" data-token-kind="gang" data-token-ref="${esc(g.id)}" data-token-name="${esc(g.name)}" data-token-src="${src}"`;
}

export function createGangs() {
  const state = { all: [], ready: false, id: "", q: "", kind: "all" };
  const $ = (sel) => document.querySelector(sel);

  async function ensure() {
    if (state.ready) return;
    const res = await fetch("data/gangs.json", { cache: "no-store" });
    if (!res.ok) throw new Error("gangs missing");
    state.all = (await res.json()).slice().sort(byName);
    state.ready = true;
    if (!state.id && state.all[0]) state.id = state.all[0].id;
  }

  function current() {
    return state.all.find((g) => g.id === state.id) || state.all[0] || null;
  }

  function filtered() {
    const q = state.q.trim().toLowerCase();
    let rows = state.all;
    if (state.kind !== "all") rows = rows.filter((g) => g.kind === state.kind);
    if (q) {
      rows = rows.filter((g) =>
        `${g.name} ${g.aka || ""} ${g.kind} ${g.era} ${g.turf} ${g.leader} ${g.origin} ${g.text}`.toLowerCase().includes(q)
      );
    }
    return rows.slice().sort(byName);
  }

  function blockHTML(g) {
    if (!g) return `<div class="beast-empty">Choose a gang.</div>`;
    const art = artUrl(g.id);
    const paras = String(g.text || "")
      .split(/\n\n+/)
      .filter(Boolean)
      .map((p) => `<p class="beast-blurb">${esc(p)}</p>`)
      .join("");
    const hooks = (g.hooks || []).map((h) => `<p class="beast-trait">${esc(h)}</p>`).join("");
    return `
      <article class="beast-block gang-block">
        <p class="kit-drag-hint">Drag the painting onto a loaded map for a token.</p>
        <figure class="beast-art gang-art">
          <img src="${art}" alt="" decoding="async" ${tokenAttrs(g, art)} data-gang-art="${esc(g.id)}" onerror="this.classList.add('off')" />
        </figure>
        <p class="beast-kind">${esc(kindLabel(g.kind))} · ${esc(g.era)}</p>
        <h3>${esc(g.name)}</h3>
        ${g.aka ? `<p class="beast-line"><b>Also</b> ${esc(g.aka)}</p>` : ""}
        <p class="beast-line"><b>Status</b> ${esc(g.status)}</p>
        <p class="beast-line"><b>Turf</b> ${esc(g.turf)}</p>
        <p class="beast-line"><b>Colors / tell</b> ${esc(g.look)}</p>
        <p class="beast-line"><b>Who runs it</b> ${esc(g.leader)}</p>
        ${g.origin ? `<h4>Origin</h4><p class="beast-blurb">${esc(g.origin)}</p>` : ""}
        ${paras}
        ${hooks ? `<h4>At the table</h4>${hooks}` : ""}
      </article>`;
  }

  function paint() {
    const list = $("#gang-list");
    const page = $("#gang-page");
    const count = $("#gang-count");
    if (!list || !page) return;
    const rows = filtered();
    if (count) count.textContent = `${rows.length} of ${state.all.length}`;
    if (!rows.some((g) => g.id === state.id) && rows[0]) state.id = rows[0].id;
    list.innerHTML = rows.length
      ? rows
          .map((g) => {
            const on = g.id === state.id ? " on" : "";
            return `<button type="button" class="beast-row${on}" data-gang="${esc(g.id)}">
              <img class="beast-thumb" src="${artUrl(g.id)}" alt="" decoding="async" loading="lazy" data-gang-art="${esc(g.id)}" onerror="this.classList.add('off')" />
              <span><b>${esc(g.name)}</b><small>${esc(kindLabel(g.kind))} · ${esc(g.era)}</small></span>
            </button>`;
          })
          .join("")
      : `<p class="beast-empty">No gang matches.</p>`;
    page.innerHTML = blockHTML(current());
  }

  async function openBoard(id) {
    const el = $("#gang-board");
    if (!el) return;
    el.hidden = false;
    document.querySelector(".app")?.classList.add("gang-open");
    $("#gang-toggle")?.classList.add("on");
    try {
      await ensure();
      if (id && state.all.some((g) => g.id === id)) state.id = id;
      paint();
    } catch {
      const page = $("#gang-page");
      if (page) page.innerHTML = `<div class="beast-empty">The street file could not be opened.</div>`;
    }
  }

  function closeBoard() {
    const el = $("#gang-board");
    if (el) el.hidden = true;
    document.querySelector(".app")?.classList.remove("gang-open");
    $("#gang-toggle")?.classList.remove("on");
  }

  function toggle() {
    const el = $("#gang-board");
    if (!el || el.hidden) openBoard();
    else closeBoard();
  }

  function bind() {
    $("#gang-search")?.addEventListener("input", (e) => {
      state.q = e.target.value || "";
      paint();
    });
    $("#gang-kind")?.addEventListener("change", (e) => {
      state.kind = e.target.value || "all";
      paint();
    });
    $("#gang-list")?.addEventListener("click", (e) => {
      const btn = e.target.closest("[data-gang]");
      if (!btn) return;
      state.id = btn.dataset.gang;
      paint();
    });
    $("#gang-close")?.addEventListener("click", closeBoard);
  }

  return { openBoard, closeBoard, toggle, bind, ensure };
}
