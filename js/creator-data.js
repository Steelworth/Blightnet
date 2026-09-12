export const DND_RACES = [
  ["human", "Human"],
  ["elf", "Elf"],
  ["dwarf", "Dwarf"],
  ["halfling", "Halfling"],
  ["orc", "Orc"],
  ["tiefling", "Tiefling"],
  ["dragonborn", "Dragonborn"],
  ["gnome", "Gnome"],
];

export const BLIGHT_RACES = [
  ["human", "Human"],
  ["elf", "Elf"],
  ["vampire", "Vampire"],
  ["succubus", "Succubus"],
  ["incubus", "Incubus"],
];

export const PRESENTS = [
  ["femme", "Femme"],
  ["masc", "Masc"],
];

export const ETHNICITIES = [
  { id: "porcelain", name: "Porcelain pale", skin: [252, 240, 232] },
  { id: "fair", name: "Fair", skin: [246, 214, 196] },
  { id: "ivory", name: "Ivory", skin: [248, 226, 208] },
  { id: "rosy", name: "Rosy", skin: [232, 186, 170] },
  { id: "beige", name: "Beige", skin: [224, 196, 168] },
  { id: "olive", name: "Olive", skin: [194, 154, 118] },
  { id: "golden", name: "Golden", skin: [210, 164, 112] },
  { id: "tan", name: "Tan", skin: [176, 122, 78] },
  { id: "bronze", name: "Bronze", skin: [154, 96, 58] },
  { id: "brown", name: "Brown", skin: [122, 74, 44] },
  { id: "deep", name: "Deep", skin: [84, 52, 32] },
  { id: "ebony", name: "Ebony", skin: [52, 32, 22] },
  { id: "ashen", name: "Ashen", skin: [196, 184, 176] },
  { id: "ruddy", name: "Ruddy", skin: [196, 118, 96] },
];

export const BODY_TYPES = [
  { id: "slim", name: "Slim", shape: { shoulders: 0.86, chest: 0.84, waist: 0.8, belly: 0.76, hips: 0.86, lThigh: 0.82, rThigh: 0.82, lUpperArm: 0.86, rUpperArm: 0.86, height: 1.08 } },
  { id: "athletic", name: "Athletic", shape: { shoulders: 1.14, chest: 1.04, waist: 0.88, belly: 0.82, hips: 0.96, lThigh: 1.08, rThigh: 1.08, lUpperArm: 1.14, rUpperArm: 1.14, lCalf: 1.08, rCalf: 1.08 } },
  { id: "average", name: "Average", shape: { shoulders: 1, chest: 1, waist: 1, belly: 1, hips: 1 } },
  { id: "curvy", name: "Curvy", shape: { shoulders: 0.98, chest: 1.06, waist: 0.86, belly: 0.94, hips: 1.24, lThigh: 1.16, rThigh: 1.16 } },
  { id: "heavy", name: "Heavy", shape: { shoulders: 1.16, chest: 1.16, waist: 1.22, belly: 1.28, hips: 1.22, lThigh: 1.22, rThigh: 1.22, lUpperArm: 1.14, rUpperArm: 1.14 } },
];

export const BUST_SIZES = [
  { id: "xs", name: "Petite", chest: 0.76 },
  { id: "s", name: "Small", chest: 0.88 },
  { id: "m", name: "Medium", chest: 1 },
  { id: "l", name: "Full", chest: 1.2 },
  { id: "xl", name: "Very full", chest: 1.38 },
];

export const FEMME_FACES = [
  { id: 0, name: "Porcelain oval" },
  { id: 1, name: "Warm round" },
  { id: 2, name: "Olive long" },
  { id: 3, name: "Deep heart" },
  { id: 4, name: "Golden square" },
];

export const BODY_PARTS = [
  { id: "head", name: "Head" },
  { id: "neck", name: "Neck" },
  { id: "shoulders", name: "Shoulders" },
  { id: "chest", name: "Chest" },
  { id: "waist", name: "Waist" },
  { id: "belly", name: "Belly" },
  { id: "hips", name: "Hips" },
  { id: "lUpperArm", name: "L upper arm" },
  { id: "rUpperArm", name: "R upper arm" },
  { id: "lForearm", name: "L forearm" },
  { id: "rForearm", name: "R forearm" },
  { id: "lHand", name: "L hand" },
  { id: "rHand", name: "R hand" },
  { id: "lThigh", name: "L thigh" },
  { id: "rThigh", name: "R thigh" },
  { id: "lCalf", name: "L calf" },
  { id: "rCalf", name: "R calf" },
  { id: "lFoot", name: "L foot" },
  { id: "rFoot", name: "R foot" },
  { id: "height", name: "Height" },
];

