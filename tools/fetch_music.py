#!/usr/bin/env python3
"""Download a large Kevin MacLeod music expansion (CC BY 4.0)."""

from __future__ import annotations

import subprocess
import urllib.error
import urllib.parse
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RAW = ROOT / "tools" / "_raw" / "music"
AUDIO = ROOT / "audio" / "music"
UA = "Blightnet/3.4 (local table app)"

# Already in the library under other ids — do not fetch again.
SKIP_TITLES = {
    "Minstrel Guild",
    "Magic Forest",
    "Night Cave",
    "Five Armies",
    "Angevin",
    "Skye Cuillin",
    "Rites",
    "Teller of the Tales",
    "Folk Round",
    "Enchanted Valley",
    "The Ice Giants",
    "Dragon and Toast",
    "Celtic Impulse",
    "Sneaky Adventure",
    "Wizardtorium",
    "Oppressive Gloom",
    "Stormfront",
    "Hidden Past",
    "Heroic Age",
    "The Path of the Goblin King v2",
    "Thatched Villagers",
    "The Descent",
    "The Pyre",
    "Clash Defiant",
    "Morgana Rides",
    "Lost Time",
    "Crusade",
    "Darkest Child",
    "Ghost Story",
    "One-eyed Maestro",
    "Lord of the Land",
    "Achaidh Cheide",
    "Call to Adventure",
    "Black Vortex",
    "Comfortable Mystery",
    "Ossuary 6 - Air",
    "Feral Chase",
    "At Rest",
    "Curse of the Scarab",
    "Interloper",
    "Pippin the Hunchback",
    "Darkling",
    "Moorland",
    "Ritual",
    "Blue Feather",
    "The Chamber",
    "Water Prelude",
    "Floating Cities",
    "With the Sea",
    "Shores of Avalon",
}

