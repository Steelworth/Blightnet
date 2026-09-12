function esc(value) {
  return String(value ?? "").replace(/[&<>"']/g, (ch) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[ch])
  );
}

const ART_V = "1";

function byName(a, b) {
  return String(a.name || "").localeCompare(String(b.name || ""), undefined, { sensitivity: "base" });
}

function artUrl(id) {
  return `assets/corps/${esc(id)}.jpg?v=${ART_V}`;
}

export function createCorps() {
  const state = { all: [], ready: false, id: "", q: "" };
  const $ = (sel) => document.querySelector(sel);

  async function ensure() {
    if (state.ready) return;
    const res = await fetch("data/corps.json", { cache: "no-store" });
    if (!res.ok) throw new Error("corps missing");
    state.all = (await res.json()).slice().sort(byName);
    state.ready = true;
    if (!state.id && state.all[0]) state.id = state.all[0].id;
  }

  function current() {
    return state.all.find((c) => c.id === state.id) || state.all[0] || null;
  }

  function filtered() {
    const q = state.q.trim().toLowerCase();
    const rows = !q
      ? state.all
      : state.all.filter((c) =>
          `${c.name} ${c.ticker} ${c.focus} ${c.origin} ${c.text}`.toLowerCase().includes(q)
        );
    return rows.slice().sort(byName);
  }

  function blockHTML(c) {
    if (!c) return `<div class="beast-empty">Choose a corp.</div>`;
    return `
      <article class="beast-block corp-block">
        <figure class="beast-art corp-art">
          <img src="${artUrl(c.id)}" alt="" decoding="async" data-corp-art="${esc(c.id)}" />
        </figure>
        <p class="beast-kind">${esc(c.ticker)} · ${esc(c.origin)}</p>
        <h3>${esc(c.name)}</h3>
        <p class="beast-line"><b>HQ</b> ${esc(c.hq)}</p>
        <p class="beast-line"><b>Focus</b> ${esc(c.focus)}</p>
        <p class="beast-blurb">${esc(c.text)}</p>
      </article>`;
  }

  function paint() {
    const list = $("#corp-list");
    const page = $("#corp-page");
    const count = $("#corp-count");
    if (!list || !page) return;
    const rows = filtered();
    if (count) count.textContent = `${rows.length} of ${state.all.length}`;
    if (!rows.some((c) => c.id === state.id) && rows[0]) state.id = rows[0].id;
    list.innerHTML = rows.length
      ? rows
          .map((c) => {
            const on = c.id === state.id ? " on" : "";
            return `<button type="button" class="beast-row${on}" data-corp="${esc(c.id)}">
              <img class="beast-thumb" src="${artUrl(c.id)}" alt="" decoding="async" loading="lazy" data-corp-art="${esc(c.id)}" />
              <span><b>${esc(c.name)}</b><small>${esc(c.ticker)} · ${esc(c.focus)}</small></span>
            </button>`;
          })
          .join("")
      : `<p class="beast-empty">No corp matches.</p>`;
    page.innerHTML = blockHTML(current());
  }

  async function openBoard(id) {
    const el = $("#corp-board");
    if (!el) return;
    el.hidden = false;
    document.querySelector(".app")?.classList.add("corp-open");
    $("#corp-toggle")?.classList.add("on");
    try {
      await ensure();
      if (id && state.all.some((c) => c.id === id)) state.id = id;
      paint();
    } catch {
      const page = $("#corp-page");
      if (page) page.innerHTML = `<div class="beast-empty">The dossier could not be opened.</div>`;
    }
  }

  function closeBoard() {
    const el = $("#corp-board");
    if (el) el.hidden = true;
    document.querySelector(".app")?.classList.remove("corp-open");
    $("#corp-toggle")?.classList.remove("on");
  }

  function toggle() {
    const el = $("#corp-board");
    if (!el || el.hidden) openBoard();
    else closeBoard();
  }

  function bind() {
    $("#corp-search")?.addEventListener("input", (e) => {
      state.q = e.target.value || "";
      paint();
    });
    $("#corp-list")?.addEventListener("click", (e) => {
      const btn = e.target.closest("[data-corp]");
      if (!btn) return;
      state.id = btn.dataset.corp;
      paint();
    });
    $("#corp-close")?.addEventListener("click", closeBoard);
    document.getElementById("stock-tape")?.addEventListener("click", (e) => {
      const hit = e.target.closest("[data-corp]");
      if (!hit) return;
      openBoard(hit.dataset.corp);
    });
  }

  return { openBoard, closeBoard, toggle, bind, ensure };
}
