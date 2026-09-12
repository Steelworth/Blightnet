#!/usr/bin/env python3
"""Append Cyberpunk RED Black Chrome catalog to Night Market. Original table notes."""
from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path("/home/steelworth/projects/blightnet")
KIT = ROOT / "data" / "red-kit.json"

def slug(name: str) -> str:
    s = re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-")
    return "bc-" + s


def item(kind, name, cat, cost, text, **extra):
    row = {"kind": kind, "name": name, "cat": cat, "cost": cost, "text": text, "world": "blight", "id": extra.pop("id", slug(name))}
    row.update(extra)
    return row


ITEMS = [
    # Apps
    item("gear", "Digital Gladiator", "app", "100 eb", "Agent app. Street fights, odds, and a crowd that wants blood."),
    item("gear", "4tify", "app", "50 eb", "Agent app. Maps a block's cameras. The cameras also map you."),
    item("gear", "NCPD Crime Database", "app", "500 eb", "Upload a face. In an hour: warrants, bounties. NCPD keeps the photo."),
    item("gear", "Trauma Team MedScan", "app", "20 eb", "Photo a wound. A specialist talks you through the stitch. Extra eb for a real session."),
    item("gear", "Ziggurat City Database", "app", "100 eb", "Agent app. Addresses, rumours, and the version of the city the Net will admit."),
    item("gear", "CBK Night Market", "app", "100 eb", "Browse a Night Market stall list from your Agent. Pickup is still in person."),
    # Cyberware
    item("chrome", "Budget Chipware Socket", "neuralware", "100 eb", "A cheap socket. One chip. It jams if you look at it wrong.", hl="3", slot="jack"),
    item("chrome", "Borgware Hardened Shielding", "borgware", "1000 eb", "EMP-hard the fullborg bits. HL hurts. Lightning doesn't.", hl="14", slot="dermal"),
    item("chrome", "Discount Cyberaudio Suite", "cyberaudio", "100 eb", "Ears from a barrel. Two option slots. Ads optional.", hl="7"),
    item("chrome", "Dynalar Modular Finger Enthusiast Cyberhand", "cyberlimb", "500 eb", "Eight fingers. More toys. Looks like a problem in a handshake.", hl="7", slot="arm"),
    item("chrome", "Explicit Memory Chip", "neuralware", "500 eb", "A memory you can slot. Someone else's life. Don't drown.", hl="0", slot="jack"),
    item("chrome", "Extra-Jointed Cyberlimb Upgrade", "cyberlimb", "500 eb", "The arm bends where arms should not. Climbing, grapples, nightmares.", hl="3", slot="arm"),
    item("chrome", "Flashbulb", "cyberoptics", "100 eb", "A strobe in the eye. Rude. Effective. Needs a cybereye.", hl="2", slot="eyes"),
    item("chrome", "Hardened Cybereye Casing", "cyberoptics", "500 eb", "The eye survives EMP and a fist. Still a camera.", hl="2", slot="eyes"),
    item("chrome", "Heuristic Health Monitor", "internal", "500 eb", "It yells before you drop. Fashionware-adjacent, actually useful.", hl="2"),
    item("chrome", "Integrated Cyberdeck Upgrade", "neuralware", "1000 eb", "A deck in the link. Jack in without a bag. Heat is your problem.", hl="7", slot="jack"),
    item("chrome", "Internal Body Cyberware Hardened Shielding", "internal", "1000 eb", "EMP-hard the meat-side implants.", hl="7", slot="dermal"),
    item("chrome", "Modular Finger Cyberhand", "cyberlimb", "500 eb", "Five fingers, swap the tips. Hypo, dart, pick, spare round.", hl="3", slot="arm"),
    item("chrome", "Neo-Soviet Cyberarm", "cyberlimb", "100 eb", "Heavy, ugly, cheap. Two option slots. It will outlive you.", hl="7", slot="arm"),
    item("chrome", "Popup Net Launcher", "cyberlimb", "500 eb", "A net from the forearm. Hold them. Then decide.", hl="7", slot="arm"),
    item("chrome", "Popup Shotgun", "cyberlimb", "500 eb", "A street howitzer in the arm. One shot. Everyone hears it.", hl="7", slot="arm", dmg="5d6"),
    item("chrome", "RacerBracer", "external", "500 eb", "Neck brace chrome. High-speed. Less whiplash. More looks.", hl="3", slot="dermal"),
    item("chrome", "Sponsored Cybereye", "cyberoptics", "50 eb", "Free-ish eye. Ads in the HUD. They own a piece of what you see.", hl="7", slot="eyes"),
    item("chrome", "Trauma Response Nanomatrix", "internal", "1000 eb", "Keeps Skinweave and subdermal from peeling off when you should be dead.", hl="7", slot="dermal"),
    item("chrome", "Airhypo Cyberfinger", "cyberlimb", "100 eb", "A finger that injects. Modular hand option.", hl="0", slot="arm"),
    item("chrome", "Dartgun Cyberfinger", "cyberlimb", "100 eb", "A poison dart in the fingertip.", hl="0", slot="arm", dmg="1d6"),
    item("chrome", "Lockpick Cyberfinger", "cyberlimb", "100 eb", "The lock thinks you have a key.", hl="0", slot="arm"),
    item("chrome", "Bullet Storage Cyberfinger", "cyberlimb", "50 eb", "A round in the finger. Last shot, last joke.", hl="0", slot="arm"),
    item("chrome", "Flashlight Cyberfinger", "cyberlimb", "50 eb", "Point. Light. Night City still has dark rooms.", hl="0", slot="arm"),
    item("chrome", "Multitool Cyberfinger", "cyberlimb", "100 eb", "Driver, blade, cutter. A Tech's handshake.", hl="0", slot="arm"),
    item("chrome", "Cyberfinger Set", "cyberlimb", "500 eb", "A pouch of swappable tips for a modular hand.", hl="0", slot="arm"),
    # Fashion & armor
    item("armor", "Dirk Combat Jacket", "body", "100 eb", "Looks like a jacket. Acts like light armorjack if they don't scan.", ac="11"),
    item("armor", "Fire Brand Bunker Gear", "body", "500 eb", "Firekit that is also armor. Heat hates it. Fashion does not.", ac="13"),
    item("armor", "Gibson Shock Armor", "body", "500 eb", "Anti-grapple plates. They bounce off. You don't.", ac="12"),
    item("armor", "Gibson Tactical Smart Armor", "body", "500 eb", "Medium armorjack with a HUD in the weave. Looks tactical because it is.", ac="12"),
    item("armor", "Montage Variable Clothing", "body", "500 eb", "Light armorjack that changes pattern on a thought. Camo or a date.", ac="11"),
    item("armor", "Street Viper Riding Suit", "body", "500 eb", "Bike armor. Elbow blades for a slide. Zonda's idea of romance.", ac="11"),
    item("armor", "Corporate Island", "body", "500 eb", "Boardroom kevlar that looks like a suit. Nu-Tek smile.", ac="11"),
    item("armor", "SkidRow Trench", "body", "100 eb", "An armored trench. Classic. Stains tell stories.", ac="11"),
    item("armor", "Masetto Holo-Wear", "body", "1000 eb", "Clothes made of light. Armor underneath if you paid. Sometimes you didn't.", ac="7"),
    item("armor", "Scavenged Armor", "body", "20 eb", "Plates from three jackets. SP if you're lucky. Style if you're not.", ac="7"),
    item("armor", "Leopard LEO Dress", "body", "100 eb", "Looks like a night out. Stops a knife. Maybe a round.", ac="7"),
    item("armor", "Kevlar Evening Wear", "body", "100 eb", "Gold thread over plates. The party is still a fight.", ac="7"),
    item("armor", "Nomad Road Leathers+", "body", "100 eb", "Pack-stamped. Dust, chrome, and 7 SP if the Fixer didn't lie.", ac="7"),
    # Gear
    item("gear", "Drink Master 3000", "gear", "1000 eb", "A rolling bar. Three thousand mixes. Smash is on the menu. 100 eb refill / 10 uses."),
    item("gear", "Shower-in-a-Can", "gear", "10 eb", "Foam, wipe, pretend you have plumbing. Biotechnica pine or 'classic clean'."),
    item("gear", "One Touch Habitat", "gear", "100 eb", "A tent that stands itself. Climate control. Nomad gospel."),
    item("gear", "AirWell 50", "gear", "100 eb", "Pulls half a litre of water from the air every five hours. Solar."),
    item("gear", "Jeeves Executive Garment Bag", "gear", "500 eb", "Cleans and mends a suit while you sleep. Judges you."),
    item("gear", "RapiDeploy Sheath", "gear", "500 eb", "The blade is in your hand before the sentence ends."),
    item("gear", "Petrochem Nitro Ultra9", "gear", "100 eb", "Fuel additive. The engine screams. So will the cop."),
    item("gear", "SovOil Lubricant", "gear", "20 eb", "It says industrial. Street uses vary. Don't ask."),
    item("gear", "WorldSat Aerial Sphere", "gear", "1000 eb", "A balloon camera. Looks like weather. Isn't."),
    item("gear", "Triti-Fizz", "gear", "10 eb", "A soda that glows. Radioactive in the marketing, not the lab. Probably."),
    item("gear", "Piranha Smash", "gear", "20 eb", "Street drug. Meaner Smash. Teeth optional."),
    item("gear", "Bullet to Slug Adapter", "gear", "20 eb", "Pistol rounds in a shotgun. Ugly. Legal-ish."),
    item("gear", "Junk Ammunition", "gear", "10 eb", "Reloads from a dumpster. Jams love these."),
    item("gear", "Small Game Ammunition", "gear", "10 eb", "For pests. Also for knees if you're cruel."),
    item("gear", "Solo of Fortune Bodypillow", "gear", "100 eb", "A joke until someone sleeps with it on a stakeout."),
    item("gear", "Molotov Cocktail", "gear", "20 eb", "A bottle and a rag. Fire. Night City still uses these."),
    # Linear frames
    item("chrome", "Fuma Kotaro Linear Frame", "borgware", "5000 eb", "Yakuza spec. Strength plus quiet. Internal or external. BODY 12-ish.", hl="14"),
    item("chrome", "LF-001 SWAT Linear Frame", "borgware", "5000 eb", "Polska. Climb and swim like the armor isn't there. BODY 12.", hl="14"),
    item("chrome", "Vermillion Linear Frame", "borgware", "5000 eb", "Sleek. Fast. BODY 12, no jump penalty, skate-feet analog.", hl="14"),
    item("chrome", "EL-F4-NT Linear Frame", "borgware", "5000 eb", "Zhirafa construction exo. External only. Work, not a fashion show.", hl="7"),
    item("gear", "Fuma Kotaro Frame (worn)", "gear", "5000 eb", "External hire. Two interface plugs. You look like a problem."),
    item("gear", "LF-001 SWAT Frame (worn)", "gear", "5000 eb", "External SWAT exo. Climb, swim, scare the block."),
    item("gear", "Vermillion Frame (worn)", "gear", "5000 eb", "External speed frame. Skate the street without the surgery."),
    item("gear", "EL-F4-NT Frame (worn)", "gear", "5000 eb", "Construction exo. Lift the van. Don't dance."),
    # Vehicles
    item("gear", "Dayton SH-45 Patroller", "vehicle", "40000 eb", "One-seat gyro. Roof work. Loud."),
    item("gear", "Dayton SH-45 Patroller Law", "vehicle", "50000 eb", "Bulletproof gyro with a gun. NCPD yellow."),
    item("gear", "Dayton TDT 004", "vehicle", "30000 eb", "A Dayton that thinks it's a car and a kite."),
    item("gear", "Tanson Bellhop", "vehicle", "20000 eb", "A gyro that folds into a suitcase. Trendy. Fragile."),
    item("gear", "Zetatech AeroVox", "vehicle", "51000 eb", "Economy aerodyne for the corpo who isn't one yet."),
    item("gear", "Zetatech AeroCop", "vehicle", "58000 eb", "Armored police AV. Do not steal unless you mean it."),
    item("gear", "Zetatech Destination", "vehicle", "40000 eb", "Compact AV. Four seats if you like each other."),
    item("gear", "Zetatech Herakles", "vehicle", "63000 eb", "Troop AV. SP and attitude."),
    item("gear", "AmeriCar EconoCompact", "vehicle", "10000 eb", "Three seats. No frills. It starts."),
    item("gear", "AmeriCar Family Star Van", "vehicle", "20000 eb", "A van that has seen things. Still a van."),
    item("gear", "Diego Motors Chupacabra", "vehicle", "30000 eb", "Nomad muscle. Eats dirt. Spits cops."),
    item("gear", "Diego Motors Range Trike", "vehicle", "31000 eb", "Three wheels, armor, a pack on the back."),
    item("gear", "Harvey 100", "vehicle", "20000 eb", "A bike. Honest. Fast enough."),
    item("gear", "Zonda Parallax", "vehicle", "100000 eb", "Cybercycle. 300 mph if the street is a rumour."),
    item("gear", "Zonda Sliver", "vehicle", "20000 eb", "A thin bike. Looks illegal. Is."),
    item("gear", "Zonda Molly 1K", "vehicle", "30000 eb", "Street bike with a name and a waiting list."),
    item("gear", "TetraCorp America MegaHauler", "vehicle", "100000 eb", "A land train. Armor. Cargo. A rolling fortress."),
    item("gear", "TetraCorp America Badger Corporate Bus", "vehicle", "80000 eb", "A boardroom that drives. Converts. Hides."),
    item("gear", "Yang's Wheels Rickshaw", "vehicle", "21000 eb", "A cycle-rickshaw that thinks it's a taxi. Night City weather included."),
    item("gear", "Tanson JetBoy Hoverboard", "vehicle", "1000 eb", "A foot off the ground. 30 mph. Pride is extra."),
    item("gear", "Makigai Ebi", "vehicle", "15000 eb", "A hatchback so cute people forget it has a trunk for a body."),
    item("gear", "Quadra Thunder-X", "vehicle", "40000 eb", "Muscle. Night City still makes these myths."),
    item("gear", "Paladin 500", "vehicle", "40000 eb", "A cruiser that wants a highway that doesn't exist."),
    item("gear", "Zacatzontli Pickup", "vehicle", "25000 eb", "Nomad truck. Off-road. A bed for the pack."),
    item("gear", "Militech Gorgon Security Van", "vehicle", "60000 eb", "A squad box. Enters firefights. Leaves some of you."),
    item("gear", "Grundy", "vehicle", "10000 eb", "A junker with a soul. Or just rust."),
    # Weapons — explosives
    item("weapon", "Arachnid Grenade", "grenade", "100 eb", "Goo. They stick. Then you walk over.", dmg="—", rof="1", shots="1"),
    item("weapon", "Shuriken Tornado Grenade", "grenade", "100 eb", "A room full of mono-stars. Don't be in it.", dmg="4d6", rof="1", shots="1"),
    item("weapon", "GunMart Door Cracker", "grenade", "50 eb", "A focused bang for a hinge. Cover dies. So might you if it's Poor.", dmg="4d6", rof="1", shots="1"),
    item("weapon", "Micro Hydrogen Combustor", "grenade", "500 eb", "A tiny sun. Don't pocket it next to Smash.", dmg="6d6", rof="1", shots="1"),
    item("weapon", "KillChip 100 Micro-Bomb", "grenade", "100 eb", "Looks like a memory chip. Explodes like a grenade. Rude.", dmg="6d6", rof="1", shots="1"),
    item("weapon", "KTech Security Grenade", "grenade", "100 eb", "Less-lethal if you believe the label. Still a grenade.", dmg="3d6", rof="1", shots="1"),
    item("weapon", "OUTLet Explosive", "grenade", "50 eb", "Looks like a wall wart. Is a bomb. Hotel work.", dmg="4d6", rof="1", shots="1"),
    item("weapon", "TR-4 Detonator Fluid", "grenade", "100 eb", "Paint it. Leave. It cooks. Demo in a bottle.", dmg="5d6", rof="1", shots="1"),
    # Melee
    item("weapon", "Arasaka Weeping Reaver Katana", "melee", "1000 eb", "A katana that weeps fire, acid, or salt into the cut. Excellent. Mean.", dmg="4d6", rof="2"),
    item("weapon", "Kendachi Mono-Katana", "melee", "500 eb", "Mono edge. Armor as half. The classic.", dmg="4d6", rof="2"),
    item("weapon", "Solo Wolf and Bot Mono-Katana", "melee", "500 eb", "A named Kendachi cousin. Looks like a poster. Cuts like one.", dmg="4d6", rof="2"),
    item("weapon", "Kendachi Mono-Wakizashi", "melee", "500 eb", "Shorter mono. Concealable if you lie well.", dmg="3d6", rof="2"),
    item("weapon", "White Hornet Tanto", "melee", "100 eb", "A tanto that wants a kidney. Fast.", dmg="2d6", rof="2"),
    item("weapon", "Rostovic Kleaver", "melee", "500 eb", "A cleaver that starts fires. Two hands. No friends.", dmg="4d6", rof="1"),
    item("weapon", "Zhirafa Rhinocefist", "melee", "500 eb", "A fist like a piston. Cyberarm cousin. Buildings notice.", dmg="4d6", rof="1"),
    item("weapon", "Faisal's Magna Knuckles", "melee", "500 eb", "A punch that also stuns chrome. EMP in the knuckles.", dmg="2d6", rof="2"),
    item("weapon", "Kendachi Mono-Guard", "melee", "500 eb", "A mono whip. Concealable. Reach. Ugly on a dance floor.", dmg="3d6", rof="2"),
    item("weapon", "Utility Tomahawk", "melee", "100 eb", "Tool and thrown. Nomad gospel.", dmg="2d6", rof="2"),
    item("weapon", "Ranger Combat Boomerang", "melee", "100 eb", "Thrown. Comes back if you have the chrome. Otherwise it's a stick.", dmg="3d6", rof="1"),
    item("weapon", "Kendachi Mono-Star", "melee", "100 eb", "A shuriken that spins itself. Mono. Don't catch it.", dmg="2d6", rof="1"),
    item("weapon", "Big Dreem", "melee", "50 eb", "A street club with a name. Molly sells these. They work.", dmg="2d6", rof="2"),
    item("weapon", "Sanroo Hello Cutie 1TruLuv", "melee", "100 eb", "A cute blade. Kawaii until it isn't.", dmg="2d6", rof="2"),
    # Firearms
    item("weapon", "Arasaka Prototype Variable Automatic Rifle", "rifle", "5000 eb", "Rare. With the right parts it is a railgun. SP 7 and under cry.", dmg="5d6", rof="1", shots="20"),
    item("weapon", "E-TACK Public Defender", "pistol", "100 eb", "No trigger. Lethal or less-lethal. Lawman toy. Street copies exist.", dmg="3d6", rof="2", shots="8"),
    item("weapon", "Eagletech Survivalist", "rifle", "500 eb", "A crossbow glued to an assault rifle. Nomad picnic.", dmg="5d6", rof="1", shots="20"),
    item("weapon", "GunMart Engage Rocket Launcher", "heavy", "100 eb", "Cheap rocket. Poor quality. It might kill you first.", dmg="8d6", rof="1", shots="1"),
    item("weapon", "GunMart Special", "pistol", "50 eb", "A pistol from a vending wall. Jams. Affordable last words.", dmg="2d6", rof="2", shots="8"),
    item("weapon", "Georgia Arms Matchmaker", "shotgun", "100 eb", "A jury-rigged shotgun. Cheap. Loud. Sometimes both barrels.", dmg="5d6", rof="1", shots="2"),
    item("weapon", "Militech Mastiff SMG", "smg", "500 eb", "SMG that thinks it's a shotgun. Two personalities. One trigger.", dmg="3d6", rof="1", shots="30"),
    item("weapon", "Militech Perseus", "pistol", "500 eb", "Very heavy pistol. Kinetic recapture. Fires faster if you keep shooting.", dmg="4d6", rof="1", shots="8"),
    item("weapon", "Nomad Rocker", "exotic", "100 eb", "An air gun that throws rocks. Silent. Humiliating. Effective.", dmg="3d6", rof="1", shots="1"),
    item("weapon", "Superchrome Glam Rifle", "rifle", "500 eb", "Looks like a concert. Shoots like a rifle. Bonus to looking expensive.", dmg="5d6", rof="1", shots="20"),
    item("weapon", "Superchrome Javelin", "exotic", "500 eb", "A stylish spear-gun. Walk in like a poster.", dmg="4d6", rof="1", shots="1"),
    item("weapon", "Superchrome Sidearm", "pistol", "500 eb", "A pretty heavy pistol. The club lets you in.", dmg="3d6", rof="2", shots="8"),
    item("weapon", "Everest VentureWare SportMaster", "rifle", "100 eb", "A hunting rifle that wandered into a war.", dmg="5d6", rof="1", shots="4"),
    item("weapon", "Everest VentureWare SurvivalMaster", "rifle", "100 eb", "The SportMaster's meaner cousin. A toolbox with a barrel.", dmg="5d6", rof="1", shots="4"),
    item("weapon", "Timeless WW1 Rifle to Pistol Conversion", "pistol", "50 eb", "A hundred-year-old idea. Still shoots. History as a crime.", dmg="3d6", rof="1", shots="1"),
    item("weapon", "GunMart Door Gun", "heavy", "100 eb", "If the Door Cracker was a gun. Poor. Loud. A hole.", dmg="6d6", rof="1", shots="1"),
]


def main() -> None:
    kit = json.loads(KIT.read_text(encoding="utf-8"))
    have = {r["id"] for r in kit}
    have_n = {r["name"].lower() for r in kit}
    added = 0
    for row in ITEMS:
        if row["id"] in have or row["name"].lower() in have_n:
            continue
        kit.append(row)
        have.add(row["id"])
        added += 1
    KIT.write_text(json.dumps(kit, ensure_ascii=False), encoding="utf-8")
    print("added", added, "total", len(kit))


if __name__ == "__main__":
    main()
