import { layerById, worldOf } from "./catalog.js";

export const LIVE_STATIONS = [
  { id: "defcon", call: "DEFCON", freq: "104.4", name: "SomaFM DEF CON", url: "https://ice4.somafm.com/defcon-128-mp3", vibe: "rebellious" },
  { id: "digitalis", call: "GLITCH", freq: "107.1", name: "SomaFM Digitalis", url: "https://ice4.somafm.com/digitalis-128-mp3", vibe: "rebellious" },
  { id: "poptron", call: "POP", freq: "101.9", name: "SomaFM PopTron", url: "https://ice4.somafm.com/poptron-128-mp3", vibe: "rebellious" },
  { id: "bagel", call: "BAGEL", freq: "99.3", name: "SomaFM BAGeL Radio", url: "https://ice4.somafm.com/bagel-128-mp3", vibe: "rebellious" },
  { id: "metal", call: "METAL", freq: "88.1", name: "SomaFM Metal", url: "https://ice4.somafm.com/metal-128-mp3", vibe: "brutal" },
  { id: "dubstep", call: "BASS", freq: "87.7", name: "SomaFM Dub Step Beyond", url: "https://ice4.somafm.com/dubstep-128-mp3", vibe: "brutal" },
  { id: "thetrip", call: "TRIP", freq: "89.4", name: "SomaFM The Trip", url: "https://ice4.somafm.com/thetrip-128-mp3", vibe: "brutal" },
  { id: "cliqhop", call: "IDM", freq: "90.2", name: "SomaFM cliqhop idm", url: "https://ice4.somafm.com/cliqhop-128-mp3", vibe: "brutal" },
  { id: "darkzone", call: "DARK", freq: "91.3", name: "SomaFM Dark Zone", url: "https://ice4.somafm.com/darkzone-128-mp3", vibe: "melancholic" },
  { id: "deepspace", call: "VOID", freq: "93.0", name: "SomaFM Deep Space One", url: "https://ice4.somafm.com/deepspaceone-128-mp3", vibe: "melancholic" },
  { id: "secretagent", call: "NOIR", freq: "94.7", name: "SomaFM Secret Agent", url: "https://ice4.somafm.com/secretagent-128-mp3", vibe: "melancholic" },
  { id: "illstreet", call: "LOUNGE", freq: "95.5", name: "SomaFM Illinois Street Lounge", url: "https://ice4.somafm.com/illstreet-128-mp3", vibe: "melancholic" },
  { id: "dronezone", call: "DRONE", freq: "96.1", name: "SomaFM Drone Zone", url: "https://ice4.somafm.com/dronezone-128-mp3", vibe: "relaxing" },
  { id: "groovesalad", call: "GROOVE", freq: "97.4", name: "SomaFM Groove Salad", url: "https://ice4.somafm.com/groovesalad-128-mp3", vibe: "relaxing" },
  { id: "spacestation", call: "ORBIT", freq: "98.8", name: "SomaFM Space Station", url: "https://ice4.somafm.com/spacestation-128-mp3", vibe: "relaxing" },
  { id: "lush", call: "LUSH", freq: "102.6", name: "SomaFM Lush", url: "https://ice4.somafm.com/lush-128-mp3", vibe: "relaxing" },
  { id: "u80s", call: "RETRO", freq: "103.3", name: "SomaFM Underground 80s", url: "https://ice4.somafm.com/u80s-128-mp3", vibe: "relaxing" },
  { id: "fluid", call: "FLUID", freq: "105.8", name: "SomaFM Fluid", url: "https://ice4.somafm.com/fluid-128-mp3", vibe: "relaxing" },
  { id: "mission", call: "NASA", freq: "106.6", name: "SomaFM Mission Control", url: "https://ice4.somafm.com/missioncontrol-128-mp3", vibe: "melancholic" },
  { id: "sonic", call: "JAZZ", freq: "108.0", name: "SomaFM Sonic Universe", url: "https://ice4.somafm.com/sonicuniverse-128-mp3", vibe: "melancholic" },
];