export const FINGERS = [
  ["lThumb", "L thumb"],
  ["lIndex", "L index"],
  ["lMiddle", "L middle"],
  ["lRing", "L ring"],
  ["lPinky", "L pinky"],
  ["rThumb", "R thumb"],
  ["rIndex", "R index"],
  ["rMiddle", "R middle"],
  ["rRing", "R ring"],
  ["rPinky", "R pinky"],
];

export const LOSS_SLOTS = [
  ["lArm", "Left arm"],
  ["rArm", "Right arm"],
  ["lLeg", "Left leg"],
  ["rLeg", "Right leg"],
  ["lEye", "Left eye"],
  ["rEye", "Right eye"],
  ["lEar", "Left ear"],
  ["rEar", "Right ear"],
  ...FINGERS,
];

export const CHROME_SLOTS = [
  ["skull", "Skull plate"],
  ["jaw", "Jaw"],
  ["lEye", "Left eye"],
  ["rEye", "Right eye"],
  ["lEar", "Left ear"],
  ["rEar", "Right ear"],
  ["neck", "Neck"],
  ["chest", "Chest"],
  ["spine", "Spine"],
  ["lShoulder", "L shoulder"],
  ["rShoulder", "R shoulder"],
  ["lUpperArm", "L upper arm"],
  ["rUpperArm", "R upper arm"],
  ["lForearm", "L forearm"],
  ["rForearm", "R forearm"],
  ["lHand", "L hand"],
  ["rHand", "R hand"],
  ...FINGERS,
  ["lThigh", "L thigh"],
  ["rThigh", "R thigh"],
  ["lCalf", "L calf"],
  ["rCalf", "R calf"],
  ["lFoot", "L foot"],
  ["rFoot", "R foot"],
];

export const CHROME_STYLES = [
  ["chrome", "Chrome"],
  ["steel", "Black steel"],
  ["brass", "Brass"],
  ["carbon", "Carbon"],
  ["ceramic", "Ceramic"],
  ["ivory", "Ivory machine"],
  ["neon", "Neon vein"],
  ["bone", "Bone-steel"],
];

export const SCAR_KINDS = [
  ["none", "None"],
  ["cut", "Cut"],
  ["suture", "Suture"],
  ["burn", "Burn"],
  ["claw", "Claw"],
  ["surgical", "Surgical"],
  ["shrapnel", "Shrapnel"],
  ["brand", "Brand"],
  ["pox", "Pox"],
  ["keloid", "Keloid"],
];

export const SCAR_PARTS = [
  ["face", "Face"],
  ["neck", "Neck"],
  ["chest", "Chest"],
  ["belly", "Belly"],
  ["lArm", "Left arm"],
  ["rArm", "Right arm"],
  ["lHand", "Left hand"],
  ["rHand", "Right hand"],
  ["lLeg", "Left leg"],
  ["rLeg", "Right leg"],
  ["back", "Back"],
];

export const FACE_COUNT = 20;

export const FACE_NAMES = [
  "Oval", "Heart", "Square", "Long", "Round",
  "Diamond", "Soft", "Sharp", "Broad", "Narrow",
  "High cheek", "Heavy jaw", "Fine", "Weathered", "Young",
  "Hawk", "Soft jaw", "Wide", "Angular", "Full",
];

