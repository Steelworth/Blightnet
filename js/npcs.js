const KINDS_RED = [
  ["legend", "Legend"],
  ["corpo", "Corpo"],
  ["hope", "Forlorn Hope"],
  ["street", "Street"],
  ["gang", "Gang"],
  ["nomad", "Nomad"],
  ["law", "Law"],
  ["net", "Net"],
  ["media", "Media"],
];

const KINDS_SRD = [
  ["town", "Town"],
  ["martial", "Martial"],
  ["magic", "Magic"],
  ["faith", "Faith"],
  ["criminal", "Criminal"],
  ["wild", "Wild"],
];

const STATS_RED = [
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

const STATS_SRD = [
  ["str", "STR"],
  ["dex", "DEX"],
  ["con", "CON"],
  ["int", "INT"],
  ["wis", "WIS"],
  ["cha", "CHA"],
];

function byName(a, b) {
  return String(a.name || "").localeCompare(String(b.name || ""), undefined, { sensitivity: "base" });
}

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

function line(label, value) {
  if (value == null || value === "") return "";
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

function scoreRed(label, n) {
  return `<div class="beast-score"><span>${esc(label)}</span><b>${esc(n)}</b></div>`;
}

function score5e(label, n) {
  return `<div class="beast-score"><span>${esc(label)}</span><b>${esc(n)}</b><small>(${mod(n)})</small></div>`;
}

export function createNpcs(opts = {}) {
  const prefix = opts.prefix || "npc";
  const dataUrl = opts.data || "data/npcs.json";
  const artDir = opts.art || "assets/npcs";
  const artV = opts.artV || "1";
  const kinds = opts.kinds || KINDS_RED;
  const openClass = opts.openClass || "npc-open";
  const tokenKind = opts.tokenKind || "npc";
  const fivee = Boolean(opts.fivee);
  const emptyPick = opts.empty || (fivee ? "Pick a person." : "Pick a face.");
  const emptyNone = opts.emptyNone || (fivee ? "No NPCs match." : "No faces match.");
  const emptyFail = opts.emptyFail || (fivee ? "The NPCs would not load." : "The faces would not load.");

  const state = {
    all: [],
    ready: false,
    id: "",
    q: "",
    kind: "all",
  };

  const $ = (part) => document.getElementById(`${prefix}-${part}`);

  function artUrl(id, bust) {
    const q = bust ? `v=${artV}&r=${bust}` : `v=${artV}`;
    return `${artDir}/${esc(id)}.jpg?${q}`;
  }

  function recoverArt(img) {
    if (!(img instanceof HTMLImageElement) || !img.dataset.npcArt) return;
    if (img.dataset.retried) {
      img.hidden = true;
      if (img.parentElement?.classList.contains("beast-art")) img.parentElement.hidden = true;
      return;
    }
    img.dataset.retried = "1";
    img.src = artUrl(img.dataset.npcArt, Date.now());
  }

  function tokenAttrs(m, src) {
    return `draggable="true" data-npc-art="${esc(m.id)}" data-token-kind="${esc(tokenKind)}" data-token-ref="${esc(m.id)}" data-token-name="${esc(m.name)}" data-token-src="${src}"`;
  }

  function blockHTML(m) {
    if (!m) return `<div class="beast-empty">${esc(emptyPick)}</div>`;
    const art = artUrl(m.id);
    const weps = (m.weapons || [])
      .map((w) => `<p class="beast-trait"><b>${esc(w.n)}.</b> ${esc(w.d)}</p>`)
      .join("");
    const paras = String(m.text || "")
      .split(/\n\n+/)
      .filter(Boolean)
      .map((p) => `<p class="beast-blurb">${esc(p)}</p>`)
      .join("");
    if (fivee || m.str != null) {
      const kind = [m.size, m.align, m.cr ? `CR ${m.cr}` : "", m.kind].filter(Boolean).join(" · ");
      const xp = m.xp != null && m.xp !== "" ? `${Number(m.xp).toLocaleString()} XP` : "";
      return `
    <article class="beast-block">
      <figure class="beast-art">
        <img src="${art}" alt="" decoding="async" ${tokenAttrs(m, art)} />
      </figure>
      <h3>${esc(m.name)}</h3>
      <p class="beast-kind">${esc(kind)}</p>
      ${line("Role", m.role || m.aka)}
      ${m.blurb ? `<p class="beast-blurb">${esc(m.blurb)}</p>` : ""}
      ${line("Armor Class", m.ac)}
      ${line("Hit Points", m.hp)}
      ${line("Speed", m.speed)}
      <div class="beast-scores">
        ${STATS_SRD.map(([id, lab]) => score5e(lab, m[id])).join("")}
      </div>
      ${line("Saving Throws", m.saves)}
      ${line("Skills", m.skills)}
      ${line("Senses", m.senses)}
      ${line("Languages", m.lang)}
      ${line("Challenge", [m.cr ? `CR ${m.cr}` : "", xp].filter(Boolean).join(" · "))}
      ${weps ? `<h4>Actions</h4>${weps}` : ""}
      ${section("Traits", m.traits)}
      ${paras}
      ${section("Hooks", m.hooks)}
    </article>`;
    }
    const kind = [m.role, m.aka, m.status].filter(Boolean).join(" · ");
    return `
    <article class="beast-block">
      <figure class="beast-art">
        <img src="${art}" alt="" decoding="async" ${tokenAttrs(m, art)} />
      </figure>
      <h3>${esc(m.name)}</h3>
      <p class="beast-kind">${esc(kind)} · ${esc(m.kind)}</p>
      ${line("District", m.district)}
      ${line("Affiliation", m.affiliation)}
      ${line("Reputation", m.rep)}
      ${line("Hit points", m.hp)}
      ${line("Armor SP (head/body)", m.sp)}
      ${line("Humanity", m.humanity)}
      <div class="beast-scores">
        ${STATS_RED.map(([id, lab]) => scoreRed(lab, m[id])).join("")}
      </div>
      ${weps ? `<h4>Weapons</h4>${weps}` : ""}
      ${line("Skills", m.skills)}
      ${line("Cyberware", m.chrome)}
      ${line("Gear", m.gear)}
      ${paras}
      ${section("Hooks", m.hooks)}
    </article>`;
  }

  async function ensure() {
    if (state.ready) return;
    const res = await fetch(dataUrl, { cache: "no-store" });
    if (!res.ok) throw new Error("npcs missing");
    state.all = (await res.json()).slice().sort(byName);
    state.ready = true;
    if (!state.id && state.all[0]) state.id = state.all[0].id;
  }

  function filtered() {
    const q = state.q.trim().toLowerCase();
    return state.all
      .filter((m) => {
        if (state.kind !== "all" && m.kind !== state.kind) return false;
        if (!q) return true;
        const hay = `${m.name} ${m.aka || ""} ${m.role || ""} ${m.kind} ${m.district || ""} ${m.affiliation || ""} ${m.blurb || ""} ${m.text || ""} ${m.skills || ""}`.toLowerCase();
        return hay.includes(q);
      })
      .slice()
      .sort(byName);
  }

  function current() {
    return state.all.find((m) => m.id === state.id) || filtered()[0] || null;
  }

  function paintKinds() {
    const sel = $("kind");
    if (!sel) return;
    const keep = kinds.some(([id]) => id === state.kind) ? state.kind : "all";
    state.kind = keep;
    sel.innerHTML =
      `<option value="all">All kinds</option>` +
      kinds.map(([id, label]) => `<option value="${esc(id)}"${id === keep ? " selected" : ""}>${esc(label)}</option>`).join("");
  }

  function paint() {
    const list = $("list");
    const page = $("page");
    const count = $("count");
    if (!list || !page) return;
    const rows = filtered();
    if (count) count.textContent = `${rows.length} of ${state.all.length}`;
    if (!rows.some((m) => m.id === state.id) && rows[0]) state.id = rows[0].id;
    const sig = `${state.q}\0${state.kind}\0${rows.length}\0${state.all.length}`;
    if (sig !== state.listSig) {
      state.listSig = sig;
      list.innerHTML = rows.length
        ? rows
            .map((m) => {
              const on = m.id === state.id ? " on" : "";
              const sub = fivee || m.str != null
                ? [m.role || m.kind, m.cr ? `CR ${m.cr}` : ""].filter(Boolean).join(" · ")
                : [m.role || m.kind, m.district].filter(Boolean).join(" · ");
              const src = artUrl(m.id);
              return `<button type="button" class="beast-row${on}" data-npc="${esc(m.id)}"><img class="beast-thumb" src="${src}" alt="" decoding="async" loading="lazy" ${tokenAttrs(m, src)} /><span><b>${esc(m.name)}</b><small>${esc(sub)}</small></span></button>`;
            })
            .join("")
        : `<p class="beast-empty">${esc(emptyNone)}</p>`;
    } else {
      for (const btn of list.querySelectorAll(".beast-row")) {
        btn.classList.toggle("on", btn.dataset.npc === state.id);
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
    const el = $("board");
    if (!el) return;
    el.hidden = false;
    document.querySelector(".app")?.classList.add(openClass);
    document.getElementById(`${prefix}-toggle`)?.classList.add("on");
    ensure()
      .then(() => {
        paintKinds();
        paint();
      })
      .catch(() => {
        const page = $("page");
        if (page) page.innerHTML = `<div class="beast-empty">${esc(emptyFail)}</div>`;
      });
  }

  function closeBoard() {
    const el = $("board");
    if (el) el.hidden = true;
    document.querySelector(".app")?.classList.remove(openClass);
    document.getElementById(`${prefix}-toggle`)?.classList.remove("on");
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
      const btn = e.target.closest("[data-npc]");
      if (!btn) return;
      state.id = btn.dataset.npc;
      paint();
    });
    const onArtErr = (e) => recoverArt(e.target);
    $("list")?.addEventListener("error", onArtErr, true);
    $("page")?.addEventListener("error", onArtErr, true);
    $("close")?.addEventListener("click", closeBoard);
  }

  return { openBoard, closeBoard, toggle, bind, KINDS: kinds };
}

export { KINDS_RED as KINDS, KINDS_SRD };
