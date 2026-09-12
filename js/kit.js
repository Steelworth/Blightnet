import { worldOf } from "./catalog.js";
import { kitThumb, kitHero, bindKitArt } from "./kit-art.js";

function esc(value) {
  return String(value ?? "").replace(/[&<>"']/g, (ch) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[ch])
  );
}

const KINDS_SRD = [
  ["all", "All"],
  ["weapon", "Weapons"],
  ["armor", "Armor"],
  ["gear", "Gear"],
  ["magic", "Magic"],
  ["spell", "Spells"],
];

const KINDS_RED = [
  ["all", "All"],
  ["weapon", "Weapons"],
  ["armor", "Armor"],
  ["gear", "Gear"],
  ["chrome", "Chrome"],
  ["quickhack", "Quickhacks"],
  ["spell", "Net spells"],
];

function line(label, value) {
  if (value == null || value === "" || value === false) return "";
  if (value === true) value = "yes";
  return `<p class="beast-line"><b>${esc(label)}</b> ${esc(value)}</p>`;
}

function kindLabel(k) {
  return ({
    weapon: "Weapons",
    armor: "Armor",
    gear: "Gear",
    magic: "Magic",
    spell: "Spells",
    chrome: "Chrome",
    quickhack: "Quickhacks",
  }[k] || k);
}

function compareName(a, b) {
  return String(a.name || "").localeCompare(b.name || "", "en", { numeric: true, sensitivity: "base" });
}

function kindsFor(world) {
  return world === "blight" ? KINDS_RED : KINDS_SRD;
}

function kindOrder(world) {
  return kindsFor(world).filter(([id]) => id !== "all").map(([id]) => id);
}

export function createKit() {
  const cache = { hearthsong: null, blight: null };
  const state = {
    world: "hearthsong",
    all: [],
    id: "",
    q: "",
    kind: "all",
  };

  const $ = (sel) => document.querySelector(sel);

  function urlFor(world) {
    return world === "blight" ? "data/red-kit.json" : "data/srd-kit.json";
  }

  async function ensure(world) {
    const w = world || worldOf();
    if (cache[w]) {
      state.world = w;
      state.all = cache[w];
      return;
    }
    const res = await fetch(urlFor(w), { cache: "no-store" });
    if (!res.ok) throw new Error("kit missing");
    const rows = await res.json();
    const order = kindOrder(w);
    rows.sort((a, b) => {
      const ia = order.indexOf(a.kind);
      const ib = order.indexOf(b.kind);
      const da = ia < 0 ? 99 : ia;
      const db = ib < 0 ? 99 : ib;
      if (da !== db) return da - db;
      return compareName(a, b);
    });
    cache[w] = rows;
    state.world = w;
    state.all = rows;
    if (!rows.some((r) => r.id === state.id)) state.id = rows[0]?.id || "";
  }

  function item(id) {
    if (!id) return null;
    for (const list of Object.values(cache)) {
      const hit = list?.find((r) => r.id === id);
      if (hit) return hit;
    }
    return state.all.find((r) => r.id === id) || null;
  }

  function filtered() {
    const q = state.q.trim().toLowerCase();
    const order = kindOrder(state.world);
    const rows = state.all.filter((m) => {
      if (state.kind !== "all" && m.kind !== state.kind) return false;
      if (!q) return true;
      const hay = `${m.name} ${m.kind} ${m.cat || ""} ${m.text || ""} ${m.school || ""} ${m.classes || ""}`.toLowerCase();
      return hay.includes(q);
    });
    rows.sort((a, b) => {
      const ia = order.indexOf(a.kind);
      const ib = order.indexOf(b.kind);
      const da = ia < 0 ? 99 : ia;
      const db = ib < 0 ? 99 : ib;
      if (da !== db) return da - db;
      return compareName(a, b);
    });
    return rows;
  }

  function current() {
    return state.all.find((m) => m.id === state.id) || filtered()[0] || null;
  }

  function blockHTML(m) {
    if (!m) return `<div class="beast-empty">Choose a piece from the list. Drag it onto a character sheet to add it.</div>`;
    const sub = [kindLabel(m.kind).replace(/s$/, "") || m.kind, m.cat, m.school ? `lv ${m.lv ?? 0} ${m.school}` : "", m.rarity]
      .filter(Boolean)
      .join(" · ");
    return `
      <article class="beast-block" draggable="true" data-kit-id="${esc(m.id)}">
        <p class="kit-drag-hint">Drag this onto an open character to add it.</p>
        ${kitHero(m)}
        <h3>${esc(m.name)}</h3>
        <p class="beast-kind">${esc(sub)}</p>
        ${line("Cost", m.cost)}
        ${line("Damage", m.dmg)}
        ${line("Properties", m.props)}
        ${line("Weight", m.wt)}
        ${line("Armor", m.ac)}
        ${line("Stealth", m.stealth)}
        ${line("STR req", m.str)}
        ${line("ROF", m.rof)}
        ${line("Shots", m.shots)}
        ${line("Humanity", m.hl != null && m.hl !== "" ? m.hl : "")}
        ${line("Attunement", m.attune)}
        ${line("Casting time", m.time)}
        ${line("Range", m.range)}
        ${line("Components", m.comp)}
        ${line("Duration", m.dur)}
        ${line("Classes", m.classes)}
        ${m.text ? `<p class="beast-blurb">${esc(m.text)}</p>` : ""}
        <p class="kit-drag-hint">Or click Add. It lands on Combat, Magic, Chrome, or Gear, ready to roll.</p>
        <button type="button" class="primary" data-kit-add="${esc(m.id)}">Add to open sheet</button>
      </article>`;
  }

  function paintKinds() {
    const sel = $("#kit-kind");
    if (!sel) return;
    const opts = kindsFor(state.world);
    const keep = opts.some(([id]) => id === state.kind) ? state.kind : "all";
    state.kind = keep;
    sel.innerHTML = opts.map(([id, label]) => `<option value="${id}"${id === keep ? " selected" : ""}>${label}</option>`).join("");
  }

  function paint() {
    const list = $("#kit-list");
    const page = $("#kit-page");
    const count = $("#kit-count");
    const src = $("#kit-src");
    const title = $("#kit-toggle");
    if (src) src.textContent = state.world === "blight" ? "CYBERPUNK RED" : "5e SRD";
    if (title && !title.dataset.locked) {
      /* label set from app */
    }
    if (!list || !page) return;
    const rows = filtered();
    if (count) count.textContent = `${rows.length} of ${state.all.length}`;
    if (!rows.some((m) => m.id === state.id) && rows[0]) state.id = rows[0].id;
    const sig = `${state.world}\0${state.q}\0${state.kind}\0${rows.length}`;
    if (sig !== state.listSig) {
      state.listSig = sig;
      if (!rows.length) {
        list.innerHTML = `<p class="beast-empty">Nothing matches.</p>`;
      } else {
        const labels = Object.fromEntries(kindsFor(state.world));
        const counts = {};
        for (const m of rows) counts[m.kind] = (counts[m.kind] || 0) + 1;
        let last = "";
        let html = "";
        for (const m of rows) {
          if (m.kind !== last) {
            last = m.kind;
            const label = labels[m.kind] || kindLabel(m.kind);
            html += `<div class="kit-kind-head" data-kit-kind="${esc(m.kind)}">${esc(label)} <small>${counts[m.kind]}</small></div>`;
          }
          const on = m.id === state.id ? " on" : "";
          const small = m.kind === "spell"
            ? (m.lv == null || m.lv === "" ? esc(m.cat || "Net") : `Lv ${m.lv} · ${esc(m.school || "")}`)
            : `${esc(m.cat || kindLabel(m.kind))}${m.cost ? ` · ${esc(m.cost)}` : ""}`;
          html += `<button type="button" class="beast-row${on}" draggable="true" data-kit-id="${esc(m.id)}">${kitThumb(m)}<span><b>${esc(m.name)}</b><small>${small}</small></span></button>`;
        }
        list.innerHTML = html;
      }
    } else {
      for (const btn of list.querySelectorAll(".beast-row")) {
        btn.classList.toggle("on", btn.dataset.kitId === state.id);
      }
    }
    if (state.pageId !== state.id) {
      state.pageId = state.id;
      page.innerHTML = blockHTML(current());
    }
  }

  async function openBoard() {
    const el = $("#kit-board");
    if (!el) return;
    el.hidden = false;
    document.querySelector(".app")?.classList.add("kit-open");
    $("#kit-toggle")?.classList.add("on");
    try {
      await ensure(worldOf());
      paintKinds();
      paint();
    } catch {
      const page = $("#kit-page");
      if (page) page.innerHTML = `<div class="beast-empty">The locker could not be opened.</div>`;
    }
  }

  function closeBoard() {
    const el = $("#kit-board");
    if (el) el.hidden = true;
    document.querySelector(".app")?.classList.remove("kit-open");
    $("#kit-toggle")?.classList.remove("on");
  }

  function toggle() {
    const el = $("#kit-board");
    if (!el || el.hidden) openBoard();
    else closeBoard();
  }

  function syncTheme() {
    const blight = worldOf() === "blight";
    const btn = $("#kit-toggle");
    if (btn) {
      btn.title = blight
        ? "Weapons, chrome, gear, and quickhacks. Drag onto a character."
        : "Weapons, armor, gear, magic, and spells. Drag onto a character.";
    }
    if (!$("#kit-board")?.hidden) {
      state.listSig = "";
      state.pageId = "";
      ensure(worldOf()).then(() => {
        paintKinds();
        paint();
      });
    }
  }

  function bind() {
    bindKitArt();
    $("#kit-search")?.addEventListener("input", (e) => {
      state.q = e.target.value || "";
      paint();
    });
    $("#kit-kind")?.addEventListener("change", (e) => {
      state.kind = e.target.value || "all";
      paint();
    });
    $("#kit-list")?.addEventListener("click", (e) => {
      const head = e.target.closest("[data-kit-kind]");
      if (head?.dataset.kitKind) {
        state.kind = state.kind === head.dataset.kitKind ? "all" : head.dataset.kitKind;
        paintKinds();
        paint();
        return;
      }
      const btn = e.target.closest("[data-kit-id]");
      if (!btn) return;
      state.id = btn.dataset.kitId;
      paint();
    });
    $("#kit-list")?.addEventListener("dblclick", (e) => {
      const btn = e.target.closest("[data-kit-id]");
      if (!btn?.dataset.kitId) return;
      document.dispatchEvent(new CustomEvent("blight-kit-add", { detail: { id: btn.dataset.kitId } }));
    });
    $("#kit-page")?.addEventListener("click", (e) => {
      const add = e.target.closest("[data-kit-add]");
      if (!add?.dataset.kitAdd) return;
      document.dispatchEvent(new CustomEvent("blight-kit-add", { detail: { id: add.dataset.kitAdd } }));
    });
    $("#kit-close")?.addEventListener("click", closeBoard);
  }

  return { openBoard, closeBoard, toggle, bind, item, ensure, syncTheme };
}