export const HAIRS = [
  { id: 0, name: "Shaved", len: 0.02, side: 0.04, bang: 0, puff: 0, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 1, afro: 0, locs: 0, rows: 0 },
  { id: 1, name: "Buzz", len: 0.05, side: 0.08, bang: 0, puff: 0.05, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0.8, afro: 0, locs: 0, rows: 0 },
  { id: 2, name: "Crew", len: 0.08, side: 0.06, bang: 0.1, puff: 0.08, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.45, fade: 0.7, afro: 0, locs: 0, rows: 0 },
  { id: 3, name: "Caesar", len: 0.1, side: 0.1, bang: 0.35, puff: 0.1, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0.4, afro: 0, locs: 0, rows: 0 },
  { id: 4, name: "Ivy", len: 0.12, side: 0.12, bang: 0.2, puff: 0.12, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.35, fade: 0.3, afro: 0, locs: 0, rows: 0 },
  { id: 5, name: "Undercut short", len: 0.14, side: 0.02, bang: 0.15, puff: 0.2, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.3, fade: 1, afro: 0, locs: 0, rows: 0 },
  { id: 6, name: "Undercut long", len: 0.42, side: 0.02, bang: 0.25, puff: 0.25, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.28, fade: 1, afro: 0, locs: 0, rows: 0 },
  { id: 7, name: "Side part", len: 0.16, side: 0.14, bang: 0.2, puff: 0.12, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.22, fade: 0.2, afro: 0, locs: 0, rows: 0 },
  { id: 8, name: "Slick back", len: 0.18, side: 0.1, bang: 0, puff: 0.08, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0.1, afro: 0, locs: 0, rows: 0 },
  { id: 9, name: "Pompadour", len: 0.22, side: 0.08, bang: 0, puff: 0.55, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0.4, afro: 0, locs: 0, rows: 0 },
  { id: 10, name: "Quiff", len: 0.2, side: 0.1, bang: 0.1, puff: 0.4, spike: 0.15, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.45, fade: 0.3, afro: 0, locs: 0, rows: 0 },
  { id: 11, name: "Spikes", len: 0.16, side: 0.1, bang: 0, puff: 0.1, spike: 0.85, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0.2, afro: 0, locs: 0, rows: 0 },
  { id: 12, name: "Mohawk", len: 0.28, side: 0, bang: 0, puff: 0.2, spike: 0.4, braid: 0, bun: 0, tail: 0, mohawk: 0.7, part: 0.5, fade: 1, afro: 0, locs: 0, rows: 0 },
  { id: 13, name: "Faux hawk", len: 0.18, side: 0.08, bang: 0, puff: 0.25, spike: 0.2, braid: 0, bun: 0, tail: 0, mohawk: 0.35, part: 0.5, fade: 0.5, afro: 0, locs: 0, rows: 0 },
  { id: 14, name: "High top", len: 0.22, side: 0.04, bang: 0, puff: 0.5, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0.2, part: 0.5, fade: 1, afro: 0.2, locs: 0, rows: 0 },
  { id: 15, name: "Cornrows", len: 0.35, side: 0.12, bang: 0, puff: 0, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0, rows: 7 },
  { id: 16, name: "Twists short", len: 0.16, side: 0.14, bang: 0.1, puff: 0.2, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0.2, afro: 0.15, locs: 0.4, rows: 0 },
  { id: 17, name: "Locs short", len: 0.2, side: 0.16, bang: 0.1, puff: 0.2, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0.8, rows: 0 },
  { id: 18, name: "Locs long", len: 0.7, side: 0.18, bang: 0.15, puff: 0.15, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 1, rows: 0 },
  { id: 19, name: "Afropuff", len: 0.28, side: 0.22, bang: 0, puff: 0.7, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0, afro: 0.85, locs: 0, rows: 0 },
  { id: 20, name: "Afro", len: 0.38, side: 0.32, bang: 0.1, puff: 0.9, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0, afro: 1, locs: 0, rows: 0 },
  { id: 21, name: "Taper fade", len: 0.12, side: 0.06, bang: 0.12, puff: 0.15, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.4, fade: 0.9, afro: 0, locs: 0, rows: 0 },
  { id: 22, name: "Bald fade", len: 0.1, side: 0.02, bang: 0.08, puff: 0.18, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 1, afro: 0.1, locs: 0, rows: 0 },
  { id: 23, name: "Bowl", len: 0.18, side: 0.18, bang: 0.7, puff: 0.15, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 24, name: "Pageboy", len: 0.22, side: 0.2, bang: 0.55, puff: 0.12, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 25, name: "Bob", len: 0.28, side: 0.22, bang: 0.45, puff: 0.18, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.4, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 26, name: "Lob", len: 0.38, side: 0.2, bang: 0.3, puff: 0.16, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.42, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 27, name: "Shoulder wave", len: 0.48, side: 0.22, bang: 0.25, puff: 0.22, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.38, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 28, name: "Long straight", len: 0.78, side: 0.2, bang: 0.2, puff: 0.08, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.35, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 29, name: "Long wavy", len: 0.8, side: 0.24, bang: 0.22, puff: 0.28, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.4, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 30, name: "Long curly", len: 0.72, side: 0.28, bang: 0.2, puff: 0.45, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0, afro: 0.35, locs: 0, rows: 0 },
  { id: 31, name: "Layered long", len: 0.7, side: 0.26, bang: 0.35, puff: 0.3, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.32, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 32, name: "Curtain bangs", len: 0.55, side: 0.2, bang: 0.7, puff: 0.18, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 33, name: "Blunt bangs", len: 0.5, side: 0.2, bang: 0.95, puff: 0.12, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 34, name: "Side bangs", len: 0.52, side: 0.2, bang: 0.55, puff: 0.16, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.18, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 35, name: "Pixie", len: 0.12, side: 0.14, bang: 0.4, puff: 0.2, spike: 0.1, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.4, fade: 0.2, afro: 0, locs: 0, rows: 0 },
  { id: 36, name: "Pixie undercut", len: 0.14, side: 0.02, bang: 0.45, puff: 0.22, spike: 0.15, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.25, fade: 1, afro: 0, locs: 0, rows: 0 },
  { id: 37, name: "Shag", len: 0.4, side: 0.24, bang: 0.5, puff: 0.35, spike: 0.2, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.45, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 38, name: "Mullet", len: 0.55, side: 0.12, bang: 0.2, puff: 0.2, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0.3, afro: 0, locs: 0, rows: 0 },
  { id: 39, name: "Wolf cut", len: 0.5, side: 0.22, bang: 0.55, puff: 0.4, spike: 0.15, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.42, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 40, name: "Ponytail high", len: 0.16, side: 0.1, bang: 0.2, puff: 0.12, spike: 0, braid: 0, bun: 0, tail: 1, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 41, name: "Ponytail low", len: 0.2, side: 0.14, bang: 0.15, puff: 0.1, spike: 0, braid: 0, bun: 0, tail: 0.7, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 42, name: "Twin tails", len: 0.18, side: 0.12, bang: 0.3, puff: 0.12, spike: 0, braid: 0, bun: 0, tail: 2, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 43, name: "Bun high", len: 0.12, side: 0.1, bang: 0.15, puff: 0.1, spike: 0, braid: 0, bun: 1, tail: 0, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 44, name: "Bun low", len: 0.14, side: 0.12, bang: 0.1, puff: 0.08, spike: 0, braid: 0, bun: 0.6, tail: 0, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 45, name: "Messy bun", len: 0.16, side: 0.14, bang: 0.35, puff: 0.3, spike: 0.1, braid: 0, bun: 0.85, tail: 0, mohawk: 0, part: 0.4, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 46, name: "Top knot", len: 0.1, side: 0.06, bang: 0, puff: 0.08, spike: 0, braid: 0, bun: 1.1, tail: 0, mohawk: 0, part: 0.5, fade: 0.4, afro: 0, locs: 0, rows: 0 },
  { id: 47, name: "Braided crown", len: 0.14, side: 0.16, bang: 0.1, puff: 0.08, spike: 0, braid: 2, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0, rows: 2 },
  { id: 48, name: "Single braid", len: 0.22, side: 0.12, bang: 0.15, puff: 0.1, spike: 0, braid: 1, bun: 0, tail: 0.4, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 49, name: "Twin braids", len: 0.2, side: 0.12, bang: 0.2, puff: 0.1, spike: 0, braid: 2, bun: 0, tail: 0.3, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 50, name: "Boxer braids", len: 0.55, side: 0.1, bang: 0.1, puff: 0.05, spike: 0, braid: 2, bun: 0, tail: 0.2, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0, rows: 2 },
  { id: 51, name: "French braid", len: 0.5, side: 0.12, bang: 0.1, puff: 0.08, spike: 0, braid: 1, bun: 0, tail: 0.5, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0, rows: 1 },
  { id: 52, name: "Fishtail", len: 0.62, side: 0.12, bang: 0.15, puff: 0.08, spike: 0, braid: 1.4, bun: 0, tail: 0.45, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 53, name: "Goddess braids", len: 0.6, side: 0.16, bang: 0, puff: 0.1, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0, rows: 5 },
  { id: 54, name: "Bantu knots", len: 0.12, side: 0.14, bang: 0, puff: 0.15, spike: 0, braid: 0, bun: 2, tail: 0, mohawk: 0, part: 0.5, fade: 0, afro: 0.2, locs: 0, rows: 0 },
  { id: 55, name: "Half up", len: 0.58, side: 0.2, bang: 0.25, puff: 0.2, spike: 0, braid: 0, bun: 0.5, tail: 0.3, mohawk: 0, part: 0.4, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 56, name: "Space buns", len: 0.14, side: 0.12, bang: 0.3, puff: 0.15, spike: 0, braid: 0, bun: 2, tail: 0, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 57, name: "Wet look", len: 0.3, side: 0.12, bang: 0.05, puff: 0.02, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 58, name: "Windswept", len: 0.45, side: 0.22, bang: 0.4, puff: 0.35, spike: 0.25, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.6, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 59, name: "Widow peak", len: 0.2, side: 0.12, bang: 0.15, puff: 0.12, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0.2, afro: 0, locs: 0, rows: 0 },
  { id: 60, name: "Long undercut", len: 0.65, side: 0.02, bang: 0.3, puff: 0.22, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.2, fade: 1, afro: 0, locs: 0, rows: 0 },
  { id: 61, name: "Razor part", len: 0.14, side: 0.1, bang: 0.1, puff: 0.15, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.12, fade: 0.6, afro: 0, locs: 0, rows: 0 },
  { id: 62, name: "Dread hawk", len: 0.4, side: 0, bang: 0, puff: 0.15, spike: 0.2, braid: 0, bun: 0, tail: 0, mohawk: 0.8, part: 0.5, fade: 1, afro: 0, locs: 1, rows: 0 },
  { id: 63, name: "Coils", len: 0.32, side: 0.24, bang: 0.15, puff: 0.4, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0, afro: 0.55, locs: 0.3, rows: 0 },
  { id: 64, name: "Rollerset", len: 0.42, side: 0.26, bang: 0.3, puff: 0.5, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.35, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 65, name: "Hime", len: 0.58, side: 0.22, bang: 1, puff: 0.08, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0, rows: 0 },
  { id: 66, name: "One-side shave", len: 0.5, side: 0.08, bang: 0.35, puff: 0.2, spike: 0, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.15, fade: 0.5, afro: 0, locs: 0, rows: 0 },
  { id: 67, name: "Battle braid", len: 0.7, side: 0.1, bang: 0.1, puff: 0.08, spike: 0, braid: 1, bun: 0, tail: 0.7, mohawk: 0, part: 0.5, fade: 0, afro: 0, locs: 0, rows: 1 },
  { id: 68, name: "Wild mane", len: 0.85, side: 0.3, bang: 0.4, puff: 0.6, spike: 0.3, braid: 0, bun: 0, tail: 0, mohawk: 0, part: 0.45, fade: 0, afro: 0.2, locs: 0, rows: 0 },
];

