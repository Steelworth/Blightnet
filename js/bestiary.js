import { icon } from "./icons.js";

const TYPES = [
  "aberration",
  "beast",
  "celestial",
  "construct",
  "dragon",
  "elemental",
  "fey",
  "fiend",
  "giant",
  "humanoid",
  "monstrosity",
  "ooze",
  "plant",
  "undead",
  "swarm",
];

function esc(value) {
  return String(value ?? "").replace(/[&<>"']/g, (ch) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[ch])
  );
}

function mod(score) {
  const n = Number(score);
  if (!Number.isFinite(n)) return "—";
  const m = Math.floor((n - 10) / 2);
  return (m >= 0 ? "+" : "") + m;
}

function typeKey(m) {
  const t = String(m.type || "").toLowerCase();
  if (t.includes("swarm")) return "swarm";
  return t;
}

function crBucket(m) {
  const cr = Number(m.cr);
  if (!Number.isFinite(cr) || cr <= 0) return "0";
  if (cr < 1) return "frac";
  if (cr <= 4) return "1-4";
  if (cr <= 10) return "5-10";
  if (cr <= 16) return "11-16";
  return "17+";
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
  return `<div class="beast-score"><span>${esc(label)}</span><b>${esc(n)}</b><small>(${mod(n)})</small></div>`;
}

const ART_V = "3";

function artUrl(id, bust) {
  const q = bust ? `v=${ART_V}&r=${bust}` : `v=${ART_V}`;
  return `assets/bestiary/${esc(id)}.jpg?${q}`;
}

function recoverArt(img) {
  if (!(img instanceof HTMLImageElement) || !img.dataset.beastArt) return;
  if (img.dataset.retried) {
    img.hidden = true;
    if (img.parentElement?.classList.contains("beast-art")) img.parentElement.hidden = true;
    return;
  }
  img.dataset.retried = "1";
  img.src = artUrl(img.dataset.beastArt, Date.now());
}

function blockHTML(m) {
  if (!m) {
    return `<div class="beast-empty">Choose a creature from the list.</div>`;
  }
  const kind = [m.size, m.type, m.sub ? `(${m.sub})` : "", m.align ? `, ${m.align}` : ""]
    .filter(Boolean)
    .join(" ");
  const ac = m.acNote ? `${m.ac} (${m.acNote})` : String(m.ac ?? "—");
  const hp = m.hd ? `${m.hp} (${m.hd})` : String(m.hp ?? "—");
  const cr = m.xp ? `CR ${m.crLabel} (${m.xp.toLocaleString()} XP)` : `CR ${m.crLabel || "—"}`;
  const art = artUrl(m.id);
  return `
    <article class="beast-block">
      <figure class="beast-art">
        <img src="${art}" alt="" decoding="async" draggable="true" data-beast-art="${esc(m.id)}" data-token-kind="beast" data-token-ref="${esc(m.id)}" data-token-name="${esc(m.name)}" data-token-src="${art}" />
      </figure>
      <h3>${esc(m.name)}</h3>
      <p class="beast-kind">${esc(kind)}</p>
      ${line("Armor Class", ac)}
      ${line("Hit Points", hp)}
      ${line("Speed", m.speed)}
      <div class="beast-scores">
        ${score("STR", m.str)}${score("DEX", m.dex)}${score("CON", m.con)}
        ${score("INT", m.int)}${score("WIS", m.wis)}${score("CHA", m.cha)}
      </div>
      ${line("Saving Throws", m.saves)}
      ${line("Skills", m.skills)}
      ${line("Damage Vulnerabilities", m.vuln)}
      ${line("Damage Resistances", m.resist)}
      ${line("Damage Immunities", m.immune)}
      ${line("Condition Immunities", m.cimm)}
      ${line("Senses", m.senses)}
      ${line("Languages", m.lang)}
      ${line("Challenge", cr + (m.pb ? ` · Proficiency Bonus ${m.pb >= 0 ? "+" : ""}${m.pb}` : ""))}
      ${m.blurb ? `<p class="beast-blurb">${esc(m.blurb)}</p>` : ""}
      ${section("Traits", m.traits)}
      ${section("Actions", m.actions)}
      ${section("Reactions", m.reactions)}
      ${section("Legendary Actions", m.legendary)}
    </article>`;
}

export function createBestiary() {
  const state = {
    all: [],
    ready: false,
    id: "",
    q: "",
    type: "all",
    cr: "all",
  };

  const $ = (sel) => document.querySelector(sel);

  async function ensure() {
    if (state.ready) return;
    const res = await fetch("data/bestiary.json", { cache: "no-store" });
    if (!res.ok) throw new Error("bestiary missing");
    state.all = await res.json();
    state.ready = true;
    if (!state.id && state.all[0]) state.id = state.all[0].id;
  }

  function filtered() {
    const q = state.q.trim().toLowerCase();
    return state.all.filter((m) => {
      if (state.type !== "all" && typeKey(m) !== state.type) return false;
      if (state.cr !== "all" && crBucket(m) !== state.cr) return false;
      if (!q) return true;
      const hay = `${m.name} ${m.type} ${m.sub || ""} ${m.crLabel}`.toLowerCase();
      return hay.includes(q);
    });
  }

  function current() {
    return state.all.find((m) => m.id === state.id) || filtered()[0] || null;
  }

  function paint() {
    const list = $("#beast-list");
    const page = $("#beast-page");
    const count = $("#beast-count");
    if (!list || !page) return;
    const rows = filtered();
    if (count) count.textContent = `${rows.length} of ${state.all.length}`;
    if (!rows.some((m) => m.id === state.id) && rows[0]) state.id = rows[0].id;
    const sig = `${state.q}\0${state.type}\0${state.cr}\0${rows.length}\0${state.all.length}`;
    if (sig !== state.listSig) {
      state.listSig = sig;
      list.innerHTML = rows.length
        ? rows
            .map((m) => {
              const on = m.id === state.id ? " on" : "";
              return `<button type="button" class="beast-row${on}" data-beast="${esc(m.id)}"><img class="beast-thumb" src="${artUrl(m.id)}" alt="" decoding="async" loading="lazy" draggable="true" data-beast-art="${esc(m.id)}" data-token-kind="beast" data-token-ref="${esc(m.id)}" data-token-name="${esc(m.name)}" data-token-src="${artUrl(m.id)}" /><span><b>${esc(m.name)}</b><small>CR ${esc(m.crLabel)} · ${esc(m.type)}</small></span></button>`;
            })
            .join("")
        : `<p class="beast-empty">No creatures match.</p>`;
    } else {
      for (const btn of list.querySelectorAll(".beast-row")) {
        btn.classList.toggle("on", btn.dataset.beast === state.id);
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
    const el = $("#bestiary-board");
    if (!el) return;
    el.hidden = false;
    document.querySelector(".app")?.classList.add("bestiary-open");
    $("#bestiary-toggle")?.classList.add("on");
    ensure()
      .then(paint)
      .catch(() => {
        const page = $("#beast-page");
        if (page) page.innerHTML = `<div class="beast-empty">The bestiary could not be opened.</div>`;
      });
  }

  function closeBoard() {
    const el = $("#bestiary-board");
    if (el) el.hidden = true;
    document.querySelector(".app")?.classList.remove("bestiary-open");
    $("#bestiary-toggle")?.classList.remove("on");
  }

  function toggle() {
    const el = $("#bestiary-board");
    if (!el || el.hidden) openBoard();
    else closeBoard();
  }

  function bind() {
    $("#beast-search")?.addEventListener("input", (e) => {
      state.q = e.target.value || "";
      paint();
    });
    $("#beast-type")?.addEventListener("change", (e) => {
      state.type = e.target.value || "all";
      paint();
    });
    $("#beast-cr")?.addEventListener("change", (e) => {
      state.cr = e.target.value || "all";
      paint();
    });
    $("#beast-list")?.addEventListener("click", (e) => {
      const btn = e.target.closest("[data-beast]");
      if (!btn) return;
      state.id = btn.dataset.beast;
      paint();
    });
    const onArtErr = (e) => recoverArt(e.target);
    $("#beast-list")?.addEventListener("error", onArtErr, true);
    $("#beast-page")?.addEventListener("error", onArtErr, true);
    $("#beast-close")?.addEventListener("click", closeBoard);
  }

  return { openBoard, closeBoard, toggle, bind, TYPES };
}
