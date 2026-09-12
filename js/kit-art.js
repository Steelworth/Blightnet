function esc(value) {
  return String(value ?? "").replace(/[&<>"']/g, (ch) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[ch])
  );
}

function hashHue(id) {
  let h = 0;
  const s = String(id || "");
  for (let i = 0; i < s.length; i++) h = (h * 33 + s.charCodeAt(i)) >>> 0;
  return h % 360;
}

function mix(hex, hueShift) {
  const n = hex.replace("#", "");
  const r = parseInt(n.slice(0, 2), 16);
  const g = parseInt(n.slice(2, 4), 16);
  const b = parseInt(n.slice(4, 6), 16);
  const p = hueShift / 720;
  const rr = Math.max(0, Math.min(255, Math.round(r + (g - r) * p)));
  const gg = Math.max(0, Math.min(255, Math.round(g + (b - g) * p)));
  const bb = Math.max(0, Math.min(255, Math.round(b + (r - b) * p)));
  return `#${rr.toString(16).padStart(2, "0")}${gg.toString(16).padStart(2, "0")}${bb.toString(16).padStart(2, "0")}`;
}

const SCHOOL = {
  abjuration: { ink: "#7ec8ff", paper: "#122033", mark: "ward" },
  conjuration: { ink: "#c9a0ff", paper: "#1a1230", mark: "ring" },
  divination: { ink: "#7ef0e0", paper: "#0e2424", mark: "eye" },
  enchantment: { ink: "#ff9ad6", paper: "#2a1224", mark: "heart" },
  evocation: { ink: "#ff7a3a", paper: "#2a1208", mark: "burst" },
  illusion: { ink: "#b8a0ff", paper: "#1a1630", mark: "mask" },
  necromancy: { ink: "#7dff9a", paper: "#0c1a10", mark: "skull" },
  transmutation: { ink: "#f0d060", paper: "#241a08", mark: "spark" },
};

const KIND = {
  weapon: { ink: "#e8dcc4", paper: "#1a120c", frame: "#c4a574" },
  armor: { ink: "#c8d0d8", paper: "#12161c", frame: "#8a9aaa" },
  gear: { ink: "#d2c4a0", paper: "#16120c", frame: "#8a7a52" },
  magic: { ink: "#f0c86a", paper: "#1a1408", frame: "#e2b34a" },
  spell: { ink: "#9ad8ff", paper: "#0c1420", frame: "#5aa0d0" },
  chrome: { ink: "#00f0ff", paper: "#080c10", frame: "#fcee0a" },
  quickhack: { ink: "#ff2d6a", paper: "#140010", frame: "#00f0ff" },
};

const MARKS = {
  sword: '<path d="M32 8l3 3-14 28-5 1-1-5z"/><path d="M18 36l8 8"/><rect x="26" y="44" width="12" height="4" rx="1"/><path d="M31 48v8h2v-8"/>',
  axe: '<path d="M30 10v36"/><path d="M30 16c10-2 16 4 16 10-8 0-12-2-16-2v-8z"/><path d="M28 50h6v4h-6z"/>',
  mace: '<path d="M31 22v30h2V22"/><circle cx="32" cy="16" r="8"/><path d="M32 6v4M24 12l-3-3M40 12l3-3M22 18h-4M42 18h4"/>',
  hammer: '<path d="M31 22v30h2V22"/><path d="M18 12h28v10H18z"/>',
  spear: '<path d="M32 8l4 10H28z"/><path d="M31 18v38h2V18"/>',
  dagger: '<path d="M32 10l2 2-10 22-3 1-1-3z"/><path d="M21 32l6 6"/><path d="M28 40h8v3h-8z"/>',
  bow: '<path d="M18 10c14 8 14 36 0 44"/><path d="M18 10l20 22L18 54"/><path d="M38 32h10"/>',
  crossbow: '<path d="M16 28h32"/><path d="M32 16v32"/><path d="M20 28l-4 8M44 28l4 8"/><path d="M28 48h8v4h-8z"/>',
  whip: '<path d="M22 12c18 0 22 10 8 16S18 42 36 52"/><circle cx="22" cy="12" r="3"/>',
  net: '<path d="M16 16h32v32H16z"/><path d="M16 32h32M32 16v32M22 16l20 32M42 16L22 48"/>',
  gun: '<path d="M14 28h28l4-6h6v10H46l-2 12H28l-4-10H14z"/><path d="M24 34h6v10h-4z"/>',
  smg: '<path d="M12 26h34l2-6h6v8H50l-2 16H30l-4-8H12z"/><path d="M22 36h8v14h-5z"/><path d="M14 22h10v4H14z"/>',
  rifle: '<path d="M8 30h40l8-8h6v8l-6 4H36l-4 12H24l-4-12H8z"/><path d="M20 34h8v16h-4z"/>',
  shotgun: '<path d="M10 28h38l6-4h8v10H52l-4 14H28l-6-10H10z"/><path d="M22 34h10v16h-5z"/>',
  grenade: '<circle cx="32" cy="36" r="14"/><path d="M28 22h8v6h-8z"/><path d="M32 16v6M26 18h12"/>',
  shield: '<path d="M32 8l20 8v16c0 14-12 24-20 28-8-4-20-14-20-28V16z"/><path d="M32 18v28M22 28h20"/>',
  plate: '<path d="M16 16l16-6 16 6v12c0 16-8 26-16 30-8-4-16-14-16-30z"/><path d="M20 28h24"/>',
  leather: '<path d="M20 14l12-4 12 4v34l-12 6-12-6z"/><path d="M24 26h16M24 36h16"/>',
  helm: '<path d="M16 36c0-12 8-22 16-22s16 10 16 22v6H16z"/><path d="M16 38h32"/><path d="M28 28h8v8h-8z"/>',
  potion: '<path d="M26 12h12v8l8 8v16a8 8 0 0 1-8 8H26a8 8 0 0 1-8-8V28l8-8z"/><path d="M24 12h16"/>',
  ring: '<circle cx="32" cy="34" r="12"/><circle cx="32" cy="34" r="6"/><path d="M32 16v6"/>',
  wand: '<path d="M18 48l22-28 4 4-22 28z"/><path d="M40 16l6-6M38 12h10M44 8v10"/>',
  staff: '<path d="M30 58V18"/><circle cx="32" cy="14" r="8"/><path d="M32 6v4"/>',
  scroll: '<path d="M18 16h28v32H18z"/><path d="M18 16c0-4 6-4 6 0M46 48c0 4-6 4-6 0"/><path d="M24 26h16M24 34h16M24 42h10"/>',
  book: '<path d="M16 14h14v36H20c-2 0-4 2-4 2V14zM48 14H34v36h10c2 0 4 2 4 2V14z"/>',
  bag: '<path d="M20 24h24l4 28H16z"/><path d="M24 24c0-8 16-8 16 0"/><path d="M32 24v10"/>',
  cloak: '<path d="M32 10c6 0 10 6 10 12 8 4 12 14 12 24H10c0-10 4-20 12-24 0-6 4-12 10-12z"/>',
  boot: '<path d="M22 14h10v22h16v10H18V28c0-8 4-14 4-14z"/>',
  amulet: '<path d="M32 8v14"/><circle cx="32" cy="36" r="12"/><path d="M24 10c4 4 12 4 16 0"/>',
  pack: '<path d="M18 20h28v28H18z"/><path d="M22 20v-6h20v6M18 32h28"/>',
  torch: '<path d="M30 28h4v28h-4z"/><path d="M32 8c6 6 2 12 6 16-8 0-14-6-14-14 6 0 8-2 8-2z"/>',
  rope: '<path d="M16 20c8-8 16 8 24 0s16 8 8 16-16-8-24 0-16-8-8-16z"/>',
  lantern: '<path d="M24 16h16v6H24z"/><path d="M26 22h12v24H26z"/><path d="M32 10v6M22 46h20"/>',
  lock: '<path d="M22 28h20v20H22z"/><path d="M26 28v-8a6 6 0 0 1 12 0v8"/><circle cx="32" cy="40" r="3"/>',
  tools: '<path d="M18 18l12 12-4 4-12-12z"/><path d="M34 30l14 14-4 4-14-14z"/><circle cx="20" cy="20" r="5"/>',
  food: '<ellipse cx="32" cy="36" rx="16" ry="10"/><path d="M16 36c4-12 24-12 32 0"/>',
  tent: '<path d="M8 48l24-32 24 32z"/><path d="M32 16v32"/>',
  eye: '<path d="M10 32c8-12 36-12 44 0-8 12-36 12-44 0z"/><circle cx="32" cy="32" r="8"/><circle cx="32" cy="32" r="3"/>',
  jack: '<rect x="20" y="18" width="24" height="28" rx="3"/><path d="M26 18V10h4v8M34 18V10h4v8"/><path d="M26 32h12"/>',
  arm: '<path d="M20 12h12c8 0 14 8 14 16v20h-10V30c0-4-2-8-6-8H20z"/><circle cx="44" cy="50" r="6"/>',
  spine: '<path d="M32 8v48"/><path d="M24 16h16M24 28h16M24 40h16M26 52h12"/>',
  chip: '<rect x="16" y="16" width="32" height="32" rx="3"/><path d="M16 24h-6M16 32h-6M16 40h-6M48 24h6M48 32h6M48 40h6"/><rect x="24" y="24" width="16" height="16"/>',
  skull: '<circle cx="32" cy="28" r="14"/><circle cx="26" cy="28" r="3"/><circle cx="38" cy="28" r="3"/><path d="M26 44h12v8H26zM28 40v4M32 40v4M36 40v4"/>',
  burst: '<path d="M32 8l4 16 16 4-16 4-4 16-4-16-16-4 16-4z"/>',
  ward: '<path d="M32 8l18 8v16c0 12-10 22-18 26-8-4-18-14-18-26V16z"/>',
  heart: '<path d="M32 52s-18-12-18-24c0-8 6-12 10-12 4 0 8 4 8 4s4-4 8-4c4 0 10 4 10 12 0 12-18 24-18 24z"/>',
  mask: '<path d="M16 24c0-8 8-14 16-14s16 6 16 14c0 12-6 22-16 22S16 36 16 24z"/><path d="M24 28h4M36 28h4M26 38c4 4 8 4 12 0"/>',
  spark: '<path d="M32 8v16M32 40v16M12 32h16M36 32h16M18 18l10 10M36 36l10 10M46 18L36 28M28 36L18 46"/>',
  bolt: '<path d="M34 8L18 34h12L26 56l22-30H36z"/>',
  bug: '<ellipse cx="32" cy="34" rx="10" ry="14"/><path d="M32 20v28M22 28h20M22 36h20M22 24l-8-6M42 24l8-6M22 44l-6 8M42 44l6 8"/>',
  drone: '<circle cx="32" cy="32" r="8"/><path d="M12 20h12v8H12zM40 20h12v8H40zM12 36h12v8H12zM40 36h12v8H40z"/>',
  car: '<path d="M12 36l8-12h24l8 12v10H12z"/><circle cx="22" cy="48" r="5"/><circle cx="42" cy="48" r="5"/>',
  radio: '<rect x="18" y="20" width="28" height="28" rx="2"/><circle cx="32" cy="34" r="8"/><path d="M32 12v8"/>',
  needle: '<path d="M32 8v40"/><path d="M26 48h12l-2 8h-8z"/><circle cx="32" cy="20" r="4"/>',
  coin: '<circle cx="32" cy="32" r="16"/><path d="M32 20v24M24 28h16M24 36h16"/>',
  crystal: '<path d="M32 8l12 16-12 32L20 24z"/><path d="M20 24h24"/>',
  orb: '<circle cx="32" cy="32" r="16"/><path d="M20 28c6-8 18-8 24 0"/>',
  default: '<rect x="20" y="20" width="24" height="24" rx="4"/>',
};

function pickMark(item) {
  const n = `${item.name || ""} ${item.cat || ""} ${item.kind || ""} ${item.dmg || ""} ${item.school || ""}`.toLowerCase();
  const hit = (...keys) => keys.some((k) => n.includes(k));
  if (item.kind === "spell") return SCHOOL[item.school]?.mark || "burst";
  if (item.kind === "quickhack") {
    if (hit("ping", "scan", "see", "wisp", "raven")) return "eye";
    if (hit("kill", "suicide", "burnout", "death", "hell")) return "skull";
    if (hit("armor", "shield", "wall", "fort")) return "ward";
    if (hit("sword", "bolt", "zap", "short", "overheat")) return "bolt";
    if (hit("virus", "worm", "bug", "daemon", "contagion")) return "bug";
    if (hit("ice", "blackwall", "asp", "hound", "kraken")) return "skull";
    if (hit("camera", "turret", "control", "remote")) return "drone";
    return "chip";
  }
  if (item.kind === "chrome") {
    if (hit("eye", "optic", "tact", "virtuality")) return "eye";
    if (hit("arm", "hand", "knuck", "ripper", "wolver", "slice")) return "arm";
    if (hit("jack", "link", "plug", "chip", "socket", "neural")) return "jack";
    if (hit("sandevistan", "kerenzikov", "speed", "reflex", "spine")) return "spine";
    if (hit("audio", "ear", "radio")) return "radio";
    if (hit("skin", "dermal", "weave", "armor")) return "plate";
    if (hit("leg", "foot", "jump", "skate")) return "boot";
    return "chip";
  }
  if (hit("pistol", "gun", "smg") && hit("heavy smg", "smg")) return "smg";
  if (hit("assault", "rifle", "sniper", "rail")) return "rifle";
  if (hit("shotgun")) return "shotgun";
  if (hit("pistol", "gun", "dartgun", "microwaver", "stun gun", "air pistol")) return "gun";
  if (hit("grenade", "rocket", "c4", "napalm")) return "grenade";
  if (hit("crossbow")) return "crossbow";
  if (hit("bow", "arrow")) return "bow";
  if (hit("axe", "halberd", "glaive", "pick")) return "axe";
  if (hit("hammer", "maul", "club", "bat", "pipe", "crowbar")) return "hammer";
  if (hit("mace", "flail", "morningstar")) return "mace";
  if (hit("spear", "javelin", "pike", "lance", "trident")) return "spear";
  if (hit("dagger", "knife", "dart", "needle", "sickle", "ripper")) return "dagger";
  if (hit("whip", "chain")) return "whip";
  if (hit("net")) return "net";
  if (hit("sword", "scimitar", "rapier", "glaive") || item.kind === "weapon") {
    if (hit("gun", "rifle", "pistol")) return "gun";
    return "sword";
  }
  if (hit("shield")) return "shield";
  if (hit("helm", "helmet")) return "helm";
  if (hit("plate", "mail", "flak", "armorjack", "metalgear", "kevlar", "splint")) return "plate";
  if (hit("leather", "hide", "padded", "studded")) return "leather";
  if (item.kind === "armor") return "plate";
  if (hit("potion", "oil", "philter", "ointment", "flask", "vial", "acid", "alchem")) return "potion";
  if (hit("ring")) return "ring";
  if (hit("wand")) return "wand";
  if (hit("staff", "rod")) return "staff";
  if (hit("scroll")) return "scroll";
  if (hit("book", "tome", "manual", "spellbook")) return "book";
  if (hit("bag", "haversack", "pouch", "sack")) return "bag";
  if (hit("cloak", "cape", "robe")) return "cloak";
  if (hit("boot", "shoe", "slipper")) return "boot";
  if (hit("amulet", "medallion", "periapt", "necklace", "brooch")) return "amulet";
  if (hit("pack", "backpack", "chest")) return "pack";
  if (hit("torch")) return "torch";
  if (hit("lantern", "lamp", "candle", "glow")) return "lantern";
  if (hit("rope", "chain (10")) return "rope";
  if (hit("lock", "manacle", "handcuff")) return "lock";
  if (hit("tool", "kit", "tinker", "smith", "thieves")) return "tools";
  if (hit("ration", "kibble", "food", "feast", "berry")) return "food";
  if (hit("tent", "camp")) return "tent";
  if (hit("crystal", "gem", "ioun")) return "crystal";
  if (hit("orb", "sphere", "globe", "ball")) return "orb";
  if (hit("coin", "eb", "gp") && item.kind === "gear") return "coin";
  if (hit("agent", "phone", "computer", "deck")) return "chip";
  if (hit("drone")) return "drone";
  if (hit("car", "cycle", "vehicle", "bike")) return "car";
  if (hit("hypo", "stim", "drug", "poison", "toxin")) return "needle";
  if (item.kind === "magic") return "spark";
  if (item.kind === "gear") return "pack";
  return "default";
}

function palette(item) {
  const base = KIND[item.kind] || KIND.gear;
  if (item.kind === "spell" && SCHOOL[item.school]) {
    return { ...base, ink: SCHOOL[item.school].ink, paper: SCHOOL[item.school].paper };
  }
  const hue = hashHue(item.id || item.name);
  return {
    paper: base.paper,
    ink: mix(base.ink, (hue % 40) - 20),
    frame: mix(base.frame, (hue % 24) - 12),
  };
}

const DIRECT_LOOK = new Set([
  "battleaxe", "blowgun", "club", "dagger", "dart", "flail", "glaive", "greataxe", "greatclub",
  "greatsword", "halberd", "hand-crossbow", "handaxe", "heavy-crossbow", "javelin", "lance",
  "light-crossbow", "light-hammer", "longbow", "longsword", "mace", "maul", "morningstar", "net",
  "pike", "quarterstaff", "rapier", "scimitar", "shortbow", "shortsword", "sickle", "sling",
  "spear", "trident", "war-pick", "warhammer", "whip",
  "breastplate", "chain-mail", "chain-shirt", "half-plate", "hide", "leather", "padded", "plate",
  "ring-mail", "scale-mail", "shield", "splint", "studded-leather",
]);

function hay(item) {
  return `${item.id || ""} ${item.name || ""} ${item.cat || ""} ${item.kind || ""} ${item.school || ""} ${item.dmg || ""}`.toLowerCase();
}

export function lookOf(item) {
  if (!item) return "pack";
  const id = String(item.id || "");
  if (DIRECT_LOOK.has(id)) return id;
  const n = hay(item);
  const hit = (...keys) => keys.some((k) => n.includes(k));

  if (hit("flame tongue", "sun blade", "burning", "flaming") && hit("sword", "blade", "scimitar", "weapon")) return "flaming-sword";
  if (hit("frost brand", "staff of frost", "winter") && (item.kind === "magic" || item.kind === "weapon")) return "frost-weapon";
  if (hit("holy avenger", "mace of disruption", "sun blade")) return "holy-weapon";
  if (hit("vorpal", "sword of sharpness", "nine lives")) return "keen-sword";
  if (hit("luck blade", "weapon, +", "vicious weapon")) return "longsword";
  if (hit("oathbow")) return "longbow";
  if (hit("javelin of lightning")) return "javelin";
  if (hit("dwarven thrower")) return "warhammer";
  if (hit("hammer of thunderbolts")) return "maul";

  if (item.kind === "spell") {
    if (hit("fireball", "burning hands", "fire bolt", "scorching", "heat metal", "wall of fire", "flame strike", "hellish")) return "spell-fire";
    if (hit("lightning", "call lightning", "shocking", "witch bolt", "chain lightning", "storm")) return "spell-lightning";
    if (hit("magic missile", "eldritch blast", "disintegrate", "force")) return "spell-force";
    if (hit("cure", "heal", "healing word", "prayer of healing", "mass heal", "spare the dying", "revivify", "raise dead", "regenerate")) return "spell-heal";
    if (hit("shield", "mage armor", "armor of agathys", "protection from", "counterspell", "dispel", "globe", "sanctuary", "ward")) return "spell-ward";
    if (hit("invisibility", "blur", "mirror image", "silent image", "major image", "hypnotic", "color spray", "phantasmal")) return "spell-illusion";
    if (hit("hold person", "charm", "dominate", "suggestion", "sleep", "calm emotions", "command", "compulsion")) return "spell-charm";
    if (hit("misty step", "dimension door", "teleport", "thunder step", "far step")) return "spell-blink";
    if (hit("fly", "levitate", "feather fall", "gaseous")) return "spell-fly";
    if (hit("web", "entangle", "black tentacles", "evard's")) return "spell-web";
    if (hit("ice", "cone of cold", "ray of frost", "sleet", "ice storm", "armor of agathys")) return "spell-ice";
    if (hit("acid", "melf", "vitriolic", "cloudkill", "stinking", "poison")) return "spell-acid";
    if (hit("animate dead", "blight", "inflict", "chill touch", "vampiric", "finger of death", "circle of death", "spirit", "necro")) return "spell-necrotic";
    if (hit("summon", "conjure", "unseen servant", "spiritual weapon", "spirit guardians", "find familiar", "hunger of hadar")) return "spell-summon";
    if (hit("guidance", "detect", "identify", "scrying", "augury", "commune", "see invisibility", "true seeing", "locate")) return "spell-divination";
    if (hit("barkskin", "shillelagh", "moonbeam", "call lightning", "plant", "goodberry", "spike growth")) return "spell-nature";
    if (hit("sacred flame", "guiding bolt", "spirit guardians", "holy", "bless", "bane", "shield of faith")) return "spell-radiant";
    const school = {
      evocation: "spell-fire",
      abjuration: "spell-ward",
      conjuration: "spell-summon",
      divination: "spell-divination",
      enchantment: "spell-charm",
      illusion: "spell-illusion",
      necromancy: "spell-necrotic",
      transmutation: "spell-blink",
    };
    return school[item.school] || "spell-force";
  }

  if (item.kind === "quickhack" || (item.kind === "spell" && item.world === "blight")) {
    if (hit("ping", "scan", "wisp", "raven", "see")) return "hack-ping";
    if (hit("suicide", "kill", "death", "hellhound", "kraken", "asp", "black ice", "ice")) return "hack-kill";
    if (hit("short", "overheat", "zap", "synapse", "burnout", "reboot")) return "hack-short";
    if (hit("weapon", "jam", "glitch", "control")) return "hack-gun";
    if (hit("camera", "turret", "remote", "vehicle")) return "hack-camera";
    if (hit("virus", "worm", "contagion", "daemon")) return "hack-virus";
    if (hit("memory", "wipe", "whispers", "scream")) return "hack-mind";
    return "hack-breach";
  }

  if (item.kind === "chrome") {
    if (hit("sandevistan", "kerenzikov", "speedware", "reflex", "boosted reflexes")) return "chrome-sandevistan";
    if (hit("eye", "optic", "kiroshi", "virtuality", "anti-dazzle", "targeting", "teleoptics", "micro-optics")) return "chrome-eye";
    if (hit("audio", "ear", "hearing", "radio", "voice", "audio recorder", "level damper", "sound editor")) return "chrome-audio";
    if (hit("arm", "hand", "cyberarm", "popup", "tool hand", "gorilla", "hydraulic", "rhinocefist", "magna knuckle")) return "chrome-arm";
    if (hit("leg", "foot", "jump", "skate", "grip foot", "talon")) return "chrome-leg";
    if (hit("rippers", "wolvers", "slice", "vampires", "big knucks", "cybersnake")) return "chrome-rippers";
    if (hit("jack", "link", "plug", "chipware", "socket", "neural", "processor", "sandevistan") || hit("interface", "chip")) return "chrome-jack";
    if (hit("dermal", "skin", "weave", "subdermal", "armor")) return "chrome-dermal";
    if (hit("linear", "frame", "exoskel", "grafted muscle", "muscle graft", "bone lace", "skin weave")) return "chrome-frame";
    if (hit("pain editor", "toxin", "nasal", "independent", "impossibl", "mr. studd", "midnight")) return "chrome-internal";
    return "chrome-jack";
  }

  if (hit("katana", "wakizashi", "reaver")) return "katana";
  if (hit("tanto", "hornet")) return "tanto";
  if (hit("boomerang")) return "boomerang";
  if (hit("chainsaw")) return "chainsaw";
  if (hit("nunchaku")) return "nunchaku";
  if (hit("baseball", "spiked bat")) return "baseball-bat";
  if (hit("crowbar")) return "crowbar";
  if (hit("lead pipe", "lead-pipe")) return "lead-pipe";
  if (hit("machete", "kleaver")) return "machete";
  if (hit("bowie", "combat knife", "knife")) return "combat-knife";
  if (hit("tomahawk")) return "tomahawk";
  if (hit("brass knuck", "knucks", "magna knuck")) return "brass-knuckles";
  if (hit("stun baton")) return "stun-baton";
  if (hit("rippers", "wolvers", "slice & dice", "vampires")) return "chrome-rippers";
  if (hit("railgun", "javelin") && hit("superchrome", "rail", "exotic")) return "railgun";
  if (hit("flamethrower")) return "flamethrower";
  if (hit("rocket", "antitank")) return "rocket-launcher";
  if (hit("grenade launcher", "door gun", "engage rocket")) return "grenade-launcher";
  if (hit("sniper")) return "sniper-rifle";
  if (hit("assault rifle", "variable automatic", "glam rifle", "sportmaster", "survivalmaster", "survivalist")) return "assault-rifle";
  if (hit("shotgun", "mastiff", "matchmaker", "georgia arms")) return "shotgun";
  if (hit("heavy smg", "mastiff-smg")) return "heavy-smg";
  if (hit("smg")) return "smg";
  if (hit("very heavy pistol", "perseus")) return "very-heavy-pistol";
  if (hit("heavy pistol")) return "heavy-pistol";
  if (hit("air pistol", "dartgun")) return "dartgun";
  if (hit("stun gun", "microwaver", "nomad rocker")) return "stun-gun";
  if (hit("pistol", "sidearm", "public defender", "gunmart special", "matchmaker", "e-tack", "ww1")) return "medium-pistol";
  if (hit("bows", "bow") && item.world === "blight") return "shortbow";
  if (hit("crossbow") && item.world === "blight") return "heavy-crossbow";
  if (item.cat === "ammo" || /\b(ammo|arrows|bolts|shells)\b/.test(n)) return "ammo";
  if (hit("grenade", "flashbang", "c4", "micro-bomb", "combustor", "detonator", "explosive")) return "grenade";
  if (hit("sword") && item.kind === "weapon") return "longsword";
  if (hit("hammer") && item.kind === "weapon") return "warhammer";

  if (hit("metalgear")) return "metalgear";
  if (hit("flak")) return "flak";
  if (hit("heavy armorjack")) return "heavy-armorjack";
  if (hit("medium armorjack")) return "medium-armorjack";
  if (hit("light armorjack") || hit("armorjack")) return "light-armorjack";
  if (hit("kevlar")) return "kevlar";
  if (hit("leathers", "road leather", "nomad road")) return "leathers";
  if (hit("trench", "skidrow")) return "trench";
  if (hit("helmet", "helm") && item.kind === "armor") return "helmet";
  if (hit("evening", "holo-wear", "leo dress", "island", "montage", "variable clothing")) return "fashion-armor";
  if (hit("combat jacket", "bunker", "shock armor", "smart armor", "scavenged")) return "medium-armorjack";
  if (hit("mithral", "plate armor of", "armor, +") && item.kind === "magic") return "plate";
  if (hit("shield") && (item.kind === "armor" || item.kind === "magic")) return "shield";

  if (hit("potion of poison", "oil of", "philter", "ointment") && hit("poison")) return "potion-green";
  if (hit("potion of invisibility", "potion of gaseous", "potion of flying", "potion of speed", "potion of climbing", "potion of water")) return "potion-blue";
  if (hit("potion of giant", "heroism", "fire")) return "potion-gold";
  if (hit("potion of healing", "superior", "supreme", "greater", "cure")) return "potion-red";
  if (hit("potion", "philter", "flask", "elixir", "oil of", "universal solvent", "sovereign glue")) return "potion-red";
  if (hit("ring of")) return "ring";
  if (hit("wand of", "wand")) return "wand";
  if (hit("staff of fire")) return "staff-fire";
  if (hit("staff of frost")) return "staff-frost";
  if (hit("staff of healing", "woodlands", "python")) return "staff-wood";
  if (hit("staff of power", "staff of the magi", "staff of striking", "staff of thunder", "staff of withering", "staff of swarming")) return "staff-power";
  if (hit("staff", "rod of")) return "staff-wood";
  if (hit("cloak of", "cape", "wings of flying")) return "cloak";
  if (hit("winged boots", "boots of", "slippers")) return "winged-boots";
  if (hit("bag of holding", "handy haversack", "portable hole")) return "bag-holding";
  if (hit("spell scroll", "scroll")) return "scroll";
  if (hit("robe of")) return "robe";
  if (hit("ioun")) return "ioun";
  if (hit("horn of")) return "horn";
  if (hit("crystal", "gem", "pearl of", "stone of")) return "crystal";
  if (hit("amulet", "medallion", "periapt", "necklace", "brooch", "scarab")) return "amulet";
  if (hit("helm of", "headband")) return "helmet";
  if (hit("gauntlets", "bracers", "belt of")) return "chrome-arm";
  if (hit("lantern of")) return "lantern";
  if (hit("rope of")) return "rope";
  if (hit("tome", "manual of", "book")) return "spellbook";
  if (hit("immovable rod")) return "staff-wood";

  if (hit("kibble")) return "kibble";
  if (hit("hypo", "stim", "drug", "synthcoke", "speedheal", "trauma")) return "hypo";
  if (hit("agent", "phone", "computer", "cyberdeck", "deck")) return "agent";
  if (hit("drone")) return "drone";
  if (hit("vehicle", "quadra", "panzer", "truck") || /\bcar\b/.test(n)) return "vehicle-car";
  if (hit("cycle", "bike", "yamaha") || /\bbike\b/.test(n)) return "vehicle-bike";
  if (hit("radio", "agent")) return "radio";
  if (hit("backpack", "pack")) return "backpack";
  if (hit("rope")) return "rope";
  if (hit("torch")) return "torch";
  if (hit("lantern", "lamp", "candle")) return "lantern";
  if (hit("lock", "manacle", "handcuff")) return "lock";
  if (hit("thieves", "tinker", "smith", "tool", "kit") && item.kind === "gear") return "tools";
  if (hit("ration", "food", "feed")) return "rations";
  if (hit("tent", "bedroll", "sleep")) return "tent";
  if (hit("spyglass", "binocular")) return "spyglass";
  if (hit("holy symbol", "amulet")) return "holy-symbol";
  if (hit("component pouch", "focus", "arcane")) return "pouch";
  if (hit("spellbook")) return "spellbook";
  if (hit("healer", "medkit", "first aid")) return "healer-kit";
  if (hit("crowbar") && item.kind === "gear") return "crowbar";
  if (hit("hammer") && item.kind === "gear") return "light-hammer";
  if (hit("grappling")) return "rope";
  if (hit("caltrop", "ball bearing")) return "ammo";
  if (hit("clothes", "costume", "fashion")) return "fashion-armor";
  if (hit("mirror", "vial", "bottle", "jug", "barrel", "chest", "basket", "pot", "pan", "soap", "ink", "paper", "parchment", "sealing")) return "pack";
  if (item.kind === "magic") return "crystal";
  if (item.kind === "weapon") return "longsword";
  if (item.kind === "armor") return "leather";
  if (item.kind === "gear") return "backpack";
  return "pack";
}

function kitPic(item, size) {
  const look = lookOf(item);
  return `<span class="kit-thumb kit-pic" draggable="true" data-kit-id="${esc(item.id)}" data-kit-look="${esc(look)}" style="--kit-size:${size}px"><img src="assets/kit/look/${esc(look)}.jpg" alt="" draggable="false" /><span class="kit-svg" hidden>${kitArtSvg(item, size)}</span></span>`;
}

export function kitArtSvg(item, size = 64) {
  const pal = palette(item);
  const mark = pickMark(item);
  const body = MARKS[mark] || MARKS.default;
  const blight = item.world === "blight" || item.kind === "chrome" || item.kind === "quickhack";
  const gid = "k" + String(item.id || "x").replace(/[^a-z0-9_-]/gi, "") + String(size);
  const clip = blight
    ? "M8 4h48l8 8v48l-8 8H8l-8-8V12z"
    : "M8 6h48a6 6 0 0 1 6 6v40a6 6 0 0 1-6 6H8a6 6 0 0 1-6-6V12a6 6 0 0 1 6-6z";
  const inner = blight
    ? "M12 8h40l4 4v40l-4 4H12l-4-4V12z"
    : "M12 10h40a4 4 0 0 1 4 4v36a4 4 0 0 1-4 4H12a4 4 0 0 1-4-4V14a4 4 0 0 1 4-4z";
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="${size}" height="${size}" aria-hidden="true">
    <defs>
      <linearGradient id="${gid}" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="${pal.paper}"/>
        <stop offset="1" stop-color="#050608"/>
      </linearGradient>
    </defs>
    <path d="${clip}" fill="${pal.frame}"/>
    <path d="${inner}" fill="url(#${gid})"/>
    <g fill="none" stroke="${pal.ink}" stroke-width="2.2" stroke-linejoin="round" stroke-linecap="round">${body}</g>
  </svg>`;
}

export function kitThumb(item) {
  return kitPic(item, 48);
}

export function kitHero(item) {
  const look = lookOf(item);
  return `<figure class="kit-art kit-pic" draggable="true" data-kit-id="${esc(item.id)}" data-kit-look="${esc(look)}"><img src="assets/kit/look/${esc(look)}.jpg" alt="" draggable="false" /><span class="kit-svg" hidden>${kitArtSvg(item, 220)}</span></figure>`;
}

let kitArtBound = false;
export function bindKitArt(root = document) {
  if (kitArtBound) return;
  kitArtBound = true;
  root.addEventListener(
    "error",
    (e) => {
      const img = e.target;
      if (!(img instanceof HTMLImageElement)) return;
      const wrap = img.closest(".kit-pic");
      if (!wrap || img.dataset.kitFailed) return;
      img.dataset.kitFailed = "1";
      img.hidden = true;
      const svg = wrap.querySelector(".kit-svg");
      if (svg) svg.hidden = false;
    },
    true
  );
}