export const EYES = [
  { id: 0, name: "Almond", w: 1, h: 0.42, tilt: 0.12, lid: 0.28 },
  { id: 1, name: "Round", w: 0.92, h: 0.62, tilt: 0, lid: 0.18 },
  { id: 2, name: "Hooded", w: 1, h: 0.36, tilt: 0.05, lid: 0.55 },
  { id: 3, name: "Monolid", w: 1.02, h: 0.32, tilt: 0.08, lid: 0.12 },
  { id: 4, name: "Upturned", w: 1, h: 0.4, tilt: 0.28, lid: 0.22 },
  { id: 5, name: "Downturned", w: 1, h: 0.4, tilt: -0.22, lid: 0.3 },
  { id: 6, name: "Deep set", w: 0.9, h: 0.38, tilt: 0.06, lid: 0.4 },
  { id: 7, name: "Wide set", w: 0.88, h: 0.42, tilt: 0.08, lid: 0.22 },
  { id: 8, name: "Close set", w: 0.86, h: 0.44, tilt: 0.08, lid: 0.22 },
  { id: 9, name: "Large", w: 1.12, h: 0.58, tilt: 0.06, lid: 0.16 },
  { id: 10, name: "Narrow", w: 1.08, h: 0.26, tilt: 0.1, lid: 0.35 },
  { id: 11, name: "Cat", w: 1.1, h: 0.34, tilt: 0.32, lid: 0.2 },
  { id: 12, name: "Doe", w: 1.05, h: 0.55, tilt: -0.06, lid: 0.14 },
  { id: 13, name: "Sleepy", w: 1, h: 0.3, tilt: -0.1, lid: 0.5 },
  { id: 14, name: "Sharp", w: 1.06, h: 0.34, tilt: 0.18, lid: 0.25 },
  { id: 15, name: "Heavy lid", w: 1, h: 0.38, tilt: 0.04, lid: 0.62 },
  { id: 16, name: "Wide awake", w: 1.08, h: 0.6, tilt: 0.04, lid: 0.08 },
  { id: 17, name: "Slanted", w: 1.04, h: 0.36, tilt: 0.38, lid: 0.2 },
  { id: 18, name: "Round wide", w: 1.1, h: 0.66, tilt: 0, lid: 0.1 },
  { id: 19, name: "Fox", w: 1.12, h: 0.32, tilt: 0.34, lid: 0.24 },
  { id: 20, name: "Droopy", w: 0.98, h: 0.4, tilt: -0.28, lid: 0.38 },
  { id: 21, name: "Tight", w: 0.82, h: 0.3, tilt: 0.1, lid: 0.4 },
  { id: 22, name: "Soft", w: 0.96, h: 0.46, tilt: 0.04, lid: 0.2 },
  { id: 23, name: "Hunter", w: 1.04, h: 0.34, tilt: 0.16, lid: 0.32 },
  { id: 24, name: "Owl", w: 1.14, h: 0.7, tilt: 0, lid: 0.12 },
  { id: 25, name: "Snake slit", w: 1.08, h: 0.28, tilt: 0.12, lid: 0.18, slit: 1 },
  { id: 26, name: "Goat", w: 0.9, h: 0.5, tilt: 0, lid: 0.16, slit: 2 },
  { id: 27, name: "Glow", w: 1, h: 0.42, tilt: 0.1, lid: 0.2, glow: 1 },
  { id: 28, name: "Double lid", w: 1, h: 0.44, tilt: 0.08, lid: 0.45 },
  { id: 29, name: "Thin almond", w: 1.06, h: 0.3, tilt: 0.14, lid: 0.22 },
  { id: 30, name: "Heavy round", w: 0.95, h: 0.58, tilt: 0, lid: 0.4 },
  { id: 31, name: "Tapered", w: 1.08, h: 0.36, tilt: 0.2, lid: 0.28 },
  { id: 32, name: "Moon", w: 0.88, h: 0.5, tilt: -0.08, lid: 0.22 },
  { id: 33, name: "Hawk", w: 1.1, h: 0.3, tilt: 0.24, lid: 0.3 },
  { id: 34, name: "Sad", w: 0.98, h: 0.4, tilt: -0.32, lid: 0.34 },
  { id: 35, name: "Fierce", w: 1.08, h: 0.38, tilt: 0.3, lid: 0.26 },
  { id: 36, name: "Innocent", w: 1, h: 0.56, tilt: 0.02, lid: 0.12 },
  { id: 37, name: "Lidded fox", w: 1.1, h: 0.3, tilt: 0.28, lid: 0.48 },
  { id: 38, name: "Sunken", w: 0.86, h: 0.34, tilt: 0.04, lid: 0.5 },
  { id: 39, name: "Bulb", w: 0.9, h: 0.64, tilt: 0, lid: 0.14 },
  { id: 40, name: "Slash", w: 1.16, h: 0.22, tilt: 0.18, lid: 0.2 },
  { id: 41, name: "Peach", w: 0.94, h: 0.48, tilt: 0.06, lid: 0.24 },
  { id: 42, name: "Crystal", w: 1, h: 0.44, tilt: 0.1, lid: 0.18, glow: 1 },
  { id: 43, name: "Wolf", w: 1.06, h: 0.36, tilt: 0.14, lid: 0.22, slit: 1 },
  { id: 44, name: "Doll", w: 1.12, h: 0.62, tilt: 0, lid: 0.1 },
  { id: 45, name: "Weary", w: 1, h: 0.34, tilt: -0.14, lid: 0.52 },
  { id: 46, name: "Keen", w: 1.02, h: 0.32, tilt: 0.22, lid: 0.2 },
  { id: 47, name: "Soft up", w: 0.98, h: 0.44, tilt: 0.16, lid: 0.18 },
  { id: 48, name: "Half moon", w: 1, h: 0.28, tilt: 0, lid: 0.58 },
  { id: 49, name: "Starlit", w: 1.04, h: 0.46, tilt: 0.08, lid: 0.16, glow: 1 },
];

