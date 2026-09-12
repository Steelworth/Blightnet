export const SETTINGS = [
  { id: "castle", name: "Highlands", icon: "mountain", indoor: false },
  { id: "tavern", name: "Tavern", icon: "mug", indoor: true },
  { id: "dungeon", name: "Dungeon", icon: "dungeon", indoor: true },
  { id: "crypt", name: "Crypt", icon: "tomb", indoor: true },
  { id: "city", name: "City", icon: "square", indoor: false },
  { id: "training", name: "Training Grounds", icon: "swords", indoor: false },
  { id: "harbor", name: "Harbor", icon: "ship", indoor: false },
  { id: "beach", name: "Beach", icon: "wave", indoor: false },
  { id: "underwater", name: "Underwater", icon: "fish", indoor: true },
  { id: "court", name: "Royal Court", icon: "crown", indoor: true },
  { id: "forest", name: "Forest", icon: "trees", indoor: false },
  { id: "swamp", name: "Swamp", icon: "swamp", indoor: false },
  { id: "fields", name: "Fields", icon: "farm", indoor: false },
  { id: "temple", name: "Temple", icon: "temple", indoor: true },
  { id: "cavern", name: "Cavern", icon: "cave", indoor: true },
  { id: "desert", name: "Desert", icon: "dune", indoor: false },
  { id: "jungle", name: "Jungle", icon: "jungle", indoor: false },
  { id: "camp", name: "Camp", icon: "fire", indoor: false },
  { id: "forge", name: "Forge", icon: "anvil", indoor: true },
  { id: "library", name: "Library", icon: "book", indoor: true },
];

export const BLIGHT_SETTINGS = [
  { id: "nightcity", name: "Night City", icon: "square", indoor: false },
  { id: "city-center", name: "City Center", icon: "square", indoor: false },
  { id: "corpo-plaza", name: "Corpo Plaza", icon: "crown", indoor: false },
  { id: "downtown", name: "Downtown", icon: "square", indoor: false },
  { id: "watson", name: "Watson", icon: "square", indoor: false },
  { id: "little-china", name: "Little China", icon: "temple", indoor: false },
  { id: "kabuki", name: "Kabuki", icon: "market", indoor: false },
  { id: "northside", name: "Northside", icon: "anvil", indoor: false },
  { id: "arasaka-waterfront", name: "Arasaka Waterfront", icon: "ship", indoor: false },
  { id: "westbrook", name: "Westbrook", icon: "crown", indoor: false },
  { id: "japantown", name: "Japantown", icon: "temple", indoor: false },
  { id: "charter-hill", name: "Charter Hill", icon: "mountain", indoor: false },
  { id: "north-oak", name: "North Oak", icon: "crown", indoor: false },
  { id: "heywood", name: "Heywood", icon: "square", indoor: false },
  { id: "the-glen", name: "The Glen", icon: "trees", indoor: false },
  { id: "vista-del-rey", name: "Vista del Rey", icon: "square", indoor: false },
  { id: "wellsprings", name: "Wellsprings", icon: "wave", indoor: false },
  { id: "santo-domingo", name: "Santo Domingo", icon: "anvil", indoor: false },
  { id: "arroyo", name: "Arroyo", icon: "anvil", indoor: false },
  { id: "rancho-coronado", name: "Rancho Coronado", icon: "farm", indoor: false },
  { id: "pacifica", name: "Pacifica", icon: "wave", indoor: false },
  { id: "coastview", name: "Coastview", icon: "wave", indoor: false },
  { id: "west-wind", name: "West Wind Estate", icon: "fire", indoor: false },
  { id: "dogtown", name: "Dogtown", icon: "dungeon", indoor: false },
  { id: "badlands", name: "Badlands", icon: "dune", indoor: false },
  { id: "luna", name: "The Moon", icon: "moon", indoor: false },
  { id: "casino", name: "Casino", icon: "crown", indoor: true },
];

const BLIGHT_ALIAS = {
  castle: "nightcity",
  tavern: "japantown",
  dungeon: "northside",
  crypt: "city-center",
  city: "downtown",
  training: "corpo-plaza",
  harbor: "arasaka-waterfront",
  beach: "pacifica",
  underwater: "coastview",
  court: "north-oak",
  forest: "the-glen",
  swamp: "arroyo",
  fields: "rancho-coronado",
  temple: "little-china",
  cavern: "northside",
  desert: "badlands",
  jungle: "pacifica",
  camp: "watson",
  forge: "arroyo",
  library: "downtown",
};

const BY_ID = new Map(SETTINGS.map((s) => [s.id, s]));
const BY_BLIGHT = new Map(BLIGHT_SETTINGS.map((s) => [s.id, s]));

export function worldOf() {
  return document.documentElement?.dataset?.theme === "blight" ? "blight" : "hearthsong";
}

export function settingById(id) {
  if (worldOf() === "blight") {
    if (BY_BLIGHT.has(id)) return BY_BLIGHT.get(id);
    const mapped = BLIGHT_ALIAS[id];
    if (mapped && BY_BLIGHT.has(mapped)) return BY_BLIGHT.get(mapped);
    return BLIGHT_SETTINGS[0];
  }
  return BY_ID.get(id) || SETTINGS[0];
}

export function settingsOf() {
  return worldOf() === "blight" ? BLIGHT_SETTINGS.slice() : SETTINGS.slice();
}

export function settingSrc(id, hour) {
  const setting = settingById(id);
  const time = ["morning", "day", "evening", "night"].includes(hour) ? hour : "day";
  const folder = worldOf() === "blight" ? "assets/places-blight" : "assets/places";
  return `${folder}/${setting.id}-${time}.jpg`;
}

export const SCENE_SETTING = {
  tavern: "tavern",
  packed: "tavern",
  hearthside: "tavern",
  "drizzle-lane": "tavern",
  forest: "forest",
  "wet-woods": "forest",
  "woods-night": "forest",
  glade: "forest",
  hunt: "forest",
  shrine: "forest",
  camp: "camp",
  watch: "camp",
  ride: "camp",
  crawl: "dungeon",
  thieves: "dungeon",
  under: "dungeon",
  crypt: "crypt",
  ghosts: "crypt",
  city: "city",
  square: "city",
  stealth: "city",
  festival: "city",
  goblin: "city",
  clocktower: "city",
  battle: "training",
  battles: "training",
  warroad: "training",
  pyre: "training",
  sea: "harbor",
  docks: "harbor",
  leeward: "harbor",
  strand: "beach",
  "night-shore": "beach",
  depths: "underwater",
  wreck: "underwater",
  court: "court",
  mire: "swamp",
  moor: "swamp",
  caravan: "desert",
  smithy: "forge",
  village: "fields",
  barnyard: "fields",
  dawn: "fields",
  sanctum: "temple",
  bells: "temple",
  lair: "cavern",
  beasthouse: "cavern",
  winter: "castle",
  eyrie: "castle",
  storm: "castle",
  hailstorm: "castle",
  windwall: "castle",
  wilds: "jungle",
  stacks: "library",
  tower: "library",
  catacombs: "crypt",
  lily: "fields",
  autumn: "forest",
  scare: "forest",
  dunes: "desert",
  fairground: "city",
  "canopy-storm": "jungle",
  firehall: "court",
  chase: "forest",
  rest: "camp",
  "mystery-hall": "library",
  rite: "crypt",
  "chamber-walk": "dungeon",
  heath: "fields",
  call: "castle",
  threshold: "tavern",
  "killing-moon": "luna",
  earthrise: "luna",
  copernicus: "luna",
  goldwire: "casino",
  "high-roller": "casino",
};