# id -> (incompetech title, display name, mood, icon)
TRACKS: dict[str, tuple[str, str, str, str]] = {
    "for_originz": ("For Originz", "For Originz", "heroic", "banner"),
    "prelude_action": ("Prelude and Action", "Prelude and Action", "battle", "swords"),
    "rising_game": ("Rising Game", "Rising Game", "battle", "swords"),
    "inspired": ("Inspired", "Inspired", "heroic", "banner"),
    "dark_walk": ("Dark Walk", "Dark Walk", "dark", "moon"),
    "ghost_processional": ("Ghost Processional", "Ghost Processional", "dark", "ghost"),
    "unlight": ("Unlight", "Unlight", "dark", "moon"),
    "echoes_time": ("Echoes of Time", "Echoes of Time", "mystery", "clock"),
    "hitman": ("Hitman", "Hitman", "mystery", "sneak"),
    "danse_macabre": ("Danse Macabre", "Danse Macabre", "dark", "ghost"),
    "decline": ("Decline", "Decline", "dark", "tomb"),
    "crypto": ("Crypto", "Crypto", "mystery", "sneak"),
    "the_complex": ("The Complex", "The Complex", "mystery", "dungeon"),
    "hidden_agenda": ("Hidden Agenda", "Hidden Agenda", "mystery", "mask"),
    "mystery_2": ("Comfortable Mystery 2", "Comfortable Mystery II", "mystery", "sneak"),
    "fluidscape": ("Fluidscape", "Fluidscape", "calm", "wave"),
    "almost_in_f": ("Almost in F", "Almost in F", "calm", "lute"),
    "peaceful_desolation": ("Peaceful Desolation", "Peaceful Desolation", "calm", "fog"),
    "dreamy_flashback": ("Dreamy Flashback", "Dreamy Flashback", "calm", "spark"),
    "meditation_01": ("Meditation Impromptu 01", "Meditation", "calm", "leaf"),
    "air_prelude": ("Air Prelude", "Air Prelude", "calm", "wind"),
    "earth_prelude": ("Earth Prelude", "Earth Prelude", "wild", "mountain"),
    "heartbreaking": ("Heartbreaking", "Heartbreaking", "calm", "lute"),
    "long_note_four": ("Long Note Four", "Long Note Four", "calm", "fog"),
    "frozen_star": ("Frozen Star", "Frozen Star", "mystery", "snow"),
    "thunderbird": ("Thunderbird", "Thunderbird", "heroic", "eagle"),
    "volatile_reaction": ("Volatile Reaction", "Volatile Reaction", "battle", "storm"),
    "relent": ("Relent", "Relent", "battle", "swords"),
    "river_fire": ("River Fire", "River Fire", "wild", "fire"),
    "tabuk": ("Tabuk", "Tabuk", "travel", "dune"),
    "virtutes_instrumenti": ("Virtutes Instrumenti", "Virtutes Instrumenti", "sacred", "temple"),
    "agnus_dei": ("Agnus Dei X", "Agnus Dei", "sacred", "bell"),
    "lost_frontier": ("Lost Frontier", "Lost Frontier", "travel", "path"),
    "magistar": ("Magistar", "Magistar", "mystery", "wizard"),
    "firesong": ("Firesong", "Firesong", "wild", "fire"),
    "desert_city": ("Desert City", "Desert City", "travel", "dune"),
    "jalandhar": ("Jalandhar", "Jalandhar", "travel", "path"),
    "rite_of_passage": ("Rite of Passage", "Rite of Passage", "sacred", "spark"),
    "spy_glass": ("Spy Glass", "Spy Glass", "mystery", "sneak"),
    "deep_haze": ("Deep Haze", "Deep Haze", "mystery", "fog"),
    "immersed": ("Immersed", "Immersed", "sea", "fish"),
    "virtutes_vocis": ("Virtutes Vocis", "Virtutes Vocis", "sacred", "temple"),
    "ossuary_turn": ("Ossuary 2 - Turn", "Ossuary Turn", "dark", "tomb"),
    "ossuary_rest": ("Ossuary 5 - Rest", "Ossuary Rest", "dark", "tomb"),
    "past_the_edge": ("Past the Edge", "Past the Edge", "mystery", "path"),
    "sad_trio": ("Sad Trio", "Sad Trio", "calm", "lute"),
    "ice_flow": ("Ice Flow", "Ice Flow", "calm", "snow"),
    "giant_wyrm": ("Giant Wyrm", "Giant Wyrm", "battle", "dragon"),
    "heroic_adventure": ("Heroic Adventure", "Heroic Adventure", "heroic", "banner"),
    "strength_titans": ("Strength of the Titans", "Strength of the Titans", "battle", "swords"),
    "rallying_defense": ("Rallying the Defense", "Rallying the Defense", "heroic", "banner"),
    "truth_legend": ("Truth of the Legend", "Truth of the Legend", "heroic", "crown"),
    "honor_bound": ("Honor Bound", "Honor Bound", "heroic", "banner"),
    "crossing_chasm": ("Crossing the Chasm", "Crossing the Chasm", "travel", "mountain"),
    "impact_prelude": ("Impact Prelude", "Impact Prelude", "battle", "storm"),
    "the_throne": ("The Throne", "The Throne", "heroic", "crown"),
    "master_feast": ("Master of the Feast", "Master of the Feast", "merry", "lute"),
    "village_consort": ("Village Consort", "Village Consort", "merry", "farm"),
    "woodland_lullaby": ("Woodland Lullaby", "Woodland Lullaby", "calm", "trees"),
    "garden_music": ("Garden Music", "Garden Music", "calm", "leaf"),
    "lone_harvest": ("Lone Harvest", "Lone Harvest", "travel", "farm"),
    "river_flute": ("River Flute", "River Flute", "calm", "river"),
    "dawn_fairies": ("Dawn of the Fairies", "Dawn of the Fairies", "wild", "spark"),
    "welcome_magic": ("Welcome to Magic Land", "Welcome to Magic Land", "mystery", "wizard"),
    "overworld": ("Overworld", "Overworld", "travel", "path"),
    "magic_scout": ("Magic Scout", "Magic Scout", "wild", "leaf"),
    "lightless_dawn": ("Lightless Dawn", "Lightless Dawn", "dark", "moon"),
    "gathering_darkness": ("Gathering Darkness", "Gathering Darkness", "dark", "moon"),
    "unseen_horrors": ("Unseen Horrors", "Unseen Horrors", "dark", "ghost"),
    "parish_lost": ("Parish of Lost Souls", "Parish of Lost Souls", "dark", "bell"),
    "other_side": ("The Other Side of the Door", "The Other Side of the Door", "mystery", "dungeon"),
    "feeling_dark": ("Feeling Dark", "Feeling Dark", "dark", "moon"),
    "colorless_aura": ("Colorless Aura", "Colorless Aura", "mystery", "fog"),
    "numina": ("Numina", "Numina", "sacred", "spark"),
    "ether_vox": ("Ether Vox", "Ether Vox", "mystery", "wizard"),
    "mesmerizing_galaxy": ("Mesmerizing Galaxy", "Mesmerizing Galaxy", "mystery", "spark"),
    "constellations": ("Constellations", "Constellations", "calm", "moon"),
    "luminous_rain": ("Luminous Rain", "Luminous Rain", "calm", "rain"),
    "endless_storm": ("Endless Storm", "Endless Storm", "battle", "storm"),
    "blackmoor_tides": ("Blackmoor Tides", "Blackmoor Tides", "sea", "wave"),
    "walking_poseidon": ("Walking with Poseidon", "Walking with Poseidon", "sea", "wave"),
    "island_music": ("Island Music", "Island Music", "sea", "wave"),
    "voyage": ("Voyage", "Voyage", "sea", "ship"),
    "wagon_wheel": ("Wagon Wheel", "Wagon Wheel", "travel", "horse"),
    "traveller": ("Traveller", "Traveller", "travel", "path"),
    "in_the_west": ("In the West", "In the West", "travel", "dune"),
    "arid_foothills": ("Arid Foothills", "Arid Foothills", "travel", "dune"),
    "deserted": ("Deserted", "Deserted", "mystery", "fog"),
    "exotic_battle": ("Exotic Battle", "Exotic Battle", "battle", "swords"),
    "jungle_chase": ("Jungle Chase", "Jungle Chase", "wild", "jungle"),
    "crowd_hammer": ("Crowd Hammer", "Crowd Hammer", "battle", "anvil"),
    "castle_envy": ("Castle of Envy", "Castle of Envy", "dark", "dungeon"),
    "snow_queen": ("The Snow Queen", "The Snow Queen", "calm", "snow"),
    "daybreak": ("Daybreak", "Daybreak", "calm", "dawn"),
    "lasting_hope": ("Lasting Hope", "Lasting Hope", "heroic", "banner"),
    "tragic_story": ("Tragic Story", "Tragic Story", "calm", "lute"),
    "heart_nowhere": ("Heart of Nowhere", "Heart of Nowhere", "calm", "path"),
    "wounded": ("Wounded", "Wounded", "calm", "moon"),
    "plaint": ("Plaint", "Plaint", "calm", "lute"),
    "private_reflection": ("Private Reflection", "Private Reflection", "calm", "book"),
    "deliberate_thought": ("Deliberate Thought", "Deliberate Thought", "mystery", "book"),
    "art_of_silence": ("Art of Silence", "Art of Silence", "calm", "fog"),
    "wholesome": ("Wholesome", "Wholesome", "merry", "farm"),
    "bittersweet": ("Bittersweet", "Bittersweet", "calm", "leaf"),
    "far_away": ("Far Away", "Far Away", "travel", "path"),
    "vanishing": ("Vanishing", "Vanishing", "mystery", "fog"),
    "undaunted": ("Undaunted", "Undaunted", "heroic", "banner"),
    "chosen": ("Chosen", "Chosen", "heroic", "crown"),
    "destiny_day": ("Destiny Day", "Destiny Day", "heroic", "sun"),
    "to_the_ends": ("To the Ends", "To the Ends", "travel", "path"),
    "silver_flame": ("Silver Flame", "Silver Flame", "sacred", "fire"),
    "divinitus": ("Divinitus", "Divinitus", "sacred", "temple"),
    "transcendence": ("Transcendence", "Transcendence", "sacred", "spark"),
    "tempting_secrets": ("Tempting Secrets", "Tempting Secrets", "mystery", "mask"),
    "malicious": ("Malicious", "Malicious", "dark", "mask"),
    "mystic_force": ("Mystic Force", "Mystic Force", "mystery", "wizard"),
    "automaton": ("Automaton", "Automaton", "mystery", "clock"),
    "cipher": ("Cipher", "Cipher", "mystery", "sneak"),
    "anamalie": ("Anamalie", "Anamalie", "mystery", "fog"),
    "acralate": ("Accralate", "Accralate", "travel", "path"),
    "bit_quest": ("Bit Quest", "Bit Quest", "travel", "path"),
    "the_builder": ("The Builder", "The Builder", "travel", "anvil"),
    "this_house": ("This House", "This House", "mystery", "dungeon"),
    "nightdreams": ("Nightdreams", "Nightdreams", "calm", "moon"),
    "nowhere_land": ("Nowhere Land", "Nowhere Land", "travel", "fog"),
    "dream_culture": ("Dream Culture", "Dream Culture", "calm", "spark"),
    "decision": ("Decision", "Decision", "heroic", "banner"),
    "hero_down": ("Hero Down", "Hero Down", "dark", "banner"),
    "heavy_interlude": ("Heavy Interlude", "Heavy Interlude", "dark", "dungeon"),
    "intended_force": ("Intended Force", "Intended Force", "battle", "swords"),
    "inevitable": ("Inevitable", "Inevitable", "dark", "moon"),
    "finding_movement": ("Finding Movement", "Finding Movement", "travel", "path"),
    "classic_horror": ("Classic Horror 1", "Classic Horror", "dark", "ghost"),
    "gymnopedie": ("Gymnopedie No 1", "Gymnopedie", "calm", "lute"),
    "hall_mountain": ("Hall of the Mountain King", "Hall of the Mountain King", "battle", "mountain"),
    "parting_glass": ("The Parting Glass", "The Parting Glass", "merry", "mug"),
    "minstrel_dance": ("Dances and Dames", "Dances and Dames", "merry", "lute"),
    "run_amok": ("Run Amok", "Run Amok", "merry", "lute"),
    "super_polka": ("Super Polka", "Super Polka", "merry", "lute"),
    "happy_alley": ("Happy Alley", "Happy Alley", "merry", "farm"),
    "fluffing_duck": ("Fluffing a Duck", "Fluffing a Duck", "merry", "bird"),
    "scheming_weasel": ("Scheming Weasel", "Scheming Weasel", "merry", "sneak"),
    "spooky_ride": ("Spooky Ride", "Spooky Ride", "dark", "horse"),
    "split_nemesis": ("Split In Nemesis", "Split In Nemesis", "battle", "swords"),
    "summon_rawk": ("Summon the Rawk", "Summon the Rawk", "battle", "storm"),
    "future_gladiator": ("Future Gladiator", "Future Gladiator", "battle", "swords"),
    "night_chaos": ("Night of Chaos", "Night of Chaos", "battle", "storm"),
    "swamp_stomp": ("Swamp Stomp", "Swamp Stomp", "wild", "swamp"),
    "woodland": ("Woodland", "Woodland", "wild", "trees"),
    "prime": ("Primus Inter Pares", "Primus Inter Pares", "heroic", "crown"),
    "how_it_begins": ("How it Begins", "How it Begins", "heroic", "banner"),
    "on_my_way": ("On My Way", "On My Way", "travel", "path"),
    "take_a_chance": ("Take a Chance", "Take a Chance", "travel", "path"),
    "chasing_daylight": ("Chasing Daylight", "Chasing Daylight", "travel", "sun"),
    "time_passing": ("Time Passing By", "Time Passing", "calm", "clock"),
    "string_impromptu": ("String Impromptu Number 1", "String Impromptu", "calm", "lute"),
    "legrand_library": ("Legrand Library", "Legrand Library", "mystery", "book"),
    "hiding_reality": ("Hiding Your Reality", "Hiding Your Reality", "mystery", "sneak"),
    "evil_plan": ("Evil Plan", "Evil Plan", "dark", "mask"),
    "groaning": ("Groaning", "Groaning", "dark", "ghost"),
    "ghostpocalypse": ("Ghostpocalypse - 6 Crossing the Threshold", "Ghostpocalypse", "dark", "ghost"),
    "supernatural": ("Supernatural", "Supernatural", "dark", "spark"),
    "unpromised": ("Unpromised", "Unpromised", "calm", "fog"),
    "wallflowers": ("Wallflowers", "Wallflowers", "calm", "leaf"),
    "painter": ("Painter", "Painter", "calm", "spark"),
    "minima": ("Minima", "Minima", "calm", "fog"),
    "persiandad": ("Persiandad", "Persiandad", "travel", "dune"),
    "dongfeng": ("Dongfeng", "Dongfeng", "travel", "path"),
    "royal_coupling": ("Royal Coupling", "Royal Coupling", "heroic", "crown"),
    "angevin_b": ("Angevin B", "Angevin B", "sacred", "temple"),
}