export const HAIR_COLORS = [
  ["#120e0c", "Black"],
  ["#1c1410", "Off-black"],
  ["#2a1c14", "Dark brown"],
  ["#4a2e1a", "Chestnut"],
  ["#6b3a1e", "Auburn"],
  ["#8a3a14", "Copper"],
  ["#b44a18", "Ginger"],
  ["#c46a48", "Strawberry"],
  ["#8a6a3a", "Dirty blonde"],
  ["#c4a05a", "Honey"],
  ["#e8d8a8", "Platinum"],
  ["#f2efe6", "White"],
  ["#c8c8c8", "Silver"],
  ["#8a8a92", "Ash"],
  ["#6a6e78", "Steel"],
  ["#8a1020", "Blood"],
  ["#6a1838", "Wine"],
  ["#5a2088", "Violet"],
  ["#2a2068", "Indigo"],
  ["#0a6868", "Teal"],
  ["#146838", "Emerald"],
  ["#68c818", "Toxic"],
  ["#d4a020", "Gold"],
  ["#e878a0", "Pink"],
  ["#f2c8d4", "Blush"],
  ["#183060", "Blue-black"],
  ["#00d4ff", "Cyan"],
  ["#fcee0a", "Neon gold"],
  ["#ff3a00", "Ember"],
  ["#9ad0c8", "Seafoam"],
  ["#d8c4a0", "Flax"],
  ["#3a2018", "Espresso"],
];

