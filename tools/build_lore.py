#!/usr/bin/env python3
"""Night City 2045 street lore. Original table dossiers, not book copy."""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path("/home/steelworth/projects/blightnet")
OUT = ROOT / "data" / "lore.json"

# id, name, kind, blurb, text, hooks
LORE = [
    (
        "birds",
        "The Empty Sky",
        "sky",
        "Ask a kid in Heywood what a bird is. They will point at a delivery drone, or a kibble mascot, or nothing.",
        """Night City still prints birds on ads. Songbirds on insomnia meds. Gulls on fish-paste. A chrome hawk on a Nomad panzer. The sky itself does not return the favor.

They did not leave all at once. First the insects went thin, because CHOOH2 wheat is a desert that looks like a farm: one crop, one pesticide, one corporation's seed. No bugs, no bait. Then the migratory routes crossed burn-off from the 4th Corporate War — fuel dumps, glassed yards, the kind of air that strips keratin. Then the Hot Zone taught every surviving flock that this metro is a bad idea. Acid rain finishes the lesson. Feathers do not like pH that belongs in a battery.

What you get instead: rats the size of a small dog in the storm drains. Roaches that have learned to nest in abandoned linear frames. Combat dogs with too much chrome and not enough handler. At Pacifica you might still see a gull. Do not eat it. Do not touch it. The docks say they glow wrong at night. The docks are not being poetic.

Corpos keep songbirds. Not as a public good. As a flex. A sealed rooftop aviary over Corpo Plaza costs more per year than a Watson cube. The birds up there have never been outside. They sing on a timer. If you are invited, you are being shown that this person can afford a sky.

Street myth says Biotechnica has a vault of eggs in the Badlands and will thaw a species when the brand needs a mascot. Street myth is sometimes a leaked catalog.

At the table: a wild bird is an omen because it is rare, not because it is magic. A kid who has never seen one will follow it into a bad alley. A corpo will pay stupid money for a live one. A ganger will shoot it for the story.""",
        [
            "A live sparrow in a Watson pet-shop tank. The shop says 'vintage.' Three corpos and a Nomad shaman have already made offers.",
            "Delivery drones over Heywood have started flocking like starlings. Someone loaded bird-behavior into the mesh. The mesh is teaching the street.",
        ],
    ),
    (
        "meat",
        "How the Meat Is Made",
        "food",
        "Steak is a status report. Everything else is a process.",
        """If it is cheap, it was never a cow. If it is a cow, you are in a room with a dress code.

The stack, bottom to top:

Kibble is the floor. Pressed protein — soy, krill, algae, recovered animal, the week's surplus from a vat — extruded into bricks, pellets, cups of wet mash. It is dyed. It is flavored. It is not lying when the packet says 'chicken.' It is also not telling a story you would want at dinner. A Watson family can live on kibble. A lot of them do. The aftertaste is the same in every district. That is the point. Biotechnica and a half-dozen licensees own the recipes. Night Market knockoffs taste like the factory next door burned.

Scoop is the next step up: hot protein from a stall, shaved or ladled, spices loud enough to hide the source. Sometimes it is vat. Sometimes it is kibble re-melted. Sometimes it is an animal that was not on any license. Street vendors who last a year have a fixer. Street vendors who do not, have a story the NCPD files under 'food.'

Vat meat is the honest middle. Grown in tanks, printed in sheets, marbled by machine. Biotechnica's good lines can fool a tourist. The cheap lines have a grain like insulation. Hospitals buy vat because it is sterile. Ripperdocs buy it because patients still have to eat. A real restaurant in Japantown will serve vat and charge as if it had a mother.

Real meat — muscle that walked — is a luxury SKU. It comes from sealed ranch arcs in the Badlands, from Nomad herds that still bother, from black-market hunts, from corpo cafeterias that want to remind the juniors who they are. A steak dinner is a meeting. The animal had a name on a tablet. You are eating a spreadsheet with blood in it.

Body banks are not restaurants. Anyone who jokes otherwise has not been in one. That said, Night City has always had people who will process anything that used to move if the eddies are right. Trauma Team's leftovers go to research, to chrome salvage, to a furnace. The furnace is the version you should hope for.

At the table: 'we eat' is a scene. A kibble brick says poverty. A vat burger says the fixer is not showing off. A real steak means someone is recruiting, threatening, or celebrating a corpse. If the meat is sweet in a way food should not be, the party has a problem that is not culinary.""",
        [
            "A Kabuki stall's scoop tested as human-adjacent. The vendor is gone. The recipe is still selling two streets over.",
            "A Biotechnica exec will pay for a live cow smuggled into City Center. Not for food. For a boardroom stunt. The cow will not survive the metaphor.",
        ],
    ),
    (
        "chooh2",
        "What They Pump",
        "fuel",
        "The cars do not drink gasoline. Gasoline is a history lesson and a fire hazard.",
        """CHOOH2 is the blood in the street. Synthetic fuel, alcohol-family, grown more than drilled. Wheat and other agri go in. A fuel that will run a Quadra, a panzer, a backup generator, and a bad decision comes out. Petrochem and SovOil fight over who owns the pumps. Biotechnica owns too much of what goes into the mash. You can smell a station from a block away: sweet-sharp, like a distillery that also sells tires.

Night City pumps are color-coded if anyone has bothered to keep the paint. Street knowledge is better. Gold-grade CHOOH2 for anything with a turbo and a pride problem. City-grade for cabs and gig-heaps. Red-cut for anything a ganger will set on fire later. Cut fuel is the city's other industry: water, methanol, spite. A Nomad can smell a cut. A corpo kid learns when the engine knocks on the freeway.

Old gasoline still exists in drums, in museums, in the kind of bunker a 2020 holdout never opened. Pour it in a 2045 engine and you are buying a rebuild. Pour it on a street and you are buying a riot. The fire department bills in eddies.

Nomad stills are a theology. Clans run mash in the Badlands, hide the smell under livestock and dust, and run cleaner fuel than some licensed stations because their lives are in the tank. A clan that sells to the city has a Petrochem problem. A clan that does not sell has a Petrochem problem later.

AV turbines drink a hotter mix. Trauma Team does not buy off a street pump. If you see an AV topping from a civilian station, it is stolen, dying, or about to become a crime scene.

At the table: fuel is a clock. A chase ends when the tank does. A Nomad favor is a full jerry. A corpo can always fill up; that is part of what they bought. If the pumps in a district go dry, it is not a shortage. It is a message.""",
        [
            "Every pump in Santo Domingo is selling sweet. Engines are cooking. Petrochem says sabotage. The Nomads say the mash was never wheat.",
            "A jerry of pre-war gasoline is in a Watson storage unit. Three collectors and one pyromaniac already know.",
        ],
    ),
    (
        "water",
        "What Comes Out of the Tap",
        "water",
        "You do not drink the tap. You negotiate with it.",
        """Night City water is a utility, a racket, and a medical event.

The official story is that the water is treated. The official story has a logo. In Corpo Plaza it tastes like nothing, which is the luxury. In Watson it tastes like pipes. In the Combat Zone it tastes like a dare. Filters are furniture. A cube without a filter pitcher is a cube that has given up. Boiling helps the biology. It does not help the metals.

Bottled water is a class marker. The cheap bottles are tap from a better zip code. The expensive bottles claim glacier, claim orbital, claim a process you need a degree to pronounce. Some of it is true. A lot of it is a label.

Showers are timed in poor stacks. In corpo towers they are a mood. Public fountains are for pigeons that do not exist and for people who will regret it. The storm drains are a second city. They overflow in the acid rain. They hide bodies, cables, and the occasional chapel.

Water riots are in living memory. Turn off a district for a week and you do not get a protest. You get a war with buckets. The NCPD knows this. So do the gangs. So do the corpos who own the treatment plants. A leak in a main is a political object.

At the table: thirsty NPCs are not flavor. They are a clock. A fixer who offers 'clean water for a month' is offering a lifestyle. If the party's hideout tap runs brown after a job, someone upstairs noticed them.""",
        [
            "A Watson stack's filter subscription was cancelled from a corpo desk. The stack is three days from a riot. The desk will not take calls.",
            "Someone is selling 'orbital ice' that tests as condensed AC runoff from a Militech tower. It is still cleaner than the tap. The tower would like the dripping to stop.",
        ],
    ),
    (
        "air",
        "The Mix You Breathe",
        "air",
        "The sky is a color. The color is not blue. Your lungs have opinions.",
        """Night City air is a product. On a good day it is smog with a view. On a bad day it is a medical advisory and a street full of paper masks that do not work.

Red dust blows in from the Badlands when the wind is wrong. Industrial burn from Santo Domingo when the wind is ruder. The Hot Zone contributes a taste people describe as 'batteries and old teeth.' Acid rain is not a metaphor. It etches cars, billboards, the cheap chrome on a street samurai who skipped the sealant. Umbrellas are plastic for a reason. Cotton is a souvenir.

Filter masks are fashion in some districts and survival in others. Corpo towers condition the air like they condition the staff: cold, dry, scented with something that means money. Street clinics sell inhalers that are half medicine, half habit. A Medtech can tell where you live from a sputum sample. They usually do not say so.

Cigarettes still exist. Of course they do. The city that poisoned the sky was not going to lose a vice. Kiroshi makes eyes that compensate for haze. They cannot compensate for a lifetime.

At the table: weather is not small talk. A red-dust day grounds AVs and shortens gunfights because nobody can see a rooftop. Acid rain is a timer on exposed electronics and on unprotected skin. If an NPC takes off a mask to speak, they are either corpo-safe or making a point.""",
        [
            "A dust storm is sitting on Santo Domingo. A job is on a roof. The client will not reschedule. The roof has no rail.",
            "A street clinic's inhaler batch was cut with something that makes users loyal to a jingle. The jingle is a corp. The corp is not Biotechnica. That is the interesting part.",
        ],
    ),
    (
        "kibble",
        "Kibble",
        "food",
        "The daily brick. The taste of not dying. The aftertaste of the city.",
        """Kibble is not a brand. It is a category, like 'rent' or 'bullet.' You can buy it in a cup, a brick, a bag the size of a child, a vending slot that takes eddies and sometimes takes a finger.

Flavor wheels are a joke everyone is in on. 'Ranch.' 'Yuzu.' 'Night Market Fire.' 'Nostalgia Chicken.' The base is the same: compressed protein, binder, salt enough to make you buy water, a vitamin premix so the city does not have a scurvy headline. Kids grow up on it. Some corpos were raised on it and will not admit it. A Nomad eating kibble in the city is either broke or making fun of you.

The factories are in Santo Domingo and the Badlands fringe. They smell like a wet wallet. Shifts are long. Fingers go into the mix more often than the safety poster allows. The poster is still up.

There is gourmet kibble now. Of course there is. Japantown will texturize it, sear it, serve it on a stone, and charge as if the brick had a childhood. It is still kibble. The stone is the product.

At the table: a hideout with a pallet of kibble is a hideout that planned. A hideout with none is a hideout that will be out in three days. Gangs tax kibble shipments because you cannot eat a reputation. If a flavor disappears city-wide, a factory died or a corp is teaching a lesson.""",
        [
            "A limited 'Blue Sky' flavor sold out in an hour and people who ate it dreamed of birds. Biotechnica denies the batch. The dreams have a map in them.",
            "A kibble plant's night shift walked out. The vats are still running. Something in vat four is not on the ingredient list and is trying to stand up.",
        ],
    ),
    (
        "trash",
        "Where the City Puts the Rest",
        "street",
        "Night City does not throw things 'away.' It throws them down, out, or at someone poorer.",
        """Garbage is a district. The official landfills are Badlands scars, fenced, taxed, on fire when the methane mood is right. The unofficial ones are the Combat Zone, the storm drains, any vacant lot that lost a lawsuit.

Chrome goes to scavs before it goes to a dump. A dead linear frame is a buffet. A crashed AV is a riot. Body banks take the wet parts. Tech takes the dry. What is left is plastic, slag, and the kind of battery that will outlive the people who dumped it.

Night Markets sell the city's leftovers with better lighting. A ganger's 'new' pistol may have been a Trauma Team sidearm last week. Serial numbers are a prayer.

The ocean takes what the drains do not. Pacifica's shore is a museum of bags, drones, and the occasional sealed barrel nobody should open. Nomads run junk trains. They are the closest thing the metro has to a recycling ethic, and they will still leave a husk if the clan is moving.

At the table: a dump is a dungeon. It has factions (scavs, animals, a corpo cleanup crew with flamethrowers). It has treasure (the thing someone was stupid enough to throw). It has tetanus as a wandering monster. If the party needs to disappear a van, this is the theology.""",
        [
            "A corpo trash-barge dumped in Pacifica instead of the licensed pit. The barrels are ticking. The neighborhood already opened one.",
            "A scav king will trade a working linear frame for the party's help keeping Militech's cleanup squad out of his pile for one night.",
        ],
    ),
    (
        "net",
        "After the NET",
        "net",
        "The old NET is a ruin with teeth. What you jack into now is smaller, meaner, and not yours.",
        """Rache Bartmoss burned the architecture. That sentence is a tombstone and a business model. The global NET of the 2020s is a haunted house. Black ICE still patrols wreckage. Netwatch will tell you not to go. Netrunners still go, in teams, on a timer, for a file that might be a myth.

2045's working net is local: corporate subnets, city grids, shard-sized pockets, the kind of 'cloud' that is actually a room in a basement with a gun on the door. You do not 'go online.' You go into a place. The place has a landlord.

Shards are how civilians carry a piece of the ghost. A chip in the neck, a message, a skill you did not earn, a virus that smiles. The street treats shards the way it treats ammo: labeled, unlabeled, and 'trust me.'

The Blackwall, in the stories, is the fence between what is left of the old NET and the things that grew in the fire. Night City netrunners argue about whether it is a wall, a god, or a Netwatch bedtime story. The ones who have touched it do not argue. They shake.

At the table: a netrun is a dungeon in another physics. The meat body is a hostage in a chair. If the runner dies in the net, the table should feel it in the room. If they come back with a file that looks at them, the job is not over.""",
        [
            "A shard labeled 'weather' is making people remember a sky with birds. Netwatch wants it. A Bartmoss cult wants it more.",
            "A local subnet in Kabuki started answering to a dead runner's handle. The handle is offering jobs. The jobs are real. The pay is in a currency that should not exist.",
        ],
    ),
    (
        "chrome",
        "The Cost of More Than Meat",
        "chrome",
        "Chrome is a tool, a fashion, a debt, and a hole where a person used to fit.",
        """You can live in Night City without chrome. You will be slower, blinder, and hired last. The street assumes a little metal: a link, a nail, a cheap cybereye that still shows ads. Real chrome is a clinic, a second mortgage, and a conversation with your remaining humanity.

Ripperdocs are the priests. The good ones keep a clean chair, a named inventory, and a rule about how much they will put in you in one sitting. The bad ones will put a cannon in a teenager because the eddies cleared. Humanity is not a vibe. It is a meter the table can see: empathy going, tempers shortening, the moment a person starts treating meat like furniture.

Black market chrome has no warranty and a history. That arm belonged to someone. The someone may still want it. Trauma Team salvages the expensive stuff off the dying when the contract allows, and sometimes when it does not.

Linear frames, full borg conversions, the legend-tier bodies — those are corpo toys and war stories. A person who has replaced that much of themselves is a faction. Treat them like one.

At the table: every implant is a plot hook. Maintenance. Rejection. A recall. A backdoor. A scav who recognized the serial. If a player wants to be more gun than person, the world will let them. It will also start billing them in ways that are not eddies.""",
        [
            "A used Sandevistan is for sale in Kabuki. It still twitches toward a building in City Center at noon.",
            "A ripperdoc's chair is empty and the waiting room is full of people who all have the same new hand. The hand has a team-up instinct. The doc is in the back, not blinking.",
        ],
    ),
    (
        "trauma",
        "Who Comes When You Bleed",
        "war",
        "Trauma Team is not an ambulance. It is a private army with a defibrillator and a bill.",
        """If you have a card, they come. If you do not, they fly over you on the way to someone who does. That is the whole ethic, painted red and white, loud enough to rattle a stack.

An AV-4 over a street means someone paid. It also means gunfire, because the Team shoots a landing zone the way a ganger shoots a doorway. Civilians learn to get down. NCPD learns to look busy. Gangs learn which rooftops are not worth the heat.

The medicine is excellent. The invoice is a weapon. A corpo junior will go into debt for a revival. A street kid will die in an alley under the same sky as the AV. Medtechs on the Team are some of the best in the city. They are also some of the most tired. They have seen the inside of too many famous people.

There are knockoffs. Street docs with a siren. Gangs with a stolen crash cart. They will save you. They will also own a piece of you.

At the table: a Trauma Team arrival is a scene change. Time slows, then explodes. If the party has cards, they have a panic button with consequences (cops, cameras, a corp knowing they were there). If they do not, a dying PC is a choice: ripperdoc sprint, stolen AV, or a speech.""",
        [
            "A Team AV went down in the Combat Zone. The crew is alive. The cardholders they were flying toward are not. Both sides of the wreck have salvage rights in mind.",
            "Someone is forging Trauma cards. The Team wants it stopped without a press cycle. The forger is saving people the official city had already priced out.",
        ],
    ),
    (
        "nuke",
        "The Hole",
        "war",
        "2023 is not history class. It is a fence, a cough, and a skyline with a missing tooth.",
        """Arasaka Tower went up in a way that taught the city a new kind of quiet. The nuke was small as these things are billed. It was not small in the people. Night City 2045 is a reconstruction that never finished, a Hot Zone that still ticks, a generation that can point to the empty place in the skyline without looking.

The Hot Zone is fenced, posted, ignored by anyone with a death wish and a Geiger. Scavs go in. So do kids on a dare. So do corpos with a cleanup contract and a disposable crew. What you bring out is radioactive, haunted, or both. What you leave is another story the fence was supposed to stop.

Reconstruction money built new glass next to old burn. Some districts got towers. Some got a speech. Pacifica got a promise and then a war. The city's personality — loud, armed, trying to look rich — is in part a refusal to sit shiva for itself.

Veterans of the 4th Corporate War are in every bar that still has stools. They will tell you who dropped it if you buy the bottle. They will tell you a different name if you buy the next one.

At the table: the Hole is a dungeon with a dress code (suits, dosimeters, a reason). Fallout is a timer, not a vibe. NPCs have opinions about Arasaka that will get the party shot if they pick the wrong booth.""",
        [
            "A tour company is running 'authorized' Hot Zone looks. Their last group did not come back. The fence camera shows them walking in. It does not show them on the ground.",
            "A piece of the old tower is for sale on a Night Market. It still hums. Arasaka wants it. A veteran wants it buried. A netrunner says it is a shard.",
        ],
    ),
    (
        "nomads",
        "The Roads Between",
        "road",
        "The city thinks it is the world. The clans know the world is the highway, and the city is a rest stop with a body count.",
        """Nomads are not 'homeless with cars.' They are families, companies, nations on treads. They run salvage, smuggling, protection, long-haul, the kind of agriculture that still looks like a plant. A clan has a name, a paint, a grudge, and a map that is not on any corpo GPS.

Night City needs them. Food, CHOOH2 mash, spare parts, the ability to leave. The city also hates them, because a people who can leave cannot be rented. NCPD shakes them down at the gates. Corps hire them and then try to brand the hire. Gangs steal a panzer and learn what a clan funeral costs.

A pack in the Badlands is a moving town: stills, clinics, a school in a trailer, guns on the roof because the road has animals and worse. They will trade. They will not be charity. If you are useful, you can sit at the fire. If you are a tourist, you can sit on the hood and think about the walk back.

At the table: a Nomad ally is logistics. Fuel, a ride, a place the city cannot easily bomb. A Nomad enemy is a campaign. They will not fight fair in your alley. They will wait on the only road out.""",
        [
            "A clan's child was taken at a Night City gate 'for processing.' The clan is parked outside the wall and the engines are on. They will wait until sundown. Then they will not wait.",
            "Petrochem wants a still. The clan will burn the still before they sell it. They will hire the party to make the burn look like an accident Petrochem caused.",
        ],
    ),
    (
        "rain",
        "The Rain That Eats",
        "weather",
        "When it rains in Night City, you do not go for a walk. You go for a lid.",
        """Acid rain is the city's other clock. It comes in off a sky that already had opinions. It etches paint, cheap chrome, the eyes of anyone who looks up too long. Billboards scab. AVs stay higher. Street chrome without sealant pits in a season.

People own rain-kit the way they own locks: ponchos, plastic umbrellas, coated jackets, a spare for the one that will melt. Fashion rainwear exists. It is a dare. Corpo towers have canopies that cost more than a block of Watson. The Combat Zone has doorways.

The rain is useful. It washes blood to the drains. It hides a gunshot if the thunder is in a good mood. It shorts a drone. It makes every neon sign a weapon of reflection. Netrunners hate it because rooftop dishes get stupid. Solos hate it because footing is a skill check.

At the table: call the rain when you want the street to change physics. Footprints. Shorted cameras. A chase that cannot use rooftops. An NPC who will not come out, which tells you they are either poor or smart.""",
        [
            "A storm is eating a Kabuki billboard. Under the ad is an older ad, and under that a map. The rain will take the map in an hour.",
            "Acid rain is worse in one alley than the forecast. Someone is venting from a clinic. The clinic's chimney is a person.",
        ],
    ),
    (
        "coffins",
        "Where the City Sleeps",
        "sleep",
        "A bed is a luxury SKU. Most of the city rents a box, an hour, or a chemical.",
        """Capsule hotels — coffins — are the honest version of Night City rest. A tube, a shutter, a vent that sometimes works, a screen that never shuts up unless you pay. You can lock it. You should. You can sleep. You will not dream well.

Cubes are the next step: a room that is a room. Hotplate, shower if the stack is kind, a door that is not a shutter. Families stack in them. Gangs tax them. Landlords are a combat role.

Corpo housing is a benefit and a leash. The apartment is nice. The lease has a clause about your chrome, your mouth, your next of kin. Designer sleep — hypno, braindance, a shard that plays a beach — is how people who cannot afford a window buy a sky. It is also how advertisers get into the last private room you had.

Insomnia is a district-wide condition. The city does not clock out. Neither do the ads. Sleep guns and street sedatives are a Night Market aisle. Overdose is a neighbor.

At the table: where the party sleeps is a statement. A coffin row is a target-rich hideout. A cube with a name on the buzzer is a home, which means it can be hit. A corpo suite is a camera. If nobody in the party has slept in two days, the next roll should know.""",
        [
            "A coffin stack's shutters locked with people inside. The vendor wants them out before the news. The people inside are humming a jingle in unison.",
            "A braindance of a perfect night's sleep is selling out. Users wake up missing a day. The missing day was used to walk somewhere. The somewhere is a warehouse.",
        ],
    ),
    (
        "rats",
        "The Real Wildlife",
        "street",
        "The birds left. The rats filed a claim on the vacancy.",
        """Night City fauna is a list you do not want on a shirt: rats, roaches, feral dogs, the occasional combat-animal that slipped a leash, carp in the poisoned canals that should not be alive and are.

The rats are the success story. They eat kibble spill, vat runoff, each other, and the fingers of people who sleep on the wrong grate. Some are the size of a cat. Some are the size of a rumor. Scavs swear a pack in the Combat Zone has a king with a chrome tooth. Scavs swear a lot of things. The tooth was for sale last week.

Roaches nest in warm chassis. A parked linear frame is a condo. Techs who skip the seal find out. Dogs are everywhere the gangs are: cheap alarm, cheap love, cheap weapon. A well-trained dog in 2045 is a status symbol that can still look at you without a HUD.

There are zoos in catalogs. There are not zoos you take a child to without a press pass. Biotechnica's animal vaults are a rumor with a fence. Nomads still have horses in the deep Badlands. A horse in Night City is a parade or a crime.

At the table: animals are the city's honest citizens. They do not have a brand. A rat swarm is a wandering monster that does not care about your street cred. A dog can be an NPC. If you put chrome in an animal, you have made a statement the animal cannot consent to. Someone at the table should notice.""",
        [
            "A pack of drain-rats is avoiding one tunnel. The tunnel has food. That is why it is interesting.",
            "A corpo's 'therapy dog' is a cloned extinct breed. It got out. Biotechnica wants it back alive. Animal lovers want it free. The dog wants a sky with nothing in it.",
        ],
    ),
    (
        "bodybank",
        "The Cold Inventory",
        "death",
        "Night City prices a body the way it prices a car: year, parts, and whether it still runs.",
        """Body banks are legal in the way a lot of Night City is legal: licensed, logged, and standing next to something that is not. They take organs, chrome, skin, the rare still-viable pair of eyes. They pay. They pay less if you cannot argue. They pay more if the parts are corpo-grade.

The drawers are cold. The tags are honest on a good day. A Medtech can walk the aisles like a supermarket. A civilian should not. Trauma Team has a pipeline. So do scavs. So do ripperdocs who will not ask a name. The furnace is the ethical version. Not every bank has one that is on.

Cortical transfer, soul-in-a-chip, the legend of living forever in a mainframe — that is corpo myth, netrunner religion, and a scam with excellent lighting. 2045 does not sell immortality at a kiosk. It sells the rumor. People still buy the rumor.

Funerals exist. They are expensive. Most of the street gets a listing, a drawer, or a fire. Nomads bury their own if the ground will take them. Gangers tag a wall. Corpos get a garden that is a brand.

At the table: a body bank is a dungeon, a shop, and a moral test. The party can buy a liver. They can also find a friend's tattoo on a tray. If you let players treat people as loot, the city will agree with them. Decide if you want that to feel like winning.""",
        [
            "A drawer tagged with a PC's alias is occupied. The face is close. The chrome is closer. The bank will sell. A fixer says do not buy.",
            "Someone is emptying a bank at night and leaving the chrome. They only take the meat. A cult, a chef, or a clinic is the betting pool.",
        ],
    ),
]


def main() -> None:
    rows = []
    for i, (lid, name, kind, blurb, text, hooks) in enumerate(LORE):
        rows.append(
            {
                "id": lid,
                "name": name,
                "kind": kind,
                "blurb": blurb,
                "text": text.strip(),
                "hooks": hooks,
                "sort": i,
            }
        )
    OUT.write_text(json.dumps(rows, ensure_ascii=False, indent=2) + "\n")
    print(f"wrote {len(rows)} lore files")


if __name__ == "__main__":
    main()
