const KINDS = [
  ["celtic", "Celtic"],
  ["greek", "Greek"],
  ["egyptian", "Egyptian"],
  ["norse", "Norse"],
  ["baldur", "Baldur's Gate"],
];

const ART_V = "2";

function byName(a, b) {
  return String(a.name || "").localeCompare(String(b.name || ""), undefined, { sensitivity: "base" });
}

function esc(value) {
  return String(value ?? "").replace(/[&<>"']/g, (ch) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[ch])
  );
}

function artUrl(id, bust) {
  const q = bust ? `v=${ART_V}&r=${bust}` : `v=${ART_V}`;
  return `assets/gods/${esc(id)}.jpg?${q}`;
}

function sealUrl(id, name) {
  const c = document.createElement("canvas");
  c.width = 480;
  c.height = 640;
  const g = c.getContext("2d");
  let h = 0;
  for (const ch of String(id || "god")) h = (h * 33 + ch.charCodeAt(0)) >>> 0;
  const hue = h % 360;
  g.fillStyle = `hsl(${hue}, 28%, 12%)`;
  g.fillRect(0, 0, 480, 640);
  const rad = g.createRadialGradient(240, 220, 20, 240, 260, 320);
  rad.addColorStop(0, `hsl(${(hue + 40) % 360}, 55%, 42%)`);
  rad.addColorStop(1, `hsl(${hue}, 30%, 10%)`);
  g.fillStyle = rad;
  g.beginPath();
  g.arc(240, 260, 170, 0, Math.PI * 2);
  g.fill();
  g.fillStyle = "rgba(232, 196, 110, 0.92)";
  g.font = "bold 180px serif";
  g.textAlign = "center";
  g.textBaseline = "middle";
  g.fillText(String(name || "?").slice(0, 1).toUpperCase(), 240, 270);
  return c.toDataURL("image/jpeg", 0.85);
}

function recoverArt(img) {
  if (!(img instanceof HTMLImageElement) || !img.dataset.godArt) return;
  if (img.dataset.retried) {
    img.src = sealUrl(img.dataset.godArt, img.dataset.tokenName || img.alt || img.dataset.godArt);
    img.hidden = false;
    if (img.parentElement?.classList.contains("beast-art")) img.parentElement.hidden = false;
    return;
  }
  img.dataset.retried = "1";
  img.src = artUrl(img.dataset.godArt, Date.now());
}

function line(label, value) {
  if (!value) return "";
  return `<p class="beast-line"><b>${esc(label)}</b> ${esc(value)}</p>`;
}

function section(title, rows) {
  if (!rows?.length) return "";
  return `<h4>${esc(title)}</h4>${rows
    .map((r) => {
      if (typeof r === "string") return `<p class="beast-trait">${esc(r)}</p>`;
      return `<p class="beast-trait"><b>${esc(r.n)}.</b> ${esc(r.d)}</p>`;
    })
    .join("")}`;
}

function panLabel(id) {
  return (KINDS.find(([k]) => k === id) || [id, String(id || "").replace(/^\w/, (c) => c.toUpperCase())])[1];
}

function tokenAttrs(m, src) {
  return `draggable="true" data-god-art="${esc(m.id)}" data-god-id="${esc(m.id)}" data-token-kind="god" data-token-ref="${esc(m.id)}" data-token-name="${esc(m.name)}" data-token-src="${src}"`;
}

function blockHTML(m) {
  if (!m) return `<div class="beast-empty">Pick a god.</div>`;
  const art = artUrl(m.id);
  const paras = String(m.text || "")
    .split(/\n\n+/)
    .filter(Boolean)
    .map((p) => `<p class="beast-blurb">${esc(p)}</p>`)
    .join("");
  const pan = panLabel(m.pantheon || m.kind);
  return `
    <article class="beast-block" draggable="true" data-god-id="${esc(m.id)}">
      <p class="kit-drag-hint">Drag onto an open character to set their deity. Drag the portrait onto a map for a token.</p>
      <figure class="beast-art">
        <img src="${art}" alt="" decoding="async" ${tokenAttrs(m, art)} />
      </figure>
      <h3>${esc(m.name)}</h3>
      <p class="beast-kind">${esc(m.aka || pan)} · ${esc(pan)}</p>
      ${m.blurb ? `<p class="beast-blurb">${esc(m.blurb)}</p>` : ""}
      ${line("Alignment", m.align)}
      ${line("Domains", m.domains)}
      ${line("Symbol", m.symbol)}
      ${line("Rank", m.rank)}
      ${line("Appears as", m.look)}
      ${paras}
      ${m.worship ? `<h4>At the table</h4><p class="beast-blurb">${esc(m.worship)}</p>` : ""}
      ${section("Hooks", m.hooks)}
      <p class="kit-owned" style="margin-top:12px"><span><b>Add to open sheet</b><small>Sets deity, and domain if they have none</small></span><button type="button" class="ghost" data-god-add="${esc(m.id)}">Add</button></p>
    </article>`;
}

export function createGods() {
  const state = { all: [], ready: false, id: "", q: "", kind: "all" };
  const $ = (part) => document.getElementById(`god-${part}`);

  async function ensure() {
    if (state.ready) return;
    const res = await fetch("data/gods.json", { cache: "no-store" });
    if (!res.ok) throw new Error("gods missing");
    state.all = (await res.json()).slice().sort(byName);
    state.ready = true;
    if (!state.id && state.all[0]) state.id = state.all[0].id;
  }

  function item(id) {
    return state.all.find((g) => g.id === id) || null;
  }

  function filtered() {
    const q = state.q.trim().toLowerCase();
    return state.all
      .filter((m) => {
        if (state.kind !== "all" && m.pantheon !== state.kind && m.kind !== state.kind) return false;
        if (!q) return true;
        const hay = `${m.name} ${m.aka || ""} ${m.pantheon} ${m.domains || ""} ${m.symbol || ""} ${m.blurb || ""} ${m.text || ""}`.toLowerCase();
        return hay.includes(q);
      })
      .slice()
      .sort(byName);
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
            const pan = panLabel(m.pantheon);
            return `<button type="button" class="beast-row${on}" data-god="${esc(m.id)}" draggable="true" data-god-id="${esc(m.id)}"><img class="beast-thumb" src="${src}" alt="" decoding="async" loading="lazy" ${tokenAttrs(m, src)} /><span><b>${esc(m.name)}</b><small>${esc(pan)} · ${esc(m.domains || m.aka || "")}</small></span></button>`;
          })
          .join("")
      : `<p class="beast-empty">No gods match.</p>`;
    page.innerHTML = blockHTML(current());
  }

  function openBoard() {
    const el = $("board");
    if (!el) return;
    el.hidden = false;
    document.querySelector(".app")?.classList.add("god-open");
    document.getElementById("god-toggle")?.classList.add("on");
    ensure()
      .then(paint)
      .catch(() => {
        const page = $("page");
        if (page) page.innerHTML = `<div class="beast-empty">The gods would not load.</div>`;
      });
  }

  function closeBoard() {
    const el = $("board");
    if (el) el.hidden = true;
    document.querySelector(".app")?.classList.remove("god-open");
    document.getElementById("god-toggle")?.classList.remove("on");
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
      const btn = e.target.closest("[data-god]");
      if (!btn) return;
      state.id = btn.dataset.god;
      paint();
    });
    const onArtErr = (e) => recoverArt(e.target);
    $("list")?.addEventListener("error", onArtErr, true);
    $("page")?.addEventListener("error", onArtErr, true);
    $("close")?.addEventListener("click", closeBoard);
    $("page")?.addEventListener("click", (e) => {
      const add = e.target.closest("[data-god-add]");
      if (!add?.dataset.godAdd) return;
      document.dispatchEvent(new CustomEvent("blight-god-add", { detail: { id: add.dataset.godAdd } }));
    });
    $("list")?.addEventListener("dblclick", (e) => {
      const btn = e.target.closest("[data-god]");
      if (!btn?.dataset.god) return;
      document.dispatchEvent(new CustomEvent("blight-god-add", { detail: { id: btn.dataset.god } }));
    });
  }

  return { openBoard, closeBoard, toggle, bind, ensure, item, KINDS };
}
