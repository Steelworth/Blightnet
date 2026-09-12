/** 5e SRD classes + Cyberpunk RED advancement. Feature text is original table notes. */

export const XP_TO_LEVEL = [
  0, 0, 300, 900, 2700, 6500, 14000, 23000, 34000, 48000, 64000, 85000, 100000, 120000, 140000, 165000, 195000, 225000, 265000, 305000, 355000,
];

const FULL = [
  [2, 0, 0, 0, 0, 0, 0, 0, 0],
  [3, 0, 0, 0, 0, 0, 0, 0, 0],
  [4, 2, 0, 0, 0, 0, 0, 0, 0],
  [4, 3, 0, 0, 0, 0, 0, 0, 0],
  [4, 3, 2, 0, 0, 0, 0, 0, 0],
  [4, 3, 3, 0, 0, 0, 0, 0, 0],
  [4, 3, 3, 1, 0, 0, 0, 0, 0],
  [4, 3, 3, 2, 0, 0, 0, 0, 0],
  [4, 3, 3, 3, 1, 0, 0, 0, 0],
  [4, 3, 3, 3, 2, 0, 0, 0, 0],
  [4, 3, 3, 3, 2, 1, 0, 0, 0],
  [4, 3, 3, 3, 2, 1, 0, 0, 0],
  [4, 3, 3, 3, 2, 1, 1, 0, 0],
  [4, 3, 3, 3, 2, 1, 1, 0, 0],
  [4, 3, 3, 3, 2, 1, 1, 1, 0],
  [4, 3, 3, 3, 2, 1, 1, 1, 0],
  [4, 3, 3, 3, 2, 1, 1, 1, 1],
  [4, 3, 3, 3, 3, 1, 1, 1, 1],
  [4, 3, 3, 3, 3, 2, 1, 1, 1],
  [4, 3, 3, 3, 3, 2, 2, 1, 1],
];

const HALF = [
  [0, 0, 0, 0, 0, 0, 0, 0, 0],
  [2, 0, 0, 0, 0, 0, 0, 0, 0],
  [3, 0, 0, 0, 0, 0, 0, 0, 0],
  [3, 0, 0, 0, 0, 0, 0, 0, 0],
  [4, 2, 0, 0, 0, 0, 0, 0, 0],
  [4, 2, 0, 0, 0, 0, 0, 0, 0],
  [4, 3, 0, 0, 0, 0, 0, 0, 0],
  [4, 3, 0, 0, 0, 0, 0, 0, 0],
  [4, 3, 2, 0, 0, 0, 0, 0, 0],
  [4, 3, 2, 0, 0, 0, 0, 0, 0],
  [4, 3, 3, 0, 0, 0, 0, 0, 0],
  [4, 3, 3, 0, 0, 0, 0, 0, 0],
  [4, 3, 3, 1, 0, 0, 0, 0, 0],
  [4, 3, 3, 1, 0, 0, 0, 0, 0],
  [4, 3, 3, 2, 0, 0, 0, 0, 0],
  [4, 3, 3, 2, 0, 0, 0, 0, 0],
  [4, 3, 3, 3, 1, 0, 0, 0, 0],
  [4, 3, 3, 3, 1, 0, 0, 0, 0],
  [4, 3, 3, 3, 2, 0, 0, 0, 0],
  [4, 3, 3, 3, 2, 0, 0, 0, 0],
];

const ART = [[2, 0, 0, 0, 0, 0, 0, 0, 0], ...HALF.slice(1)];

function f(lv, text) {
  return { lv, text };
}

function sub(id, name, features) {
  return { id, name, features };
}

