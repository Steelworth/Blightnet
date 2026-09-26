//! Blight's own stations, plus live streams that publish their own address.

#[derive(Clone, Copy)]
pub struct Station {
    pub id: &'static str,
    pub call: &'static str,
    pub freq: &'static str,
    pub name: &'static str,
    pub mood: &'static str,
    pub needle: &'static str,
    /// None plays files already on this deck. Some is a live stream.
    pub url: Option<&'static str>,
}

const LOCAL: &[Station] = &[
    Station { id: "rebellious", call: "RIOT", freq: "104.4", name: "Riot FM", mood: "rebellious", needle: "", url: None },
    Station { id: "melancholic", call: "GLOOM", freq: "91.3", name: "Gloom Wire", mood: "melancholic", needle: "", url: None },
    Station { id: "relaxing", call: "DUSK", freq: "96.1", name: "Dusk Channel", mood: "relaxing", needle: "", url: None },
    Station { id: "brutal", call: "RAVE", freq: "88.1", name: "Warehouse", mood: "brutal", needle: "", url: None },
    Station { id: "afterlife", call: "AFTER", freq: "107.9", name: "Afterlife", mood: "melancholic", needle: "night|club|neon|ether", url: None },
    Station { id: "bodyheat", call: "HEAT", freq: "102.2", name: "Body Heat", mood: "relaxing", needle: "chill|wave|lounge|flow", url: None },
    Station { id: "trauma", call: "TRAU", freq: "89.7", name: "Trauma Tunes", mood: "brutal", needle: "metal|rock|burn|aggress", url: None },
    Station { id: "netwatch", call: "WATCH", freq: "95.5", name: "NetWatch", mood: "rebellious", needle: "cyber|digital|net|bit|shift", url: None },
    Station { id: "combatz", call: "ZONE", freq: "90.1", name: "Combat Zone", mood: "brutal", needle: "rave|dance|edm|laser|trance", url: None },
    Station { id: "pacifica", call: "PACI", freq: "97.3", name: "Pacifica", mood: "relaxing", needle: "cloud|beauty|equator|ambler", url: None },
    Station { id: "watson", call: "WAT", freq: "101.7", name: "Watson Drive", mood: "rebellious", needle: "ninja|space|fighter|ace", url: None },
    Station { id: "chromeam", call: "CHRM", freq: "66.0", name: "Chrome AM", mood: "melancholic", needle: "horizon|groove|lemon|wave", url: None },
    Station { id: "morro", call: "MORR", freq: "103.5", name: "Morro Rock", mood: "brutal", needle: "rock|high|ace|burn", url: None },
    Station { id: "samizdat", call: "SAMI", freq: "94.2", name: "Samizdat", mood: "rebellious", needle: "reform|shift|blip|bit", url: None },
    Station { id: "ritual", call: "RIT", freq: "98.8", name: "Ritual FM", mood: "melancholic", needle: "waltz|newer|brain|night", url: None },
    Station { id: "growl", call: "GROWL", freq: "106.6", name: "Growl FM", mood: "brutal", needle: "monster|energy|pack|rave", url: None },
];

fn live(id: &'static str, call: &'static str, name: &'static str, url: &'static str) -> Station {
    Station { id, call, freq: "LIVE", name, mood: "", needle: "", url: Some(url) }
}

