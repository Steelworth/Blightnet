import { worldOf } from "./catalog.js";
import { mergeCreator, AUGS } from "./portrait.js";
import { createDice, parseDice } from "./dice.js";
import { kitThumb, bindKitArt } from "./kit-art.js";
import {
  CLASSES,
  classById,
  classByName,
  subclassById,
  xpForLevel,
  xpToNext,
  slotsFor,
  avgHpGain,
  rollHpGain,
  startingHp,
  isAsiLevel,
  applyClassBasics,
  grantLevelFeatures,
  skillIpCost,
  roleIpCost,
  statIpCost,
  roleBlurb,
} from "./advance.js";

const KEY = "hearthsong.characters";

const ABILS = [
  ["str", "STR", "Strength"],
  ["dex", "DEX", "Dexterity"],
  ["con", "CON", "Constitution"],
  ["int", "INT", "Intelligence"],
  ["wis", "WIS", "Wisdom"],
  ["cha", "CHA", "Charisma"],
];

const SKILLS = [
  ["acrobatics", "Acrobatics", "dex"],
  ["animalHandling", "Animal Handling", "wis"],
  ["arcana", "Arcana", "int"],
  ["athletics", "Athletics", "str"],
  ["deception", "Deception", "cha"],
  ["history", "History", "int"],
  ["insight", "Insight", "wis"],
  ["intimidation", "Intimidation", "cha"],
  ["investigation", "Investigation", "int"],
  ["medicine", "Medicine", "wis"],
  ["nature", "Nature", "int"],
  ["perception", "Perception", "wis"],
  ["performance", "Performance", "cha"],
  ["persuasion", "Persuasion", "cha"],
  ["religion", "Religion", "int"],
  ["sleightOfHand", "Sleight of Hand", "dex"],
  ["stealth", "Stealth", "dex"],
  ["survival", "Survival", "wis"],
];

const ALIGNMENTS = [
  "Lawful Good",
  "Neutral Good",
  "Chaotic Good",
  "Lawful Neutral",
  "True Neutral",
  "Chaotic Neutral",
  "Lawful Evil",
  "Neutral Evil",
  "Chaotic Evil",
  "Unaligned",
];

const TABS = [
  ["bio", "Bio"],
  ["stats", "Stats"],
  ["combat", "Combat"],
  ["magic", "Magic"],
  ["features", "Features"],
  ["gear", "Gear"],
  ["story", "Story"],
];

const TABS_RED = [
  ["bio", "Bio"],
  ["stats", "Stats"],
  ["combat", "Combat"],
  ["skills", "Skills"],
  ["chrome", "Chrome"],
  ["gear", "Gear"],
  ["life", "Life"],
];

const CP_STATS = [
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

const CP_ROLES = ["Gamemaster", "Rockerboy", "Solo", "Netrunner", "Tech", "Medtech", "Media", "Exec", "Lawman", "Fixer", "Nomad"];

const CP_SKILLS = [
  ["athletics", "Athletics", "dex"],
  ["brawling", "Brawling", "dex"],
  ["evasion", "Evasion", "dex"],
  ["melee", "Melee", "dex"],
  ["handgun", "Handgun", "ref"],
  ["autofire", "Autofire", "ref"],
  ["stealth", "Stealth", "dex"],
  ["perception", "Perception", "int"],
  ["conversation", "Conversation", "emp"],
  ["persuasion", "Persuasion", "cool"],
  ["streetwise", "Streetwise", "cool"],
  ["concentration", "Concentration", "will"],
  ["education", "Education", "int"],
  ["firstAid", "First Aid", "tech"],
  ["basicTech", "Basic Tech", "tech"],
  ["cybertech", "Cybertech", "tech"],
  ["driveLand", "Drive Land", "ref"],
  ["conceal", "Conceal/Reveal", "int"],
];

function escapeHtml(value) {
  return String(value ?? "").replace(/[&<>"']/g, (ch) => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
  }[ch]));
}

function modifier(score) {
  const n = Number(score);
  if (!Number.isFinite(n)) return 0;
  return Math.floor((n - 10) / 2);
}

function signed(n) {
  const v = Number(n) || 0;
  return v >= 0 ? `+${v}` : String(v);
}

function profForLevel(level) {
  const lv = Math.min(20, Math.max(1, Number(level) || 1));
  return Math.ceil(lv / 4) + 1;
}

function boolMap(keys, raw) {
  const out = {};
  for (const key of keys) out[key] = Boolean(raw && raw[key]);
  return out;
}

function blankChar() {
  const abilities = {};
  const saveProf = {};
  for (const [id] of ABILS) {
    abilities[id] = 10;
    saveProf[id] = false;
  }
  const skillProf = {};
  const skillExpert = {};
  for (const [id] of SKILLS) {
    skillProf[id] = false;
    skillExpert[id] = false;
  }
  return {
    id: "c_" + Date.now().toString(36) + Math.random().toString(36).slice(2, 7),
    name: "New character",
    portrait: "",
    fullbody: "",
    player: "",
    race: "",
    className: "",
    classId: "",
    subclass: "",
    subclassId: "",
    level: 1,
    background: "",
    alignment: "",
    xp: 0,
    age: "",
    height: "",
    weight: "",
    eyes: "",
    skin: "",
    hair: "",
    abilities,
    saveProf,
    skillProf,
    skillExpert,
    ac: 10,
    initiative: "",
    speed: "30 ft.",
    hpMax: 0,
    hp: 0,
    hpTemp: 0,
    hitDice: "1d8",
    deathSuccess: 0,
    deathFail: 0,
    downed: false,
    dead: false,
    inspiration: false,
    exhaustion: 0,
    attacks: [{ name: "", bonus: "", damage: "" }],
    features: "",
    feats: "",
    proficiencies: "",
    languages: "",
    equipment: "",
    kit: [],
    programs: "",
    treasure: "",
    cp: 0,
    sp: 0,
    ep: 0,
    gp: 0,
    pp: 0,
    spellClass: "",
    spellAbility: "",
    cantrips: "",
    slotsMax: [0, 0, 0, 0, 0, 0, 0, 0, 0],
    slotsUsed: [0, 0, 0, 0, 0, 0, 0, 0, 0],
    spells: ["", "", "", "", "", "", "", "", ""],
    personality: "",
    ideals: "",
    bonds: "",
    flaws: "",
    backstory: "",
    notes: "",
    allies: "",
    deity: "",
    deityId: "",
    deityDomain: "",
    handle: "",
    role: "Solo",
    roleRank: 4,
    ip: 0,
    statsRed: { int: 6, ref: 6, dex: 6, tech: 5, cool: 5, will: 5, luck: 5, move: 5, body: 6, emp: 5 },
    redSkills: {},
    humanity: 50,
    cyberware: "",
    chrome: { eyes: false, jack: false, arm: false, dermal: false, speed: false },
    lifepath: "",
    creator: mergeCreator({ race: "human", present: "femme" }),
  };
}

function hydrate(raw) {
  const base = blankChar();
  if (!raw || typeof raw !== "object") return base;
  const next = { ...base, ...raw };
  next.id = typeof raw.id === "string" && raw.id ? raw.id : base.id;
  next.abilities = { ...base.abilities, ...(raw.abilities || {}) };
  next.saveProf = boolMap(ABILS.map(([id]) => id), raw.saveProf);
  next.skillProf = boolMap(SKILLS.map(([id]) => id), raw.skillProf);
  next.skillExpert = boolMap(SKILLS.map(([id]) => id), raw.skillExpert);
  next.attacks = Array.isArray(raw.attacks) && raw.attacks.length
    ? raw.attacks.map((row) => ({
        name: row?.name || "",
        bonus: row?.bonus || "",
        damage: row?.damage || "",
      }))
    : base.attacks;
  next.slotsMax = Array.from({ length: 9 }, (_, i) => Number(raw.slotsMax?.[i]) || 0);
  next.slotsUsed = Array.from({ length: 9 }, (_, i) => Number(raw.slotsUsed?.[i]) || 0);
  next.spells = Array.from({ length: 9 }, (_, i) => String(raw.spells?.[i] || ""));
  next.statsRed = { ...base.statsRed, ...(raw.statsRed || {}) };
  next.redSkills = { ...(raw.redSkills || {}) };
  next.chrome = { ...base.chrome, ...(raw.chrome || {}) };
  next.kit = Array.isArray(raw.kit)
    ? raw.kit
        .filter((row) => row && row.name)
        .map((row) => ({
          id: row.id || "k_" + Math.random().toString(36).slice(2, 8),
          catalogId: row.catalogId || "",
          name: String(row.name || ""),
          kind: String(row.kind || "gear"),
          qty: Math.max(1, Number(row.qty) || 1),
          detail: String(row.detail || ""),
          dmg: String(row.dmg || ""),
        }))
    : [];
  next.dead = Boolean(raw.dead);
  next.deathSuccess = Math.max(0, Math.min(3, Number(raw.deathSuccess) || 0));
  next.deathFail = Math.max(0, Math.min(3, Number(raw.deathFail) || 0));
  next.downed =
    !next.dead &&
    (Boolean(raw.downed) ||
      ((Number(next.hp) || 0) <= 0 && (Number(next.hpMax) || 0) > 0));
  next.programs = String(raw.programs || "");
  next.portrait = typeof raw.portrait === "string" ? raw.portrait : "";
  next.fullbody = typeof raw.fullbody === "string" ? raw.fullbody : "";
  next.creator = mergeCreator(raw.creator);
  next.classId = String(raw.classId || classByName(next.className)?.id || "");
  next.subclassId = String(raw.subclassId || "");
  next.ip = Number(raw.ip) || 0;
  return next;
}

const catalogCache = new Map();

async function loadCatalog(url) {
  if (catalogCache.has(url)) return catalogCache.get(url);
  const res = await fetch(url, { cache: "no-store" });
  if (!res.ok) throw new Error("catalog missing");
  const rows = await res.json();
  const map = new Map((Array.isArray(rows) ? rows : []).map((r) => [r.id, r]));
  catalogCache.set(url, map);
  return map;
}

async function lookupCreature(kind, ref, src) {
  const id = String(ref || "");
  if (!id) return null;
  if (kind === "beast") {
    const map = await loadCatalog("data/bestiary.json");
    return map.get(id) ? { world: "hearthsong", row: map.get(id) } : null;
  }
  if (kind === "shard") {
    const map = await loadCatalog("data/datashard.json");
    return map.get(id) ? { world: "blight", row: map.get(id) } : null;
  }
  if (kind === "srdnpc") {
    const srd = await loadCatalog("data/srd-npcs.json");
    return srd.get(id) ? { world: "hearthsong", row: srd.get(id) } : null;
  }
  if (kind === "npc") {
    const fromBestiary = /bestiary/i.test(String(src || ""));
    if (fromBestiary || worldOf() !== "blight") {
      const srd = await loadCatalog("data/srd-npcs.json");
      if (srd.get(id)) return { world: "hearthsong", row: srd.get(id) };
    }
    const faces = await loadCatalog("data/npcs.json");
    if (faces.get(id)) return { world: "blight", row: faces.get(id) };
    const srd = await loadCatalog("data/srd-npcs.json");
    if (srd.get(id)) return { world: "hearthsong", row: srd.get(id) };
  }
  return null;
}

function uniqueSheetName(list, name) {
  const base = String(name || "Creature").trim() || "Creature";
  const used = new Set((list || []).map((c) => String(c.name || "")));
  if (!used.has(base)) return base;
  let n = 2;
  while (used.has(`${base} ${n}`)) n += 1;
  return `${base} ${n}`;
}

function parseActionAttack(row) {
  const name = String(row?.n || row?.name || "Attack");
  const d = String(row?.d || row?.text || "");
  const bonus = (d.match(/([+-]\s*\d+)\s+to hit/i) || [])[1];
  const hit = d.match(/Hit:\s*\d+\s*\(([^)]+)\)(?:\s*([a-z]+))?/i) || d.match(/(\d+\s*d\s*\d+(?:\s*[+-]\s*\d+)?)/i);
  let damage = "";
  if (hit) {
    damage = String(hit[1] || "").replace(/\s+/g, " ").trim();
    if (hit[2]) damage = `${damage} ${hit[2]}`.trim();
  }
  if (!bonus && !damage) return null;
  return { name, bonus: bonus ? bonus.replace(/\s+/g, "") : "", damage };
}

function parseSkillProfs(text) {
  const out = {};
  const raw = String(text || "");
  for (const [id, name] of SKILLS) {
    if (new RegExp(`\\b${name.replace(/\s+/g, "\\s+")}\\b`, "i").test(raw)) out[id] = true;
  }
  return out;
}

function parseSaveProfs(text) {
  const out = {};
  const raw = String(text || "").toUpperCase();
  for (const [id, short] of ABILS) {
    if (raw.includes(short)) out[id] = true;
  }
  return out;
}

function levelFromCr(cr) {
  const s = String(cr ?? "");
  if (!s || s === "0") return 1;
  if (s.includes("/")) return 1;
  const n = Number(s);
  return Math.min(20, Math.max(1, Number.isFinite(n) ? Math.round(n) : 1));
}

function linesFrom(rows) {
  return (rows || [])
    .map((r) => {
      if (typeof r === "string") return r;
      if (Array.isArray(r)) return r.filter(Boolean).join(". ");
      const n = r?.n || r?.name || "";
      const d = r?.d || r?.text || "";
      return n && d ? `${n}. ${d}` : n || d;
    })
    .filter(Boolean)
    .join("\n");
}

function parseSpellTrait(text) {
  const cantrips = [];
  const spells = ["", "", "", "", "", "", "", "", ""];
  const slotsMax = [0, 0, 0, 0, 0, 0, 0, 0, 0];
  const raw = String(text || "");
  const c = raw.match(/Cantrips[^:\n]*:\s*([^\n]+)/i);
  if (c) cantrips.push(...c[1].split(/[,;]/).map((s) => s.trim()).filter(Boolean));
  for (let lv = 1; lv <= 9; lv++) {
    const re = new RegExp(String.raw`${lv}(?:st|nd|rd|th)\s+level\s*(?:\((\d+)\s*slots?\))?\s*:\s*([^\n]+)`, "i");
    const m = raw.match(re);
    if (!m) continue;
    if (m[1]) slotsMax[lv - 1] = Number(m[1]) || 0;
    spells[lv - 1] = m[2]
      .split(/[,;]/)
      .map((s) => s.trim())
      .filter(Boolean)
      .join("\n");
  }
  return { cantrips: cantrips.join("\n"), spells, slotsMax };
}

