import { worldOf } from "./catalog.js";
import { kitThumb, kitHero, bindKitArt } from "./kit-art.js";
import { formatMoney, parsePrice, sellPrice, walletText } from "./money.js";
import { classById, classByName, slotsFor } from "./advance.js";

function esc(value) {
  return String(value ?? "").replace(/[&<>"']/g, (ch) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[ch])
  );
}

const STALLS_5E = [
  { id: "square", name: "Market square", kinds: ["gear"], blurb: "Rations, rope, lamps. Honest prices." },
  { id: "forge", name: "The Forge", kinds: ["weapon"], blurb: "Steel that still has a smith's name on it." },
  { id: "mail", name: "Mail & Hide", kinds: ["armor"], blurb: "Leather to plate. Shields on the wall." },
  { id: "curio", name: "Curio stall", kinds: ["magic"], blurb: "Trinkets that hum. Maybe magic." },
  { id: "scribe", name: "The Scribe", kinds: ["spell"], blurb: "Ink, formulae, and things best read twice." },
];

const STALLS_RED = [
  { id: "market", name: "Night Market", kinds: ["gear"], blurb: "Kibble, tape, and things that beep." },
  { id: "gunmart", name: "Gunmart", kinds: ["weapon"], blurb: "If it fires, they sold it twice." },
  { id: "ripper", name: "Ripperdoc", kinds: ["chrome", "armor"], blurb: "Clinic chair. Chrome and jackets." },
  { id: "net", name: "Net stall", kinds: ["quickhack", "spell"], blurb: "Chips. Don't ask where from." },
  { id: "fixer", name: "Fixer desk", kinds: ["all"], blurb: "Anything. They buy your junk at half." },
];

function stalls() {
  return worldOf() === "blight" ? STALLS_RED : STALLS_5E;
}

function rarityFloor(raw) {
  const first = String(raw || "").toLowerCase().split(" to ")[0].trim();
  if (!first) return "";
  if (first.includes("legendary")) return "legendary";
  if (first.includes("very rare")) return "very rare";
  if (first.includes("uncommon")) return "uncommon";
  if (first.includes("rare")) return "rare";
  if (first.includes("common")) return "common";
  return first;
}

function fiveeMinLevel(item) {
  const kind = item.kind;
  const name = String(item.name || "").toLowerCase();
  if (kind === "spell") return 0;
  const r = rarityFloor(item.rarity);
  if (r === "legendary") return 17;
  if (r === "very rare") return 11;
  if (r === "rare") return 5;
  if (r === "uncommon") return 3;
  if (r === "common") return 1;
  if (kind === "armor") {
    if (name === "plate") return 5;
    if (name === "splint" || name === "half plate" || name === "breastplate") return 3;
  }
  return 1;
}

function blightMinRank(item) {
  const eb = parsePrice(item.cost);
  let rank = 1;
  if (eb > 100000) rank = 10;
  else if (eb > 20000) rank = 9;
  else if (eb > 5000) rank = 7;
  else if (eb > 1000) rank = 5;
  else if (eb > 500) rank = 3;
  else if (eb > 100) rank = 2;
  const hl = Number(item.hl) || 0;
  if (hl >= 14) rank = Math.max(rank, 5);
  else if (hl >= 7) rank = Math.max(rank, 3);
  return rank;
}

function maxSpellSlotLevel(sheet) {
  if (!sheet) return 0;
  const cls = classById(sheet.classId) || classByName(sheet.className);
  const lv = Math.max(1, Number(sheet.level) || 1);
  let slots = cls?.caster ? slotsFor(cls, lv) : null;
  if (!slots) slots = sheet.slotsMax;
  if (!Array.isArray(slots)) return cls?.caster ? 0 : -1;
  let max = -1;
  for (let i = 0; i < slots.length; i += 1) {
    if (Number(slots[i]) > 0) max = i + 1;
  }
  if (max < 0 && cls?.caster) return 0;
  return max;
}

function isCasterSheet(sheet) {
  if (!sheet) return false;
  const cls = classById(sheet.classId) || classByName(sheet.className);
  if (cls?.caster) return true;
  if (String(sheet.spellClass || "").trim()) return true;
  return (sheet.slotsMax || []).some((n) => Number(n) > 0);
}

function itemFits(item, sheet, world) {
  if (!item) return false;
  const blight = world === "blight";
  const grade = blight ? Math.max(1, Number(sheet?.roleRank) || 1) : Math.max(1, Number(sheet?.level) || 1);
  if (blight) return blightMinRank(item) <= grade;
  if (item.kind === "spell") {
    const need = Number(item.lv) || 0;
    if (need <= 0) return isCasterSheet(sheet);
    return need <= maxSpellSlotLevel(sheet);
  }
  return fiveeMinLevel(item) <= grade;
}

function compareName(a, b) {
  return String(a.name || "").localeCompare(b.name || "", "en", { numeric: true, sensitivity: "base" });
}

export function createVendors(hooks) {
  const cache = { hearthsong: null, blight: null };
  const state = { world: "hearthsong", all: [], id: "", q: "", stall: "" };

  const $ = (sel) => document.querySelector(sel);

  function api() {
    return typeof hooks === "function" ? hooks() : hooks;
  }

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
    if (!res.ok) throw new Error("vendors");
    const rows = await res.json();
    rows.sort(compareName);
    cache[w] = rows;
    state.world = w;
    state.all = rows;
  }

  function stallOf() {
    const list = stalls();
    return list.find((s) => s.id === state.stall) || list[0];
  }

  function shopper() {
    const sheet = ch();
    if (!sheet || api()?.isRemote?.(sheet)) return null;
    return sheet;
  }

  function stockLabel() {
    const sheet = shopper();
    const blight = worldOf() === "blight";
    if (!sheet) return blight ? "Street stock · open a sheet" : "Common stock · open a sheet";
    const name = sheet.name || "Unnamed";
    return blight
      ? `${name} · rank ${Math.max(1, Number(sheet.roleRank) || 1)}`
      : `${name} · level ${Math.max(1, Number(sheet.level) || 1)}`;
  }

  function filtered() {
    const stall = stallOf();
    const q = state.q.trim().toLowerCase();
    const sheet = shopper();
    const world = worldOf();
    return state.all.filter((m) => {
      if (stall.kinds[0] !== "all" && !stall.kinds.includes(m.kind)) return false;
      if (!itemFits(m, sheet, world)) return false;
      if (!q) return true;
      const hay = `${m.name} ${m.kind} ${m.cat || ""} ${m.text || ""} ${m.cost || ""} ${m.rarity || ""}`.toLowerCase();
      return hay.includes(q);
    });
  }

  function current() {
    return filtered().find((m) => m.id === state.id) || filtered()[0] || null;
  }

  function ch() {
    return api()?.active?.() || null;
  }

  function walletLine() {
    const sheet = ch();
    if (!sheet) return "Open a character sheet to buy or sell.";
    if (api()?.isRemote?.(sheet)) return "That sheet is someone else's. Open yours.";
    return (sheet.name || "Unnamed") + " · " + walletText(sheet);
  }

  function blockHTML(m) {
    if (!m) return `<div class="beast-empty">Pick a stall, then a piece. Buy puts it on the open sheet.</div>`;
    const price = parsePrice(m.cost);
    const sheet = ch();
    const mine = sheet && !api()?.isRemote?.(sheet);
    const afford = mine && api()?.canPay?.(sheet, price);
    return `
      <article class="beast-block">
        ${kitHero(m)}
        <h3>${esc(m.name)}</h3>
        <p class="beast-kind">${esc(m.kind)} · ${esc(m.cat || "")}</p>
        <p class="vendor-price">${esc(m.cost || formatMoney(price))}</p>
        ${m.dmg ? `<p class="beast-line"><b>Damage</b> ${esc(m.dmg)}</p>` : ""}
        ${m.ac ? `<p class="beast-line"><b>Armor</b> ${esc(m.ac)}</p>` : ""}
        ${m.text ? `<p class="beast-blurb">${esc(m.text)}</p>` : ""}
        <p class="vendor-wallet">${esc(walletLine())}</p>
        <button type="button" class="primary" data-vendor-buy="${esc(m.id)}" ${!mine || (price && !afford) ? "disabled" : ""}>${
          !mine ? "Open your sheet" : price && !afford ? "Too expensive" : "Buy"
        }</button>
      </article>`;
  }

  function sellHTML() {
    const sheet = ch();
    if (!sheet || api()?.isRemote?.(sheet)) {
      return `<p class="char-hint">Open your sheet to sell from inventory.</p>`;
    }
    const rows = (sheet.kit || []).map((k, i) => {
      const full = parsePrice(k.cost);
      const half = sellPrice(k.cost);
      const label = half ? formatMoney(half) : "no price";
      return `<div class="kit-owned">
        ${kitThumb({ id: k.catalogId || k.id, name: k.name, kind: k.kind, dmg: k.dmg, world: worldOf() })}
        <span><b>${esc(k.name)}</b><small>${esc(k.kind)}${k.qty > 1 ? ` ×${k.qty}` : ""} · sell ${esc(label)}</small></span>
        <button type="button" class="ghost" data-vendor-sell="${i}" ${half ? "" : "disabled"}>${half ? "Sell" : "—"}</button>
      </div>`;
    });
    return rows.length ? rows.join("") : `<p class="char-hint">Nothing in inventory to sell.</p>`;
  }

  function paintStalls() {
    const bar = $("#vendor-stalls");
    if (!bar) return;
    const list = stalls();
    if (!list.some((s) => s.id === state.stall)) state.stall = list[0].id;
    bar.innerHTML = list
      .map(
        (s) =>
          `<button type="button" class="ghost${s.id === state.stall ? " on" : ""}" data-vendor-stall="${esc(s.id)}">${esc(s.name)}</button>`
      )
      .join("");
  }

  function paint() {
    const list = $("#vendor-list");
    const page = $("#vendor-page");
    const count = $("#vendor-count");
    const src = $("#vendor-src");
    const sell = $("#vendor-sell");
    const purse = $("#vendor-purse");
    const stall = stallOf();
    if (src) src.textContent = worldOf() === "blight" ? "NIGHT CITY" : "THE STALLS";
    if (purse) purse.textContent = walletLine() + " · " + stockLabel();
    if (!list || !page) return;
    const rows = filtered();
    if (count) count.textContent = `${rows.length}`;
    if (!rows.some((m) => m.id === state.id) && rows[0]) state.id = rows[0].id;
    if (!rows.length) {
      const sheet = shopper();
      list.innerHTML = sheet
        ? `<p class="beast-empty">Nothing here for ${esc(stockLabel())}. Try another stall, or come back after you advance.</p>`
        : `<p class="beast-empty">Open your sheet. Stalls only stock what that person can use.</p>`;
    } else {
      list.innerHTML = rows
        .map((m) => {
          const on = m.id === state.id ? " on" : "";
          return `<button type="button" class="beast-row${on}" data-vendor-id="${esc(m.id)}">${kitThumb(m)}<span><b>${esc(m.name)}</b><small>${esc(m.cost || m.kind)}</small></span></button>`;
        })
        .join("");
    }
    page.innerHTML =
      `<p class="char-hint">${esc(stall.blurb)} Stocked for ${esc(stockLabel())}.</p>` + blockHTML(current());
    if (sell) sell.innerHTML = `<div class="char-section-label">Sell (half price)</div>` + sellHTML();
  }

  async function openBoard() {
    const el = $("#vendor-board");
    if (!el) return;
    el.hidden = false;
    document.querySelector(".app")?.classList.add("kit-open", "vendor-open");
    $("#vendor-toggle")?.classList.add("on");
    try {
      await ensure(worldOf());
      paintStalls();
      paint();
    } catch {
      const page = $("#vendor-page");
      if (page) page.innerHTML = `<div class="beast-empty">The stalls would not open.</div>`;
    }
  }

  function closeBoard() {
    const el = $("#vendor-board");
    if (el) el.hidden = true;
    document.querySelector(".app")?.classList.remove("vendor-open");
    if ($("#kit-board")?.hidden) document.querySelector(".app")?.classList.remove("kit-open");
    $("#vendor-toggle")?.classList.remove("on");
  }

  function toggle() {
    const el = $("#vendor-board");
    if (!el || el.hidden) openBoard();
    else closeBoard();
  }

  function syncTheme() {
    const blight = worldOf() === "blight";
    const btn = $("#vendor-toggle");
    if (btn) {
      btn.title = blight ? "Buy and sell at Night City stalls." : "Buy and sell at the market stalls.";
    }
    if (!$("#vendor-board")?.hidden) {
      state.stall = "";
      ensure(worldOf()).then(() => {
        paintStalls();
        paint();
      });
    }
  }

  function bind() {
    bindKitArt();
    $("#vendor-search")?.addEventListener("input", (e) => {
      state.q = e.target.value || "";
      paint();
    });
    $("#vendor-stalls")?.addEventListener("click", (e) => {
      const btn = e.target.closest("[data-vendor-stall]");
      if (!btn) return;
      state.stall = btn.dataset.vendorStall;
      paintStalls();
      paint();
    });
    $("#vendor-list")?.addEventListener("click", (e) => {
      const btn = e.target.closest("[data-vendor-id]");
      if (!btn) return;
      state.id = btn.dataset.vendorId;
      paint();
    });
    $("#vendor-page")?.addEventListener("click", (e) => {
      const buy = e.target.closest("[data-vendor-buy]");
      if (!buy) return;
      const item = (cache[worldOf()] || state.all).find((m) => m.id === buy.dataset.vendorBuy);
      if (item && !itemFits(item, shopper(), worldOf())) {
        api()?.flash?.("Not stocked for this level.");
        paint();
        return;
      }
      const out = api()?.buyItem?.(item);
      if (out?.error) api()?.flash?.(out.error);
      paint();
    });
    $("#vendor-sell")?.addEventListener("click", (e) => {
      const sell = e.target.closest("[data-vendor-sell]");
      if (!sell) return;
      const out = api()?.sellItem?.(Number(sell.dataset.vendorSell));
      if (out?.error) api()?.flash?.(out.error);
      paint();
    });
    $("#vendor-close")?.addEventListener("click", closeBoard);
    document.addEventListener("blightnet-sheet", () => {
      if (!$("#vendor-board")?.hidden) paint();
    });
  }

  return { openBoard, closeBoard, toggle, bind, syncTheme, paint };
}