def mp3_url(title: str) -> str:
    return "https://incompetech.com/music/royalty-free/mp3-royaltyfree/" + urllib.parse.quote(title) + ".mp3"


def run(cmd: list[str]) -> None:
    subprocess.check_call(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def dur(path: Path) -> float:
    out = subprocess.check_output(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(path)],
        text=True,
    ).strip()
    try:
        return float(out)
    except ValueError:
        return 0.0


def head_ok(url: str) -> bool:
    req = urllib.request.Request(url, method="HEAD", headers={"User-Agent": UA})
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            return 200 <= r.status < 400
    except Exception:
        return False


def download(url: str, dest: Path) -> bool:
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.exists() and dest.stat().st_size > 4000:
        return True
    req = urllib.request.Request(url, headers={"User-Agent": UA})
    try:
        with urllib.request.urlopen(req, timeout=120) as r, open(dest, "wb") as f:
            while True:
                chunk = r.read(1024 * 256)
                if not chunk:
                    break
                f.write(chunk)
        return dest.stat().st_size > 2000
    except Exception as e:
        print("  fail", dest.name, e)
        dest.unlink(missing_ok=True)
        return False


def loop_ogg(src: Path, dest: Path, max_dur: float, xf: float, loud: str) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    wav = dest.with_name(dest.stem + ".prep.wav")
    take = min(max_dur, max(dur(src) - 0.05, 1.0))
    xf = min(xf, take / 3)
    run(
        [
            "ffmpeg", "-y", "-i", str(src),
            "-af", f"{loud},atrim=0:{take:.3f},asetpts=PTS-STARTPTS",
            "-ar", "44100", "-ac", "2", str(wav),
        ]
    )
    d = dur(wav)
    looped = dest.with_name(dest.stem + ".loop.wav")
    if d > xf * 2 + 0.4:
        head = d - xf
        run(
            [
                "ffmpeg", "-y", "-i", str(wav),
                "-filter_complex",
                f"[0]atrim=0:{head:.3f},asetpts=PTS-STARTPTS[h];"
                f"[0]atrim={head:.3f},asetpts=PTS-STARTPTS[t];"
                f"[t][h]acrossfade=d={xf:.3f}:c1=tri:c2=tri",
                str(looped),
            ]
        )
        wav.unlink()
        wav = looped
    run(["ffmpeg", "-y", "-i", str(wav), "-c:a", "libvorbis", "-q:a", "5", str(dest)])
    wav.unlink(missing_ok=True)
    looped.unlink(missing_ok=True)