export const LIBRARY_STATIONS = [
  { id: "rebellious", call: "RIOT", freq: "104.4", name: "Riot FM", vibe: "rebellious" },
  { id: "melancholic", call: "GLOOM", freq: "91.3", name: "Gloom Wire", vibe: "melancholic" },
  { id: "relaxing", call: "DUSK", freq: "96.1", name: "Dusk Channel", vibe: "relaxing" },
  { id: "brutal", call: "RAVE", freq: "88.1", name: "Warehouse", vibe: "brutal" },
];

/** @deprecated mood-keyed live map; prefer LIVE_STATIONS */
export const LIVE_STREAMS = Object.fromEntries(
  LIVE_STATIONS.filter((s, i, all) => all.findIndex((x) => x.vibe === s.vibe) === i).map((s) => [s.vibe, s])
);

export const STATIONS = {
  rebellious: LIBRARY_STATIONS[0],
  melancholic: LIBRARY_STATIONS[1],
  relaxing: LIBRARY_STATIONS[2],
  brutal: LIBRARY_STATIONS[3],
  synthwave: LIBRARY_STATIONS[1],
  techno: LIBRARY_STATIONS[3],
  rave: LIBRARY_STATIONS[3],
  lofi: LIBRARY_STATIONS[2],
  punk: LIBRARY_STATIONS[0],
  rock: LIBRARY_STATIONS[0],
};

export function stationForMood(mood) {
  return STATIONS[mood] || null;
}

export function liveStationById(id) {
  return LIVE_STATIONS.find((s) => s.id === id) || null;
}

export function libraryStationById(id) {
  return LIBRARY_STATIONS.find((s) => s.id === id) || stationForMood(id);
}

export function defaultLiveForMood(mood) {
  return LIVE_STATIONS.find((s) => s.vibe === mood) || LIVE_STATIONS[0];
}

export function stationRoster() {
  return isLiveRadio() ? LIVE_STATIONS : LIBRARY_STATIONS;
}

export function playingMusicLayers(mixer) {
  const ids = typeof mixer.playingIds === "function" ? mixer.playingIds() : [];
  return ids.map((id) => layerById(id)).filter((l) => l && l.category === "music");
}

export function isLiveRadio() {
  try {
    return localStorage.getItem("hearthsong.liveRadio") === "1";
  } catch {
    return false;
  }
}

export function setLiveRadio(on) {
  try {
    localStorage.setItem("hearthsong.liveRadio", on ? "1" : "0");
  } catch {
    /* ignore */
  }
}

export function selectedStationId() {
  try {
    return localStorage.getItem("hearthsong.station") || "";
  } catch {
    return "";
  }
}

export function setSelectedStationId(id) {
  try {
    if (id) localStorage.setItem("hearthsong.station", id);
  } catch {
    /* ignore */
  }
}

export function tunedStation(mixer) {
  if (worldOf() !== "blight") return null;
  if (isLiveRadio()) {
    const id = mixer?.liveStation || selectedStationId();
    return liveStationById(id) || defaultLiveForMood(mixer?.liveMood || "brutal");
  }
  const live = playingMusicLayers(mixer);
  const mood = mixer?.liveMood || live[0]?.mood || selectedStationId() || "brutal";
  return libraryStationById(mood) || stationForMood(mood) || LIBRARY_STATIONS[0];
}

export function currentStation(mixer) {
  const st = tunedStation(mixer);
  if (!st) return null;
  if (isLiveRadio()) {
    const on = Boolean(mixer?.livePlaying);
    return { ...st, track: { name: on ? "LIVE" : "STANDBY" }, mood: st.vibe, live: true, playing: on };
  }
  const live = playingMusicLayers(mixer);
  if (!live.length) return { ...st, track: { name: "Library" }, mood: st.vibe || st.id, live: false, playing: false };
  return { ...st, track: live[0], mood: st.vibe || st.id, live: false, playing: true };
}