pub fn all() -> Vec<Station> {
    let mut v = LOCAL.to_vec();
    v.extend([
        live("soma-groove", "GROOVE", "SomaFM Groove Salad", "https://somafm.com/groovesalad.pls"),
        live("soma-gsclassic", "GSC", "SomaFM Groove Salad Classic", "https://somafm.com/gsclassic.pls"),
        live("soma-drone", "DRONE", "SomaFM Drone Zone", "https://somafm.com/dronezone.pls"),
        live("soma-deep", "DEEP", "SomaFM Deep Space One", "https://somafm.com/deepspaceone.pls"),
        live("soma-space", "SPACE", "SomaFM Space Station", "https://somafm.com/spacestation.pls"),
        live("soma-secret", "AGENT", "SomaFM Secret Agent", "https://somafm.com/secretagent.pls"),
        live("soma-defcon", "DEFCON", "SomaFM DEF CON", "https://somafm.com/defcon.pls"),
        live("soma-trip", "TRIP", "SomaFM The Trip", "https://somafm.com/thetrip.pls"),
        live("soma-beat", "BEAT", "SomaFM Beat Blender", "https://somafm.com/beatblender.pls"),
        live("soma-dub", "DUB", "SomaFM Dub Step Beyond", "https://somafm.com/dubstep.pls"),
        live("soma-cliq", "CLIQ", "SomaFM Cliqhop", "https://somafm.com/cliqhop.pls"),
        live("soma-fluid", "FLUID", "SomaFM Fluid", "https://somafm.com/fluid.pls"),
        live("soma-vapor", "VAPOR", "SomaFM Vaporwaves", "https://somafm.com/vaporwaves.pls"),
        live("soma-doomed", "DOOM", "SomaFM Doomed", "https://somafm.com/doomed.pls"),
        live("soma-u80s", "U80", "SomaFM Underground 80s", "https://somafm.com/u80s.pls"),
        live("soma-lush", "LUSH", "SomaFM Lush", "https://somafm.com/lush.pls"),
        live("soma-indie", "INDIE", "SomaFM Indie Pop Rocks", "https://somafm.com/indiepop.pls"),
        live("soma-pop", "POP", "SomaFM PopTron", "https://somafm.com/poptron.pls"),
        live("soma-sonic", "SONIC", "SomaFM Sonic Universe", "https://somafm.com/sonicuniverse.pls"),
        live("soma-lounge", "LOUNGE", "SomaFM Illinois Street Lounge", "https://somafm.com/illstreet.pls"),
        live("soma-70s", "70S", "SomaFM Left Coast 70s", "https://somafm.com/seventies.pls"),
        live("soma-metal", "METAL", "SomaFM Metal Detector", "https://somafm.com/metal.pls"),
        live("soma-covers", "COVER", "SomaFM Covers", "https://somafm.com/covers.pls"),
        live("soma-folk", "FOLK", "SomaFM Folk Forward", "https://somafm.com/folkfwd.pls"),
        live("soma-digital", "DIGI", "SomaFM Digitalis", "https://somafm.com/digitalis.pls"),
        live("soma-thistle", "THIST", "SomaFM Thistle", "https://somafm.com/thistle.pls"),
        live("soma-boot", "BOOT", "SomaFM Boot Liquor", "https://somafm.com/bootliquor.pls"),
        live("soma-goa", "GOA", "SomaFM Suburbs of Goa", "https://somafm.com/suburbsofgoa.pls"),
        live("soma-reggae", "REGGAE", "SomaFM Heavyweight Reggae", "https://somafm.com/reggae.pls"),
        live("soma-mission", "MISSION", "SomaFM Mission Control", "https://somafm.com/missioncontrol.pls"),
        live("soma-syn", "SYN", "SomaFM Synphaera", "https://somafm.com/synphaera.pls"),
        live("soma-bossa", "BOSSA", "SomaFM Bossa Beyond", "https://somafm.com/bossa.pls"),
        live("soma-dark", "DARK", "SomaFM Dark Zone", "https://somafm.com/darkzone.pls"),
        live("soma-n5", "N5MD", "SomaFM n5MD", "https://somafm.com/n5md.pls"),
        live("nr-main", "NRIDE", "Nightride FM", "https://stream.nightride.fm/nightride.ogg"),
        live("nr-chill", "CHILL", "Nightride Chillsynth", "https://stream.nightride.fm/chillsynth.ogg"),
        live("nr-data", "DATA", "Nightride Datawave", "https://stream.nightride.fm/datawave.ogg"),
        live("nr-horror", "HORROR", "Nightride Horrorsynth", "https://stream.nightride.fm/horrorsynth.ogg"),
        live("nr-space", "SYNTH", "Nightride Spacesynth", "https://stream.nightride.fm/spacesynth.ogg"),
        live("plaza", "PLAZA", "Nightwave Plaza", "https://plaza.one/mp3"),
        live("rp", "RP", "Radio Paradise", "https://stream.radioparadise.com/mp3-128"),
        live("rp-mellow", "RPM", "Radio Paradise Mellow", "https://stream.radioparadise.com/mellow-128"),
        live("rp-rock", "RPR", "Radio Paradise Rock", "https://stream.radioparadise.com/rock-128"),
    ]);
    v
}

pub fn get(id: &str) -> Option<Station> {
    all().into_iter().find(|s| s.id == id)
}