def main() -> int:
    RAW.mkdir(parents=True, exist_ok=True)
    AUDIO.mkdir(parents=True, exist_ok=True)

    wanted = {k: v for k, v in TRACKS.items() if v[0] not in SKIP_TITLES}
    print(f"== probe {len(wanted)} ==")
    ok = {}
    with ThreadPoolExecutor(max_workers=12) as pool:
        futs = {pool.submit(head_ok, mp3_url(v[0])): k for k, v in wanted.items()}
        for fut in as_completed(futs):
            key = futs[fut]
            title = wanted[key][0]
            if fut.result():
                ok[key] = wanted[key]
                print("  OK", title)
            else:
                print("  miss", title)
    print(f"OK {len(ok)} / miss {len(wanted) - len(ok)}")

    print("== download ==")
    raws = {}
    with ThreadPoolExecutor(max_workers=6) as pool:
        futs = {}
        for key, (title, *_rest) in ok.items():
            dest = RAW / f"{key}.mp3"
            futs[pool.submit(download, mp3_url(title), dest)] = (key, dest)
        for fut in as_completed(futs):
            key, dest = futs[fut]
            good = fut.result()
            print(" ", "got" if good else "miss", key)
            if good:
                raws[key] = dest

    print("== convert ==")

    def convert_one(key: str, name: str, mood: str, icon: str) -> tuple[str, tuple | None]:
        dest = AUDIO / f"{key}.ogg"
        if dest.exists() and dest.stat().st_size > 4000:
            return f"skip {key}", (key, name, mood, icon)
        raw = raws.get(key)
        if not raw:
            return f"miss convert {key}", None
        try:
            loop_ogg(raw, dest, 68, 2.6, "loudnorm=I=-16:LRA=11:TP=-1.5")
            return f"ok {key} {dest.stat().st_size // 1024}kb", (key, name, mood, icon)
        except Exception as e:
            dest.unlink(missing_ok=True)
            return f"fail convert {key} {e}", None

    converted = []
    with ThreadPoolExecutor(max_workers=4) as pool:
        futs = [
            pool.submit(convert_one, key, name, mood, icon)
            for key, (_title, name, mood, icon) in ok.items()
        ]
        for fut in as_completed(futs):
            msg, row = fut.result()
            print(" ", msg)
            if row:
                converted.append(row)

    catalog = ROOT / "tools" / "_music_catalog.json"
    import json
    catalog.write_text(json.dumps(converted, indent=2))
    print("wrote", catalog, "n=", len(converted))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
