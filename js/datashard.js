import { icon } from "./icons.js";

const KINDS = [
  ["edgerunner", "Edgerunner"],
  ["street", "Street"],
  ["law", "Law"],
  ["corpo", "Corpo"],
  ["net", "Net"],
  ["nomad", "Nomad"],
  ["trauma", "Trauma"],
  ["civilian", "Civilian"],
  ["chrome", "Chrome"],
];

const ROLES = [
  "Rockerboy",
  "Solo",
  "Netrunner",
  "Tech",
  "Medtech",
  "Media",
  "Exec",
  "Lawman",
  "Fixer",
  "Nomad",
];

const STATS = [
  ["int", "INT"],
  ["ref", "REF"],
  ["dex", "DEX"],
  ["tech", "TECH"],
  ["cool", "COOL"],
  ["will", "WILL"],
  ["luck", "LUCK"],
  ["move", "MOVE"],
  ["body", "BODY"],
  ["emp", "EMP"],
];

const ART_V = "1";

function esc(value) {
  return String(value ?? "").replace(/[&<>"']/g, (ch) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[ch])
  );
}

function artUrl(id, bust) {
  const q = bust ? `v=${ART_V}&r=${bust}` : `v=${ART_V}`;
  return `assets/datashard/${esc(id)}.jpg?${q}`;
}

function recoverArt(img) {
  if (!(img instanceof HTMLImageElement) || !img.dataset.shardArt) return;
  if (img.dataset.retried) {
    img.hidden = true;
    if (img.parentElement?.classList.contains("beast-art")) img.parentElement.hidden = true;
    return;
  }
  img.dataset.retried = "1";
  img.src = artUrl(img.dataset.shardArt, Date.now());
}

function line(label, value) {
  if (!value) return "";
  return `<p class="beast-line"><b>${esc(label)}</b> ${esc(value)}</p>`;
}

function section(title, rows) {
  if (!rows?.length) return "";
  return `<h4>${esc(title)}</h4>${rows
    .map((r) => `<p class="beast-trait"><b>${esc(r.n)}.</b> ${esc(r.d)}</p>`)
    .join("")}`;
}

function score(label, n) {
  return `<div class="beast-score"><span>${esc(label)}</span><b>${esc(n)}</b></div>`;
}

function blockHTML(m) {
  if (!m) {
    return `<div class="beast-empty">Jack a name from the shard.</div>`;
  }
  const kind = [m.role || "Unaffiliated", m.tier, m.rank ? `Rank ${m.rank}` : ""]
    .filter(Boolean)
    .join(" · ");
  const art = artUrl(m.id);
  const weps = (m.weapons || [])
    .map((w) => `<p class="beast-trait"><b>${esc(w.n)}.</b> ${esc(w.d)}</p>`)
    .join("");
  return `
    <article class="beast-block">
      <figure class="beast-art">
        <img src="${art}" alt="" decoding="async" draggable="true" data-shard-art="${esc(m.id)}" data-token-kind="shard" data-token-ref="${esc(m.id)}" data-token-name="${esc(m.name)}" data-token-src="${art}" />
      </figure>
      <h3>${esc(m.name)}</h3>
      <p class="beast-kind">${esc(kind)} · ${esc(m.kind)}</p>
      ${line("Hit points", m.hp)}
      ${line("Armor SP (head/body)", m.sp)}
      ${line("Initiative", m.init)}
      ${line("MOVE", m.move)}
      ${line("Reputation", m.rep)}
      ${line("Humanity", m.humanity)}
      <div class="beast-scores">
        ${STATS.map(([id, lab]) => score(lab, m[id])).join("")}
      </div>
      ${weps ? `<h4>Weapons</h4>${weps}` : ""}
      ${line("Skills", m.skills)}
      ${line("Cyberware", m.chrome)}
      ${line("Gear", m.gear)}
      ${m.blurb ? `<p class="beast-blurb">${esc(m.blurb)}</p>` : ""}
      ${section("Notes", m.traits)}
    </article>`;
}

export function createDatashard() {
  const state = {
    all: [],
    ready: false,
    id: "",
    q: "",
    kind: "all",
    role: "all",
  };

  const $ = (sel) => document.querySelector(sel);

  async function ensure() {
    if (state.ready) return;
    const res = await fetch("data/datashard.json", { cache: "no-store" });
    if (!res.ok) throw new Error("datashard missing");
    state.all = await res.json();
    state.ready = true;
    if (!state.id && state.all[0]) state.id = state.all[0].id;
  }

  function filtered() {
    const q = state.q.trim().toLowerCase();
    return state.all.filter((m) => {
      if (state.kind !== "all" && m.kind !== state.kind) return false;
      if (state.role !== "all" && m.role !== state.role) return false;
      if (!q) return true;
      const hay = `${m.name} ${m.role} ${m.kind} ${m.tier} ${m.blurb || ""}`.toLowerCase();
      return hay.includes(q);
    });
  }

  function current() {
    return state.all.find((m) => m.id === state.id) || filtered()[0] || null;
  }

  function paint() {
    const list = $("#shard-list");
    const page = $("#shard-page");
    const count = $("#shard-count");
    if (!list || !page) return;
    const rows = filtered();
    if (count) count.textContent = `${rows.length} of ${state.all.length}`;
    if (!rows.some((m) => m.id === state.id) && rows[0]) state.id = rows[0].id;
    const sig = `${state.q}\0${state.kind}\0${state.role}\0${rows.length}\0${state.all.length}`;
    if (sig !== state.listSig) {
      state.listSig = sig;
      list.innerHTML = rows.length
        ? rows
            .map((m) => {
              const on = m.id === state.id ? " on" : "";
              const sub = [m.role || m.kind, m.tier].filter(Boolean).join(" · ");
              return `<button type="button" class="beast-row${on}" data-shard="${esc(m.id)}"><img class="beast-thumb" src="${artUrl(m.id)}" alt="" decoding="async" loading="lazy" draggable="true" data-shard-art="${esc(m.id)}" data-token-kind="shard" data-token-ref="${esc(m.id)}" data-token-name="${esc(m.name)}" data-token-src="${artUrl(m.id)}" /><span><b>${esc(m.name)}</b><small>${esc(sub)}</small></span></button>`;
            })
            .join("")
        : `<p class="beast-empty">No files match.</p>`;
    } else {
      for (const btn of list.querySelectorAll(".beast-row")) {
        btn.classList.toggle("on", btn.dataset.shard === state.id);
      }
    }
    if (state.pageId !== state.id) {
      state.pageId = state.id;
      page.innerHTML = blockHTML(current());
    }
    if (state._scrolled !== state.id) {
      state._scrolled = state.id;
      list.querySelector(".beast-row.on")?.scrollIntoView({ block: "nearest" });
    }
  }

  function openBoard() {
    const el = $("#datashard-board");
    if (!el) return;
    el.hidden = false;
    document.querySelector(".app")?.classList.add("datashard-open");
    $("#datashard-toggle")?.classList.add("on");
    ensure()
      .then(paint)
      .catch(() => {
        const page = $("#shard-page");
        if (page) page.innerHTML = `<div class="beast-empty">The shard would not mount.</div>`;
      });
  }

  function closeBoard() {
    const el = $("#datashard-board");
    if (el) el.hidden = true;
    document.querySelector(".app")?.classList.remove("datashard-open");
    $("#datashard-toggle")?.classList.remove("on");
  }

  function toggle() {
    const el = $("#datashard-board");
    if (!el || el.hidden) openBoard();
    else closeBoard();
  }

  function bind() {
    $("#shard-search")?.addEventListener("input", (e) => {
      state.q = e.target.value || "";
      paint();
    });
    $("#shard-kind")?.addEventListener("change", (e) => {
      state.kind = e.target.value || "all";
      paint();
    });
    $("#shard-role")?.addEventListener("change", (e) => {
      state.role = e.target.value || "all";
      paint();
    });
    $("#shard-list")?.addEventListener("click", (e) => {
      const btn = e.target.closest("[data-shard]");
      if (!btn) return;
      state.id = btn.dataset.shard;
      paint();
    });
    const onArtErr = (e) => recoverArt(e.target);
    $("#shard-list")?.addEventListener("error", onArtErr, true);
    $("#shard-page")?.addEventListener("error", onArtErr, true);
    $("#shard-close")?.addEventListener("click", closeBoard);
  }

  return { openBoard, closeBoard, toggle, bind, KINDS, ROLES };
}