export const CLASSES = [
  {
    id: "artificer",
    name: "Artificer",
    hd: 8,
    saves: ["con", "int"],
    skills: 2,
    skillList: ["arcana", "history", "investigation", "medicine", "nature", "perception", "sleightOfHand"],
    armor: "Light and medium armor, shields. Simple weapons, firearms if the table uses them. Thieves' tools, tinker's tools.",
    caster: "art",
    spell: "int",
    subclassLv: 3,
    asi: [4, 8, 12, 16, 19],
    features: [
      f(1, "Magical Tinkering. Infuse a tiny object with a cantrip-like trick. Spellcasting (INT)."),
      f(2, "Infuse Item. Imbue nonmagical objects; attune-ready replicas of common magic."),
      f(3, "Artificer Specialist. The Right Tool for the Job — produce a set of artisan's tools."),
      f(5, "Artificer Specialist feature."),
      f(6, "Tool Expertise. Double proficiency with tools you are proficient with."),
      f(7, "Flash of Genius. Add INT mod to a check or save, uses = INT mod / rest."),
      f(10, "Magic Item Adept. Attune 4 items. Craft common/uncommon faster."),
      f(11, "Spell-Storing Item. Store an artificer spell of 1st or 2nd in a held object."),
      f(14, "Magic Item Savant. Attune 5 items. Ignore class restrictions on attunement."),
      f(18, "Magic Item Master. Attune 6 items."),
      f(20, "Soul of Artifice. +1 to saves per attuned item. Cheat death once per rest at 1 HP."),
    ],
    subclasses: [
      sub("alchemist", "Alchemist", [f(3, "Experimental Elixir. Brew potions on a rest. Bonus cantrip: acid splash or similar."), f(5, "Alchemical Savant. Spells that deal acid/fire/necrotic/poison or restore HP gain INT mod."), f(9, "Restorative Reagents. Elixirs grant temp HP. Lesser restoration a few times a day."), f(15, "Chemical Mastery. Resist acid and poison. Greater restoration / heal once per rest.")]),
      sub("artillerist", "Artillerist", [f(3, "Eldritch Cannon. Tiny turret: flamethrower, force ballista, or protector."), f(5, "Arcane Firearm. A wand/rod/staff as a spell focus; extra 1d8 on a damage spell."), f(9, "Explosive Cannon. Bigger turret, detonate it."), f(15, "Fortified Position. Half cover near the cannon. Two cannons.")]),
      sub("battlesmith", "Battle Smith", [f(3, "Steel Defender. Companion construct. Battle Ready — extra attack with magic weapons via INT."), f(5, "Extra Attack."), f(9, "Arcane Jolt. On a magic-weapon hit or defender hit, extra force or heal."), f(15, "Improved Defender. More HP, Arcane Jolt upgrades.")]),
    ],
  },
  {
    id: "barbarian",
    name: "Barbarian",
    hd: 12,
    saves: ["str", "con"],
    skills: 2,
    skillList: ["animalHandling", "athletics", "intimidation", "nature", "perception", "survival"],
    armor: "Light and medium armor, shields. Simple and martial weapons.",
    caster: "",
    spell: "",
    subclassLv: 3,
    asi: [4, 8, 12, 16, 19],
    features: [
      f(1, "Rage (2/day). Advantage on STR checks and saves; bonus rage damage on STR melee; resistance to bludgeoning, piercing, slashing. Unarmored Defense: 10 + DEX + CON."),
      f(2, "Reckless Attack. Advantage on STR melee; attacks against you have advantage. Danger Sense. Advantage on DEX saves you can see."),
      f(3, "Primal Path. Rage 3/day."),
      f(5, "Extra Attack. Fast Movement. +10 ft while not in heavy armor."),
      f(6, "Path feature. Rage 4/day."),
      f(7, "Feral Instinct. Advantage on initiative. Enter rage if surprised."),
      f(9, "Brutal Critical. Extra weapon die on a melee crit."),
      f(11, "Relentless Rage. If you drop to 0 while raging, CON save to stay at 1 HP."),
      f(12, "Rage 5/day."),
      f(13, "Brutal Critical (2 extra dice)."),
      f(15, "Persistent Rage. Rage lasts until you drop it or fall unconscious."),
      f(17, "Brutal Critical (3 extra dice). Rage 6/day."),
      f(18, "Indomitable Might. STR checks use your STR score if the roll is lower."),
      f(20, "Primal Champion. STR and CON +4 (max 24). Unlimited rage."),
    ],
    subclasses: [
      sub("berserker", "Path of the Berserker", [f(3, "Frenzy. While raging, a bonus melee attack each turn; one exhaustion when the rage ends."), f(6, "Mindless Rage. Can't be charmed or frightened while raging."), f(10, "Intimidating Presence. WIS save or frightened."), f(14, "Retaliation. When hit in melee, melee attack back as a reaction.")]),
    ],
  },
  {
    id: "bard",
    name: "Bard",
    hd: 8,
    saves: ["dex", "cha"],
    skills: 3,
    skillList: ["acrobatics", "animalHandling", "arcana", "athletics", "deception", "history", "insight", "intimidation", "investigation", "medicine", "nature", "perception", "performance", "persuasion", "religion", "sleightOfHand", "stealth", "survival"],
    armor: "Light armor. Simple weapons, hand crossbows, longswords, rapiers, shortswords.",
    caster: "full",
    spell: "cha",
    subclassLv: 3,
    asi: [4, 8, 12, 16, 19],
    features: [
      f(1, "Spellcasting (CHA). Bardic Inspiration (d6), uses = CHA mod / rest."),
      f(2, "Jack of All Trades. Half proficiency on checks that don't already add it. Song of Rest. Extra healing die on a short rest."),
      f(3, "Bard College. Expertise in two skills."),
      f(5, "Bardic Inspiration d8. Font of Inspiration. Inspiration returns on a short rest."),
      f(6, "Countercharm. Performance to give advantage vs charm and fear. College feature."),
      f(10, "Bardic Inspiration d10. Expertise in two more skills. Magical Secrets — two spells from any list."),
      f(14, "Magical Secrets. College feature."),
      f(15, "Bardic Inspiration d12."),
      f(18, "Magical Secrets."),
      f(20, "Superior Inspiration. If you roll initiative with none left, gain one."),
    ],
    subclasses: [
      sub("lore", "College of Lore", [f(3, "Bonus Proficiencies (3 skills). Cutting Words. Subtract inspiration from an enemy attack, check, or damage."), f(6, "Additional Magical Secrets."), f(14, "Peerless Skill. Add inspiration to your own check.")]),
    ],
  },
  {
    id: "cleric",
    name: "Cleric",
    hd: 8,
    saves: ["wis", "cha"],
    skills: 2,
    skillList: ["history", "insight", "medicine", "persuasion", "religion"],
    armor: "Light and medium armor, shields. Simple weapons.",
    caster: "full",
    spell: "wis",
    subclassLv: 1,
    asi: [4, 8, 12, 16, 19],
    features: [
      f(1, "Spellcasting (WIS). Divine Domain."),
      f(2, "Channel Divinity (1/rest). Turn Undead — WIS save or undead flee."),
      f(5, "Destroy Undead (CR 1/2)."),
      f(6, "Channel Divinity (2/rest). Domain feature."),
      f(8, "Destroy Undead (CR 1). Domain feature."),
      f(10, "Divine Intervention. 1% × level chance the god acts."),
      f(11, "Destroy Undead (CR 2)."),
      f(14, "Destroy Undead (CR 3)."),
      f(17, "Destroy Undead (CR 4). Domain feature."),
      f(18, "Channel Divinity (3/rest)."),
      f(20, "Divine Intervention succeeds automatically."),
    ],
    subclasses: [
      sub("knowledge", "Knowledge Domain", [f(1, "Blessings of Knowledge. Extra languages and expertise in two INT skills. Domain spells."), f(2, "Channel: Knowledge of the Ages. Proficiency in one skill or tool for 10 minutes."), f(6, "Channel: Read Thoughts."), f(8, "Potent Spellcasting. Add WIS to cleric cantrip damage."), f(17, "Visions of the Past.")]),
      sub("life", "Life Domain", [f(1, "Disciple of Life. Healing spells of 1st+ restore extra 2 + spell level HP. Heavy armor. Domain spells."), f(2, "Channel: Preserve Life. Heal 5 × cleric level, split among creatures within 30 ft, up to half their max."), f(6, "Blessed Healer. When you heal others, you regain 2 + spell level HP."), f(8, "Divine Strike. Extra 1d8 radiant once per turn."), f(14, "Divine Strike 2d8."), f(17, "Supreme Healing. Maximize dice on healing spells.")]),
      sub("light", "Light Domain", [f(1, "Warding Flare. Impose disadvantage on an attack. Light cantrip. Domain spells."), f(2, "Channel: Radiance of the Dawn. Dispel dark and burn foes."), f(6, "Improved Flare. Flare for an ally."), f(8, "Potent Spellcasting."), f(17, "Corona of Light. Aura of sunlight; disadvantage on saves vs your fire/radiant spells.")]),
      sub("nature", "Nature Domain", [f(1, "Acolyte of Nature. One druid cantrip and a nature skill. Heavy armor. Domain spells."), f(2, "Channel: Charm Animals and Plants."), f(6, "Dampen Elements. Resistance to one energy type as a reaction."), f(8, "Divine Strike (elemental)."), f(17, "Master of Nature. Command animals and plants.")]),
      sub("tempest", "Tempest Domain", [f(1, "Wrath of the Storm. Reaction thunder or lightning when hit. Heavy armor, martial weapons."), f(2, "Channel: Destructive Wrath. Maximize thunder or lightning dice."), f(6, "Thunderbolt Strike. Large or smaller shoved 10 ft on a lightning hit."), f(8, "Divine Strike (thunder)."), f(17, "Stormborn. Fly in open air when it storms.")]),
      sub("trickery", "Trickery Domain", [f(1, "Blessing of the Trickster. Advantage on Stealth for an ally. Domain spells."), f(2, "Channel: Invoke Duplicity. Illusion double."), f(6, "Channel: Cloak of Shadows. Invisible until your next turn."), f(8, "Divine Strike (poison)."), f(17, "Improved Duplicity. Four doubles.")]),
      sub("war", "War Domain", [f(1, "War Priest. Bonus weapon attack, uses = WIS mod. Heavy armor, martial weapons."), f(2, "Channel: Guided Strike. +10 to an attack."), f(6, "Channel: War God's Blessing. +10 to an ally's attack as a reaction."), f(8, "Divine Strike."), f(17, "Avatar of Battle. Resistance to nonmagical weapons.")]),
    ],
  },
  {
    id: "druid",
    name: "Druid",
    hd: 8,
    saves: ["int", "wis"],
    skills: 2,
    skillList: ["arcana", "animalHandling", "insight", "medicine", "nature", "perception", "religion", "survival"],
    armor: "Light and medium armor (nonmetal), shields (nonmetal). Clubs, daggers, darts, javelins, maces, quarterstaffs, scimitars, sickles, slings, spears.",
    caster: "full",
    spell: "wis",
    subclassLv: 2,
    asi: [4, 8, 12, 16, 19],
    features: [
      f(1, "Spellcasting (WIS). Druidic. Druidcraft."),
      f(2, "Wild Shape. Beast CR 1/4, no fly/swim. 2/rest. Druid Circle."),
      f(4, "Wild Shape CR 1/2, swim ok."),
      f(6, "Circle feature."),
      f(8, "Wild Shape CR 1, fly ok."),
      f(10, "Circle feature."),
      f(14, "Circle feature."),
      f(18, "Timeless Body. Beast Spells — cast while wild shaped."),
      f(20, "Archdruid. Unlimited Wild Shape. Ignore verbal/somatic and non-costly materials."),
    ],
    subclasses: [
      sub("land", "Circle of the Land", [f(2, "Natural Recovery. Regain spell slots on a short rest. Bonus cantrip. Circle spells from your land."), f(6, "Land's Stride. Nonmagical difficult terrain doesn't slow you; advantage vs plants that restrain."), f(10, "Nature's Ward. Immune to charm/fear from elementals and fey. Immune to poison and disease."), f(14, "Nature's Sanctuary. Beasts and plants must save to attack you.")]),
    ],
  },
  {
    id: "fighter",
    name: "Fighter",
    hd: 10,
    saves: ["str", "con"],
    skills: 2,
    skillList: ["acrobatics", "animalHandling", "athletics", "history", "insight", "intimidation", "perception", "survival"],
    armor: "All armor, shields. Simple and martial weapons.",
    caster: "",
    spell: "",
    subclassLv: 3,
    asi: [4, 6, 8, 12, 14, 16, 19],
    features: [
      f(1, "Fighting Style. Second Wind. Bonus action 1d10 + fighter level HP, 1/rest."),
      f(2, "Action Surge. Extra action, 1/rest."),
      f(3, "Martial Archetype."),
      f(5, "Extra Attack."),
      f(9, "Indomitable. Reroll a save, 1/rest."),
      f(11, "Extra Attack (2)."),
      f(13, "Indomitable 2/rest."),
      f(17, "Action Surge 2/rest. Indomitable 3/rest."),
      f(20, "Extra Attack (3)."),
    ],
    subclasses: [
      sub("champion", "Champion", [f(3, "Improved Critical. Crit on 19–20."), f(7, "Remarkable Athlete. Add half proficiency to STR/DEX/CON checks that don't already get it. Running long jump +STR ft."), f(10, "Additional Fighting Style."), f(15, "Superior Critical. Crit on 18–20."), f(18, "Survivor. If below half HP at the start of your turn, regain 5 + CON HP.")]),
    ],
  },
  {
    id: "monk",
    name: "Monk",
    hd: 8,
    saves: ["str", "dex"],
    skills: 2,
    skillList: ["acrobatics", "athletics", "history", "insight", "religion", "stealth"],
    armor: "No armor. Simple weapons and shortswords.",
    caster: "",
    spell: "",
    subclassLv: 3,
    asi: [4, 8, 12, 16, 19],
    features: [
      f(1, "Unarmored Defense: 10 + DEX + WIS. Martial Arts. Dex monk weapons and unarmed; bonus unarmed strike."),
      f(2, "Ki (monk level points). Flurry of Blows, Patient Defense, Step of the Wind. Unarmored Movement +10 ft."),
      f(3, "Monastic Tradition. Deflect Missiles."),
      f(4, "Slow Fall."),
      f(5, "Extra Attack. Stunning Strike."),
      f(6, "Ki-Empowered Strikes. Magical unarmed. Movement +15 ft. Tradition feature."),
      f(7, "Evasion. Stillness of Mind."),
      f(9, "Unarmored Movement improved — walk walls and water on your turn."),
      f(10, "Purity of Body. Immune to disease and poison. Movement +20 ft."),
      f(13, "Tongue of the Sun and Moon. Any language."),
      f(14, "Diamond Soul. Proficiency in all saves. Spend ki to reroll a save. Movement +25 ft."),
      f(15, "Timeless Body."),
      f(18, "Empty Body. Invisibility or astral projection for ki. Movement +30 ft."),
      f(20, "Perfect Self. If you start a fight with 0 ki, gain 4."),
    ],
    subclasses: [
      sub("open-hand", "Way of the Open Hand", [f(3, "Open Hand Technique. Flurry can knock prone, push, or deny reactions."), f(6, "Wholeness of Body. Heal 3 × monk level, 1/rest."), f(11, "Tranquility. Sanctuary until you attack."), f(17, "Quivering Palm. On a hit, a later save-or-drop-to-0.")]),
    ],
  },
  {
    id: "paladin",
    name: "Paladin",
    hd: 10,
    saves: ["wis", "cha"],
    skills: 2,
    skillList: ["athletics", "insight", "intimidation", "medicine", "persuasion", "religion"],
    armor: "All armor, shields. Simple and martial weapons.",
    caster: "half",
    spell: "cha",
    subclassLv: 3,
    asi: [4, 8, 12, 16, 19],
    features: [
      f(1, "Divine Sense. Lay on Hands. Pool = 5 × paladin level HP."),
      f(2, "Fighting Style. Spellcasting (CHA). Divine Smite. Spend a slot on a melee hit for 2d8 radiant + 1d8 per slot above 1st (max 5d8, +1d8 vs undead/fiend)."),
      f(3, "Divine Health. Immune to disease. Sacred Oath. Channel Divinity."),
      f(5, "Extra Attack."),
      f(6, "Aura of Protection. You and allies within 10 ft add CHA to saves."),
      f(10, "Aura of Courage. You and allies within 10 ft can't be frightened."),
      f(11, "Improved Divine Smite. Extra 1d8 radiant on every melee hit."),
      f(14, "Cleansing Touch. End a spell on yourself or an ally, uses = CHA mod."),
      f(18, "Auras 30 ft."),
      f(20, "Oath feature — a once-per-rest transformation."),
    ],
    subclasses: [
      sub("devotion", "Oath of Devotion", [f(3, "Channel: Sacred Weapon (+CHA to attack, shine). Turn the Unholy."), f(7, "Aura of Devotion. You and allies within 10 ft can't be charmed."), f(15, "Purity of Spirit. Protection from evil and good, always on."), f(20, "Holy Nimbus. Aura of sunlight, extra radiant, saves vs fiend/undead spells with advantage.")]),
    ],
  },
  {
    id: "ranger",
    name: "Ranger",
    hd: 10,
    saves: ["str", "dex"],
    skills: 3,
    skillList: ["animalHandling", "athletics", "insight", "investigation", "nature", "perception", "stealth", "survival"],
    armor: "Light and medium armor, shields. Simple and martial weapons.",
    caster: "half",
    spell: "wis",
    subclassLv: 3,
    asi: [4, 8, 12, 16, 19],
    features: [
      f(1, "Favored Enemy. Natural Explorer."),
      f(2, "Fighting Style. Spellcasting (WIS)."),
      f(3, "Ranger Archetype. Primeval Awareness."),
      f(5, "Extra Attack."),
      f(8, "Land's Stride."),
      f(10, "Hide in Plain Sight. Natural Explorer extra terrain."),
      f(14, "Vanish. Hide as a bonus action. Can't be tracked except by magic. Favored Enemy extra."),
      f(18, "Feral Senses. No disadvantage vs invisible you can hear. Pinpoint hidden within 30 ft."),
      f(20, "Foe Slayer. Add WIS to attack or damage vs a favored enemy, once per turn."),
    ],
    subclasses: [
      sub("hunter", "Hunter", [f(3, "Hunter's Prey. Colossus Slayer (+1d8 if the target is already hurt), Giant Killer, or Horde Breaker."), f(7, "Defensive Tactics. Escape the Horde, Multiattack Defense, or Steel Will."), f(11, "Multiattack. Volley or Whirlwind Attack."), f(15, "Superior Hunter's Defense. Evasion, Stand Against the Tide, or Uncanny Dodge.")]),
    ],
  },
  {
    id: "rogue",
    name: "Rogue",
    hd: 8,
    saves: ["dex", "int"],
    skills: 4,
    skillList: ["acrobatics", "athletics", "deception", "insight", "intimidation", "investigation", "perception", "performance", "persuasion", "sleightOfHand", "stealth"],
    armor: "Light armor. Simple weapons, hand crossbows, longswords, rapiers, shortswords. Thieves' tools.",
    caster: "",
    spell: "",
    subclassLv: 3,
    asi: [4, 8, 10, 12, 16, 19],
    features: [
      f(1, "Expertise (2). Sneak Attack 1d6. Thieves' Cant."),
      f(2, "Cunning Action. Dash, Disengage, or Hide as a bonus action."),
      f(3, "Roguish Archetype. Sneak Attack 2d6."),
      f(5, "Uncanny Dodge. Halve a seen attack as a reaction. Sneak Attack 3d6."),
      f(6, "Expertise (2 more)."),
      f(7, "Evasion. Sneak Attack 4d6."),
      f(9, "Archetype. Sneak Attack 5d6."),
      f(11, "Reliable Talent. Can't roll below 10 on proficient checks. Sneak Attack 6d6."),
      f(13, "Archetype. Sneak Attack 7d6."),
      f(14, "Blindsense. 10 ft."),
      f(15, "Slippery Mind. Proficiency in WIS saves. Sneak Attack 8d6."),
      f(17, "Archetype. Sneak Attack 9d6."),
      f(18, "Elusive. No attack has advantage against you while you aren't incapacitated."),
      f(20, "Stroke of Luck. Turn a miss into a hit, or a failed check into a 20, 1/rest. Sneak Attack 10d6."),
    ],
    subclasses: [
      sub("thief", "Thief", [f(3, "Fast Hands. Sleight of Hand, thieves' tools, or Use an Object as a bonus action. Second-Story Work. Climb at full speed; running jump +DEX ft."), f(9, "Supreme Sneak. Advantage on Stealth if you move no more than half speed."), f(13, "Use Magic Device. Ignore class, race, and level on magic-item use."), f(17, "Thief's Reflexes. Two turns in the first round; the second at initiative −10.")]),
    ],
  },
  {
    id: "sorcerer",
    name: "Sorcerer",
    hd: 6,
    saves: ["con", "cha"],
    skills: 2,
    skillList: ["arcana", "deception", "insight", "intimidation", "persuasion", "religion"],
    armor: "Daggers, darts, slings, quarterstaffs, light crossbows. No armor.",
    caster: "full",
    spell: "cha",
    subclassLv: 1,
    asi: [4, 8, 12, 16, 19],
    features: [
      f(1, "Spellcasting (CHA). Sorcerous Origin."),
      f(2, "Font of Magic. Sorcery points = sorcerer level. Convert points and slots."),
      f(3, "Metamagic (2 options)."),
      f(6, "Origin feature."),
      f(10, "Metamagic (3 options)."),
      f(14, "Origin feature."),
      f(17, "Metamagic (4 options)."),
      f(18, "Origin feature."),
      f(20, "Sorcerous Restoration. 4 sorcery points on a short rest."),
    ],
    subclasses: [
      sub("draconic", "Draconic Bloodline", [f(1, "Dragon Ancestor. Draconic Resilience. +1 HP per sorcerer level. Unarmored AC 13 + DEX."), f(6, "Elemental Affinity. Add CHA to damage of your dragon's element. Spend a sorcery point to resist it."), f(14, "Dragon Wings. Fly."), f(18, "Draconic Presence. Awe or fear aura.")]),
    ],
  },
  {
    id: "warlock",
    name: "Warlock",
    hd: 8,
    saves: ["wis", "cha"],
    skills: 2,
    skillList: ["arcana", "deception", "history", "intimidation", "investigation", "nature", "religion"],
    armor: "Light armor. Simple weapons.",
    caster: "pact",
    spell: "cha",
    subclassLv: 1,
    asi: [4, 8, 12, 16, 19],
    features: [
      f(1, "Otherworldly Patron. Pact Magic (CHA). Recover slots on a short rest."),
      f(2, "Eldritch Invocations (2)."),
      f(3, "Pact Boon. Chain, Blade, or Tome."),
      f(6, "Patron feature."),
      f(10, "Patron feature."),
      f(11, "Mystic Arcanum (6th)."),
      f(13, "Mystic Arcanum (7th)."),
      f(14, "Patron feature."),
      f(15, "Mystic Arcanum (8th)."),
      f(17, "Mystic Arcanum (9th)."),
      f(20, "Eldritch Master. Regain all pact slots, 1/rest."),
    ],
    subclasses: [
      sub("fiend", "The Fiend", [f(1, "Dark One's Blessing. Temp HP = CHA + warlock level when you drop a foe."), f(6, "Dark One's Own Luck. Add 1d10 to a check or save, 1/rest."), f(10, "Fiendish Resilience. Choose a damage resistance after each rest (not silvered magic weapons)."), f(14, "Hurl Through Hell. On a hit, banish a foe for a turn; 10d10 psychic when they return, 1/rest.")]),
    ],
  },
  {
    id: "wizard",
    name: "Wizard",
    hd: 6,
    saves: ["int", "wis"],
    skills: 2,
    skillList: ["arcana", "history", "insight", "investigation", "medicine", "religion"],
    armor: "Daggers, darts, slings, quarterstaffs, light crossbows. No armor.",
    caster: "full",
    spell: "int",
    subclassLv: 2,
    asi: [4, 8, 12, 16, 19],
    features: [
      f(1, "Spellcasting (INT). Arcane Recovery. Regain slots totaling up to half wizard level (rounded up) on a short rest, once a day."),
      f(2, "Arcane Tradition."),
      f(6, "Tradition feature."),
      f(10, "Tradition feature."),
      f(14, "Tradition feature."),
      f(18, "Spell Mastery. Two 1st- and 2nd-level spells at will."),
      f(20, "Signature Spells. Two 3rd-level spells always prepared; 1/rest each without a slot."),
    ],
    subclasses: [
      sub("evocation", "School of Evocation", [f(2, "Evocation Savant. Sculpt Spells. Drop allies out of your evocation's area."), f(6, "Potent Cantrip. Cantrips still deal half damage on a save or miss."), f(10, "Empowered Evocation. Add INT to one damage roll of an evocation wizard spell."), f(14, "Overchannel. Max damage on a 1st–5th wizard spell; repeated use hurts you.")]),
    ],
  },
];