function sheetFrom5e(row, spec, list) {
  const ch = blankChar();
  ch.name = uniqueSheetName(list, row.name);
  ch.player = "NPC";
  ch.race = [row.size, row.type, row.sub].filter(Boolean).join(" ") || row.size || "";
  ch.className = row.role || row.aka || row.type || "NPC";
  ch.level = levelFromCr(row.crLabel || row.cr);
  ch.alignment = row.align || "";
  ch.xp = Number(row.xp) || 0;
  ch.ac = Number(row.ac) || 10;
  ch.hp = Number(row.hp) || 0;
  ch.hpMax = Number(row.hp) || 0;
  ch.speed = row.speed || "30 ft.";
  ch.hitDice = row.hd || ch.hitDice;
  for (const [id] of ABILS) {
    if (row[id] != null && row[id] !== "") ch.abilities[id] = Number(row[id]) || 10;
  }
  ch.saveProf = { ...ch.saveProf, ...parseSaveProfs(row.saves) };
  ch.skillProf = { ...ch.skillProf, ...parseSkillProfs(row.skills) };
  const leftover = [];
  const attacks = [];
  for (const a of row.actions || row.weapons || []) {
    const atk = parseActionAttack(a);
    if (atk) attacks.push(atk);
    else leftover.push(a);
  }
  ch.attacks = attacks.length ? attacks : ch.attacks;
  ch.languages = row.lang || "";
  ch.features = [linesFrom(row.traits), linesFrom(leftover), linesFrom(row.reactions), linesFrom(row.legendary)]
    .filter(Boolean)
    .join("\n\n");
  const spellTrait = (row.traits || []).find((t) => /spellcasting/i.test(t?.n || ""));
  if (spellTrait) {
    const parsed = parseSpellTrait(spellTrait.d);
    ch.cantrips = parsed.cantrips;
    ch.spells = parsed.spells;
    ch.slotsMax = parsed.slotsMax;
    ch.spellClass = ch.className;
    const wis = modifier(ch.abilities.wis);
    const intel = modifier(ch.abilities.int);
    const cha = modifier(ch.abilities.cha);
    ch.spellAbility = wis >= intel && wis >= cha ? "wis" : intel >= cha ? "int" : "cha";
  }
  ch.notes = [row.blurb, row.text, row.senses ? `Senses: ${row.senses}` : "", row.skills ? `Skills: ${row.skills}` : ""]
    .filter(Boolean)
    .join("\n\n");
  ch.catalogKind = spec.kind || "beast";
  ch.catalogId = row.id;
  return ch;
}

function parseRedSkillRanks(text, stats) {
  const ranks = {};
  const byName = Object.fromEntries(CP_SKILLS.map(([id, name, abil]) => [name.toLowerCase(), { id, abil }]));
  for (const part of String(text || "").split(/[·,]/)) {
    const m = part.trim().match(/^(.+?)\s+(\d+)\s*$/);
    if (!m) continue;
    const hit = byName[m[1].trim().toLowerCase()];
    if (!hit) continue;
    const total = Number(m[2]) || 0;
    const stat = Number(stats[hit.abil]) || 0;
    ranks[hit.id] = Math.max(0, Math.min(10, total - stat));
  }
  return ranks;
}

function parseRedAttack(row) {
  const name = String(row?.n || row?.name || "Weapon");
  const d = String(row?.d || "");
  const damage = (d.match(/(\d+\s*d\s*\d+(?:\s*[+-]\s*\d+)?)/i) || [])[1] || d.split("·")[0].trim();
  return { name, bonus: "", damage: damage.replace(/\s+/g, "") };
}

function sheetFromRed(row, spec, list) {
  const ch = blankChar();
  ch.name = uniqueSheetName(list, row.name);
  ch.handle = row.name || "";
  ch.player = "NPC";
  ch.role = row.role || "Solo";
  ch.roleRank = Number(row.rank) || 4;
  ch.hp = Number(row.hp) || 0;
  ch.hpMax = Number(row.hp) || 0;
  const sp = String(row.sp || "");
  const bodySp = sp.includes("/") ? Number(sp.split("/")[1]) : Number(sp);
  ch.ac = Number.isFinite(bodySp) ? bodySp : Number(row.ac) || 0;
  ch.humanity = Number(row.humanity) || 50;
  ch.speed = row.move != null ? `${row.move} m` : ch.speed;
  const stats = { ...ch.statsRed };
  for (const [id] of CP_STATS) {
    if (row[id] != null && row[id] !== "") stats[id] = Number(row[id]) || stats[id];
  }
  ch.statsRed = stats;
  ch.redSkills = parseRedSkillRanks(row.skills, stats);
  const attacks = (row.weapons || []).map(parseRedAttack).filter((a) => a.name);
  ch.attacks = attacks.length ? attacks : ch.attacks;
  ch.cyberware = row.chrome || "";
  ch.equipment = [row.gear, row.skills].filter(Boolean).join("\n");
  ch.features = linesFrom(row.traits);
  ch.lifepath = [row.blurb, row.text, row.look, row.district ? `District: ${row.district}` : "", row.affiliation ? `Affiliation: ${row.affiliation}` : ""]
    .filter(Boolean)
    .join("\n\n");
  ch.notes = ch.lifepath;
  ch.catalogKind = spec.kind || "npc";
  ch.catalogId = row.id;
  return ch;
}

function loadStore() {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) || "null");
    if (Array.isArray(raw)) return { list: raw.map(hydrate), activeId: raw[0]?.id || null };
    if (raw && Array.isArray(raw.list)) {
      const list = raw.list.map(hydrate);
      const activeId = list.some((c) => c.id === raw.activeId) ? raw.activeId : list[0]?.id || null;
      return { list, activeId };
    }
  } catch {
    /* ignore */
  }
  return { list: [], activeId: null };
}

export function resizeImage(file, maxEdge = 256, quality = 0.82) {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file);
    const img = new Image();
    img.onload = () => {
      let w = img.width;
      let h = img.height;
      const long = Math.max(w, h);
      if (long > maxEdge) {
        const s = maxEdge / long;
        w = Math.max(1, Math.round(w * s));
        h = Math.max(1, Math.round(h * s));
      }
      const canvas = document.createElement("canvas");
      canvas.width = w;
      canvas.height = h;
      const ctx = canvas.getContext("2d");
      ctx.drawImage(img, 0, 0, w, h);
      URL.revokeObjectURL(url);
      resolve(canvas.toDataURL("image/jpeg", quality));
    };
    img.onerror = () => {
      URL.revokeObjectURL(url);
      reject(new Error("Could not read that picture."));
    };
    img.src = url;
  });
}

function field(label, html) {
  return `<label class="char-field"><span>${label}</span>${html}</label>`;
}

function textInput(key, value, extra = "") {
  return `<input data-f="${key}" type="text" value="${escapeHtml(value)}" ${extra} />`;
}

function numInput(key, value, extra = "") {
  return `<input data-f="${key}" type="number" value="${escapeHtml(value)}" ${extra} />`;
}

function area(key, value, extra = "") {
  return `<textarea data-f="${key}" ${extra}>${escapeHtml(value)}</textarea>`;
}

function whoName(ch) {
  return ch?.handle || ch?.name || "Character";
}

function isHealLabel(s) {
  return /\b(heal|healing|cure wounds|cure|revivify|restore|soothe|lay on hands|prayer of healing|spare the dying|goodberry|healing word|mass heal|aura of vitality|aid)\b/i.test(
    String(s || "")
  );
}

function isRevivifyLabel(s) {
  return /\b(revivify|raise dead|resurrection|true resurrection|reincarnat)\b/i.test(String(s || ""));
}

function isHarmLabel(s) {
  return /\b(fireball|fire bolt|scorching|lightning|ice storm|ray of|magic missile|eldritch|inflict|blight|smite|disintegrate|cloudkill|acid|poison spray|chill touch|vampiric|finger of death|spirit guardians|spiritual weapon|guiding bolt|sacred flame|witch bolt|chaos bolt)\b/i.test(
    String(s || "")
  );
}

function isRangedName(s) {
  return /bow|crossbow|dart|sling|gun|pistol|rifle|shotgun|smg|thrown|javelin|firearm|sniper|arrow|bullet|launcher/i.test(
    String(s || "")
  );
}

function isFinesseName(s) {
  return /rapier|dagger|scimitar|shortsword|whip|finesse/i.test(String(s || ""));
}

function combatLevel(ch) {
  if (worldOf() === "blight") return Math.max(1, Math.min(10, Number(ch?.roleRank) || 4));
  return Math.max(1, Math.min(20, Number(ch?.level) || 1));
}

function extraCombatDice(ch) {
  return Math.floor((combatLevel(ch) - 1) / 4);
}

function abilityForAction(ch, label, opts = {}) {
  const blight = worldOf() === "blight";
  const name = String(label || "");
  if (blight) {
    const s = ch?.statsRed || {};
    if (opts.hack || /hack|quickhack|program|breach|ice/i.test(name)) {
      return { id: "int", score: Number(s.int) || 5, name: "INT" };
    }
    if (opts.heal || isHealLabel(name)) {
      return { id: "tech", score: Number(s.tech) || 5, name: "TECH" };
    }
    if (isRangedName(name)) {
      return { id: "ref", score: Number(s.ref) || 5, name: "REF" };
    }
    return { id: "body", score: Number(s.body) || 5, name: "BODY" };
  }
  if (opts.hack) {
    return { id: "int", score: Number(ch?.abilities?.int) || 10, name: "INT" };
  }
  if (opts.heal || opts.spell || isHealLabel(name) || isHarmLabel(name)) {
    const id = ch?.spellAbility || (isHealLabel(name) ? "wis" : "int");
    return { id, score: Number(ch?.abilities?.[id]) || 10, name: String(id).toUpperCase() };
  }
  const str = Number(ch?.abilities?.str) || 10;
  const dex = Number(ch?.abilities?.dex) || 10;
  if (isFinesseName(name)) {
    return modifier(dex) >= modifier(str)
      ? { id: "dex", score: dex, name: "DEX" }
      : { id: "str", score: str, name: "STR" };
  }
  if (isRangedName(name)) return { id: "dex", score: dex, name: "DEX" };
  return { id: "str", score: str, name: "STR" };
}

function abilityDamageBonus(abil) {
  const score = Number(abil?.score);
  if (worldOf() === "blight") return Math.max(0, (Number.isFinite(score) ? score : 5) - 4);
  return modifier(score);
}

function formatDice(n, sides, bonus) {
  const core = `${Math.max(1, Number(n) || 1)}d${Number(sides) || 6}`;
  const b = Number(bonus) || 0;
  if (!b) return core;
  return b > 0 ? `${core}+${b}` : `${core}${b}`;
}