export const EYE_COLORS = [
  ["#3a2414", "Brown"],
  ["#6a3a14", "Amber"],
  ["#6a5a28", "Hazel"],
  ["#2a5a28", "Green"],
  ["#3a6a48", "Sage"],
  ["#4a5a68", "Grey"],
  ["#2a4a78", "Blue"],
  ["#88c8e8", "Ice"],
  ["#5a2a88", "Violet"],
  ["#c4a020", "Gold"],
  ["#8a1020", "Red"],
  ["#111111", "Black"],
  ["#c0c8d0", "Chrome"],
  ["#00d4ff", "Cyan"],
  ["#f2efe6", "White"],
  ["#e878a0", "Pink"],
  ["#68c818", "Toxic"],
  ["#fcee0a", "Neon"],
  ["#00f0a0", "Mint"],
  ["#ff6a00", "Copper"],
];

export const WEAR_SLOTS = [
  {
    id: "top",
    name: "Top",
    items: [
      ["none", "None"],
      ["tank", "Tank"],
      ["shirt", "Shirt"],
      ["tunic", "Tunic"],
      ["blouse", "Blouse"],
      ["vest", "Vest"],
      ["mail", "Mail shirt"],
      ["harness", "Harness"],
      ["sweater", "Sweater"],
      ["robe", "Robe"],
      ["wrap", "Wrap"],
      ["armor", "Chest armor"],
      ["hoodie", "Hoodie"],
      ["jacket", "Jacket"],
    ],
  },
  {
    id: "bottom",
    name: "Bottom",
    items: [
      ["none", "None"],
      ["trousers", "Trousers"],
      ["breeches", "Breeches"],
      ["skirt", "Skirt"],
      ["shorts", "Shorts"],
      ["kilt", "Kilt"],
      ["leather", "Leather pants"],
      ["maillegs", "Mail legs"],
      ["dress", "Dress"],
    ],
  },
  {
    id: "shoes",
    name: "Shoes",
    items: [
      ["none", "None"],
      ["boots", "Boots"],
      ["shoes", "Shoes"],
      ["sandals", "Sandals"],
      ["wraps", "Wraps"],
      ["sabatons", "Sabatons"],
      ["sneakers", "Sneakers"],
    ],
  },
  {
    id: "outer",
    name: "Outer",
    items: [
      ["none", "None"],
      ["cloak", "Cloak"],
      ["coat", "Coat"],
      ["duster", "Duster"],
      ["cape", "Cape"],
      ["tabard", "Tabard"],
    ],
  },
  {
    id: "gloves",
    name: "Gloves",
    items: [
      ["none", "None"],
      ["leather", "Leather"],
      ["plate", "Plate"],
      ["wraps", "Wraps"],
      ["lace", "Fine"],
    ],
  },
  {
    id: "head",
    name: "Head",
    items: [
      ["none", "None"],
      ["hood", "Hood"],
      ["cap", "Cap"],
      ["circlet", "Circlet"],
      ["helm", "Helm"],
      ["bandana", "Bandana"],
      ["goggles", "Goggles"],
    ],
  },
];