export const RED_ROLES = {
  Rockerboy: "Charismatic Impact. Move a crowd, start a riot, or own a room. Rank is the size of the swing.",
  Solo: "Combat Awareness. Initiative and the extra combat options scale with rank.",
  Netrunner: "Interface. NET actions per round and the programs you can hold scale with rank.",
  Tech: "Maker. Craft, repair, and upgrade quality scale with rank.",
  Medtech: "Medicine. Trauma, pharmaceuticals, and cryo scale with rank.",
  Media: "Credibility. The story you drop, and who has to answer it, scale with rank.",
  Exec: "Teamwork. Team size and the quality of the people who show up scale with rank.",
  Lawman: "Backup. How many badges arrive, and how heavy they are, scale with rank.",
  Fixer: "Operator. Contacts, goods, and the jobs you can source scale with rank.",
  Nomad: "Moto. Family vehicles and the pack that answers scale with rank.",
  Gamemaster: "You host. Rank is flavor.",
};

export function classById(id) {
  return CLASSES.find((c) => c.id === id) || null;
}

export function classByName(name) {
  const n = String(name || "").trim().toLowerCase();
  if (!n) return null;
  return CLASSES.find((c) => c.id === n || c.name.toLowerCase() === n) || null;
}

export function subclassById(cls, id) {
  if (!cls) return null;
  return (cls.subclasses || []).find((s) => s.id === id || s.name.toLowerCase() === String(id || "").toLowerCase()) || null;
}

