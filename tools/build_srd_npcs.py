#!/usr/bin/env python3
"""SRD appendix NPCs with original table dossiers. Art reuses bestiary portraits."""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path("/home/steelworth/projects/blightnet")
BESTIARY = ROOT / "data" / "bestiary.json"
OUT = ROOT / "data" / "srd-npcs.json"

DOSSIER = {
    "acolyte": (
        "faith",
        "Temple attendant",
        "A junior of a shrine: candles, alms, and a mace they barely know. They know who comes at dawn and who never comes.",
        "Use them as the first friendly face in a holy house, or the first body when the cult downstairs is found out.\n\nThey will fetch the priest. They will also hide a letter in a hymnal if you are kind. A fight with an acolyte should feel like a mistake, not a boss.",
        ["The acolyte saw the villain in the vestry last night.", "They will trade a healing kit for a secret kept from the high priest."],
    ),
    "archmage": (
        "magic",
        "Tower-lord",
        "A spellcaster who has outlived three apprentices and two towers. The third tower has better locks.",
        "Drop an archmage when the table needs a problem money cannot buy. They do not adventure. They send people.\n\nTheir tower is a dungeon with a guest book. If the party wins, it is because the archmage wanted a conversation, not a corpse.",
        ["They hire the party to steal a book they already own, to see who else wants it.", "A ward failed. They need someone who can bleed without ruining the carpet."],
    ),
    "assassin": (
        "criminal",
        "Contract killer",
        "Polite until the poison lands. They study doors, drinks, and the way a mark laughs.",
        "An assassin is a clock. Once they have a name, the table has a deadline.\n\nThey are not a random alley fight. Put them in a feast, a bathhouse, or a confession booth. If they miss, they send a letter explaining why that was professional courtesy.",
        ["The contract is on an NPC the party likes.", "The assassin offers to cancel if the party does a worse job for them."],
    ),
    "bandit": (
        "criminal",
        "Road thief",
        "Hungry, armed, and not yet famous. They work in packs because one bandit is a story; six is a toll.",
        "Use bandits for the first blood of a road. They want purses, not martyrdom.\n\nA smart bandit runs. A stupid one dies on the wagon. Let the party talk: food, a lie, or a better job will peel them off a cruel captain.",
        ["They work for someone in the next town.", "One of them used to be a guard and still has the key."],
    ),
    "bandit-captain": (
        "criminal",
        "Crew boss",
        "The one who counts the take and decides who eats. Charming until the math goes wrong.",
        "A captain turns bandits into a faction. Give them a camp, a code, and a rival.\n\nThey will parley. They will also cut a throat at the table if the party names the wrong person. Treat them as a short-term warlord, not a mook with extra HP.",
        ["They want a letter of marque, or a fake one.", "Their lieutenant is selling them out to the knight in the next hex."],
    ),
    "berserker": (
        "martial",
        "Rage fighter",
        "A warrior who has decided armor is a conversation they are tired of having.",
        "Berserkers make noise. Put them at the front of a raid, a funeral, or a pit.\n\nThey are loyal to a cause or a person, not a paycheck. If the party can name that cause, the fight can stop. If they cannot, the fight does not.",
        ["They were hired as muscle and were not told about the children.", "Their rage is a geas. Breaking it is a quest."],
    ),
    "commoner": (
        "town",
        "Ordinary person",
        "No CR that matters. They bake, haul, gossip, and die if you treat the town as scenery.",
        "Commoners are the scoreboard. If the dragon wins, count them.\n\nGive each one a name and a job. The innkeep, the miller's kid, the clerk who saw the noble's coach. A commoner with a secret is worth more than a second ogre.",
        ["They watched the villain bury something behind the tannery.", "They will hide the party if the party hides their cousin first."],
    ),
    "cultist": (
        "faith",
        "Rank-and-file believer",
        "Robes, a knife, and a hymn they only half understand. Dangerous in a crowd.",
        "Cultists are the church's bad night. Alone they fold. Together they finish a ritual.\n\nUse them as a timer: every room cleared is one less voice in the chant. Let one surrender and you have a map of the basement.",
        ["They think the party are prophesied guests.", "Their brand matches a noble house in town."],
    ),
    "cult-fanatic": (
        "faith",
        "Priest of the wrong god",
        "The one who writes the sermons and picks who goes into the pit.",
        "A fanatic is a lieutenant with spells. They keep the cultists facing the right altar.\n\nThey will talk theology until a blade comes out. If the party debates, they might delay the summoning. If the party charges, they finish it early.",
        ["The fanatic's holy symbol is a stolen family crest.", "They want a specific PC as the last offering and will say why."],
    ),
    "druid": (
        "wild",
        "Circle walker",
        "A spellcaster who answers to weather before kings. The grove is their court.",
        "A druid is a territorial problem. Roads, mills, and logging camps are their villains.\n\nThey can be an ally who will not enter cities, or an enemy who will not leave the wood. Wild shape makes them a scout, a spy, and a second encounter in one body.",
        ["A blight in the wood is coming from the temple, not the trees.", "They will guide the party if the party leaves the axes."],
    ),
    "gladiator": (
        "martial",
        "Arena champion",
        "A fighter who has learned to make a crowd love a killing. The smile is part of the armor.",
        "Put a gladiator in a pit, a parade, or a noble's party as hired muscle who hates the job.\n\nThey fight fair when people are watching and dirty when they are not. Freedom, a name, or a better contract will move them. Gold alone buys one bout.",
        ["They know a secret exit under the arena.", "Their next match is rigged against an NPC the party must keep alive."],
    ),
    "guard": (
        "martial",
        "Watch soldier",
        "A spear, a shift, and orders from someone they may not like. The town's immune system.",
        "Guards are the law the party meets first. They can be honest, bought, or tired.\n\nA fight with guards should cost reputation, not just HP. Talking, badges, and the right name open doors. Killing them closes the city.",
        ["The night sergeant is missing. The day sergeant is lying.", "They will look the other way for proof their captain is dirty."],
    ),
    "knight": (
        "martial",
        "Sworn rider",
        "Plate, a code, and a lord. The code and the lord do not always agree.",
        "A knight is a walking faction. They arrive with horses, banners, and consequences.\n\nUse them as a rival adventurer with better funding, or as the law when guards are not enough. If the party shames them in public, the next meeting is a duel. If the party saves their honor, they become a road.",
        ["Their oath requires a deed the party is about to prevent.", "They are hunting the same villain and will not share the kill."],
    ),
    "mage": (
        "magic",
        "Working wizard",
        "Not a legend yet. Enough fireball to ruin a tavern and enough sense to run afterward.",
        "A mage is a specialist for hire: ward, translation, or a hole in a wall.\n\nThey panic like people. Give them a patron, a debt, and a spellbook they will die for. If the party steals the book, they have a recurring enemy. If they pay, they have a scared ally.",
        ["Their apprentice sold a scroll that should not exist.", "They need a bodyguard for a summoning they already started."],
    ),
    "noble": (
        "town",
        "Landed name",
        "Silk, a title, and a rapier they were taught for portraits. The real weapon is the room.",
        "Nobles move plots without swinging. Invitations, marriages, and taxes are their spells.\n\nA noble in combat is a failure state for someone. Use them at dinners, courts, and balconies. Their retainer is the actual fight. Their rumor is the actual treasure.",
        ["They will fund the party if the party ruins a rival's feast.", "The missing heir is in the party's wagon and does not know it."],
    ),
    "priest": (
        "faith",
        "Temple voice",
        "The one who marries, buries, and decides which miracles are official.",
        "A priest is infrastructure. Towns without one feel the gap in the first plague.\n\nThey can heal, hide, or hunt the party depending on the god and the last rumor. Let them be busy: a funeral at noon, a confession at dusk, a thing in the crypt after dark.",
        ["A burial they performed got up again.", "They will bless a weapon if the party returns a relic."],
    ),
    "scout": (
        "wild",
        "Trail watcher",
        "A hunter who works for an army, a village, or themselves. They see you before you see the camp.",
        "Scouts are information with a bow. They do not stand in doorways.\n\nUse them as the encounter before the encounter: tracks, a whistle, a missing horse. If the party is loud, the scout is why the next room is ready.",
        ["They will sell the party's route to both sides.", "They found a ruin and want backup, not a boss."],
    ),
    "spy": (
        "criminal",
        "Face in the crowd",
        "A shortsword under a cloak and a story that fits the room. They leave before the bodies cool.",
        "A spy is a plot that walks. They should already be in the party's inn.\n\nWhen revealed, they run, they lie, or they switch sides. Combat is the backup plan. The real fight is which letter they posted this morning.",
        ["They have been the party's hireling for two sessions.", "They offer a name in exchange for a way out of the city tonight."],
    ),
    "thug": (
        "criminal",
        "Street muscle",
        "A heavy, a club, and orders from a name they will not say until they are losing.",
        "Thugs are urban bandits. Alleys, docks, and tavern back rooms.\n\nThey pack a punch and pack a pack. Break the leader's nerve and the rest want a drink instead of a grave. If the party kills all of them, the name behind them sends professionals next.",
        ["Their boss is a noble's spare child.", "They were paid to scare, not to kill, and went too far."],
    ),
    "tribal-warrior": (
        "wild",
        "Clan fighter",
        "A spear, a shield, and a people. The map calls them barbarians. They call the map late.",
        "Warriors like this defend a camp, a herd, or a standing stone.\n\nThey are not bandits. Treat the clan as a culture: guest-right, insult, and blood price. A fight can be a raid or a misunderstanding. A gift can be a treaty.",
        ["A settler road cuts their hunting ground.", "They will guide the party through a cursed pass for a stolen totem."],
    ),
    "veteran": (
        "martial",
        "Old soldier",
        "Someone who survived a war the town would like to forget. The armor still fits. The patience does not.",
        "A veteran is a guard captain, a mercenary sergeant, or the drunk at the end of the bar who is neither.\n\nThey have seen better fighters than the party and worse causes. Hire them, fear them, or bury them with honors. A veteran who likes the party is a fortress. One who does not is a campaign.",
        ["They know the dungeon because they built it as a fort.", "Their old company is the villain's army and they still have the password."],
    ),
}