export const CANVAS_W = 384;
export const CANVAS_H = 768;

export function defaultCreator() {
  const shape = {};
  for (const p of BODY_PARTS) shape[p.id] = 1;
  const scars = {};
  for (const [id] of SCAR_PARTS) scars[id] = "none";
  const loss = {};
  for (const [id] of LOSS_SLOTS) loss[id] = false;
  const chrome = {};
  for (const [id] of CHROME_SLOTS) chrome[id] = { on: false, style: "chrome", color: "#c8d0d8" };
  return {
    race: "human",
    present: "femme",
    ethnicity: "porcelain",
    view: "front",
    face: 0,
    bodyType: "average",
    bust: "m",
    skin: 0,
    shape,
    hair: { style: 25, color: "#1c1410" },
    eyes: { shape: 0, color: "#3a2414", colorR: "#3a2414" },
    wear: {
      top: "shirt",
      topOn: true,
      topColor: "#1a1a1a",
      bottom: "trousers",
      bottomOn: true,
      bottomColor: "#1a1a1a",
      shoes: "boots",
      shoesOn: true,
      shoesColor: "#111111",
      outer: "none",
      outerOn: false,
      outerColor: "#333333",
      gloves: "none",
      glovesOn: false,
      glovesColor: "#222222",
      head: "none",
      headOn: false,
      headColor: "#222222",
    },
    scars,
    loss,
    chrome,
  };
}