export function xpForLevel(level) {
  const lv = Math.min(20, Math.max(1, Number(level) || 1));
  return XP_TO_LEVEL[lv] || 0;
}

export function xpToNext(level) {
  const lv = Math.min(20, Math.max(1, Number(level) || 1));
  if (lv >= 20) return null;
  return XP_TO_LEVEL[lv + 1];
}

export function slotsFor(cls, level) {
  const lv = Math.min(20, Math.max(1, Number(level) || 1));
  const kind = cls?.caster;
  if (kind === "full") return FULL[lv - 1].slice();
  if (kind === "half") return HALF[lv - 1].slice();
  if (kind === "art") return ART[lv - 1].slice();
  if (kind === "pact") {
    const n = lv >= 17 ? 4 : lv >= 11 ? 3 : lv >= 2 ? 2 : 1;
    const slotLv = lv >= 9 ? 5 : lv >= 7 ? 4 : lv >= 5 ? 3 : lv >= 3 ? 2 : 1;
    const row = [0, 0, 0, 0, 0, 0, 0, 0, 0];
    row[slotLv - 1] = n;
    return row;
  }
  return null;
}

export function avgHpGain(hd, conMod) {
  return Math.max(1, Math.floor(hd / 2) + 1 + (Number(conMod) || 0));
}