def main():
    bestiary = {r["id"]: r for r in json.loads(BESTIARY.read_text())}
    out = []
    for nid, (kind, aka, blurb, text, hooks) in DOSSIER.items():
        b = bestiary.get(nid)
        if not b:
            raise SystemExit(f"missing bestiary {nid}")
        acts = b.get("actions") or []
        weps = [{"n": a.get("n") or a.get("name") or "Attack", "d": a.get("d") or a.get("text") or ""} for a in acts[:4]]
        out.append(
            {
                "id": nid,
                "name": b["name"],
                "aka": aka,
                "role": aka,
                "kind": kind,
                "cr": b.get("crLabel") or str(b.get("cr") or ""),
                "size": b.get("size") or "",
                "align": b.get("align") or "",
                "ac": b.get("ac"),
                "hp": b.get("hp"),
                "speed": b.get("speed") or "",
                "str": b.get("str"),
                "dex": b.get("dex"),
                "con": b.get("con"),
                "int": b.get("int"),
                "wis": b.get("wis"),
                "cha": b.get("cha"),
                "saves": b.get("saves") or "",
                "skills": b.get("skills") or "",
                "senses": b.get("senses") or "",
                "lang": b.get("lang") or "",
                "xp": b.get("xp"),
                "weapons": weps,
                "traits": b.get("traits") or [],
                "blurb": blurb,
                "text": text,
                "hooks": hooks,
                "look": aka,
            }
        )
    out.sort(key=lambda r: r["name"].lower())
    OUT.write_text(json.dumps(out, indent=2), encoding="utf-8")
    print(len(out), "srd npcs ->", OUT)


if __name__ == "__main__":
    main()