export function mergeCreator(raw) {
  const base = defaultCreator();
  if (!raw || typeof raw !== "object") return base;
  const next = { ...base, ...raw };
  next.shape = { ...base.shape, ...(raw.shape || {}) };
  next.hair = { ...base.hair, ...(raw.hair || {}) };
  next.eyes = { ...base.eyes, ...(raw.eyes || {}) };
  next.wear = { ...base.wear, ...(raw.wear || {}) };
  next.scars = { ...base.scars, ...(raw.scars || {}) };
  next.loss = { ...base.loss, ...(raw.loss || {}) };
  next.chrome = { ...base.chrome };
  if (raw.chrome && typeof raw.chrome === "object") {
    for (const [id, val] of Object.entries(raw.chrome)) {
      if (val && typeof val === "object") next.chrome[id] = { ...base.chrome[id], ...val };
    }
  }
  if (Array.isArray(raw.augs) && raw.augs.length) {
    if (raw.augs.includes("eyes")) {
      next.chrome.lEye = { on: true, style: "neon", color: "#00f0ff" };
      next.chrome.rEye = { on: true, style: "neon", color: "#00f0ff" };
    }
    if (raw.augs.includes("jack")) next.chrome.neck = { on: true, style: "chrome", color: "#c8d0d8" };
    if (raw.augs.includes("arm")) next.chrome.lForearm = { on: true, style: "chrome", color: "#c8d0d8" };
    if (raw.augs.includes("dermal")) next.chrome.chest = { on: true, style: "steel", color: "#8890a0" };
    if (raw.augs.includes("speed")) next.chrome.spine = { on: true, style: "neon", color: "#fcee0a" };
  }
  return next;
}