export function rollHpGain(hd, conMod) {
  const roll = 1 + Math.floor(Math.random() * Math.max(1, hd));
  return { roll, total: Math.max(1, roll + (Number(conMod) || 0)) };
}

export function startingHp(hd, conMod) {
  return Math.max(1, hd + (Number(conMod) || 0));
}

export function featuresForLevel(cls, lv) {
  return (cls?.features || []).filter((x) => x.lv === lv).map((x) => x.text);
}

export function subFeaturesForLevel(sub, lv) {
  return (sub?.features || []).filter((x) => x.lv === lv).map((x) => x.text);
}

export function isAsiLevel(cls, lv) {
  return Boolean(cls?.asi?.includes(lv));
}

export function hasTagged(text, tag) {
  return String(text || "").includes(tag);
}

export function appendTagged(text, tag, body) {
  const cur = String(text || "").trim();
  if (hasTagged(cur, tag)) return cur;
  const line = `${tag} ${body}`.trim();
  return cur ? cur + "\n" + line : line;
}

export function applyClassBasics(ch, cls) {
  if (!ch || !cls) return;
  ch.classId = cls.id;
  ch.className = cls.name;
  ch.hitDice = `${Math.max(1, Number(ch.level) || 1)}d${cls.hd}`;
  ch.spellClass = cls.caster ? cls.name : ch.spellClass || "";
  ch.spellAbility = cls.spell || "";
  for (const id of cls.saves || []) ch.saveProf[id] = true;
  const bits = [cls.armor, (cls.saves || []).map((s) => s.toUpperCase() + " saves").join(", ")].filter(Boolean);
  if (!String(ch.proficiencies || "").includes(cls.name)) {
    ch.proficiencies = [ch.proficiencies, `${cls.name}: ${bits.join(". ")}`].filter(Boolean).join("\n");
  }
  const slots = slotsFor(cls, ch.level || 1);
  if (slots) {
    ch.slotsMax = slots;
    ch.slotsUsed = ch.slotsUsed.map((n, i) => Math.min(Number(n) || 0, slots[i] || 0));
  }
}

export function grantLevelFeatures(ch, cls, lv, sub) {
  const tag = `[Lv ${lv}]`;
  const bits = featuresForLevel(cls, lv);
  if (bits.length) ch.features = appendTagged(ch.features, tag, bits.join(" · "));
  if (sub) {
    const stag = `[${sub.name} ${lv}]`;
    const sbits = subFeaturesForLevel(sub, lv);
    if (sbits.length) ch.features = appendTagged(ch.features, stag, sbits.join(" · "));
  }
}

export function skillIpCost(nextRank) {
  const n = Number(nextRank) || 1;
  return n * 10;
}

export function roleIpCost(nextRank) {
  const n = Number(nextRank) || 1;
  return n * 20;
}

export function statIpCost(nextVal) {
  const n = Number(nextVal) || 1;
  return n * 25;
}

export function roleBlurb(role, rank) {
  const line = RED_ROLES[role] || "Role ability.";
  return `[Role ${rank}] ${role || "Role"} ${rank}. ${line}`;
}