function fillStationPick(st) {
  const bar = document.getElementById("station-bar");
  const pick = document.getElementById("station-pick");
  const blight = worldOf() === "blight";
  if (bar) bar.hidden = !blight;
  if (!pick) return;
  const roster = blight ? stationRoster() : [];
  const cur = st?.id || roster[0]?.id || "";
  const ids = roster.map((s) => s.id).join("|");
  if (pick.dataset.ids !== ids) {
    pick.innerHTML = roster
      .map((s) => `<option value="${s.id}">${s.freq} ${s.call} · ${s.name}</option>`)
      .join("");
    pick.dataset.ids = ids;
  }
  if (cur && pick.value !== cur) pick.value = cur;
}

export function paintRadioChip(mixer) {
  const el = document.getElementById("radio-chip");
  const tog = document.getElementById("live-radio-toggle");
  const blight = worldOf() === "blight";
  if (tog) {
    tog.hidden = !blight;
    tog.classList.toggle("on", blight && isLiveRadio());
    tog.textContent = blight && isLiveRadio() ? "Live wire" : "Library";
  }
  const st = blight ? currentStation(mixer) : null;
  fillStationPick(st || (blight ? tunedStation(mixer) : null));
  if (!el) return;
  el.hidden = !blight;
  if (!blight) return;
  if (!st || !st.playing) {
    el.classList.remove("on");
    el.innerHTML = `<span>OFF AIR</span><small>${st ? st.name : "Tune a station"}</small>`;
    return;
  }
  el.classList.add("on");
  el.innerHTML = `<span>${st.freq} ${st.call}</span><small>${st.name} · ${st.track?.name || "…"}</small>`;
}

export function neighborStation(id, step) {
  const roster = stationRoster();
  if (!roster.length) return null;
  const i = Math.max(0, roster.findIndex((s) => s.id === id));
  const next = roster[(i + step + roster.length) % roster.length];
  return next;
}

export function createLiveRadio(mixer) {
  const audio = new Audio();
  audio.crossOrigin = "anonymous";
  audio.preload = "none";
  let hooked = false;
  function hook() {
    if (hooked || !mixer?.ctx) return;
    try {
      mixer.attachMedia(audio);
      hooked = true;
    } catch {
      hooked = false;
    }
  }
  async function playStation(id) {
    const st = liveStationById(id) || defaultLiveForMood(id) || LIVE_STATIONS[0];
    mixer.liveStation = st.id;
    mixer.liveMood = st.vibe;
    setSelectedStationId(st.id);
    hook();
    audio.volume = 1;
    if (audio.src !== st.url) {
      audio.src = st.url;
    }
    try {
      await audio.play();
      mixer.livePlaying = !audio.paused;
    } catch {
      mixer.livePlaying = false;
    }
    paintRadioChip(mixer);
    return st;
  }
  async function playMood(mood) {
    const kept = liveStationById(mixer.liveStation || selectedStationId());
    const st = kept || defaultLiveForMood(mood);
    return playStation(st.id);
  }
  function stop() {
    mixer.livePlaying = false;
    audio.pause();
    audio.volume = 1;
    audio.removeAttribute("src");
    audio.load();
    paintRadioChip(mixer);
  }
  function playing() {
    return Boolean(audio.getAttribute("src")) && !audio.paused;
  }
  function fadeStop(ms = 4200) {
    if (!playing()) {
      stop();
      return;
    }
    const start = audio.volume || 1;
    const t0 = performance.now();
    const tick = (now) => {
      const p = Math.min(1, (now - t0) / Math.max(1, ms));
      audio.volume = start * (1 - p);
      if (p < 1 && !audio.paused) window.requestAnimationFrame(tick);
      else stop();
    };
    window.requestAnimationFrame(tick);
  }
  return { playMood, playStation, stop, fadeStop, playing, audio };
}