function stripDiceText(raw) {
  return String(raw || "")
    .replace(/(\d+)\s*[dD]\s*(\d+)/g, "")
    .replace(/[+-]\s*\d+(?!d)/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

function scaleCombatFormula(formula, ch, label, opts = {}) {
  const blight = worldOf() === "blight";
  const raw = String(formula || "").trim() || (blight ? "2d6" : "1d8");
  const spec = parseDice(raw);
  const extra = extraCombatDice(ch);
  const abil = abilityForAction(ch, label, opts);
  const sides = spec.sides || (blight ? 6 : 8);
  const n = Math.max(1, (spec.n || 1) + extra);
  const bonus = (spec.bonus || 0) + abilityDamageBonus(abil);
  const type = stripDiceText(raw);
  const core = formatDice(n, sides, bonus);
  return type ? `${core} ${type}` : core;
}

function strDieSize(score, kick) {
  const s = Number(score) || 10;
  const steps = [4, 6, 8, 10, 12];
  let i = 0;
  if (s >= 13) i = 1;
  if (s >= 16) i = 2;
  if (s >= 19) i = 3;
  if (s >= 22) i = 4;
  if (kick) i = Math.min(steps.length - 1, i + 1);
  return steps[i];
}

function bodyDiceCount(score, extraKick) {
  const b = Number(score) || 5;
  let n = 1;
  if (b >= 5) n = 2;
  if (b >= 7) n = 3;
  if (b >= 9) n = 4;
  return Math.min(8, n + (extraKick ? 1 : 0));
}

function unarmedMoves(ch) {
  const blight = worldOf() === "blight";
  const extra = extraCombatDice(ch);
  const lv = combatLevel(ch);
  const scaleName = blight ? "BODY" : "Strength";
  const rung = blight ? `rank ${lv}` : `level ${lv}`;
  let punchFormula;
  let kickFormula;
  let bonus;
  let punchType = blight ? "" : " bludgeoning";
  let biteType = blight ? "" : " piercing";
  if (blight) {
    const body = Number(ch?.statsRed?.body) || 5;
    const dmgBonus = abilityDamageBonus({ score: body });
    punchFormula = formatDice(bodyDiceCount(body, 0) + extra, 6, dmgBonus);
    kickFormula = formatDice(bodyDiceCount(body, 1) + extra, 6, dmgBonus);
    bonus = signed(body + (Number(ch?.redSkills?.brawling) || 0));
  } else {
    const str = Number(ch?.abilities?.str) || 10;
    const n = 1 + extra;
    punchFormula = formatDice(n, strDieSize(str, false), modifier(str));
    kickFormula = formatDice(n, strDieSize(str, true), modifier(str));
    bonus = signed(modifier(str) + profForLevel(ch?.level));
  }
  const scaleLine = `Damage grows with ${scaleName} and ${rung}. No weapon.`;
  return [
    {
      id: "punch",
      name: "Punch",
      bonus,
      formula: punchFormula,
      damage: punchFormula + punchType,
      text: `Unarmed. A fist. ${scaleLine}`,
    },
    {
      id: "kick",
      name: "Kick",
      bonus,
      formula: kickFormula,
      damage: kickFormula + punchType,
      text: `Unarmed. A boot or a shin. Hits harder than a punch. ${scaleLine}`,
    },
    {
      id: "headbutt",
      name: "Headbutt",
      bonus,
      formula: punchFormula,
      damage: punchFormula + punchType,
      text: `Unarmed. Close. Same force as a punch. ${scaleLine}`,
    },
    {
      id: "bite",
      name: "Bite",
      bonus,
      formula: punchFormula,
      damage: punchFormula + biteType,
      text: `Unarmed. Teeth. ${blight ? "Same dice as a punch." : "Piercing."} ${scaleLine}`,
    },
  ];
}

function tokenBits(ch) {
  const name = whoName(ch);
  return `draggable="true" data-token-kind="char" data-token-ref="${escapeHtml(ch.originId || ch.id)}" data-token-sheet="${escapeHtml(ch.id)}" data-token-owner="${escapeHtml(ch.ownerId || "")}" data-token-name="${escapeHtml(name)}"`;
}

function rollBtn(ch, label, dmg, extra = "") {
  const heal = isHealLabel(label);
  const formula = dmg || (heal || isHarmLabel(label) ? (worldOf() === "blight" ? "2d6" : "1d8") : "");
  const d = formula ? ` data-dmg="${escapeHtml(formula)}"` : "";
  const h = heal ? ` data-heal="1"` : "";
  const sheet = /\bdata-sheet=/.test(extra)
    ? ""
    : ` data-sheet="${escapeHtml(ch.originId || ch.id)}" data-owner="${escapeHtml(ch.ownerId || "")}"`;
  return `<button type="button" class="roll-btn" data-roll="1" data-label="${escapeHtml(label)}" data-who="${escapeHtml(whoName(ch))}"${d}${h}${sheet}${extra}>Roll</button>`;
}

function combatRollBtn(ch, label, dmg, extra = "") {
  const heal = isHealLabel(label) || /\bdata-heal=/.test(extra);
  const hack = /\bdata-hack=/.test(extra);
  let base = dmg;
  if (!base && (heal || hack || isHarmLabel(label))) base = worldOf() === "blight" ? "2d6" : "1d8";
  if (!base) return rollBtn(ch, label, "", extra);
  const scaled = scaleCombatFormula(base, ch, label, { heal, hack });
  const bits = [
    extra,
    `data-combat="1"`,
    `data-base-dmg="${escapeHtml(base)}"`,
    `title="On a hit: ${escapeHtml(scaled)}"`,
  ]
    .filter(Boolean)
    .join(" ");
  return rollBtn(ch, label, scaled, ` ${bits}`);
}

function showBits(ch, title, text) {
  return `class="show-name" data-show="1" data-who="${escapeHtml(whoName(ch))}" data-show-title="${escapeHtml(title)}" data-show-text="${escapeHtml(text)}" title="Show this to the table"`;
}

export function createChars() {
  const store = loadStore();
  store.remote = new Map();
  let tab = "bio";
  let saveTimer = 0;
  let shareTimer = 0;
  let netShare = () => {};
  let netSelf = () => "";
  let mapTokens = () => [];
  let mapAim = () => {};
  let mapProject = () => {};
  let mapTouch = () => {};
  let mapRefresh = () => {};
  let aimCancel = null;
  let gm = false;
  const dice = createDice();

  const $ = (id) => document.getElementById(id);

  function isRemote(ch) {
    return Boolean(ch && (ch.remote || (ch.ownerId && ch.ownerId !== netSelf())));
  }

  function active() {
    const loc = store.list.find((c) => c.id === store.activeId);
    if (loc) return loc;
    for (const pack of store.remote.values()) {
      const hit = pack.list.find((c) => c.id === store.activeId);
      if (hit) return hit;
    }
    return null;
  }

  function persist() {
    const localId = store.list.some((c) => c.id === store.activeId) ? store.activeId : store.list[0]?.id || null;
    try {
      localStorage.setItem(KEY, JSON.stringify({ v: 1, activeId: localId, list: store.list }));
    } catch {
      try {
        const slim = {
          v: 1,
          activeId: localId,
          list: store.list.map((c) => ({
            ...c,
            portrait: c.id === localId ? c.portrait : "",
            fullbody: c.id === localId ? c.fullbody : "",
          })),
        };
        localStorage.setItem(KEY, JSON.stringify(slim));
      } catch {
        /* quota */
      }
    }
    shareSoon();
  }

  function saveSoon() {
    window.clearTimeout(saveTimer);
    saveTimer = window.setTimeout(persist, 180);
  }

  function shareSoon() {
    window.clearTimeout(shareTimer);
    shareTimer = window.setTimeout(shareNow, 320);
  }

  function shareNow() {
    netShare(store.list);
  }

  function setNet(hooks) {
    netShare = typeof hooks?.share === "function" ? hooks.share : () => {};
    netSelf = typeof hooks?.selfId === "function" ? hooks.selfId : () => "";
    mapTokens = typeof hooks?.mapTokens === "function" ? hooks.mapTokens : () => [];
    mapAim = typeof hooks?.mapAim === "function" ? hooks.mapAim : () => {};
    mapProject = typeof hooks?.mapProject === "function" ? hooks.mapProject : () => {};
    mapTouch = typeof hooks?.mapTouch === "function" ? hooks.mapTouch : () => {};
    mapRefresh = typeof hooks?.mapRefresh === "function" ? hooks.mapRefresh : () => {};
    dice.setShare(typeof hooks?.shareRoll === "function" ? hooks.shareRoll : null);
  }

  function seedCurrentHp(ch) {
    if (!ch || ch.dead || ch.downed || ch.catalogKind) return false;
    if ((Number(ch.deathSuccess) || 0) > 0 || (Number(ch.deathFail) || 0) > 0) return false;
    if (worldOf() === "blight") {
      const computed = redComputed(ch).hp;
      if (!computed) return false;
      const maxUnset = !Number(ch.hpMax);
      const hpUnset = ch.hp == null || ch.hp === "" || (maxUnset && (Number(ch.hp) || 0) === 0);
      if (!maxUnset && !hpUnset) return false;
      if (maxUnset) ch.hpMax = computed;
      if (hpUnset) ch.hp = Number(ch.hpMax) || computed;
      return true;
    }
    const max = Number(ch.hpMax) || 0;
    if (max && (ch.hp == null || ch.hp === "")) {
      ch.hp = max;
      return true;
    }
    return false;
  }

  function ensureHp(ch) {
    if (!ch) return;
    seedCurrentHp(ch);
    if (ch.hp == null || ch.hp === "") {
      const max = Number(ch.hpMax) || (worldOf() === "blight" ? redComputed(ch).hp : 0);
      ch.hp = max;
      if (!ch.hpMax && max) ch.hpMax = max;
    }
  }

  function hpMaxOf(ch) {
    const n = Number(ch?.hpMax);
    if (Number.isFinite(n) && n > 0) return n;
    if (worldOf() === "blight") return redComputed(ch).hp;
    return 0;
  }

  function isDead(ch) {
    return Boolean(ch?.dead);
  }

  function isDowned(ch) {
    return Boolean(ch) && !ch.dead && Boolean(ch.downed) && (Number(ch.hp) || 0) <= 0;
  }

  function clearDeathSaves(ch) {
    if (!ch) return;
    ch.deathSuccess = 0;
    ch.deathFail = 0;
    ch.downed = false;
    ch.dead = false;
  }

  function reviveAt(ch, hp) {
    const max = hpMaxOf(ch);
    const n = Math.max(1, Number(hp) || 1);
    ch.dead = false;
    ch.downed = false;
    ch.deathSuccess = 0;
    ch.deathFail = 0;
    ch.hp = max ? Math.min(max, n) : n;
    return ch.hp;
  }

  function markDead(ch) {
    ch.dead = true;
    ch.downed = false;
    ch.hp = 0;
    ch.hpTemp = 0;
    ch.deathFail = 3;
  }

  function applyDeathSave(ch, grade) {
    if (!ch) return null;
    if (isDead(ch)) return { kind: "corpse" };
    if (!isDowned(ch)) return { kind: "up" };
    if (grade === "critical success") {
      const hp = reviveAt(ch, 1);
      return { kind: "crit-revive", hp };
    }
    if (grade === "success") {
      ch.deathSuccess = Math.min(3, (Number(ch.deathSuccess) || 0) + 1);
      if (ch.deathSuccess >= 3) {
        const hp = reviveAt(ch, 1);
        return { kind: "stable-revive", hp };
      }
      return { kind: "ok", ok: ch.deathSuccess, fail: ch.deathFail || 0 };
    }
    if (grade === "critical failure") {
      ch.deathFail = Math.min(3, (Number(ch.deathFail) || 0) + 2);
    } else if (grade === "failure") {
      ch.deathFail = Math.min(3, (Number(ch.deathFail) || 0) + 1);
    } else {
      return { kind: "mixed", ok: ch.deathSuccess || 0, fail: ch.deathFail || 0 };
    }
    if (ch.deathFail >= 3) {
      markDead(ch);
      return { kind: "died" };
    }
    return { kind: "fail", ok: ch.deathSuccess || 0, fail: ch.deathFail };
  }

  function addDeathFails(ch, n) {
    if (!ch || isDead(ch) || !isDowned(ch)) return null;
    ch.deathFail = Math.min(3, (Number(ch.deathFail) || 0) + Math.max(1, n || 1));
    if (ch.deathFail >= 3) {
      markDead(ch);
      return { kind: "died" };
    }
    return { kind: "fail", ok: ch.deathSuccess || 0, fail: ch.deathFail };
  }

  function applySelfDamage(ch, n, grade) {
    ensureHp(ch);
    let left = Math.max(0, Number(n) || 0);
    if (!ch || !left) return null;
    if (isDead(ch)) return { kind: "corpse" };
    const wasDown = isDowned(ch);
    const temp = Number(ch.hpTemp) || 0;
    if (temp) {
      const use = Math.min(temp, left);
      ch.hpTemp = temp - use;
      left -= use;
    }
    if (!left) return { kind: "temp" };
    if (wasDown) {
      const fails = grade === "critical success" ? 2 : 1;
      return addDeathFails(ch, fails);
    }
    ch.hp = Math.max(0, (Number(ch.hp) || 0) - left);
    if ((Number(ch.hp) || 0) <= 0) {
      ch.hp = 0;
      if (hpMaxOf(ch) > 0) {
        ch.dead = false;
        ch.downed = true;
        ch.deathSuccess = 0;
        ch.deathFail = 0;
        return { kind: "downed" };
      }
      return { kind: "hit" };
    }
    return { kind: "hit" };
  }

  function applyHeal(ch, n, action) {
    ensureHp(ch);
    const add = Math.max(0, Number(n) || 0);
    if (!ch || !add) return null;
    if (isDead(ch)) {
      if (!isRevivifyLabel(action)) return { kind: "corpse" };
      const hp = reviveAt(ch, add);
      return { kind: "revived", hp };
    }
    const wasDown = isDowned(ch) || ((Number(ch.hp) || 0) <= 0 && hpMaxOf(ch) > 0);
    const max = hpMaxOf(ch);
    if (wasDown) {
      const hp = reviveAt(ch, add);
      return { kind: "revived", hp };
    }
    const cur = Number(ch.hp) || 0;
    ch.hp = max ? Math.min(max, cur + add) : cur + add;
    return { kind: "heal", hp: ch.hp };
  }

  function announceDeath(ch, out, extra = {}) {
    if (!ch || !out) return;
    const name = whoName(ch);
    if (out.kind === "downed") {
      dice.note(name, "Downed", `${name} is at 0 HP. Roll a death save on Combat. Three successes: stand with 1 HP. Three failures: dead. A heal brings them back with that many HP.`);
      flash("Downed");
    } else if (out.kind === "died") {
      dice.note(name, "Dead", extra.deathSave ? `${name} failed the third death save.` : `${name} took a hit while downed and failed the third death save.`);
      flash("Dead");
    } else if (out.kind === "crit-revive") {
      dice.note(name, "Stirs", `${name} critically succeeds a death save and stands with 1 HP.`);
      flash("1 HP");
    } else if (out.kind === "stable-revive") {
      dice.note(name, "Stirs", `${name} made the third death save and stands with 1 HP.`);
      flash("1 HP");
    } else if (out.kind === "revived") {
      dice.note(name, "Stands", `${name} is healed and stands with ${out.hp} HP.`);
      flash("+" + out.hp + " HP");
    } else if (out.kind === "fail" && !extra.deathSave) {
      dice.note(name, "Death save", `${name} takes a death-save failure (${out.fail}/3) from a hit while downed.`);
    } else if (out.kind === "corpse" && extra.heal) {
      dice.note(name, "Dead", `${name} already failed three death saves. A heal cannot raise them. Revivify and similar still can.`);
    }
  }

  function bareId(id) {
    const s = String(id || "");
    return s.includes("::") ? s.slice(s.lastIndexOf("::") + 2) : s;
  }

  function findChar(id, owner) {
    if (!id) return null;
    const want = bareId(id);
    for (const ch of allChars()) {
      const ids = [ch.id, ch.originId, bareId(ch.id)].filter(Boolean);
      if (!ids.includes(id) && !ids.includes(want)) continue;
      if (owner && ch.ownerId && ch.ownerId !== owner && !String(ch.id).startsWith(owner + "::")) continue;
      return ch;
    }
    return null;
  }

  function findCharForToken(tok) {
    if (!tok) return null;
    return findChar(tok.sheetId || tok.ref || "", tok.ownerId || "");
  }

  function isMyChar(ch) {
    return Boolean(ch && !isRemote(ch));
  }

  function sheetTokens() {
    return (mapTokens() || []).filter((t) => t.sheetId || (t.kind === "char" && t.ref));
  }

  function cancelAim() {
    const help = $("dice-help");
    if (help && !help.hidden) {
      help.hidden = true;
      return true;
    }
    if (!aimCancel) return false;
    aimCancel();
    return true;
  }

  function pickTarget() {
    if (aimCancel) aimCancel();
    const toks = sheetTokens();
    if (!toks.length) {
      flash("Drop a character onto the map, then roll.");
      return Promise.resolve(null);
    }
    return new Promise((resolve) => {
      const stage = $("aim-stage");
      const list = $("aim-list");
      let settled = false;
      const finish = (tok) => {
        if (settled) return;
        settled = true;
        aimCancel = null;
        mapAim(null);
        if (stage) stage.hidden = true;
        if (!tok) {
          resolve(false);
          return;
        }
        const ch = findCharForToken(tok);
        resolve({
          targetId: bareId(ch?.originId || ch?.id || tok.sheetId || tok.ref || ""),
          targetName: ch ? whoName(ch) : tok.name || "Target",
          targetOwner: ch?.ownerId || tok.ownerId || "",
          tokenId: tok.id || "",
        });
      };
      aimCancel = () => finish(null);
      if (list) {
        list.innerHTML = toks
          .map((t) => {
            const ch = findCharForToken(t);
            const label = ch ? whoName(ch) : t.name || "Token";
            const sub = ch && isRemote(ch) ? ch.ownerName || "other player" : "on the map";
            return `<button type="button" class="aim-pick" data-aim-token="${escapeHtml(t.id)}"><b>${escapeHtml(label)}</b><small>${escapeHtml(sub)}</small></button>`;
          })
          .join("");
        list.onclick = (e) => {
          const btn = e.target.closest("[data-aim-token]");
          if (!btn) return;
          finish(toks.find((t) => t.id === btn.dataset.aimToken) || null);
        };
      }
      $("aim-cancel") && ($("aim-cancel").onclick = () => finish(null));
      if (stage) stage.hidden = false;
      mapAim((tok) => finish(tok));
    });
  }

  function applyTargetHp(row, { announce = false } = {}) {
    if (row?.deathSave) {
      const ch = findChar(row.targetId, row.targetOwner);
      if (!ch) return;
      const out = applyDeathSave(ch, row.grade);
      if (isMyChar(ch)) persist();
      render();
      if (announce) announceDeath(ch, out, { deathSave: true });
      mapRefresh();
      return;
    }
    const hit = row.grade === "success" || row.grade === "critical success";
    if (!hit || !row.damage || !row.targetId) return;
    const ch = findChar(row.targetId, row.targetOwner);
    if (!ch) return;
    const out = row.heal ? applyHeal(ch, row.damage, row.action) : applySelfDamage(ch, row.damage, row.grade);
    if (isMyChar(ch)) persist();
    render();
    if (announce) announceDeath(ch, out, { heal: Boolean(row.heal) });
    mapRefresh();
  }

  function beamFor(row) {
    if (!row?.tokenId) return;
    const mine = active();
    const from = sheetTokens().find((t) => {
      if (!mine) return false;
      const sid = t.sheetId || t.ref;
      return sid === mine.id || sid === mine.originId;
    });
    const kind = row.heal ? "heal" : row.hack ? "hack" : "hit";
    mapProject(from?.id || "", row.tokenId, kind);
  }

  function applyTaken(row, { announce = false } = {}) {
    if (!row?.taken || !row.damage) return;
    const ch = findChar(row.sheetId, row.sheetOwner) || (!row.sheetId ? active() : null);
    if (!ch) return;
    const out = applySelfDamage(ch, row.damage, row.grade);
    if (isMyChar(ch)) persist();
    render();
    if (announce) announceDeath(ch, out);
    mapRefresh();
  }

  dice.setOnRecord((row) => {
    applyTargetHp(row, { announce: true });
    beamFor(row);
    applyTaken(row, { announce: true });
  });

  dice.setOnAim(() => pickTarget());

  function ingestRoll(row) {
    if (!row) return;
    if (row.from && row.from === netSelf()) return;
    dice.ingest(row);
    applyTargetHp(row, { announce: false });
    applyTaken(row, { announce: false });
    beamFor(row);
    const hit = row.grade === "success" || row.grade === "critical success";
    if (row.hack && hit) {
      const ch = findChar(row.targetId, row.targetOwner);
      if (isMyChar(ch)) dice.playHack(row.action, { victim: true });
    }
  }

  function ingestRolls(rows) {
    if (!Array.isArray(rows) || !rows.length) return;
    const self = netSelf();
    dice.ingestMany(rows.filter((row) => !row?.from || row.from !== self));
  }

  function applyRemote(from, name, list, silent) {
    const self = netSelf();
    if (!from || from === self) return;
    const rows = Array.isArray(list) ? list : [];
    if (!rows.length) {
      store.remote.delete(from);
      if (store.activeId && String(store.activeId).startsWith(from + "::")) {
        store.activeId = store.list[0]?.id || null;
      }
      if (!silent) render();
      return;
    }
    store.remote.set(from, {
      name: name || "Traveller",
      list: rows.slice(0, 16).map((raw) => {
        const ch = hydrate(raw);
        ch.remote = true;
        ch.ownerId = from;
        ch.ownerName = name || "Traveller";
        ch.originId = ch.id;
        ch.id = from + "::" + ch.id;
        return ch;
      }),
    });
    if (store.activeId && store.activeId.startsWith(from + "::") && !store.remote.get(from)?.list.some((c) => c.id === store.activeId)) {
      store.activeId = store.list[0]?.id || null;
    }
    if (!silent) render();
  }

  function applyRemoteTable(packs) {
    const keep = new Set();
    for (const pack of packs || []) {
      if (!pack?.from) continue;
      keep.add(pack.from);
      applyRemote(pack.from, pack.name, pack.list, true);
    }
    for (const id of [...store.remote.keys()]) {
      if (!keep.has(id) && id !== netSelf()) store.remote.delete(id);
    }
    render();
  }

  function clearRemote() {
    store.remote.clear();
    if (store.activeId && String(store.activeId).includes("::")) {
      store.activeId = store.list[0]?.id || null;
    }
    render();
  }

  function prof(ch) {
    return profForLevel(ch.level);
  }

  function skillBonus(ch, skillId, abil) {
    const mod = modifier(ch.abilities[abil]);
    const p = prof(ch);
    let n = mod;
    if (ch.skillProf[skillId]) n += p;
    if (ch.skillExpert[skillId]) n += p;
    return n;
  }

  function spellMod(ch) {
    const abil = ch.spellAbility;
    if (!abil || !ch.abilities[abil]) return null;
    return modifier(ch.abilities[abil]);
  }

  function renderUnarmed(ch) {
    const blight = worldOf() === "blight";
    const rows = unarmedMoves(ch)
      .map(
        (m) => `<div class="attack-row unarmed-row">
          <span><b ${showBits(ch, m.name, m.text)}>${escapeHtml(m.name)}</b></span>
          <span data-computed="unarmed-bonus-${m.id}">${escapeHtml(m.bonus)}</span>
          <span data-computed="unarmed-dmg-${m.id}">${escapeHtml(m.damage)}</span>
          <button type="button" class="ghost show-btn" data-show="1" data-who="${escapeHtml(whoName(ch))}" data-show-title="${escapeHtml(m.name)}" data-show-text="${escapeHtml(m.text)}">Show</button>
          ${rollBtn(ch, m.name + " " + m.bonus, m.formula, ` data-unarmed-roll="${m.id}" data-scaled="1" title="On a hit: ${escapeHtml(m.damage)}"`)}
        </div>`
      )
      .join("");
    return `
      <div class="char-section-label">Unarmed</div>
      <p class="char-hint">${
        blight
          ? "Always on this sheet. No weapon. Dice grow with BODY and role rank. The to-hit bonus is BODY plus Brawling."
          : "Always on this sheet. No weapon. Dice grow with Strength and level. The to-hit bonus is Strength plus proficiency."
      }</p>
      <div class="attack-head"><span>Name</span><span>Bonus</span><span>Damage</span></div>
      <div id="unarmed-list">${rows}</div>`;
  }

  function paintUnarmed(ch) {
    if (!document.getElementById("unarmed-list") || !ch) return;
    for (const m of unarmedMoves(ch)) {
      for (const node of document.querySelectorAll(`[data-computed="unarmed-bonus-${m.id}"]`)) node.textContent = m.bonus;
      for (const node of document.querySelectorAll(`[data-computed="unarmed-dmg-${m.id}"]`)) node.textContent = m.damage;
      const btn = document.querySelector(`[data-unarmed-roll="${m.id}"]`);
      if (btn) {
        btn.dataset.dmg = m.formula;
        btn.dataset.label = m.name + " " + m.bonus;
        btn.title = `On a hit: ${m.damage}`;
      }
    }
  }

  function paintCombatScale(ch) {
    if (!ch) return;
    paintUnarmed(ch);
    const blight = worldOf() === "blight";
    const fallback = blight ? "2d6" : "1d8";
    (ch.attacks || []).forEach((row, i) => {
      const label = [row.name || (blight ? "Weapon" : "Attack"), row.bonus].filter(Boolean).join(" ");
      const scaled = scaleCombatFormula(row.damage || fallback, ch, label);
      for (const node of document.querySelectorAll(`[data-atk-scale="${i}"]`)) node.textContent = "hit " + scaled;
      const btn = document.querySelector(`[data-atk-roll="${i}"]`);
      if (btn) {
        btn.dataset.dmg = scaled;
        btn.dataset.baseDmg = row.damage || fallback;
        btn.dataset.label = label;
        btn.title = `On a hit: ${scaled}`;
      }
    });
    for (const btn of document.querySelectorAll("[data-combat][data-base-dmg]")) {
      if (btn.hasAttribute("data-atk-roll")) continue;
      const scaled = scaleCombatFormula(btn.dataset.baseDmg, ch, btn.dataset.label, {
        heal: btn.dataset.heal,
        hack: btn.dataset.hack,
      });
      btn.dataset.dmg = scaled;
      btn.title = `On a hit: ${scaled}`;
    }
  }

  function computed(ch) {
    const p = prof(ch);
    const dex = modifier(ch.abilities.dex);
    const init = ch.initiative === "" || ch.initiative == null ? dex : Number(ch.initiative) || 0;
    const sm = spellMod(ch);
    return {
      prof: p,
      init,
      passivePerception: 10 + skillBonus(ch, "perception", "wis"),
      passiveInsight: 10 + skillBonus(ch, "insight", "wis"),
      passiveInvestigation: 10 + skillBonus(ch, "investigation", "int"),
      spellMod: sm,
      spellDc: sm == null ? "—" : 8 + p + sm,
      spellAtk: sm == null ? "—" : signed(p + sm),
    };
  }

  function redComputed(ch) {
    const s = ch.statsRed || {};
    const body = Number(s.body) || 5;
    const will = Number(s.will) || 5;
    const emp = Number(s.emp) || 5;
    const ref = Number(s.ref) || 5;
    const hp = 10 + 5 * Math.floor((body + will) / 2);
    return {
      hp,
      seriously: Math.ceil(hp / 2),
      death: body,
      humanity: emp * 10,
      init: ref,
    };
  }

  let advance = null;

  function clsOf(ch) {
    return classById(ch.classId) || classByName(ch.className);
  }

  function subOf(ch, cls) {
    const c = cls || clsOf(ch);
    return subclassById(c, ch.subclassId) || subclassById(c, ch.subclass);
  }

  function canAdvance5e(ch) {
    if (!ch || isRemote(ch)) return false;
    const lv = Number(ch.level) || 1;
    if (lv >= 20) return false;
    const need = xpToNext(lv);
    return gm || (need != null && (Number(ch.xp) || 0) >= need);
  }

  function canAdvanceRed(ch) {
    if (!ch || isRemote(ch)) return false;
    const rank = Number(ch.roleRank) || 1;
    if (rank >= 10) return (Number(ch.ip) || 0) > 0;
    return gm || (Number(ch.ip) || 0) >= roleIpCost(rank + 1);
  }

  function applyChosenClass(ch, classId, opts = {}) {
    const cls = classById(classId);
    if (!cls) {
      ch.classId = "";
      if (classId === "gamemaster") ch.className = "Gamemaster";
      return;
    }
    applyClassBasics(ch, cls);
    const lv = Math.max(1, Number(ch.level) || 1);
    if (!ch.hpMax || opts.fresh) {
      const hp = startingHp(cls.hd, modifier(ch.abilities.con));
      ch.hpMax = hp;
      ch.hp = hp;
    }
    ch.hitDice = `${lv}d${cls.hd}`;
    const slots = slotsFor(cls, lv);
    if (slots) ch.slotsMax = slots;
    for (let n = 1; n <= lv; n++) grantLevelFeatures(ch, cls, n, subOf(ch, cls));
  }

  function xpBarHtml(ch) {
    const lv = Math.max(1, Number(ch.level) || 1);
    const have = Number(ch.xp) || 0;
    const cur = xpForLevel(lv);
    const next = xpToNext(lv);
    if (!next) return `<div class="char-xp"><span>Level 20 · ${have.toLocaleString()} XP</span></div>`;
    const span = Math.max(1, next - cur);
    const pct = Math.max(0, Math.min(100, Math.round(((have - cur) / span) * 100)));
    const ready = have >= next;
    return `<div class="char-xp">
      <div class="char-xp-track" title="${have.toLocaleString()} / ${next.toLocaleString()} XP"><i style="width:${pct}%"></i></div>
      <span>Lv ${lv} · ${have.toLocaleString()} / ${next.toLocaleString()} XP</span>
      <label class="char-xp-add">Award <input data-xp-award type="number" min="0" step="50" value="300" /></label>
      <button type="button" class="ghost" data-xp-go>Add XP</button>
      <button type="button" class="ghost${ready ? "" : ""}" data-advance-open ${ready || gm ? "" : "disabled"}>Level up</button>
    </div>`;
  }

  function ipBarHtml(ch) {
    const rank = Number(ch.roleRank) || 1;
    const ip = Number(ch.ip) || 0;
    const cost = rank >= 10 ? null : roleIpCost(rank + 1);
    const ready = cost != null && ip >= cost;
    return `<div class="char-xp">
      <span>Rank ${rank} · ${ip} IP${cost ? ` · next rank ${cost} IP` : ""}</span>
      <label class="char-xp-add">Award <input data-ip-award type="number" min="0" step="10" value="50" /></label>
      <button type="button" class="ghost" data-ip-go>Add IP</button>
      <button type="button" class="ghost" data-advance-open>Advance</button>
    </div>`;
  }

  function closeAdvance() {
    advance = null;
    const el = $("char-advance");
    if (el) {
      el.hidden = true;
      el.innerHTML = "";
    }
  }

  function openAdvance() {
    const ch = active();
    if (!ch || isRemote(ch)) return;
    if (worldOf() === "blight") {
      advance = { world: "blight", kind: "role", target: "", feat: "" };
    } else {
      const cls = clsOf(ch);
      const nextLv = (Number(ch.level) || 1) + 1;
      advance = {
        world: "hearthsong",
        hp: cls ? avgHpGain(cls.hd, modifier(ch.abilities.con)) : 1,
        hpRoll: null,
        asi: {},
        feat: "",
        subId: ch.subclassId || "",
        nextLv,
      };
    }
    paintAdvance();
  }

  function paintAdvance() {
    const el = $("char-advance");
    const ch = active();
    const btn = $("char-advance-btn");
    if (btn) {
      const blight = worldOf() === "blight";
      btn.hidden = !ch || isRemote(ch);
      btn.textContent = blight ? "Advance" : "Level up";
      btn.disabled = Boolean(!ch || isRemote(ch) || (!blight && !canAdvance5e(ch)));
    }
    if (!el) return;
    if (!advance || !ch || isRemote(ch)) {
      el.hidden = true;
      el.innerHTML = "";
      return;
    }
    el.hidden = false;
    if (advance.world === "blight") {
      el.innerHTML = redAdvanceHtml(ch);
      return;
    }
    el.innerHTML = fiveAdvanceHtml(ch);
  }

  function fiveAdvanceHtml(ch) {
    const lv = Number(ch.level) || 1;
    const next = lv + 1;
    const cls = clsOf(ch);
    const need = xpToNext(lv);
    const have = Number(ch.xp) || 0;
    const blocked = !gm && need != null && have < need;
    if (lv >= 20) {
      return `<div class="char-advance-card"><h3>Peak</h3><p>This sheet is already 20th level.</p><button type="button" class="ghost" data-advance-close>Close</button></div>`;
    }
    if (blocked) {
      return `<div class="char-advance-card"><h3>Not yet</h3><p>${have.toLocaleString()} XP. Need ${need.toLocaleString()} to reach level ${next}.</p><button type="button" class="ghost" data-advance-close>Close</button></div>`;
    }
    const bits = cls ? cls.features.filter((x) => x.lv === next).map((x) => x.text) : [];
    const sub = subOf(ch, cls);
    const subBits = sub ? sub.features.filter((x) => x.lv === next).map((x) => x.text) : [];
    const unlockSub = cls && cls.subclassLv === next;
    const asi = cls && isAsiLevel(cls, next);
    const con = modifier(ch.abilities.con);
    const avg = cls ? avgHpGain(cls.hd, con) : 1;
    const subOpts = (cls?.subclasses || [])
      .map((s) => `<option value="${escapeHtml(s.id)}"${(advance.subId || ch.subclassId) === s.id ? " selected" : ""}>${escapeHtml(s.name)}</option>`)
      .join("");
    const asiBtns = ABILS.map(([id, short]) => {
      const n = Number(advance.asi[id]) || 0;
      const score = Number(ch.abilities[id]) || 10;
      return `<button type="button" class="ghost${n ? " on" : ""}" data-asi="${id}" ${score + n >= 20 ? "disabled" : ""}>${short} ${n ? "+" + n : "+"}</button>`;
    }).join("");
    const spent = Object.values(advance.asi).reduce((a, b) => a + (Number(b) || 0), 0);
    return `<div class="char-advance-card">
      <h3>Level ${lv} → ${next}${cls ? " · " + escapeHtml(cls.name) : ""}</h3>
      <p class="char-hint">Faithful 5e: proficiency, hit points, slots, and class features. You pick HP and any ASI.</p>
      ${bits.length ? `<p><b>You gain.</b> ${escapeHtml(bits.join(" · "))}</p>` : ""}
      ${subBits.length ? `<p><b>${escapeHtml(sub.name)}.</b> ${escapeHtml(subBits.join(" · "))}</p>` : ""}
      ${unlockSub ? `<label class="char-field"><span>Subclass</span><select data-adv-sub><option value="">Choose…</option>${subOpts}</select></label>` : ""}
      ${cls ? `<div class="char-section-label">Hit points</div>
        <div class="char-maker-row">
          <button type="button" class="ghost${advance.hpRoll == null ? " on" : ""}" data-hp-avg>Average +${avg}</button>
          <button type="button" class="ghost${advance.hpRoll != null ? " on" : ""}" data-hp-roll>Roll d${cls.hd}${advance.hpRoll != null ? ` → ${advance.hp}` : ""}</button>
        </div>` : `<p class="char-hint">Pick a class on Bio first for automatic hit dice and slots.</p>`}
      ${asi ? `<div class="char-section-label">Ability score improvement (${spent}/2)</div>
        <div class="char-asi">${asiBtns}</div>
        <label class="char-field"><span>Or a feat instead</span><input data-adv-feat type="text" maxlength="80" placeholder="Leave blank to take the ASI" value="${escapeHtml(advance.feat)}" /></label>` : ""}
      <div class="char-advance-actions">
        <button type="button" class="ghost" data-advance-close>Cancel</button>
        <button type="button" class="primary" data-advance-go>Take level ${next}</button>
      </div>
    </div>`;
  }

  function redAdvanceHtml(ch) {
    const rank = Number(ch.roleRank) || 1;
    const ip = Number(ch.ip) || 0;
    const role = ch.role || "Solo";
    const nextRole = rank >= 10 ? null : rank + 1;
    const roleCost = nextRole ? roleIpCost(nextRole) : null;
    const skills = CP_SKILLS.map(([id, name, abil]) => {
      const rankNow = Number(ch.redSkills?.[id]) || 0;
      if (rankNow >= 10) return "";
      const cost = skillIpCost(rankNow + 1);
      const on = advance.kind === "skill" && advance.target === id ? " on" : "";
      return `<button type="button" class="ghost${on}" data-red-buy="skill" data-red-id="${id}" ${ip < cost && !gm ? "disabled" : ""}>${name} ${rankNow}→${rankNow + 1} · ${cost} IP</button>`;
    }).join("");
    const stats = CP_STATS.map(([id, short]) => {
      const now = Number(ch.statsRed?.[id]) || 5;
      if (now >= 8) return "";
      const cost = statIpCost(now + 1);
      const on = advance.kind === "stat" && advance.target === id ? " on" : "";
      return `<button type="button" class="ghost${on}" data-red-buy="stat" data-red-id="${id}" ${ip < cost && !gm ? "disabled" : ""}>${short} ${now}→${now + 1} · ${cost} IP</button>`;
    }).join("");
    return `<div class="char-advance-card">
      <h3>Advance · ${escapeHtml(role)} rank ${rank}</h3>
      <p class="char-hint">Cyberpunk RED Improvement Points. Role rank × 20. Skill rank × 10. STAT × 25. Spend what you have.</p>
      <p><b>${ip} IP</b> on this sheet.</p>
      <div class="char-section-label">Role</div>
      ${nextRole ? `<button type="button" class="ghost${advance.kind === "role" ? " on" : ""}" data-red-buy="role" data-red-id="role" ${ip < roleCost && !gm ? "disabled" : ""}>Rank ${rank} → ${nextRole} · ${roleCost} IP</button>` : `<p>Role rank is already 10.</p>`}
      <div class="char-section-label">Skills</div>
      <div class="char-asi">${skills}</div>
      <div class="char-section-label">STATs (cap 8)</div>
      <div class="char-asi">${stats}</div>
      <div class="char-advance-actions">
        <button type="button" class="ghost" data-advance-close>Cancel</button>
        <button type="button" class="primary" data-advance-go ${advance.kind && advance.target ? "" : "disabled"}>Spend</button>
      </div>
    </div>`;
  }

  function commitFiveAdvance(ch) {
    const lv = Number(ch.level) || 1;
    if (lv >= 20) return false;
    const need = xpToNext(lv);
    if (!gm && need != null && (Number(ch.xp) || 0) < need) return false;
    const next = lv + 1;
    const cls = clsOf(ch);
    if (advance.subId) {
      const sub = subclassById(cls, advance.subId);
      if (sub) {
        ch.subclassId = sub.id;
        ch.subclass = sub.name;
      }
    }
    const sub = subOf(ch, cls);
    ch.level = next;
    if (cls) {
      ch.hitDice = `${next}d${cls.hd}`;
      const slots = slotsFor(cls, next);
      if (slots) {
        ch.slotsMax = slots;
        ch.slotsUsed = (ch.slotsUsed || []).map((n, i) => Math.min(Number(n) || 0, slots[i] || 0));
      }
      grantLevelFeatures(ch, cls, next, sub);
    }
    const gain = Math.max(1, Number(advance.hp) || 1);
    ch.hpMax = (Number(ch.hpMax) || 0) + gain;
    ch.hp = (Number(ch.hp) || 0) + gain;
    if (cls && isAsiLevel(cls, next) && !String(advance.feat || "").trim()) {
      for (const [id] of ABILS) {
        const n = Number(advance.asi[id]) || 0;
        if (n) ch.abilities[id] = Math.min(20, (Number(ch.abilities[id]) || 10) + n);
      }
    } else if (String(advance.feat || "").trim()) {
      ch.feats = [ch.feats, `Lv ${next}: ${advance.feat.trim()}`].filter(Boolean).join("\n");
    }
    return true;
  }

  function commitRedAdvance(ch) {
    const kind = advance.kind;
    const target = advance.target;
    const ip = Number(ch.ip) || 0;
    if (kind === "role") {
      const rank = Number(ch.roleRank) || 1;
      if (rank >= 10) return false;
      const cost = roleIpCost(rank + 1);
      if (!gm && ip < cost) return false;
      ch.ip = Math.max(0, ip - cost);
      ch.roleRank = rank + 1;
      const line = roleBlurb(ch.role, ch.roleRank);
      const kept = String(ch.features || "")
        .split("\n")
        .filter((l) => !/^\[Role /.test(l.trim()));
      ch.features = [...kept, line].filter(Boolean).join("\n");
      return true;
    }
    if (kind === "skill" && target) {
      ch.redSkills = ch.redSkills || {};
      const now = Number(ch.redSkills[target]) || 0;
      if (now >= 10) return false;
      const cost = skillIpCost(now + 1);
      if (!gm && ip < cost) return false;
      ch.ip = Math.max(0, ip - cost);
      ch.redSkills[target] = now + 1;
      return true;
    }
    if (kind === "stat" && target) {
      ch.statsRed = ch.statsRed || {};
      const now = Number(ch.statsRed[target]) || 5;
      if (now >= 8) return false;
      const cost = statIpCost(now + 1);
      if (!gm && ip < cost) return false;
      ch.ip = Math.max(0, ip - cost);
      ch.statsRed[target] = now + 1;
      return true;
    }
    return false;
  }

  function allChars() {
    const rows = [...store.list];
    for (const pack of store.remote.values()) rows.push(...pack.list);
    return rows;
  }

  function paintPortraits() {
    for (const ch of allChars()) {
      const src = ch.portrait || "";
      for (const img of document.querySelectorAll("[data-portrait]")) {
        if (img.getAttribute("data-portrait") !== ch.id) continue;
        if (src) {
          img.src = src;
          img.hidden = false;
        } else {
          img.removeAttribute("src");
          img.hidden = true;
        }
      }
      const body = ch.fullbody || "";
      for (const img of document.querySelectorAll("[data-fullbody]")) {
        if (img.getAttribute("data-fullbody") !== ch.id) continue;
        if (body) {
          img.src = body;
          img.hidden = false;
        } else {
          img.removeAttribute("src");
          img.hidden = true;
        }
      }
    }
  }

  function chipHtml(ch) {
    const on = ch.id === store.activeId ? " on" : "";
    const remote = isRemote(ch) ? " remote" : "";
    const vitals = isDead(ch) ? "Dead" : isDowned(ch) ? "Downed" : "";
    const sub = worldOf() === "blight"
      ? [ch.handle || ch.role, ch.roleRank ? `Rank ${ch.roleRank}` : "", vitals].filter(Boolean).join(" · ")
      : [ch.className, ch.level ? `Lv ${ch.level}` : "", vitals].filter(Boolean).join(" · ");
    const name = ch.name || "Unnamed";
    const state = isDead(ch) ? " dead" : isDowned(ch) ? " downed" : "";
    return `<button type="button" class="char-chip${on}${remote}${state}" data-open-char="${escapeHtml(ch.id)}">
      <span class="char-face" ${tokenBits(ch)}>${ch.portrait ? "" : name.slice(0, 1)}</span>
      <img data-portrait="${escapeHtml(ch.id)}" alt="" hidden ${tokenBits(ch)} />
      <span class="char-chip-copy"><b>${escapeHtml(name)}</b><small>${escapeHtml(sub)}</small></span>
    </button>`;
  }

  function renderRoster() {
    const el = $("char-roster");
    if (!el) return;
    const remoteCount = [...store.remote.values()].reduce((n, p) => n + p.list.length, 0);
    if (!store.list.length && !remoteCount) {
      el.innerHTML = `<p class="char-empty">Press New for a sheet on this machine. Host or Join and everyone at the table can open everyone else's sheets.</p>`;
      return;
    }
    let html = "";
    if (store.list.length) {
      if (remoteCount || gm) html += `<div class="char-roster-label">${gm ? "Gamemaster · this machine" : "This machine"}</div>`;
      html += store.list.map(chipHtml).join("");
    }
    for (const pack of store.remote.values()) {
      if (!pack.list.length) continue;
      html += `<div class="char-roster-label">From ${escapeHtml(pack.name)}</div>`;
      html += pack.list.map(chipHtml).join("");
    }
    el.innerHTML = html;
    paintPortraits();
  }

  function renderTabs() {
    const el = $("char-tabs");
    const del = $("char-del");
    if (!el) return;
    const ch = active();
    el.hidden = !ch;
    if (del) del.hidden = !ch || isRemote(ch);
    if (!ch) {
      el.innerHTML = "";
      return;
    }
    const tabs = worldOf() === "blight" ? TABS_RED : TABS;
    if (!tabs.some(([id]) => id === tab)) tab = "bio";
    el.innerHTML = tabs.map(
      ([id, label]) =>
        `<button type="button" class="char-tab${tab === id ? " on" : ""}" data-char-tab="${id}">${label}</button>`
    ).join("");
  }

  function renderBio(ch) {
    const blight = worldOf() === "blight";
    if (blight) {
      const roleOpts = CP_ROLES.map((r) => `<option value="${escapeHtml(r)}"${ch.role === r ? " selected" : ""}>${escapeHtml(r)}</option>`).join("");
      return `
      <div class="char-bio">
        <div class="char-pics">
          <button type="button" class="char-portrait" id="char-portrait-btn" title="Add or change a face picture">
            <img data-portrait="${escapeHtml(ch.id)}" alt="" hidden ${tokenBits(ch)} />
            <span>Face</span>
          </button>
          <button type="button" class="char-fullbody" id="char-fullbody-btn" title="Add or change a full-body picture">
            <img data-fullbody="${escapeHtml(ch.id)}" alt="" hidden />
            <span>Full body</span>
          </button>
        </div>
        <div class="char-bio-fields">
          ${field("Handle", textInput("handle", ch.handle || ch.name, 'maxlength="48"'))}
          ${field("Name", textInput("name", ch.name, 'maxlength="48"'))}
          ${field("Role", `<select data-f="role">${roleOpts}</select>`)}
          ${field("Role rank", numInput("roleRank", ch.roleRank || 4, 'min="1" max="10"'))}
          ${field("Improvement points", numInput("ip", ch.ip || 0, 'min="0"'))}
          ${field("Player", textInput("player", ch.player, 'maxlength="32"'))}
        </div>
      </div>
      ${gm && !isRemote(ch) ? `<p class="char-gm">Gamemaster — you host this table. Guests follow your mix and your map.</p>` : ""}
      ${isRemote(ch) ? "" : ipBarHtml(ch)}`;
    }
    const opts = ALIGNMENTS.map(
      (a) => `<option value="${escapeHtml(a)}"${ch.alignment === a ? " selected" : ""}>${escapeHtml(a)}</option>`
    ).join("");
    const cls = clsOf(ch);
    const classOpts = [`<option value="">Custom</option>`, `<option value="gamemaster"${ch.classId === "gamemaster" || ch.className === "Gamemaster" ? " selected" : ""}>Gamemaster</option>`]
      .concat(CLASSES.map((c) => `<option value="${escapeHtml(c.id)}"${(ch.classId || cls?.id) === c.id ? " selected" : ""}>${escapeHtml(c.name)}</option>`))
      .join("");
    const subOpts = (cls?.subclasses || [])
      .map((s) => `<option value="${escapeHtml(s.id)}"${(ch.subclassId || "") === s.id ? " selected" : ""}>${escapeHtml(s.name)}</option>`)
      .join("");
    return `
      <div class="char-bio">
        <div class="char-pics">
          <button type="button" class="char-portrait" id="char-portrait-btn" title="Add or change a face picture">
            <img data-portrait="${escapeHtml(ch.id)}" alt="" hidden ${tokenBits(ch)} />
            <span>Face</span>
          </button>
          <button type="button" class="char-fullbody" id="char-fullbody-btn" title="Add or change a full-body picture">
            <img data-fullbody="${escapeHtml(ch.id)}" alt="" hidden />
            <span>Full body</span>
          </button>
        </div>
        <div class="char-bio-fields">
          ${field("Name", textInput("name", ch.name, 'maxlength="48"'))}
          ${field("Player", textInput("player", ch.player, 'maxlength="32"'))}
          ${field("Class", `<select data-f="classId">${classOpts}</select>`)}
          ${cls ? field("Subclass", `<select data-f="subclassId"><option value="">Choose at level ${cls.subclassLv}…</option>${subOpts}</select>`) : field("Class name", textInput("className", ch.className, 'placeholder="Custom class"'))}
          ${field("Level", numInput("level", ch.level, 'min="1" max="20"'))}
          ${field("Race", textInput("race", ch.race))}
          ${field("Background", textInput("background", ch.background))}
          ${field("Deity", textInput("deity", ch.deity, 'placeholder="Drag from Gods, or write a name"'))}
          ${field("Alignment", `<select data-f="alignment"><option value=""></option>${opts}</select>`)}
          ${field("Experience", numInput("xp", ch.xp, 'min="0"'))}
        </div>
      </div>
      ${gm && !isRemote(ch) ? `<p class="char-gm">Gamemaster — you host this table. Guests follow your mix and your map.</p>` : ""}
      <div class="char-grid char-grid-3">
        ${field("Age", textInput("age", ch.age))}
        ${field("Height", textInput("height", ch.height))}
        ${field("Weight", textInput("weight", ch.weight))}
        ${field("Eyes", textInput("eyes", ch.eyes))}
        ${field("Skin", textInput("skin", ch.skin))}
        ${field("Hair", textInput("hair", ch.hair))}
      </div>
      ${isRemote(ch) ? "" : xpBarHtml(ch)}`;
  }

  function renderStats(ch) {
    const c = computed(ch);
    const abils = ABILS.map(([id, short, name]) => {
      const mod = signed(modifier(ch.abilities[id]));
      const save = signed(modifier(ch.abilities[id]) + (ch.saveProf[id] ? c.prof : 0));
      return `<div class="abil-card">
        <b title="${name}" ${showBits(ch, name, `${name} (${short}). Score ${ch.abilities[id]}. Modifier ${mod}. Save ${save}.`)}>${short}</b>
        <input data-abil="${id}" type="number" min="1" max="30" value="${escapeHtml(ch.abilities[id])}" />
        <div class="abil-mod" data-computed="mod-${id}">${mod}</div>
        <label class="abil-save"><input data-save="${id}" type="checkbox"${ch.saveProf[id] ? " checked" : ""} /> Save <i data-computed="save-${id}">${save}</i></label>
        ${rollBtn(ch, short + " check")}
        ${rollBtn(ch, short + " save")}
      </div>`;
    }).join("");
    const skills = SKILLS.map(([id, name, abil]) => {
      const bonus = signed(skillBonus(ch, id, abil));
      const tag = ch.skillExpert[id] ? "expertise" : ch.skillProf[id] ? "proficient" : "untrained";
      return `<div class="skill-row">
        <input data-skill="${id}" type="checkbox" title="Proficient"${ch.skillExpert[id] || ch.skillProf[id] ? " checked" : ""} />
        <input data-expert="${id}" type="checkbox" title="Expertise"${ch.skillExpert[id] ? " checked" : ""} />
        <span ${showBits(ch, name, `${name} (${abil.toUpperCase()}). ${tag}. Modifier ${bonus}.`)}>${name} <em>${abil.toUpperCase()}</em></span>
        <b data-computed="skill-${id}">${bonus}</b>
        ${rollBtn(ch, name)}
      </div>`;
    }).join("");
    return `
      <div class="char-meta-row">
        <div class="stat-pill">Proficiency <b data-computed="prof">${signed(c.prof)}</b></div>
        <label class="stat-pill"><input data-f="inspiration" type="checkbox"${ch.inspiration ? " checked" : ""} /> Inspiration</label>
        <div class="stat-pill">Passive Per <b data-computed="pp">${c.passivePerception}</b></div>
        <div class="stat-pill">Passive Ins <b data-computed="pi">${c.passiveInsight}</b></div>
        <div class="stat-pill">Passive Inv <b data-computed="pv">${c.passiveInvestigation}</b></div>
      </div>
      <div class="abil-grid">${abils}</div>
      <p class="char-hint">Left box is proficient. Right box is expertise.</p>
      <div class="skill-list">${skills}</div>`;
  }

  function attackRowHtml(ch, row, i, blight) {
    const fallback = blight ? "2d6" : "1d8";
    const noun = blight ? "Weapon" : "Attack";
    const label = [row.name || noun, row.bonus].filter(Boolean).join(" ");
    const scaled = scaleCombatFormula(row.damage || fallback, ch, label);
    return `<div class="attack-row">
          <input data-attack="${i}" data-ak="name" type="text" placeholder="${noun}" value="${escapeHtml(row.name)}" />
          <input data-attack="${i}" data-ak="bonus" type="text" placeholder="${blight ? "WA" : "+"}" value="${escapeHtml(row.bonus)}" />
          <span class="dmg-cell">
            <input data-attack="${i}" data-ak="damage" type="text" placeholder="${blight ? "2d6" : "1d8 slashing"}" value="${escapeHtml(row.damage)}" />
            <em class="hit-scale" data-atk-scale="${i}">hit ${escapeHtml(scaled)}</em>
          </span>
          <button type="button" class="ghost show-btn" data-show="1" data-who="${escapeHtml(whoName(ch))}" data-show-title="${escapeHtml(row.name || noun)}">Show</button>
          ${combatRollBtn(ch, label, row.damage || fallback, ` data-atk-roll="${i}"`)}
          <button type="button" class="ghost" data-del-attack="${i}" title="Remove">×</button>
        </div>`;
  }

  function renderDeathSaves(ch) {
    const dead = isDead(ch);
    const down = isDowned(ch);
    const death = (kind, n) =>
      [1, 2, 3]
        .map(
          (i) =>
            `<button type="button" class="death-pip ${kind}${n >= i ? " on" : ""}" data-death="${kind}" data-n="${i}" aria-label="${kind} ${i}"></button>`
        )
        .join("");
    const status = dead ? "Dead" : down ? "Downed" : "Standing";
    const hint = dead
      ? "Three failed death saves. A heal cannot raise them. Revivify and similar still can."
      : down
        ? "At 0 HP. Roll a death save. Three successes: stand with 1 HP. Three failures: dead. A heal brings them back with that many HP."
        : "Death saves start when this sheet hits 0 HP. Same rules in Hearthsong and Blight.";
    const roll = down
      ? rollBtn(
          ch,
          "Death save",
          "",
          ` data-death-save="1" data-sheet="${escapeHtml(ch.originId || ch.id)}" data-owner="${escapeHtml(ch.ownerId || "")}"`
        )
      : "";
    return `
      <div class="death-box${dead ? " is-dead" : down ? " is-down" : ""}">
        <div class="death-row">
          <span>Death saves</span>
          <b class="death-status">${status}</b>
          <div><small>Success</small> ${death("success", ch.deathSuccess || 0)}</div>
          <div><small>Fail</small> ${death("fail", ch.deathFail || 0)}</div>
          ${roll}
        </div>
        <p class="char-hint">${hint}</p>
      </div>`;
  }

  function renderCombat(ch) {
    const c = computed(ch);
    const attacks = (ch.attacks || []).map((row, i) => attackRowHtml(ch, row, i, false)).join("");
    return `
      <div class="char-grid char-grid-3">
        ${field("Armor class", numInput("ac", ch.ac, 'min="0"'))}
        ${field("Initiative", `<input data-f="initiative" type="text" placeholder="${signed(c.init)}" value="${escapeHtml(ch.initiative)}" />`)}
        ${field("Speed", textInput("speed", ch.speed))}
        ${field("Hit points", numInput("hp", ch.hp, 'min="0"'))}
        ${field("Hit point max", numInput("hpMax", ch.hpMax, 'min="0"'))}
        ${field("Temporary HP", numInput("hpTemp", ch.hpTemp, 'min="0"'))}
        ${field("Hit dice", textInput("hitDice", ch.hitDice, 'placeholder="1d8"'))}
        ${field("Exhaustion", numInput("exhaustion", ch.exhaustion, 'min="0" max="6"'))}
      </div>
      ${renderDeathSaves(ch)}
      ${renderUnarmed(ch)}
      <div class="char-section-label">Attacks</div>
      <p class="char-hint">On a hit, extra dice from level and the ability for that action (Strength, Dexterity, or spellcasting) are added to the written damage.</p>
      <div class="attack-head"><span>Name</span><span>Bonus</span><span>Damage / type</span></div>
      <div id="attack-list">${attacks}</div>
      <button type="button" class="ghost" id="char-add-attack">Add attack</button>`;
  }

  function renderMagic(ch) {
    const c = computed(ch);
    const abilOpts = ["", "int", "wis", "cha", "str", "dex", "con"]
      .map((id) => {
        const label = id ? id.toUpperCase() : "None";
        return `<option value="${id}"${ch.spellAbility === id ? " selected" : ""}>${label}</option>`;
      })
      .join("");
    const slots = Array.from({ length: 9 }, (_, i) => {
      return `<div class="slot-row">
        <span>Lv ${i + 1}</span>
        <label>Max <input data-slot-max="${i}" type="number" min="0" max="20" value="${ch.slotsMax[i] || 0}" /></label>
        <label>Used <input data-slot-used="${i}" type="number" min="0" max="20" value="${ch.slotsUsed[i] || 0}" /></label>
      </div>`;
    }).join("");
    const spells = Array.from({ length: 9 }, (_, i) =>
      field(
        `Level ${i + 1} spells`,
        `<textarea data-spell="${i}" rows="2" placeholder="One per line">${escapeHtml(ch.spells[i])}</textarea>`
      )
    ).join("");
    return `
      <div class="char-grid char-grid-2">
        ${field("Spellcasting class", textInput("spellClass", ch.spellClass))}
        ${field("Spellcasting ability", `<select data-f="spellAbility">${abilOpts}</select>`)}
      </div>
      <div class="char-meta-row">
        <div class="stat-pill">Save DC <b data-computed="sdc">${c.spellDc}</b></div>
        <div class="stat-pill">Attack <b data-computed="satk">${c.spellAtk}</b></div>
      </div>
      <div class="char-section-label">Spell slots</div>
      <div class="slot-grid">${slots}</div>
      ${field("Cantrips", area("cantrips", ch.cantrips, 'rows="3" placeholder="One per line"'))}
      <div class="skill-list">${String(ch.cantrips || "")
        .split("\n")
        .map((line) => line.trim())
        .filter(Boolean)
        .map((line) => `<div class="skill-row"><span ${showBits(ch, line, `Cantrip. ${line}`)}>${escapeHtml(line)}</span>${combatRollBtn(ch, line)}</div>`)
        .join("")}</div>
      ${spells}
      <div class="skill-list">${(ch.spells || [])
        .flatMap((block, i) =>
          String(block || "")
            .split("\n")
            .map((line) => line.trim())
            .filter(Boolean)
            .map((line) => `<div class="skill-row"><span ${showBits(ch, line, `Level ${i + 1} spell. ${line}`)}>Lv ${i + 1} ${escapeHtml(line)}</span>${combatRollBtn(ch, line)}</div>`)
        )
        .join("")}</div>`;
  }

  function renderFeatures(ch) {
    const featRolls = String(ch.features || "")
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean)
      .map((line) => `<div class="skill-row"><span ${showBits(ch, line.split(".")[0] || "Feature", line)}>${escapeHtml(line)}</span>${combatRollBtn(ch, line)}</div>`)
      .join("");
    return `
      ${field("Features & traits", area("features", ch.features, 'rows="8" placeholder="Class features, racial traits, abilities…"'))}
      <div class="skill-list">${featRolls}</div>
      ${field("Feats", area("feats", ch.feats, 'rows="3"'))}
      ${field("Proficiencies", area("proficiencies", ch.proficiencies, 'rows="3" placeholder="Armor, weapons, tools"'))}
      ${field("Languages", area("languages", ch.languages, 'rows="2"'))}`;
  }

  function renderKitList(ch, empty) {
    const rows = (ch.kit || [])
      .map(
        (k, i) => `<div class="kit-owned">
          ${kitThumb({ id: k.catalogId || k.id, name: k.name, kind: k.kind, dmg: k.dmg, world: worldOf() })}
          <span><b ${showBits(ch, k.name, [k.name, k.kind, k.dmg && `damage ${k.dmg}`, k.detail].filter(Boolean).join(". "))}>${escapeHtml(k.name)}</b><small>${escapeHtml(k.kind)}${k.qty > 1 ? ` ×${k.qty}` : ""}${k.dmg ? ` · ${escapeHtml(k.dmg)}` : ""}</small></span>
          ${k.kind === "quickhack" ? combatRollBtn(ch, k.name, "", ` data-hack="1"`) : k.kind === "spell" ? combatRollBtn(ch, k.name) : (k.dmg || k.kind === "weapon") ? combatRollBtn(ch, k.name, k.dmg) : ""}
          <button type="button" class="ghost" data-kit-del="${i}" title="Remove">×</button>
        </div>`
      )
      .join("");
    return `
      <div class="char-section-label">On this sheet</div>
      <div class="kit-owned-list">${rows || `<p class="char-hint">${empty}</p>`}</div>`;
  }

  function renderGear(ch) {
    return `
      ${renderKitList(ch, "Open Armory and drag weapons, armor, gear, magic, or spells onto this sheet.")}
      <div class="coin-row">
        ${field("CP", numInput("cp", ch.cp, 'min="0"'))}
        ${field("SP", numInput("sp", ch.sp, 'min="0"'))}
        ${field("EP", numInput("ep", ch.ep, 'min="0"'))}
        ${field("GP", numInput("gp", ch.gp, 'min="0"'))}
        ${field("PP", numInput("pp", ch.pp, 'min="0"'))}
      </div>
      ${field("Notes", area("equipment", ch.equipment, 'rows="5" placeholder="Anything that is not in the list…"'))}
      ${field("Treasure", area("treasure", ch.treasure, 'rows="3"'))}`;
  }

  function renderRedGear(ch) {
    return `
      ${renderKitList(ch, "Open Night Market and drag weapons, armor, gear, chrome, or quickhacks onto this sheet.")}
      ${field("Notes", area("equipment", ch.equipment, 'rows="5" placeholder="Anything that is not in the list…"'))}
      ${field("Programs / quickhacks", area("programs", ch.programs, 'rows="4" placeholder="One per line. Drag from Night Market to add."'))}
      <div class="skill-list">${String(ch.programs || "")
        .split("\n")
        .map((line) => line.trim())
        .filter(Boolean)
        .map((line) => `<div class="skill-row"><span ${showBits(ch, line, line)}>${escapeHtml(line)}</span>${combatRollBtn(ch, line, "", ` data-hack="1"`)}</div>`)
        .join("")}</div>`;
  }

  function renderRedStats(ch) {
    const s = ch.statsRed || {};
    const c = redComputed(ch);
    const cards = CP_STATS.map(([id, short]) => `
      <div class="abil-card">
        <b ${showBits(ch, short, `${short}. Score ${s[id] ?? 5}.`)}>${short}</b>
        <input data-red-stat="${id}" type="number" min="1" max="10" value="${escapeHtml(s[id] ?? 5)}" />
        ${rollBtn(ch, short + " check")}
      </div>`).join("");
    return `
      <div class="char-meta-row">
        <div class="stat-pill">HP <b data-computed="rhp">${Number(ch.hp) || 0}/${c.hp}</b></div>
        <div class="stat-pill">Seriously <b data-computed="rsw">${c.seriously}</b></div>
        <div class="stat-pill" title="Cyberpunk RED death save target number (BODY). Combat uses the three-pip death saves.">Death TN <b data-computed="rds">${c.death}</b></div>
        <div class="stat-pill">Humanity <b data-computed="rhum">${c.humanity}</b></div>
        <div class="stat-pill">Init <b data-computed="rinit">${c.init}</b></div>
      </div>
      <div class="abil-grid">${cards}</div>
      <p class="char-hint">Cyberpunk RED stats. HP and humanity update from BODY, WILL, and EMP.</p>`;
  }

  function renderRedCombat(ch) {
    const c = redComputed(ch);
    const attacks = (ch.attacks || []).map((row, i) => attackRowHtml(ch, row, i, true)).join("");
    return `
      <div class="char-grid char-grid-3">
        ${field("Hit points", numInput("hp", ch.hp == null || ch.hp === "" ? c.hp : ch.hp, 'min="0"'))}
        ${field("Hit point max", numInput("hpMax", ch.hpMax == null || ch.hpMax === "" ? c.hp : ch.hpMax, 'min="0"'))}
        ${field("Armor SP", numInput("ac", ch.ac, 'min="0"'))}
        ${field("Move", textInput("speed", ch.speed || (ch.statsRed?.move || 5) + " m"))}
      </div>
      ${renderDeathSaves(ch)}
      ${renderUnarmed(ch)}
      <div class="char-section-label">Weapons</div>
      <p class="char-hint">On a hit, extra dice from role rank and the STAT for that action (BODY, REF, or INT) are added to the written damage.</p>
      <div class="attack-head"><span>Name</span><span>WA</span><span>Damage</span></div>
      <div id="attack-list">${attacks}</div>
      <button type="button" class="ghost" id="char-add-attack">Add weapon</button>`;
  }

  function renderRedSkills(ch) {
    const s = ch.statsRed || {};
    const rows = CP_SKILLS.map(([id, name, abil]) => {
      const rank = Number(ch.redSkills?.[id]) || 0;
      const tot = rank + (Number(s[abil]) || 0);
      return `<div class="skill-row" style="grid-template-columns: minmax(0,1fr) 3.2rem 2.2rem auto">
        <span ${showBits(ch, name, `${name} (${abil.toUpperCase()}). Rank ${rank}. Total ${tot}.`)}>${name} <em>${abil.toUpperCase()}</em></span>
        <input data-red-skill="${id}" type="number" min="0" max="10" value="${rank}" />
        <b data-computed="rsk-${id}">${tot}</b>
        ${rollBtn(ch, name)}
      </div>`;
    }).join("");
    return `<p class="char-hint">Rank plus STAT. Total is what you roll.</p><div class="skill-list">${rows}</div>`;
  }

  function renderRedChrome(ch) {
    const cr = ch.chrome || {};
    const boxes = AUGS.map(([id, name]) => `
      <div class="skill-row"><input data-chrome="${id}" type="checkbox"${cr[id] ? " checked" : ""} /> <span ${showBits(ch, name, `${name}. ${cr[id] ? "Installed." : "Not installed."}`)}>${name}</span></div>
    `).join("");
    return `
      ${field("Humanity", numInput("humanity", ch.humanity, 'min="0"'))}
      <div class="char-section-label">Installed</div>
      ${boxes}
      ${field("Other cyberware", area("cyberware", ch.cyberware, 'rows="6" placeholder="Name, humanity cost, notes…"'))}`;
  }

  function renderRedLife(ch) {
    return `
      ${field("Lifepath", area("lifepath", ch.lifepath, 'rows="6" placeholder="Cultural origin, personality, clothing, what you want…"'))}
      ${field("Allies & enemies", area("allies", ch.allies, "rows=\"3\""))}
      ${field("Notes", area("notes", ch.notes, "rows=\"4\""))}`;
  }

  function renderStory(ch) {
    return `
      ${field("Personality traits", area("personality", ch.personality, "rows=\"3\""))}
      ${field("Ideals", area("ideals", ch.ideals, "rows=\"2\""))}
      ${field("Bonds", area("bonds", ch.bonds, "rows=\"2\""))}
      ${field("Flaws", area("flaws", ch.flaws, "rows=\"2\""))}
      ${field("Allies & organizations", area("allies", ch.allies, "rows=\"3\""))}
      ${field("Backstory", area("backstory", ch.backstory, "rows=\"6\""))}
      ${field("Notes", area("notes", ch.notes, "rows=\"4\""))}`;
  }

  function renderBody() {
    const el = $("char-body");
    if (!el) return;
    const ch = active();
    if (!ch) {
      el.innerHTML = `<p class="char-empty">No sheet open. Press New, then add a name and a picture.</p>`;
      return;
    }
    const blight = worldOf() === "blight";
    const views = blight
      ? {
          bio: renderBio,
          stats: renderRedStats,
          combat: renderRedCombat,
          skills: renderRedSkills,
          chrome: renderRedChrome,
          gear: renderRedGear,
          life: renderRedLife,
        }
      : {
          bio: renderBio,
          stats: renderStats,
          combat: renderCombat,
          magic: renderMagic,
          features: renderFeatures,
          gear: renderGear,
          story: renderStory,
        };
    if (!isRemote(ch) && seedCurrentHp(ch)) saveSoon();
    const banner = isDead(ch)
      ? `<div class="char-down-banner dead">Dead — three failed death saves</div>`
      : isDowned(ch)
        ? `<div class="char-down-banner">Downed — roll a death save on Combat. Three successes: 1 HP. A heal stands them up with that many HP.</div>`
        : "";
    el.innerHTML = banner + (views[tab] || renderBio)(ch);
    paintPortraits();
    lockRemote(ch);
  }

  function lockRemote(ch) {
    const panel = $("chars-panel");
    const remote = isRemote(ch);
    panel?.classList.toggle("char-remote", remote);
    const el = $("char-body");
    if (!el || !remote) return;
    for (const n of el.querySelectorAll("input, textarea, select")) n.disabled = true;
    for (const n of el.querySelectorAll("#char-add-attack, [data-del-attack], [data-kit-del], #char-portrait-btn, #char-fullbody-btn")) {
      n.disabled = true;
    }
  }

  function updateComputed() {
    const ch = active();
    if (!ch) return;
    const c = computed(ch);
    const set = (key, text) => {
      for (const node of document.querySelectorAll(`[data-computed="${key}"]`)) node.textContent = text;
    };
    set("prof", signed(c.prof));
    set("pp", String(c.passivePerception));
    set("pi", String(c.passiveInsight));
    set("pv", String(c.passiveInvestigation));
    set("sdc", String(c.spellDc));
    set("satk", String(c.spellAtk));
    for (const [id] of ABILS) {
      const mod = modifier(ch.abilities[id]);
      set("mod-" + id, signed(mod));
      set("save-" + id, signed(mod + (ch.saveProf[id] ? c.prof : 0)));
    }
    for (const [id, , abil] of SKILLS) set("skill-" + id, signed(skillBonus(ch, id, abil)));
    const init = document.querySelector('#char-body input[data-f="initiative"]');
    if (init && !init.value) init.placeholder = signed(c.init);
    const rc = redComputed(ch);
    set("rhp", `${Number(ch.hp) || 0}/${rc.hp}`);
    set("rsw", String(rc.seriously));
    set("rds", String(rc.death));
    set("rhum", String(rc.humanity));
    set("rinit", String(rc.init));
    const s = ch.statsRed || {};
    for (const [id, , abil] of CP_SKILLS) {
      const rank = Number(ch.redSkills?.[id]) || 0;
      set("rsk-" + id, String(rank + (Number(s[abil]) || 0)));
    }
    paintCombatScale(ch);
  }

  function render() {
    renderRoster();
    renderTabs();
    renderBody();
    paintAdvance();
  }

  function select(id) {
    store.activeId = id;
    persist();
    render();
  }

  function addChar() {
    const ch = blankChar();
    ch.player = profileNameSafe();
    if (gm) {
      if (worldOf() === "blight") ch.role = "Gamemaster";
      else if (!ch.className) ch.className = "Gamemaster";
    }
    seedCurrentHp(ch);
    store.list.push(ch);
    store.activeId = ch.id;
    tab = "bio";
    persist();
    render();
    $("char-body")?.querySelector("input[data-f='name']")?.focus();
  }

  function profileNameSafe() {
    try {
      return (localStorage.getItem("hearthsong.profileName") || "").trim().slice(0, 24);
    } catch {
      return "";
    }
  }

  function setGM(on) {
    const next = Boolean(on);
    if (gm === next) return;
    gm = next;
    render();
  }

  function removeChar() {
    const ch = active();
    if (!ch || isRemote(ch)) return;
    const label = ch.name || "this character";
    if (!window.confirm(`Delete ${label}? This machine will forget the sheet.`)) return;
    store.list = store.list.filter((c) => c.id !== ch.id);
    store.activeId = store.list[0]?.id || null;
    persist();
    render();
  }

  function writeField(ch, target) {
    const { f } = target.dataset;
    if (!f) return false;
    if (target.type === "checkbox") ch[f] = target.checked;
    else if (target.type === "number") ch[f] = target.value === "" ? 0 : Number(target.value);
    else ch[f] = target.value;
    if (f === "level") {
      const n = Number(ch.level) || 1;
      ch.level = Math.min(20, Math.max(1, n));
    }
    if (f === "classId") {
      applyChosenClass(ch, ch.classId, { fresh: !(Number(ch.hpMax) || 0) });
    }
    if (f === "subclassId") {
      const cls = clsOf(ch);
      const sub = subclassById(cls, ch.subclassId);
      ch.subclass = sub ? sub.name : "";
      if (sub) {
        const lv = Number(ch.level) || 1;
        for (let n = 1; n <= lv; n++) grantLevelFeatures(ch, cls, n, sub);
      }
    }
    if (f === "role" && worldOf() === "blight") {
      const line = roleBlurb(ch.role, ch.roleRank || 4);
      const kept = String(ch.features || "")
        .split("\n")
        .filter((l) => !/^\[Role /.test(l.trim()));
      ch.features = [...kept, line].filter(Boolean).join("\n");
    }
    if (f === "hp") {
      const n = Number(ch.hp) || 0;
      if (n > 0) clearDeathSaves(ch);
      else {
        ch.hp = 0;
        if ((Number(ch.hpMax) || 0) > 0 && !ch.dead) ch.downed = true;
      }
    }
    return true;
  }

  function onPanelInput(e) {
    const ch = active();
    if (!ch || isRemote(ch)) return;
    const t = e.target;
    let dirtyRoster = false;
    if (writeField(ch, t)) {
      dirtyRoster = ["name", "className", "classId", "subclassId", "level", "handle", "role", "roleRank", "ip", "xp"].includes(t.dataset.f);
    } else if (t.dataset.abil) {
      ch.abilities[t.dataset.abil] = t.value === "" ? 0 : Number(t.value);
    } else if (t.dataset.save) {
      ch.saveProf[t.dataset.save] = t.checked;
    } else if (t.dataset.skill) {
      ch.skillProf[t.dataset.skill] = t.checked;
      if (!t.checked) {
        ch.skillExpert[t.dataset.skill] = false;
        const box = t.closest(".skill-row")?.querySelector("[data-expert]");
        if (box) box.checked = false;
      }
    } else if (t.dataset.expert) {
      ch.skillExpert[t.dataset.expert] = t.checked;
      if (t.checked) {
        ch.skillProf[t.dataset.expert] = true;
        const box = t.closest(".skill-row")?.querySelector("[data-skill]");
        if (box) box.checked = true;
      }
    } else if (t.dataset.slotMax != null) {
      ch.slotsMax[Number(t.dataset.slotMax)] = Number(t.value) || 0;
    } else if (t.dataset.slotUsed != null) {
      ch.slotsUsed[Number(t.dataset.slotUsed)] = Number(t.value) || 0;
    } else if (t.dataset.spell != null) {
      ch.spells[Number(t.dataset.spell)] = t.value;
    } else if (t.dataset.attack != null) {
      const row = ch.attacks[Number(t.dataset.attack)];
      if (row && t.dataset.ak) row[t.dataset.ak] = t.value;
    } else if (t.dataset.redStat) {
      ch.statsRed = ch.statsRed || {};
      ch.statsRed[t.dataset.redStat] = t.value === "" ? 0 : Number(t.value);
    } else if (t.dataset.redSkill) {
      ch.redSkills = ch.redSkills || {};
      ch.redSkills[t.dataset.redSkill] = Number(t.value) || 0;
    } else if (t.dataset.chrome) {
      ch.chrome = ch.chrome || {};
      ch.chrome[t.dataset.chrome] = t.checked;
    } else {
      return;
    }
    saveSoon();
    updateComputed();
    if (dirtyRoster) renderRoster();
    if (t.dataset.f === "hp") {
      renderRoster();
      renderBody();
    }
    if (
      e.type === "change" &&
      (t.dataset.f === "programs" ||
        t.dataset.f === "cantrips" ||
        t.dataset.f === "classId" ||
        t.dataset.f === "subclassId" ||
        t.dataset.f === "role" ||
        t.dataset.spell != null)
    ) {
      render();
    }
  }

  async function onPortrait(file) {
    if (!file) return;
    if (!active()) addChar();
    const ch = active();
    if (!ch || isRemote(ch)) return;
    try {
      ch.portrait = await resizeImage(file, 256, 0.82);
      persist();
      renderRoster();
      paintPortraits();
    } catch {
      /* ignore bad image */
    }
  }

  async function onFullbody(file) {
    const ch = active();
    if (!ch || isRemote(ch) || !file) return;
    try {
      ch.fullbody = await resizeImage(file, 720, 0.78);
      persist();
      paintPortraits();
    } catch {
      /* ignore bad image */
    }
  }

  function bind() {
    const panel = $("chars-panel");
    if (!panel || panel.dataset.bound) return;
    panel.dataset.bound = "1";
    bindKitArt();
    panel.addEventListener("input", onPanelInput);
    panel.addEventListener("change", onPanelInput);
    panel.addEventListener("dragover", (e) => {
      if (![...(e.dataTransfer?.types || [])].includes("Files")) return;
      e.preventDefault();
      e.dataTransfer.dropEffect = "copy";
    });
    panel.addEventListener("drop", (e) => {
      const file = [...(e.dataTransfer.files || [])].find((f) => /^image\//.test(f.type || ""));
      if (!file) return;
      e.preventDefault();
      onPortrait(file);
    });
    dice.bind(panel);
    dice.paint();
    dice.paintLuck?.();
    const setDiceHelp = (show) => {
      const el = $("dice-help");
      if (!el) return;
      el.hidden = show == null ? !el.hidden : !show;
    };
    $("dice-help-btn")?.addEventListener("click", (e) => {
      e.preventDefault();
      setDiceHelp();
    });
    $("dice-help-close")?.addEventListener("click", (e) => {
      e.preventDefault();
      setDiceHelp(false);
    });
    panel.addEventListener("click", (e) => {
      const open = e.target.closest("[data-open-char]");
      if (open) {
        select(open.dataset.openChar);
        return;
      }
      const tabBtn = e.target.closest("[data-char-tab]");
      if (tabBtn) {
        tab = tabBtn.dataset.charTab;
        renderTabs();
        renderBody();
        return;
      }
      if (e.target.closest("#char-portrait-btn")) {
        if (isRemote(active())) return;
        $("char-portrait-file")?.click();
        return;
      }
      if (e.target.closest("#char-fullbody-btn")) {
        if (isRemote(active())) return;
        $("char-fullbody-file")?.click();
        return;
      }
      if (e.target.closest("#char-add-attack")) {
        const ch = active();
        if (!ch || isRemote(ch)) return;
        ch.attacks.push({ name: "", bonus: "", damage: "" });
        persist();
        renderBody();
        return;
      }
      const delKit = e.target.closest("[data-kit-del]");
      if (delKit) {
        const ch = active();
        if (!ch || isRemote(ch)) return;
        const i = Number(delKit.dataset.kitDel);
        if (!Number.isFinite(i) || !ch.kit) return;
        ch.kit.splice(i, 1);
        persist();
        renderBody();
        flash("Removed.");
        return;
      }
      const delAtk = e.target.closest("[data-del-attack]");
      if (delAtk) {
        const ch = active();
        if (!ch || isRemote(ch)) return;
        const i = Number(delAtk.dataset.delAttack);
        ch.attacks.splice(i, 1);
        if (!ch.attacks.length) ch.attacks.push({ name: "", bonus: "", damage: "" });
        persist();
        renderBody();
        return;
      }
      const pip = e.target.closest("[data-death]");
      if (pip) {
        const ch = active();
        if (!ch || isRemote(ch)) return;
        const key = pip.dataset.death === "fail" ? "deathFail" : "deathSuccess";
        const n = Number(pip.dataset.n);
        ch[key] = ch[key] === n ? n - 1 : n;
        if ((ch.deathFail || 0) >= 3 && ((Number(ch.hp) || 0) <= 0 || ch.dead)) markDead(ch);
        else if ((ch.deathSuccess || 0) >= 3 && ((Number(ch.hp) || 0) <= 0 || ch.downed) && !ch.dead) reviveAt(ch, 1);
        else {
          ch.dead = false;
          if ((Number(ch.hp) || 0) <= 0 && hpMaxOf(ch) > 0) ch.downed = true;
          else if ((Number(ch.hp) || 0) > 0) ch.downed = false;
        }
        persist();
        render();
        mapRefresh();
        return;
      }
      if (e.target.closest("#char-advance-btn") || e.target.closest("[data-advance-open]")) {
        openAdvance();
        return;
      }
      if (e.target.closest("[data-advance-close]")) {
        closeAdvance();
        return;
      }
      if (e.target.closest("[data-xp-go]")) {
        const ch = active();
        if (!ch || isRemote(ch)) return;
        const n = Number(panel.querySelector("[data-xp-award]")?.value) || 0;
        ch.xp = Math.max(0, (Number(ch.xp) || 0) + n);
        persist();
        render();
        flash("+" + n + " XP");
        return;
      }
      if (e.target.closest("[data-ip-go]")) {
        const ch = active();
        if (!ch || isRemote(ch)) return;
        const n = Number(panel.querySelector("[data-ip-award]")?.value) || 0;
        ch.ip = Math.max(0, (Number(ch.ip) || 0) + n);
        persist();
        render();
        flash("+" + n + " IP");
        return;
      }
      if (e.target.closest("[data-hp-avg]")) {
        const ch = active();
        const cls = ch && clsOf(ch);
        if (!advance || !cls) return;
        advance.hp = avgHpGain(cls.hd, modifier(ch.abilities.con));
        advance.hpRoll = null;
        paintAdvance();
        return;
      }
      if (e.target.closest("[data-hp-roll]")) {
        const ch = active();
        const cls = ch && clsOf(ch);
        if (!advance || !cls) return;
        const r = rollHpGain(cls.hd, modifier(ch.abilities.con));
        advance.hp = r.total;
        advance.hpRoll = r.roll;
        paintAdvance();
        return;
      }
      const asi = e.target.closest("[data-asi]");
      if (asi && advance) {
        const id = asi.dataset.asi;
        const ch = active();
        const spent = Object.values(advance.asi).reduce((a, b) => a + (Number(b) || 0), 0);
        const now = Number(advance.asi[id]) || 0;
        const score = Number(ch?.abilities?.[id]) || 10;
        if (now) {
          advance.asi[id] = 0;
        } else if (spent < 2 && score < 20) {
          const room = Math.min(2 - spent, 20 - score);
          advance.asi[id] = room >= 2 && spent === 0 ? 2 : 1;
        }
        advance.feat = "";
        paintAdvance();
        return;
      }
      const buy = e.target.closest("[data-red-buy]");
      if (buy && advance) {
        advance.kind = buy.dataset.redBuy;
        advance.target = buy.dataset.redId || "role";
        paintAdvance();
        return;
      }
      if (e.target.closest("[data-advance-go]")) {
        const ch = active();
        if (!ch || isRemote(ch) || !advance) return;
        const ok = advance.world === "blight" ? commitRedAdvance(ch) : commitFiveAdvance(ch);
        if (!ok) return;
        closeAdvance();
        persist();
        render();
        flash(worldOf() === "blight" ? "Advanced." : "Level " + ch.level);
      }
    });
    panel.addEventListener("change", (e) => {
      if (e.target?.dataset?.advSub && advance) {
        advance.subId = e.target.value;
        paintAdvance();
      }
    });
    panel.addEventListener("input", (e) => {
      if (e.target?.dataset?.advFeat && advance) {
        advance.feat = e.target.value;
        if (advance.feat) advance.asi = {};
      }
    });
    $("char-advance-btn")?.addEventListener("click", openAdvance);
    $("char-new")?.addEventListener("click", addChar);
    $("char-del")?.addEventListener("click", removeChar);
    $("char-portrait-file")?.addEventListener("change", (e) => {
      const file = e.target.files && e.target.files[0];
      e.target.value = "";
      if (file) onPortrait(file);
    });
    $("char-fullbody-file")?.addEventListener("change", (e) => {
      const file = e.target.files && e.target.files[0];
      e.target.value = "";
      if (file) onFullbody(file);
    });
    render();
  }

  function flash(text) {
    const head = $("chars-panel")?.querySelector(".chars-head h2");
    if (!head) return;
    const prev = head.dataset.label || head.textContent;
    head.dataset.label = prev;
    head.textContent = text;
    window.setTimeout(() => {
      if (head.textContent === text) head.textContent = prev;
    }, 1600);
  }

  function appendLine(block, line) {
    const cur = String(block || "").trim();
    const add = String(line || "").trim();
    if (!add) return cur;
    if (cur.split("\n").map((s) => s.trim()).includes(add)) return cur || add;
    return cur ? cur + "\n" + add : add;
  }

  function weaponBonus(ch, item) {
    if (item.wa) return String(item.wa);
    if (item.bonus) return String(item.bonus);
    if (worldOf() === "blight") return "";
    const props = String(item.props || "").toLowerCase();
    const n = String(item.name || "").toLowerCase();
    const ranged = /ammunition|thrown|firearm|range/.test(props) || /bow|crossbow|dart|sling|gun|pistol|rifle/.test(n);
    const finesse = props.includes("finesse");
    const dex = modifier(ch.abilities.dex);
    const str = modifier(ch.abilities.str);
    const abil = finesse ? Math.max(str, dex) : ranged ? dex : str;
    const plus = (String(item.name).match(/\+(\d+)/) || String(item.text || "").match(/\+(\d+)/) || [])[1];
    const extra = plus ? Number(plus) : 0;
    return signed(abil + computed(ch).prof + extra);
  }

  function weaponDamage(item) {
    if (item.dmg) return item.dmg;
    const n = String(item.name || "").toLowerCase();
    if (/dagger|dart/.test(n)) return "1d4 piercing";
    if (/bow|crossbow/.test(n)) return "1d8 piercing";
    if (/axe|sword|scimitar|glaive|halberd/.test(n)) return "1d8 slashing";
    if (/mace|hammer|staff|club/.test(n)) return "1d6 bludgeoning";
    if (/spear|javelin|pike/.test(n)) return "1d6 piercing";
    if (/pistol|rifle|shotgun|smg/.test(n)) return "2d6";
    if (/\+(\d+)/.test(item.name) || /\+(\d+)/.test(item.text || "")) return "1d8";
    return "1d6";
  }

  function looksLikeWeapon(item) {
    if (item.kind === "weapon") return true;
    if (item.dmg) return true;
    return /sword|axe|bow|mace|hammer|dagger|staff|spear|crossbow|javelin|trident|flail|glaive|halberd|lance|pike|rapier|scimitar|whip|morningstar|pistol|rifle|shotgun|smg/.test(
      String(item.name || "").toLowerCase()
    );
  }

  function applyArmor(ch, item) {
    const raw = String(item.ac ?? "");
    if (worldOf() === "blight") {
      const n = parseInt(raw, 10);
      if (Number.isFinite(n)) ch.ac = n;
      return;
    }
    const name = String(item.name || "").toLowerCase();
    const cat = String(item.cat || "").toLowerCase();
    if (cat === "shield" || name.includes("shield")) {
      const n = parseInt(raw.replace("+", ""), 10);
      ch.ac = (Number(ch.ac) || 10) + (Number.isFinite(n) ? n : 2);
      return;
    }
    const base = parseInt(raw, 10);
    if (!Number.isFinite(base)) return;
    let ac = base;
    if (/\+\s*Dex/i.test(raw)) {
      const dex = modifier(ch.abilities.dex);
      const cap = raw.match(/max\s*(\d+)/i);
      ac = base + (cap ? Math.min(dex, Number(cap[1])) : dex);
    }
    ch.ac = ac;
  }

  function receiveKit(item, charId) {
    if (!item || !item.name) return false;
    let ch = charId
      ? store.list.find((c) => c.id === charId) || allChars().find((c) => c.id === charId)
      : active();
    if (!ch) {
      if (!store.list.length) addChar();
      ch = active();
    }
    if (!ch || isRemote(ch)) return false;
    store.activeId = ch.id;
    if (!Array.isArray(ch.kit)) ch.kit = [];
    const stack = ch.kit.find((k) => k.catalogId && k.catalogId === item.id && item.kind !== "chrome");
    if (stack) stack.qty += 1;
    else {
      ch.kit.push({
        id: "k_" + Date.now().toString(36) + Math.random().toString(36).slice(2, 6),
        catalogId: item.id || "",
        name: item.name,
        kind: item.kind || "gear",
        qty: 1,
        detail: item.text || item.props || "",
        dmg: item.dmg || (looksLikeWeapon(item) ? weaponDamage(item) : ""),
      });
    }
    const kind = item.kind || "gear";
    if (looksLikeWeapon(item)) {
      const empty = (ch.attacks || []).length === 1 && !ch.attacks[0].name;
      if (empty) ch.attacks = [];
      ch.attacks.push({
        name: item.name,
        bonus: weaponBonus(ch, item),
        damage: weaponDamage(item),
      });
    }
    if (kind === "armor" || /armor|mail|plate|leather|shield/i.test(item.name) && kind === "magic") {
      applyArmor(ch, item);
    }
    if (kind === "spell") {
      const lv = Number(item.lv);
      if (item.world === "blight" || !Number.isFinite(lv)) {
        ch.programs = appendLine(ch.programs, item.name);
      } else if (!lv) {
        ch.cantrips = appendLine(ch.cantrips, item.name);
      } else if (lv >= 1 && lv <= 9) {
        if (!Array.isArray(ch.spells) || ch.spells.length < 9) ch.spells = Array.from({ length: 9 }, (_, i) => ch.spells?.[i] || "");
        ch.spells[lv - 1] = appendLine(ch.spells[lv - 1], item.name);
      }
    }
    if (kind === "chrome") {
      ch.cyberware = appendLine(ch.cyberware, item.hl ? `${item.name} (HL ${item.hl})` : item.name);
      if (item.slot) {
        ch.chrome = ch.chrome || {};
        ch.chrome[item.slot] = true;
      }
      if (item.hl) {
        const hl = Number(item.hl) || 0;
        if (hl) ch.humanity = Math.max(0, Number(ch.humanity || 0) - hl);
      }
    }
    if (kind === "quickhack") {
      ch.programs = appendLine(ch.programs, item.name);
    }
    if (kind === "gear" || kind === "magic") {
      const line = [item.name, item.rarity, item.dmg ? `dmg ${item.dmg}` : "", item.text ? "" : item.props]
        .filter(Boolean)
        .join(" · ");
      ch.equipment = appendLine(ch.equipment, line || item.name);
    }
    if (kind === "spell" && Number(item.lv) >= 0 && item.world !== "blight") {
      tab = "magic";
    } else if (kind === "chrome") {
      tab = "chrome";
    } else if (looksLikeWeapon(item) || kind === "armor") {
      tab = "combat";
    } else if (kind === "quickhack" || kind === "spell") {
      tab = worldOf() === "blight" ? "gear" : "magic";
    } else {
      tab = "gear";
    }
    persist();
    const panel = $("chars-panel");
    if (panel) {
      panel.hidden = false;
      document.querySelector(".app")?.classList.add("chars-open");
      document.getElementById("chars-toggle")?.classList.add("on");
    }
    render();
    flash("Added " + item.name);
    return true;
  }

  function receiveDeity(god) {
    const ch = active();
    if (!ch || isRemote(ch) || !god) return false;
    ch.deity = god.name;
    ch.deityId = god.id;
    ch.deityDomain = god.domains || "";
    if (clsOf(ch)?.id === "cleric" && god.domains && !ch.subclassId) {
      const want = String(god.domains).toLowerCase();
      const cls = clsOf(ch);
      const hit = (cls?.subclasses || []).find((s) => want.includes(s.name.toLowerCase().replace(" domain", "")));
      if (hit) {
        ch.subclassId = hit.id;
        ch.subclass = hit.name;
        const lv = Number(ch.level) || 1;
        for (let n = 1; n <= lv; n++) grantLevelFeatures(ch, cls, n, hit);
      }
    }
    const note = `${god.name} (${god.pantheon}). ${god.domains || ""} ${god.symbol ? "Symbol: " + god.symbol : ""}`.trim();
    if (!String(ch.notes || "").includes(god.name)) {
      ch.notes = [ch.notes, note, god.blurb].filter(Boolean).join("\n\n");
    }
    persist();
    openPanel();
    tab = "bio";
    render();
    flash("Deity: " + god.name);
    return true;
  }

  function openPanel() {
    const panel = $("chars-panel");
    if (panel) {
      panel.hidden = false;
      document.querySelector(".app")?.classList.add("chars-open");
      document.getElementById("chars-toggle")?.classList.add("on");
    }
  }

  async function fillPortrait(ch, src) {
    const raw = String(src || "");
    if (!raw) return;
    if (raw.startsWith("data:")) {
      ch.portrait = raw;
      persist();
      paintPortraits();
      return;
    }
    try {
      const res = await fetch(raw);
      if (!res.ok) return;
      const blob = await res.blob();
      ch.portrait = await resizeImage(blob, 256, 0.82);
      persist();
      paintPortraits();
    } catch {
      /* keep sheet without face */
    }
  }

  async function fromToken(spec, tok) {
    if (!spec || spec.kind === "char") return null;
    if (spec.kind !== "beast" && spec.kind !== "npc" && spec.kind !== "srdnpc" && spec.kind !== "shard") return null;
    let found = null;
    try {
      found = await lookupCreature(spec.kind, spec.ref, spec.src);
    } catch {
      return null;
    }
    if (!found?.row) return null;
    const ch =
      found.world === "blight"
        ? sheetFromRed(found.row, spec, store.list)
        : sheetFrom5e(found.row, spec, store.list);
    store.list.push(ch);
    store.activeId = ch.id;
    tab = "combat";
    persist();
    openPanel();
    render();
    flash("Sheet: " + ch.name);
    if (tok) {
      tok.sheetId = ch.id;
      tok.ownerId = "";
      mapTouch();
    }
    fillPortrait(ch, spec.src || tok?.src || "");
    return ch;
  }

  return {
    bind,
    render,
    store,
    receiveKit,
    receivePortrait: onPortrait,
    active,
    select,
    setNet,
    shareNow,
    applyRemote,
    applyRemoteTable,
    clearRemote,
    ingestRoll,
    ingestRolls,
    isRemote,
    setGM,
    fromToken,
    receiveDeity,
    cancelAim,
    tokenStatus(t) {
      const ch = findChar(t?.sheetId || t?.ref || "", t?.ownerId || "");
      if (!ch) return "";
      if (isDead(ch)) return "dead";
      if (isDowned(ch)) return "downed";
      return "";
    },
  };
}
