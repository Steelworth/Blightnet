import { CATEGORIES, LAYERS, SCENES, MUSIC_MOODS, layerById, sceneById, tracksForMood, liveLayers, liveScenes, liveMoods, worldOf } from "./catalog.js";
import { Mixer } from "./engine.js";
import { icon } from "./icons.js";
import { createVisualizer } from "./viz.js";
import { playBoot, createBootSfx } from "./boot.js";
import { weatherOf, HOURS } from "./weather.js";
import { SETTINGS, settingById, SCENE_SETTING, settingSrc, settingsOf } from "./places.js";
import { createTable, fetchInfo, postUpload, profileName, setProfileName, profilePic, setProfilePic } from "./net.js";
import { createMapTable, imageSize, tokenPayload } from "./map.js";
import { createChars, resizeImage } from "./chars.js";
import { createBestiary } from "./bestiary.js";
import { createDatashard } from "./datashard.js";
import { createKit } from "./kit.js";
import { createVendors } from "./vendors.js";
import { createBlackjack } from "./blackjack.js";
import { createCorps } from "./corps.js";
import { createNpcs, KINDS_SRD } from "./npcs.js";
import { createGods } from "./gods.js";
import { createLore } from "./lore.js";
import { createGangs } from "./gangs.js";
import {
  paintRadioChip,
  stationForMood,
  isLiveRadio,
  setLiveRadio,
  createLiveRadio,
  selectedStationId,
  setSelectedStationId,
  liveStationById,
  libraryStationById,
  defaultLiveForMood,
  neighborStation,
  currentStation,
} from "./radio.js";
import { syncNetcast } from "./netcast.js";

const mixer = new Mixer();
let liveRadio = null;
const params = new URLSearchParams(location.search);
const ui = {
  filter: "all",
  scene: null,
  search: "",
  sceneQuery: "",
  openCats: new Set(),
  held: null,
  skySlot: 0,
  place: localStorage.getItem("hearthsong.place") === "inside" ? "inside" : "outside",
  time: HOURS.includes(localStorage.getItem("hearthsong.time"))
    ? localStorage.getItem("hearthsong.time")
    : "day",
  clock: loadClock(),
  setting: settingById(localStorage.getItem("hearthsong.setting")).id,
  custom: loadCustom(),
  musicMood: "all",
  playlist: null,
};

let playlistTimer = 0;
const PLAYLIST_MS = 82000;
let holdRadio = false;
const uploads = new Map();
let applyingRemote = false;
let mixTimer = 0;
const chatLines = [];
const CONTACTS_KEY = "hearthsong.contacts";
const contacts = loadContacts();
const peerPics = new Map();
let hearthReady = false;
let pendingMix = null;
const pendingUploads = [];
let bnPage = "start";
let hearthsongOpened = false;

const $ = (sel, root = document) => root.querySelector(sel);

function loadCustom() {
  try {
    const raw = JSON.parse(localStorage.getItem("hearthsong.custom") || "[]");
    if (!Array.isArray(raw)) return [];
    return raw.filter(
      (s) => s && typeof s.id === "string" && s.layers && typeof s.layers === "object"
    );
  } catch {
    return [];
  }
}

function saveCustom() {
  localStorage.setItem("hearthsong.custom", JSON.stringify(ui.custom));
}

function allScenes() {
  return [...liveScenes(), ...ui.custom];
}

function findScene(id) {
  return sceneById(id) || ui.custom.find((s) => s.id === id);
}

function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, (ch) => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
  }[ch]));
}

function sceneMatches(scene, query) {
  if (!query) return true;
  const tokens = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  const hay = `${scene.name} ${scene.blurb || ""} ${scene.id}`.toLowerCase();
  return tokens.every((tok) => hay.includes(tok));
}

function placeCover(scene) {
  const setting = scene.setting || SCENE_SETTING[scene.id] || "castle";
  return settingSrc(setting, "day");
}

function sceneCover(scene) {
  if (document.documentElement?.dataset?.theme === "blight") return placeCover(scene);
  if (scene.cover) return scene.cover;
  if (scene.id && !String(scene.id).startsWith("custom-")) {
    return `assets/scenes/${scene.id}.jpg`;
  }
  return placeCover(scene);
}

function renderScenes(opts = {}) {
  const list = $("#scene-list");
  const keepY = list.scrollTop;
  const keepX = list.scrollLeft;
  const shown = allScenes().filter((s) => sceneMatches(s, ui.sceneQuery));
  list.innerHTML = shown.length
    ? shown
        .map(
          (s, i) => `
      <button class="scene ${ui.scene === s.id ? "active" : ""}" data-scene="${escapeHtml(s.id)}" data-mood="${escapeHtml(s.mood || "gold")}" style="--i:${i}" title="${escapeHtml(s.blurb || s.name)}">
        <span class="scene-cover">
          <img src="${escapeHtml(sceneCover(s))}" alt="" loading="lazy" data-fallback="${escapeHtml(placeCover(s))}" />
        </span>
        <span>
          <b>${escapeHtml(s.name)}</b>
          <span class="blurb">${escapeHtml(s.blurb || "Your saved mix.")}</span>
        </span>
        ${String(s.id).startsWith("custom-") ? `<i class="kill" data-forget="${escapeHtml(s.id)}" title="Remove mix">×</i>` : ""}
      </button>`
        )
        .join("")
    : `<div class="hint">No scenes match.</div>`;
  if (opts.scrollActive) {
    list.querySelector(".scene.active")?.scrollIntoView({
      inline: "center",
      block: "nearest",
      behavior: "auto",
    });
  } else {
    list.scrollTop = keepY;
    list.scrollLeft = keepX;
  }
  for (const img of list.querySelectorAll("img[data-fallback]")) {
    img.addEventListener(
      "error",
      () => {
        if (img.dataset.fell) return;
        img.dataset.fell = "1";
        img.src = img.dataset.fallback;
      },
      { once: true }
    );
  }
}

function renderTabs() {
  const items = [
    ["all", "All"],
    ["on", "Playing"],
    ...CATEGORIES.map((c) => [c.id, c.name]),
  ];
  $("#tabs").innerHTML = items
    .map(
      ([id, label]) =>
        `<button class="tab ${ui.filter === id ? "on" : ""}" data-filter="${id}">${label}</button>`
    )
    .join("");
}

function rowHTML(l) {
  if (!l) return "";
  const on = mixer.isPlaying(l.id);
  const vol = mixer.volumeOf(l.id);
  const pct = Math.round(vol * 100);
  const name = escapeHtml(l.name);
  return `
    <div class="row ${on ? "on" : ""}" data-layer="${l.id}" style="--p:${pct}%">
      <button class="toggle" data-toggle="${l.id}" aria-pressed="${on}" aria-label="${on ? "Stop" : "Play"} ${name}">
        ${icon(l.icon)}
      </button>
      <button type="button" class="name" data-toggle="${l.id}">${name}</button>
      <input type="range" min="0" max="100" value="${pct}" data-vol="${l.id}" aria-label="${name} volume">
      <div class="vol">${pct}</div>
    </div>`;
}

function searchTokens() {
  return (ui.search || "")
    .trim()
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean);
}

function layerMatches(layer, tokens) {
  if (!layer) return false;
  if (!tokens.length) return true;
  const hay = `${layer.name} ${layer.id.replace(/_/g, " ")} ${layer.category} ${layer.icon || ""} ${layer.mood || ""}`.toLowerCase();
  const words = hay.split(/[\s/_-]+/).filter(Boolean);
  return tokens.every((tok) => words.some((word) => word === tok || word.startsWith(tok)));
}

function moodChipsHTML() {
  const items = [{ id: "all", name: "All moods" }, ...liveMoods()];
  return `<div class="moods" role="group" aria-label="Music mood">${items
    .map(
      (m) =>
        `<button type="button" class="mood ${ui.musicMood === m.id ? "on" : ""}" data-mood-filter="${m.id}">${escapeHtml(m.name)}</button>`
    )
    .join("")}</div>`;
}

function musicRows(layers) {
  if (!layers.length) return `<div class="hint">No music in this mood.</div>`;
  if (ui.musicMood !== "all") return layers.map(rowHTML).join("");
  const known = new Set(liveMoods().map((m) => m.id));
  const grouped = liveMoods().map((mood) => {
    const group = layers.filter((l) => l.mood === mood.id);
    if (!group.length) return "";
    return `<div class="mood-label">${escapeHtml(mood.name)}</div>${group.map(rowHTML).join("")}`;
  }).join("");
  const leftover = layers.filter((l) => !l.mood || !known.has(l.mood));
  return leftover.length
    ? grouped + `<div class="mood-label">Other</div>${leftover.map(rowHTML).join("")}`
    : grouped;
}

function playingMusic() {
  return mixer
    .playingIds()
    .map((id) => layerById(id))
    .filter((l) => l && l.category === "music");
}

function liveWireOn() {
  return worldOf() === "blight" && isLiveRadio() && Boolean(liveRadio?.playing?.());
}

function tableIsSounding() {
  return mixer.playingIds().length > 0 || liveWireOn();
}

function clearPlaylistTimer() {
  if (playlistTimer) {
    window.clearTimeout(playlistTimer);
    playlistTimer = 0;
  }
}

function stopPlaylist() {
  clearPlaylistTimer();
  ui.playlist = null;
}

function armPlaylistTimer() {
  clearPlaylistTimer();
  if (!ui.playlist) return;
  playlistTimer = window.setTimeout(() => {
    playlistTimer = 0;
    if (!ui.playlist || ui.scene !== ui.playlist.scene) return;
    const current = ui.playlist.tracks[ui.playlist.index];
    if (!current || !mixer.isPlaying(current)) return;
    advancePlaylist(1);
  }, PLAYLIST_MS);
}

function refreshMix() {
  renderLayers();
  renderNow();
  syncFade();
  const shuffleBtn = $("#shuffle");
  if (shuffleBtn) {
    shuffleBtn.title = ui.playlist
      ? ui.playlist.radio
        ? "Next track on this station"
        : "Next fight in the Battles playlist"
      : "Play another piece of the same mood";
  }
}

function playPlaylistIndex(index, fade = 0.85) {
  if (!ui.playlist || !mixer.ctx) return;
  const tracks = ui.playlist.tracks;
  if (!tracks.length) return;
  const i = ((index % tracks.length) + tracks.length) % tracks.length;
  ui.playlist.index = i;
  const id = tracks[i];
  if (!layerById(id)) return;
  const live = playingMusic();
  const vol = live[0] ? mixer.volumeOf(live[0].id) : ui.playlist.volume;
  ui.playlist.volume = vol;
  for (const layer of live) {
    if (layer.id !== id) mixer.stop(layer.id, fade);
  }
  mixer.start(id, vol, fade);
  armPlaylistTimer();
  refreshMix();
  $("#layers")?.querySelector(".queue-track.on")?.scrollIntoView({ block: "nearest" });
}

function advancePlaylist(step = 1) {
  if (!ui.playlist) return;
  if (ui.playlist.radio && ui.playlist.tracks.length > 1) {
    const last = ui.playlist.tracks[ui.playlist.index];
    let next = ui.playlist.index + 1;
    if (next >= ui.playlist.tracks.length) {
      ui.playlist.tracks = shuffleIds(ui.playlist.tracks);
      next = 0;
      if (ui.playlist.tracks[0] === last) {
        const j = 1 + Math.floor(Math.random() * (ui.playlist.tracks.length - 1));
        const swap = ui.playlist.tracks[0];
        ui.playlist.tracks[0] = ui.playlist.tracks[j];
        ui.playlist.tracks[j] = swap;
      }
    }
    playPlaylistIndex(next);
    return;
  }
  playPlaylistIndex(ui.playlist.index + step);
}

function shuffleIds(ids) {
  const a = [...ids];
  for (let i = a.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [a[i], a[j]] = [a[j], a[i]];
  }
  return a;
}

function sceneBeds(scene) {
  const beds = {};
  for (const [id, vol] of Object.entries(scene.layers || {})) {
    const l = layerById(id);
    if (l && l.category !== "music") beds[id] = vol;
  }
  return beds;
}

function startPlaylist(scene) {
  const sceneMood = scene.playlist;
  const beds = sceneBeds(scene);
  if (worldOf() === "blight" && isLiveRadio()) {
    const kept = liveStationById(mixer.liveStation || selectedStationId());
    const st = kept || defaultLiveForMood(sceneMood);
    ui.musicMood = st.vibe;
    ui.playlist = {
      scene: scene.id,
      mood: st.vibe,
      tracks: [],
      index: 0,
      volume: 0.56,
      beds,
      radio: true,
      live: true,
    };
    mixer.applyScene({ layers: beds });
    liveRadio?.playStation(st.id);
    ui.openCats.add("music");
    paintRadioChip(mixer);
    return true;
  }
  liveRadio?.stop();
  const keepMood = worldOf() === "blight" && ui.playlist?.radio && ui.playlist.mood ? ui.playlist.mood : sceneMood;
  if (worldOf() === "blight" && ui.playlist?.radio && ui.playlist.tracks?.length && ui.playlist.mood === keepMood) {
    ui.playlist = { ...ui.playlist, scene: scene.id, beds };
    const current = ui.playlist.tracks[ui.playlist.index] || ui.playlist.tracks[0];
    mixer.applyScene({ layers: { ...beds, [current]: ui.playlist.volume || 0.56 } });
    ui.openCats.add("music");
    armPlaylistTimer();
    return true;
  }
  const tracks = shuffleIds(tracksForMood(keepMood));
  if (!tracks.length) return false;
  ui.musicMood = keepMood;
  ui.playlist = {
    scene: scene.id,
    mood: keepMood,
    tracks,
    index: 0,
    volume: 0.56,
    beds,
    radio: worldOf() === "blight",
  };
  const first = tracks[0];
  mixer.applyScene({ layers: { ...ui.playlist.beds, [first]: ui.playlist.volume } });
  ui.openCats.add("music");
  armPlaylistTimer();
  return true;
}

function playlistHTML() {
  if (!ui.playlist) return "";
  const { tracks = [], index, radio, mood } = ui.playlist;
  const current = tracks[index];
  const st = radio ? currentStation(mixer) || stationForMood(mood) : null;
  const shown = !tracks.length
    ? []
    : radio
      ? [index, ...Array.from({ length: Math.min(7, Math.max(tracks.length - 1, 0)) }, (_, k) => (index + k + 1) % tracks.length)]
      : tracks.map((_, i) => i);
  const rows = shown
    .map((i) => {
      const id = tracks[i];
      const l = layerById(id);
      if (!l) return "";
      return `<button type="button" class="queue-track ${id === current ? "on" : ""}" data-playlist-track="${i}">${icon(l.icon)}<span>${escapeHtml(l.name)}</span></button>`;
    })
    .join("");
  const title = radio ? `${st?.freq || ""} ${st?.call || "STATION"}`.trim() : "Battles playlist";
  const count = radio ? `${tracks.length} shuffle` : String(tracks.length);
  return `<div class="cat live">${escapeHtml(title)}<span>${escapeHtml(count)}</span></div><div class="queue">${rows}</div>`;
}

function shuffleMusic() {
  if (mixLocked()) return;
  if (!mixer.ctx) return;
  mixer.resume();
  if (worldOf() === "blight" && isLiveRadio()) {
    stepStation(1);
    return;
  }
  if (ui.playlist) {
    advancePlaylist(1);
    return;
  }
  const live = playingMusic();
  const preferred =
    ui.musicMood && ui.musicMood !== "all" ? ui.musicMood : live[0]?.mood || null;
  const exclude = new Set(live.map((l) => l.id));
  let pool = liveLayers().filter(
    (l) => l.category === "music" && !exclude.has(l.id) && (!preferred || l.mood === preferred)
  );
  if (!pool.length) {
    pool = liveLayers().filter((l) => l.category === "music" && !exclude.has(l.id));
  }
  if (!pool.length) return;
  const pick = pool[Math.floor(Math.random() * pool.length)];
  const vol = live[0] ? mixer.volumeOf(live[0].id) : 0.46;
  for (const layer of live) mixer.stop(layer.id, 0.7);
  mixer.start(pick.id, vol, 0.85);
  ui.openCats.add("music");
  const count = $("#mix-count");
  if (count) {
    count.textContent = "Now " + pick.name;
    window.setTimeout(() => {
      if ($("#mix-search")?.value) return;
      count.textContent = mixCountLabel();
    }, 1800);
  }
}

let layerSig = "";
function renderLayers() {
  const root = $("#layers");
  const playing = mixer.playingIds();
  const sig = [
    playing.join("\0"),
    ui.filter,
    ui.search,
    ui.musicMood,
    ui.playlist?.scene || "",
    [...ui.openCats].join("\0"),
    worldOf(),
    liveLayers().length,
  ].join("|");
  if (sig === layerSig) {
    const count = $("#mix-count");
    if (count && !$("#mix-search")?.value) count.textContent = mixCountLabel();
    return;
  }
  layerSig = sig;
  const scroll = root.scrollTop;
  const queueY = root.querySelector(".queue")?.scrollTop ?? 0;
  const active = document.activeElement;
  const volId = active?.dataset?.vol;
  const volVal = active && volId ? active.value : null;
  const tokens = searchTokens();
  const searching = tokens.length > 0;
  const liveOnly = ui.filter === "on";
  const cats = searching || ui.filter === "all" || liveOnly
    ? CATEGORIES
    : CATEGORIES.filter((c) => c.id === ui.filter);
  const liveIds = playing.filter(
    (id) => !searching || layerMatches(layerById(id), tokens)
  );
  let html = "";
  if (liveIds.length) {
    html += `<div class="cat live">On the air</div>`;
    html += liveIds.map((id) => rowHTML(layerById(id))).join("");
  }
  if (ui.playlist && !searching) html += playlistHTML();
  if (!liveOnly) {
    html += cats
      .map((cat) => {
        const layers = liveLayers().filter(
          (l) =>
            l.category === cat.id &&
            layerMatches(l, tokens) &&
            !liveIds.includes(l.id)
        );
        if (!layers.length) return "";
        const open = searching || ui.filter === cat.id || ui.openCats.has(cat.id);
        if (cat.id === "music" && !searching) {
          let shown = layers;
          if (worldOf() === "blight") shown = shown.filter((l) => !String(l.id).startsWith("br_"));
          if (ui.musicMood !== "all") shown = shown.filter((l) => l.mood === ui.musicMood);
          const body = open ? moodChipsHTML() + musicRows(shown) : "";
          return `<button type="button" class="cat ${open ? "open" : ""}" data-cat="${cat.id}" aria-expanded="${open}">${cat.name}<span>${shown.length}</span></button>${body}`;
        }
        return `<button type="button" class="cat ${open ? "open" : ""}" data-cat="${cat.id}" aria-expanded="${open}">${cat.name}<span>${layers.length}</span></button>${open ? layers.map(rowHTML).join("") : ""}`;
      })
      .join("");
  }
  if (!html) {
    html = searching
      ? `<div class="hint">No sounds match “${escapeHtml(ui.search.trim())}”.</div>`
      : liveOnly
        ? `<div class="hint">Nothing is playing. Press a scene, or open All.</div>`
        : `<div class="hint">Press a scene. Then open a group and tune any sound.</div>`;
  }
  root.innerHTML = html;
  root.scrollTop = scroll;
  const queue = root.querySelector(".queue");
  if (queue) queue.scrollTop = queueY;
  if (volId) {
    const el = root.querySelector(`[data-vol="${volId}"]`);
    if (el) {
      if (volVal !== null) {
        el.value = volVal;
        el.closest(".row")?.style.setProperty("--p", volVal + "%");
        const label = el.closest(".row")?.querySelector(".vol");
        if (label) label.textContent = volVal;
      }
      el.focus({ preventScroll: true });
    }
  }
  const count = $("#mix-count");
  if (count) count.textContent = mixCountLabel();
  syncFade();
}

function tableStatus(text, extra = {}) {
  const el = $("#table-status");
  if (el) {
    el.textContent = text;
    el.classList.toggle("on", table.state.role !== "idle");
  }
  const leave = $("#table-leave");
  const hostBtn = $("#table-host");
  const joinBtn = $("#table-join");
  if (leave) leave.hidden = table.state.role === "idle";
  if (hostBtn) hostBtn.hidden = table.state.role !== "idle";
  if (joinBtn) joinBtn.hidden = table.state.role !== "idle";
  const copyBtn = $("#table-copy");
  if (copyBtn) {
    copyBtn.hidden = table.state.role !== "host";
    if (extra.share) copyBtn.dataset.share = extra.share;
  }
  chars.setGM(table.state.role === "host");
  if (table.state.role === "idle") {
    maps?.clearEphemeral();
    chars.clearRemote();
  }
  const box = $("#bn-omnibox");
  if (extra.hosting && extra.share) {
    const raw = String(extra.share).replace(/^(blightnet|blighnet):\/\//i, "");
    if (el) el.title = raw;
    if (box) box.dataset.share = /^https?:\/\//i.test(raw) ? raw : "blightnet://" + raw;
  } else if (table.state.role === "idle" && box) {
    delete box.dataset.share;
  }
  syncBnChrome();
  paintWatch();
}

function loadContacts() {
  try {
    const raw = JSON.parse(localStorage.getItem(CONTACTS_KEY) || "[]");
    if (!Array.isArray(raw)) return [];
    return raw
      .filter((row) => row && typeof row.id === "string" && row.id)
      .map((row) => ({
        id: String(row.id).slice(0, 32),
        name: String(row.name || "Traveller").trim().slice(0, 24) || "Traveller",
        added: Number(row.added) || Date.now(),
        pic: typeof row.pic === "string" && row.pic.startsWith("data:image/") ? row.pic : "",
      }));
  } catch {
    return [];
  }
}

function saveContacts() {
  try {
    localStorage.setItem(CONTACTS_KEY, JSON.stringify(contacts));
  } catch {
    /* ignore quota */
  }
}

function isContact(id) {
  return contacts.some((c) => c.id === id);
}

function addContact(id, name) {
  const pid = String(id || "").trim();
  if (!pid || pid === table.state.selfId) return false;
  const handle = String(name || "Traveller").trim().slice(0, 24) || "Traveller";
  const hit = contacts.find((c) => c.id === pid);
  if (hit) {
    hit.name = handle;
    if (peerPics.get(pid)) hit.pic = peerPics.get(pid);
    saveContacts();
    renderContacts();
    return false;
  }
  contacts.push({ id: pid, name: handle, added: Date.now(), pic: peerPics.get(pid) || "" });
  contacts.sort((a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: "base" }));
  saveContacts();
  renderContacts();
  return true;
}

function removeContact(id) {
  const i = contacts.findIndex((c) => c.id === id);
  if (i < 0) return;
  contacts.splice(i, 1);
  saveContacts();
  renderContacts();
}

function paintVoice() {
  const v = table.voice?.() || {};
  const btn = $("#voice-toggle");
  const banner = $("#voice-banner");
  const text = $("#voice-banner-text");
  const accept = $("#voice-accept");
  const decline = $("#voice-decline");
  const hangup = $("#voice-hangup");
  if (btn) {
    btn.classList.toggle("on", Boolean(v.table && !v.callId));
    btn.disabled = table.state.role === "idle" && !v.incoming;
  }
  if (!banner || !text) return;
  if (v.incoming) {
    banner.hidden = false;
    text.textContent = (v.incoming.name || "Someone") + " is calling";
    if (accept) accept.hidden = false;
    if (decline) decline.hidden = false;
    if (hangup) hangup.hidden = true;
  } else if (v.pendingCall) {
    const peer = (table.state.peers || []).find((p) => p.id === v.pendingCall);
    banner.hidden = false;
    text.textContent = "Calling " + (peer?.name || "them") + "…";
    if (accept) accept.hidden = true;
    if (decline) decline.hidden = true;
    if (hangup) hangup.hidden = false;
  } else if (v.callId) {
    const peer = (table.state.peers || []).find((p) => p.id === v.callId);
    banner.hidden = false;
    text.textContent = "Private call · " + (peer?.name || "contact");
    if (accept) accept.hidden = true;
    if (decline) decline.hidden = true;
    if (hangup) hangup.hidden = false;
  } else if (v.table) {
    banner.hidden = false;
    text.textContent = "Table voice on";
    if (accept) accept.hidden = true;
    if (decline) decline.hidden = true;
    if (hangup) hangup.hidden = true;
  } else {
    banner.hidden = true;
  }
}

function voiceError(err) {
  const code = err?.code || "";
  if (code === "idle") return "Host or join a table to use voice.";
  if (code === "denied") return "Microphone blocked. Allow it for this window and try again.";
  if (code === "secure") return "This page cannot open the microphone. Use the Blightnet window on this machine (localhost), not a plain http LAN link.";
  if (code === "offline") return "That contact is not at this table.";
  if (code === "busy") return "Already in a call.";
  return "Could not start voice.";
}

function applyProfile(from, name, pic) {
  if (!from || from === table.state.selfId) return;
  if (pic && String(pic).startsWith("data:image/")) peerPics.set(from, pic);
  else if (pic === "") peerPics.delete(from);
  const hit = contacts.find((c) => c.id === from);
  if (hit) {
    if (name) hit.name = name;
    const next = peerPics.get(from);
    if (next) hit.pic = next;
    saveContacts();
  }
  renderContacts();
}

function applyProfiles(rows) {
  for (const row of rows || []) {
    if (!row?.from) continue;
    if (row.from === table.state.selfId) continue;
    const pic = row.pic || "";
    if (pic.startsWith("data:image/")) peerPics.set(row.from, pic);
  }
  let dirty = false;
  for (const c of contacts) {
    const pic = peerPics.get(c.id);
    if (pic && c.pic !== pic) {
      c.pic = pic;
      dirty = true;
    }
  }
  if (dirty) saveContacts();
  renderContacts();
}

function paintOwnFace() {
  const img = $("#profile-pic");
  const ph = $("#profile-pic-ph");
  const pic = profilePic();
  if (img) {
    if (pic) {
      img.src = pic;
      img.hidden = false;
    } else {
      img.removeAttribute("src");
      img.hidden = true;
    }
  }
  if (ph) ph.hidden = Boolean(pic);
}

function contactFace(pic) {
  if (pic) {
    const img = document.createElement("img");
    img.className = "contact-face";
    img.src = pic;
    img.alt = "";
    return img;
  }
  const ph = document.createElement("span");
  ph.className = "contact-face empty";
  return ph;
}

function fillWhisper(name) {
  const input = $("#chat-input");
  const panel = $("#chat-panel");
  if (!input || !panel) return;
  const quoted = /\s/.test(name) ? `"${name}"` : name;
  const cur = input.value.replace(/^\/(?:w|whisper|msg|tell)\s+(?:"[^"]*"|'[^']*'|\S+)\s*/i, "");
  input.value = `/w ${quoted} ` + cur;
  panel.hidden = false;
  $("#launcher")?.classList.add("chat-open");
  input.focus();
}

function renderContacts() {
  const liveBox = $("#contact-live");
  const savedBox = $("#contact-saved");
  if (!liveBox || !savedBox) return;
  const peers = table.state.peers || [];
  const others = peers.filter((p) => p.id !== table.state.selfId);
  const online = new Map(others.map((p) => [p.id, p]));

  let renamed = false;
  for (const p of others) {
    const hit = contacts.find((c) => c.id === p.id);
    if (hit && p.name && hit.name !== p.name) {
      hit.name = p.name;
      renamed = true;
    }
  }
  if (renamed) saveContacts();

  liveBox.replaceChildren();
  if (table.state.role === "idle") {
    liveBox.textContent = "Host or join a table to add people.";
  } else if (!others.length) {
    liveBox.textContent = "Waiting for others…";
  } else {
    others.forEach((p) => {
      const row = document.createElement("div");
      row.className = "contact-row";
      const name = document.createElement("span");
      name.className = "contact-name";
      name.textContent = p.role === "host" ? `${p.name} (GM)` : p.name;
      row.append(contactFace(peerPics.get(p.id) || ""), name);
      if (isContact(p.id)) {
        const saved = document.createElement("span");
        saved.className = "contact-meta";
        saved.textContent = "saved";
        row.append(saved);
      } else {
        const add = document.createElement("button");
        add.type = "button";
        add.className = "contact-add";
        add.dataset.addContact = p.id;
        add.dataset.addName = p.name;
        add.textContent = "Add";
        row.append(add);
      }
      liveBox.append(row);
    });
  }

  savedBox.replaceChildren();
  if (!contacts.length) {
    savedBox.textContent = "No contacts yet. Add someone at the table.";
    return;
  }
  contacts.forEach((c) => {
    const peer = online.get(c.id);
    const row = document.createElement("div");
    row.className = "contact-row" + (peer ? " on" : "");
    const dot = document.createElement("span");
    dot.className = "contact-dot" + (peer ? " on" : "");
    const name = document.createElement("span");
    name.className = "contact-name";
    name.textContent = c.name;
    const meta = document.createElement("span");
    meta.className = "contact-meta";
    meta.textContent = peer ? (peer.role === "host" ? "online · GM" : "online") : "offline";
    row.append(contactFace(c.pic || peerPics.get(c.id) || ""), dot, name, meta);
    if (peer) {
      const whisper = document.createElement("button");
      whisper.type = "button";
      whisper.className = "contact-add";
      whisper.dataset.whisper = peer.name;
      whisper.textContent = "Whisper";
      row.append(whisper);
      const call = document.createElement("button");
      call.type = "button";
      call.className = "contact-add";
      const voice = table.voice?.() || {};
      if (voice.callId === peer.id || voice.pendingCall === peer.id) {
        call.dataset.voiceHangup = peer.id;
        call.textContent = "Hang up";
      } else {
        call.dataset.voiceCall = peer.id;
        call.textContent = "Call";
      }
      row.append(call);
      const pic = document.createElement("button");
      pic.type = "button";
      pic.className = "contact-add";
      pic.dataset.mediaTo = peer.id;
      pic.dataset.mediaName = peer.name;
      pic.textContent = "Pic";
      row.append(pic);
    }
    const del = document.createElement("button");
    del.type = "button";
    del.className = "contact-del";
    del.dataset.removeContact = c.id;
    del.title = "Remove " + c.name;
    del.textContent = "Remove";
    row.append(del);
    savedBox.append(row);
  });
}

function renderPeers(peers) {
  const others = (peers || []).filter((p) => p.id !== table.state.selfId);
  const box = $("#chat-peers");
  if (!box) {
    renderContacts();
    return;
  }
  box.replaceChildren();
  if (!others.length) {
    box.textContent = table.state.role === "idle" ? "No one else is online." : "Waiting for others…";
  } else {
    others.forEach((p, i) => {
      if (i) box.append(document.createTextNode(" · "));
      const wrap = document.createElement("span");
      wrap.className = "chat-peer-wrap";
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "chat-peer";
      btn.dataset.whisper = p.name;
      btn.title = "Whisper " + p.name;
      btn.textContent = p.role === "host" ? `${p.name} (GM)` : p.name;
      wrap.append(btn);
      if (!isContact(p.id)) {
        const add = document.createElement("button");
        add.type = "button";
        add.className = "chat-peer-add";
        add.dataset.addContact = p.id;
        add.dataset.addName = p.name;
        add.title = "Add " + p.name + " to contacts";
        add.textContent = "+";
        wrap.append(add);
      }
      box.append(wrap);
    });
  }
  if (others.length && table.state.role !== "idle") {
    const panel = $("#chat-panel");
    if (panel?.hidden) {
      panel.hidden = false;
      $("#launcher")?.classList.add("chat-open");
    }
  }
  renderContacts();
}

function pushChat(line) {
  chatLines.push(line);
  if (chatLines.length > 200) {
    const gone = chatLines.shift();
    if (gone?.media && String(gone.media).startsWith("blob:")) {
      try {
        URL.revokeObjectURL(gone.media);
      } catch {
        /* ignore */
      }
    }
  }
  const log = $("#chat-log");
  if (!log) return;
  const row = document.createElement("div");
  row.className = "chat-line" + (line.sys ? " sys" : "") + (line.whisper ? " whisper" : "") + (line.media ? " media" : "");
  if (line.sys) {
    row.textContent = line.text;
  } else {
    const who = document.createElement("b");
    who.textContent = line.name || "Traveller";
    row.append(who);
    if (line.whisper) {
      const tag = document.createElement("i");
      tag.className = "whisper-tag";
      const mine = line.fromId && line.fromId === table.state.selfId;
      tag.textContent = mine ? "to " + (line.toName || "them") : "whisper";
      row.append(tag);
    }
    if (line.media) {
      const mime = String(line.mime || "");
      const el = mime.startsWith("video/") ? document.createElement("video") : document.createElement("img");
      el.className = "chat-media";
      el.src = line.media;
      el.alt = line.filename || "";
      if (el.tagName === "VIDEO") {
        el.controls = true;
        el.playsInline = true;
        el.preload = "metadata";
      }
      row.append(el);
    } else if (line.text) {
      row.append(document.createTextNode(line.text));
    }
  }
  log.append(row);
  log.scrollTop = log.scrollHeight;
}

function receiveChatMedia(meta) {
  if (!meta?.blob) return;
  if (meta.whisper && meta.to && meta.to !== table.state.selfId && meta.from !== table.state.selfId) return;
  const url = URL.createObjectURL(meta.blob);
  const peer = (table.state.peers || []).find((p) => p.id === meta.from);
  pushChat({
    name: meta.fromName || peer?.name || "Traveller",
    media: url,
    mime: meta.mime || meta.blob.type || "",
    filename: meta.name || "",
    whisper: Boolean(meta.whisper),
    toName: meta.toName || "",
    fromId: meta.from,
  });
  const panel = $("#chat-panel");
  if (panel?.hidden) {
    panel.hidden = false;
    $("#launcher")?.classList.add("chat-open");
  }
}

let mediaSendTo = null;

async function sendPickedMedia(file) {
  if (!file) return;
  if (table.state.role === "idle") {
    pushChat({ sys: true, text: "Host or join a table to send pictures." });
    return;
  }
  const target = mediaSendTo;
  mediaSendTo = null;
  const extra = target ? { to: target.id, toName: target.name, whisper: true } : {};
  if (!target) {
    const typed = ($("#chat-input")?.value || "").trim();
    if (/^\/(?:w|whisper|msg|tell)\b/i.test(typed)) {
      extra.whisper = true;
      extra.toName = typed.replace(/^\/(?:w|whisper|msg|tell)\s+/i, "");
    }
  }
  const result = await table.sendChatMedia(file, extra);
  if (result?.error) {
    pushChat({ sys: true, text: result.error });
    return;
  }
  const url = URL.createObjectURL(file);
  pushChat({
    name: profileName(),
    media: url,
    mime: file.type,
    filename: file.name,
    whisper: Boolean(result.whisper),
    toName: result.toName || target?.name || "",
    fromId: table.state.selfId,
  });
}

function registerUpload(meta, bytes) {
  const spec = {
    id: meta.id,
    name: meta.name || "Upload",
    category: ["music", "weather", "animals", "ambience"].includes(meta.category) ? meta.category : "music",
    icon: meta.icon || (meta.category === "music" ? "lute" : "spark"),
    mood: meta.mood || "calm",
    file: meta.file && !String(meta.file).startsWith("upload:") ? meta.file : "upload:" + meta.id,
    custom: true,
  };
  if (!layerById(spec.id)) {
    spec.world = document.documentElement?.dataset?.theme === "blight" ? "blight" : "hearthsong";
    LAYERS.push(spec);
  }
  mixer.addLayer(spec);
  if (bytes) {
    uploads.set(spec.id, { ...meta, id: spec.id, name: spec.name, category: spec.category, mime: meta.mime, icon: spec.icon, mood: spec.mood, bytes });
    mixer.ingestAudio(spec.file, bytes).then(() => paint());
  }
  paint();
  return spec;
}

async function ingestRemoteFile(meta, origin) {
  if (layerById(meta.id) && mixer.buffers.has("upload:" + meta.id)) return;
  if (meta.blob) {
    const bytes = await meta.blob.arrayBuffer();
    registerUpload(meta, bytes);
    return;
  }
  if (meta.file && origin) {
    try {
      const res = await fetch(new URL(meta.file, origin + "/"));
      if (res.ok) {
        const bytes = await res.arrayBuffer();
        registerUpload({ ...meta, file: "upload:" + meta.id }, bytes);
        return;
      }
    } catch {
      /* P2P will follow */
    }
  }
  registerUpload(meta);
}

function currentMix() {
  return {
    layers: mixer.snapshot(),
    scene: ui.scene,
    setting: ui.setting,
    time: ui.time,
    clock: ui.clock,
    place: ui.place,
    master: mixer.masterVolume,
  };
}

function broadcastMix() {
  if (table.state.role !== "host" || applyingRemote) return;
  window.clearTimeout(mixTimer);
  mixTimer = window.setTimeout(() => table.sendMix(currentMix()), 90);
}

function mixLocked() {
  return table.state.role === "guest";
}

function applyRemoteMix(mix) {
  if (!mix || table.state.role === "host") return;
  applyingRemote = true;
  holdRadio = true;
  stopPlaylist();
  mixer.resume();
  try {
    if (mix.setting) setSetting(mix.setting, true);
    if (mix.clock != null && mix.clock !== "") setClock(mix.clock, true);
    else if (mix.time) setTime(mix.time, true);
    if (mix.place) setPlace(mix.place, true);
    if (mix.scene) ui.scene = mix.scene;
    mixer.applyScene({ layers: mix.layers || {} }, 0.8);
    paint({ scrollActive: true });
  } finally {
    window.setTimeout(() => {
      applyingRemote = false;
      holdRadio = false;
    }, 220);
  }
}

const chars = createChars();
const table = createTable({
  onStatus(text, extra) {
    if (text === "Hosting") {
      const share = $("#table-copy")?.dataset.share;
      text = share ? "Hosting · Gamemaster · " + String(share).replace(/^https?:\/\//i, "") : "Hosting · Gamemaster";
      extra = { ...(extra || {}), hosting: true, share: share || extra?.share };
    }
    tableStatus(text, extra);
  },
  onPeers(peers) {
    renderPeers(peers);
    paintVoice();
  },
  onVoice() {
    paintVoice();
    renderContacts();
  },
  onProfile(from, name, pic) {
    applyProfile(from, name, pic);
  },
  onProfiles(rows) {
    applyProfiles(rows);
  },
  onChat(msg) {
    if (msg.sys) {
      pushChat({ sys: true, text: msg.text });
      return;
    }
    if (msg.whisper) {
      const mine = msg.id === table.state.selfId;
      const toMe = msg.to === table.state.selfId;
      if (!mine && !toMe) return;
      pushChat({
        name: msg.name,
        text: msg.text,
        whisper: true,
        toName: msg.toName,
        fromId: msg.id,
      });
      return;
    }
    pushChat({ name: msg.name, text: msg.text });
  },
  onRoll(row) {
    chars.ingestRoll(row);
  },
  onRolls(rows) {
    chars.ingestRolls(rows);
  },
  onMix(mix) {
    if (!hearthReady) {
      pendingMix = mix;
      return;
    }
    applyRemoteMix(mix);
  },
  async onFile(meta) {
    if (meta.chat || meta.kind === "chat-media" || meta.category === "chat-media") {
      receiveChatMedia(meta);
      return;
    }
    const kind = meta.kind || meta.category;
    const bytes = meta.blob ? await meta.blob.arrayBuffer() : null;
    if (kind === "map") {
      await receiveStreamedMap(meta, bytes);
      return;
    }
    if (!hearthReady) {
      pendingUploads.push({ meta, bytes });
      return;
    }
    registerUpload(meta, bytes);
    if (table.state.role === "host" && bytes) {
      postUpload(location.origin, meta, bytes).catch(() => {});
      for (const peer of table.state.peers) {
        if (peer.id !== table.state.selfId && peer.id !== meta.from) {
          table.sendFileTo(peer.id, uploads.get(meta.id));
        }
      }
    }
  },
  async onFileList(files, origin) {
    for (const rec of files || []) {
      if (rec.category === "map" || rec.kind === "map") continue;
      await ingestRemoteFile(rec, origin);
    }
  },
  onNeedPush(remoteId) {
    for (const rec of uploads.values()) table.sendFileTo(remoteId, rec);
    const rec = maps?.state.maps.get(maps.state.activeId);
    if (rec?.bytes) table.sendFileTo(remoteId, rec);
  },
  onChars(from, name, list) {
    chars.applyRemote(from, name, list);
  },
  onCharsTable(packs) {
    chars.applyRemoteTable(packs);
  },
  onReady() {
    chars.shareNow();
  },
  onMap(meta) {
    applyRemoteMap(meta);
  },
  onMapEdit(edit) {
    if (table.state.role !== "host" || !edit) return;
    if (!maps.applyMarks(edit)) return;
    const snap = maps.snapshot();
    if (snap) table.sendMap(snap);
  },
  onNeedFile(remoteId, id) {
    const rec = uploads.get(id) || maps?.state.maps.get(id);
    if (rec?.bytes) table.sendFileTo(remoteId, rec);
  },
});

let mapViewTimer = 0;
let lastStreamedMap = null;
const maps = createMapTable({
  canControl: () => table.state.role !== "guest",
  followHost: () => table.state.role === "guest",
  onImport(file) {
    addMapFile(file);
  },
  onChange(snap) {
    if (table.state.role !== "host") return;
    table.sendMap(snap);
    if (!snap) {
      lastStreamedMap = null;
      return;
    }
    if (snap.id !== lastStreamedMap) {
      lastStreamedMap = snap.id;
      streamActiveMap();
    }
  },
  onView(snap) {
    if (table.state.role !== "host" || !snap) return;
    window.clearTimeout(mapViewTimer);
    mapViewTimer = window.setTimeout(() => table.sendMap(snap), 50);
  },
  onMarks(snap) {
    if (table.state.role !== "guest" || !snap) return;
    table.sendMapEdit({ id: snap.id, tokens: snap.tokens, drawings: snap.drawings });
  },
  onToken(tok, spec) {
    if (!spec) return;
    if (spec.kind === "char") {
      tok.sheetId = spec.sheetId || spec.ref || "";
      tok.ownerId = spec.ownerId || "";
      return;
    }
    chars.fromToken(spec, tok).catch(() => {});
  },
  tokenStatus(t) {
    return chars.tokenStatus?.(t) || "";
  },
});
chars.setNet({
  share: (list) => table.sendChars(list),
  selfId: () => table.state.selfId,
  shareRoll: (row) => table.sendRoll(row),
  mapTokens: () => maps.tokens(),
  mapAim: (fn) => maps.setAim(fn),
  mapProject: (fromId, toId, kind) => maps.project(fromId, toId, kind),
  mapTouch: () => maps.nudgeMarks(),
  mapRefresh: () => maps.refreshTokens(),
});
const bestiary = createBestiary();
const datashard = createDatashard();
const kit = createKit();
const vendors = createVendors(chars);
const blackjack = createBlackjack(chars);
const corps = createCorps();
const npcs = createNpcs();
const srdNpcs = createNpcs({
  prefix: "srdnpc",
  data: "data/srd-npcs.json",
  art: "assets/bestiary",
  artV: "3",
  kinds: KINDS_SRD,
  tokenKind: "srdnpc",
  fivee: true,
  empty: "Pick a person.",
  emptyNone: "No NPCs match.",
  emptyFail: "The NPCs would not load.",
});
const gods = createGods();
const lore = createLore();
const gangs = createGangs();

function closeCatalogs(keep) {
  const boards = { bestiary, datashard, kit, vendors, corps, npcs, srdNpcs, gods, lore, gangs };
  for (const [name, board] of Object.entries(boards)) {
    if (name !== keep) board.closeBoard();
  }
}

function streamActiveMap() {
  const rec = maps.state.maps.get(maps.state.activeId);
  if (!rec?.bytes) return;
  for (const peer of table.state.peers) {
    if (peer.id !== table.state.selfId) table.sendFileTo(peer.id, rec);
  }
}

async function receiveStreamedMap(meta, bytes) {
  if (!bytes || !meta?.id) return;
  const existing = maps.state.maps.get(meta.id);
  if (existing?.url) {
    if (table.state.role === "host") existing.bytes = bytes;
    maps.hydrate(meta.id);
    return;
  }
  const blob = new Blob([bytes], { type: meta.mime || "image/jpeg" });
  const url = URL.createObjectURL(blob);
  const size = meta.w && meta.h ? { w: meta.w, h: meta.h } : await imageSize(url);
  maps.addMap({
    id: meta.id,
    name: meta.name || existing?.name || "Map",
    mime: meta.mime,
    category: "map",
    kind: "map",
    bytes: table.state.role === "host" ? bytes : null,
    url,
    w: size.w,
    h: size.h,
    ephemeral: table.state.role !== "host",
    tokens: existing?.tokens,
    drawings: existing?.drawings,
  });
  maps.hydrate(meta.id);
  if (table.state.role === "host") {
    postUpload(location.origin, { ...meta, category: "map" }, bytes).catch(() => {});
    for (const peer of table.state.peers) {
      if (peer.id !== table.state.selfId && peer.id !== meta.from) {
        table.sendFileTo(peer.id, maps.state.maps.get(meta.id));
      }
    }
  }
}

function applyRemoteMap(meta) {
  if (table.state.role === "host") return;
  if (!meta) {
    maps.closeBoard(false);
    return;
  }
  maps.applyRemote(meta);
  const rec = maps.state.maps.get(meta.id);
  if (!rec?.url) table.requestFile(meta.id);
}

async function addMapFile(file) {
  if (!file) return;
  const bytes = await file.arrayBuffer();
  const mime = file.type || "image/jpeg";
  const url = URL.createObjectURL(new Blob([bytes], { type: mime }));
  const size = await imageSize(url);
  const id = "map-" + Date.now().toString(36) + Math.random().toString(36).slice(2, 5);
  const rec = {
    id,
    name: file.name.replace(/\.[^.]+$/, "") || "Map",
    mime,
    category: "map",
    kind: "map",
    bytes,
    url,
    w: size.w,
    h: size.h,
    ephemeral: table.state.role === "guest",
  };
  maps.addMap(rec);
  maps.show(id);
  if (table.state.role === "host") {
    postUpload(location.origin, rec, bytes).catch(() => {});
  }
}

const updateSfx = createBootSfx();

function paintUpdateRain() {
  const el = $("#update-rain");
  if (!el || el.dataset.ready) return;
  if (window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches) {
    el.dataset.ready = "1";
    return;
  }
  const glyphs = "01<>/\\|#$*ICEBLIGHTNETGITHUBSYNC";
  let html = "";
  for (let i = 0; i < 26; i++) {
    let col = "";
    for (let j = 0; j < 22; j++) col += glyphs[Math.floor(Math.random() * glyphs.length)] + "\n";
    html += `<span style="left:${((i + 0.2) / 26) * 100}%;animation-duration:${(4.2 + (i % 5) * 0.55).toFixed(2)}s;animation-delay:${(-i * 0.37).toFixed(2)}s">${col}</span>`;
  }
  el.innerHTML = html;
  el.dataset.ready = "1";
}

function updateLogLine(text) {
  const log = $("#update-log");
  if (!log) return;
  const line = String(text || "").replace(/\s+/g, " ").trim().slice(0, 88);
  if (!line) return;
  const prev = log.textContent ? log.textContent.split("\n") : [];
  const tagged = "> " + line;
  if (prev[prev.length - 1] === tagged) return;
  prev.push(tagged);
  log.textContent = prev.slice(-16).join("\n");
  log.scrollTop = log.scrollHeight;
}

function paintUpdateHud(st = {}) {
  const stage = $("#update-stage");
  const title = $("#update-title");
  const status = $("#update-status");
  const bar = $("#update-bar");
  const go = $("#launch-update-go");
  const phase = String(st.phase || "");
  const msg = String(st.message || st.error || "");
  let headline = "JACKING IN";
  let goText = "WAIT";
  if (phase === "checking") headline = "HASHING LOCAL SHARDS";
  else if (phase === "downloading") headline = "PULLING REMOTE ICE";
  else if (phase === "done") {
    headline = (Number(st.changed) || 0) > 0 ? "ICE BREACHED" : "ICE CLEAR";
    goText = "DONE";
  } else if (phase === "error") {
    headline = "ICE LOCK";
    goText = "FAIL";
  }
  if (title) title.textContent = headline;
  if (status) status.textContent = (msg || "OPENING A TRACE TO MAIN…").toUpperCase();
  if (go) go.textContent = goText;
  stage?.classList.toggle("is-fail", phase === "error");
  stage?.classList.toggle("is-done", phase === "done");
  const hit = msg.match(/(\d+)\s*\/\s*(\d+)/);
  if (bar) {
    if (hit) {
      stage?.classList.add("has-progress");
      bar.style.width = Math.max(4, Math.min(100, Math.round((Number(hit[1]) / Math.max(1, Number(hit[2]))) * 100))) + "%";
    } else if (phase === "done") {
      stage?.classList.add("has-progress");
      bar.style.width = "100%";
    } else if (phase === "error") {
      stage?.classList.add("has-progress");
      bar.style.width = "100%";
    }
  }
  updateLogLine(msg);
  if (Array.isArray(st.files)) {
    const last = st.files[st.files.length - 1];
    if (last) updateLogLine("PATCH " + last);
  }
}

function closeUpdateStage() {
  const stage = $("#update-stage");
  if (stage) stage.hidden = true;
  $("#launch-update")?.classList.remove("is-busy");
  const go = $("#launch-update-go");
  if (go) go.textContent = "SYNC";
  updateSfx.stop();
}

async function runAppUpdate() {
  const row = $("#launch-update");
  if (row?.classList.contains("is-busy")) return;
  showBnPage("start");
  paintUpdateRain();
  const stage = $("#update-stage");
  const log = $("#update-log");
  if (log) log.textContent = "> TRACE OPEN\n> AUTH: PUBLIC NODE\n> TARGET STEELWORTH/BLIGHTNET#MAIN";
  if (stage) {
    stage.hidden = false;
    stage.classList.remove("is-fail", "is-done", "has-progress");
  }
  const bar = $("#update-bar");
  if (bar) bar.style.width = "";
  row?.classList.add("is-busy");
  paintUpdateHud({ phase: "checking", message: "Jacking into GitHub…" });
  try {
    updateSfx.warmup();
    updateSfx.play("modem", { volume: 0.38 });
    updateSfx.play("ice", { volume: 0.42 });
  } catch {
    /* no audio yet */
  }
  try {
    const start = await fetch("/api/update", { method: "POST", cache: "no-store" });
    let st = await start.json().catch(() => ({}));
    if (start.status === 404) {
      await refreshTable();
      paintUpdateHud({ phase: "done", message: "No updater on this build. Table uploads refreshed.", changed: 0 });
      window.setTimeout(closeUpdateStage, 2200);
      return;
    }
    if (!start.ok) throw new Error(st.error || st.message || "Could not start update");
    let ticks = 0;
    while (st.running) {
      paintUpdateHud(st);
      if (ticks % 2 === 0) updateSfx.tick();
      ticks += 1;
      await new Promise((r) => window.setTimeout(r, 420));
      const res = await fetch("/api/update", { cache: "no-store" });
      st = await res.json().catch(() => ({}));
    }
    if (st.error || st.phase === "error") throw new Error(st.error || st.message || "Update failed");
    await refreshTable();
    const n = Number(st.changed) || 0;
    paintUpdateHud({ ...st, phase: "done", message: n ? (st.message || "Shards patched") : "Already current" });
    try {
      updateSfx.play("land", { volume: 0.5 });
    } catch {
      /* ignore */
    }
    if (n > 0 && st.restart) {
      updateLogLine("SERVER FILES CHANGED — RESTART THE NODE");
      window.setTimeout(() => {
        closeUpdateStage();
        window.alert("Server files updated. Close Blightnet and start it again.");
      }, 1600);
      return;
    }
    if (n > 0) {
      updateLogLine("RELOADING SHELL");
      window.setTimeout(() => location.reload(), 1100);
      return;
    }
    window.setTimeout(closeUpdateStage, 2000);
  } catch (err) {
    const msg = String(err.message || err);
    paintUpdateHud({ phase: "error", message: msg, error: msg });
    try {
      updateSfx.play("beep", { volume: 0.45 });
    } catch {
      /* ignore */
    }
    window.setTimeout(closeUpdateStage, 3200);
  }
}

async function refreshTable() {
  const origin = table.state.role === "guest" ? table.origin() : location.origin;
  try {
    const res = await fetch(new URL("/api/uploads", origin), { cache: "no-store" });
    if (res.ok) {
      const data = await res.json();
      for (const rec of data.files || []) {
        if (rec.category === "map" || rec.kind === "map") continue;
        await ingestRemoteFile(rec, origin);
      }
    }
  } catch {
    /* local only */
  }
  if (table.state.role !== "guest") await loadHostMaps();
  paint();
  const count = $("#mix-count");
  if (count) {
    count.textContent = "Table refreshed";
    window.setTimeout(() => {
      if ($("#mix-search")?.value) return;
      count.textContent = mixCountLabel();
    }, 1600);
  }
}

async function loadHostMaps() {
  try {
    const res = await fetch("/api/uploads", { cache: "no-store" });
    if (!res.ok) return;
    const data = await res.json();
    for (const rec of data.files || []) {
      if (rec.category !== "map" || maps.state.maps.has(rec.id)) continue;
      const fileRes = await fetch(rec.file);
      if (!fileRes.ok) continue;
      const bytes = await fileRes.arrayBuffer();
      const mime = rec.mime || "image/jpeg";
      const url = URL.createObjectURL(new Blob([bytes], { type: mime }));
      const size = await imageSize(url);
      maps.addMap({ ...rec, kind: "map", bytes, url, w: size.w, h: size.h, ephemeral: false });
    }
    maps.fillSelect();
  } catch {
    /* local only */
  }
}

function isLocalHostName(host) {
  const h = String(host || "").replace(/^\[|\]$/g, "").toLowerCase();
  if (!h || h === "localhost" || h === "127.0.0.1" || h === "::1") return true;
  if (h.startsWith("10.") || h.startsWith("192.168.") || h.startsWith("169.254.")) return true;
  const m = h.match(/^172\.(\d+)\./);
  return Boolean(m && Number(m[1]) >= 16 && Number(m[1]) <= 31);
}

function tableShareFromInfo(info) {
  const port = info.port || Number(location.port) || 8765;
  const wan = info.wan || "";
  const wan6 = info.wan6 || (info.ip6 && info.ip6[0]) || "";
  const lan = (info.ips && info.ips[0]) || "";
  const relay = String(info.relay || "").replace(/\/+$/, "");
  const reachableWan = wan && info.upnp ? `${wan}:${port}` : "";
  const net = relay
    ? relay.replace(/^https?:\/\//i, "")
    : lan
      ? `${lan}:${port}`
      : reachableWan || (wan6 ? `[${wan6}]:${port}` : wan ? `${wan}:${port}` : `127.0.0.1:${port}`);
  const copy = relay || (lan ? `${lan}:${port}` : net);
  return { port, wan, wan6, lan, relay, net, copy, upnp: Boolean(info.upnp) };
}

async function copyJoinAddress(share) {
  const text = String(share || "").trim();
  if (!text) return false;
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    try {
      const el = document.createElement("textarea");
      el.value = text;
      document.body.appendChild(el);
      el.select();
      document.execCommand("copy");
      el.remove();
      return true;
    } catch {
      return false;
    }
  }
}

function applyHostShare(info) {
  if (table.state.role !== "host") return;
  const { net, lan, wan, wan6, port, relay, copy } = tableShareFromInfo(info || {});
  tableStatus("Hosting · Gamemaster · " + net, { hosting: true, share: copy || net });
  const copyBtn = $("#table-copy");
  if (copyBtn) {
    copyBtn.hidden = false;
    copyBtn.dataset.share = copy || net;
  }
  return { net, lan, wan, wan6, port, relay, copy };
}

async function hostTable() {
  table.connect(location.origin, "host");
  loadHostMaps();
  pushChat({ sys: true, text: "Table open. You are the Gamemaster. Opening a path for friends on other networks…" });
  let info = {};
  let share = null;
  for (let i = 0; i < 24; i++) {
    try {
      info = await fetchInfo(location.origin);
    } catch {
      info = {};
    }
    share = applyHostShare(info);
    if (info.relay || (info.upnp && info.wan && i >= 4)) break;
    await new Promise((r) => window.setTimeout(r, 500));
  }
  if (table.state.role !== "host") return;
  const { net, lan, wan, wan6, port, relay } = share || tableShareFromInfo(info);
  const bits = [];
  if (lan) bits.push("LAN " + lan + ":" + port);
  if (relay) bits.push("Internet " + relay);
  else if (wan) bits.push("Internet " + wan + ":" + port);
  else if (wan6) bits.push("Internet [" + wan6 + "]:" + port);
  pushChat({ sys: true, text: bits.join(" · ") || net });
  pushChat({
    sys: true,
    text: relay
      ? "Friends anywhere: click Copy address and send that link. They Join with it, or open it in a browser."
      : wan || wan6
        ? "Friends on other networks Join with " + net + ". If that fails, their network cannot reach your router — try again, or open port " + port + "."
        : "No public path yet. Same-house friends use the LAN address. Across the internet, open port " + port + " on your router or install OpenSSH (ssh) so Blightnet can open a tunnel.",
  });
}

function joinTable(addr) {
  let raw = String(addr || "").trim();
  if (!raw) return;
  raw = raw.replace(/^(blightnet|blighnet):\/\//i, "");
  if (!/^https?:\/\//i.test(raw)) {
    const hostport = raw.split("/")[0];
    const ipv4 = /^\d{1,3}(?:\.\d{1,3}){3}(?::\d+)?$/.test(hostport);
    const ipv6 = hostport.startsWith("[");
    raw = (ipv4 || ipv6 ? "http://" : "https://") + raw;
  }
  table.connect(raw, "guest");
  pushChat({ sys: true, text: "Joining " + raw });
}

async function addSoundFile(file) {
  if (!file || !mixer.ctx) return;
  const bytes = await file.arrayBuffer();
  const id = "up-" + Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
  const ext = (file.name.split(".").pop() || "ogg").toLowerCase();
  const mime = file.type || "audio/ogg";
  const meta = {
    id,
    name: file.name.replace(/\.[^.]+$/, "") || "Upload",
    category: "music",
    mime,
    size: bytes.byteLength,
    icon: "lute",
    mood: "calm",
    file: "upload:" + id,
  };
  registerUpload(meta, bytes);
  ui.openCats.add("music");
  if (table.state.role === "host") {
    postUpload(location.origin, meta, bytes).catch(() => {});
    table.send({ type: "files", files: [{ id: meta.id, name: meta.name, category: meta.category, mime: meta.mime, file: meta.file, icon: meta.icon, mood: meta.mood }] });
    for (const peer of table.state.peers) {
      if (peer.id !== table.state.selfId) table.sendFileTo(peer.id, uploads.get(id));
    }
  } else if (table.state.role === "guest") {
    postUpload(table.origin(), meta, bytes).catch(() => {});
    const host = table.state.peers.find((p) => p.role === "host");
    if (host) table.sendFileTo(host.id, uploads.get(id));
  }
  const count = $("#mix-count");
  if (count) count.textContent = "Added " + meta.name;
}

function mixCountLabel() {
  const tokens = searchTokens();
  if (tokens.length) {
    const matches = liveLayers().filter((l) => layerMatches(l, tokens)).length;
    return matches ? `${matches} sound${matches === 1 ? "" : "s"}` : "No sounds match";
  }
  if (ui.playlist) {
    if (ui.playlist.live || (ui.playlist.radio && !(ui.playlist.tracks || []).length)) {
      const st = currentStation(mixer);
      return st ? `${st.call} live` : "On the wire";
    }
    const n = ui.playlist.tracks?.length || 0;
    const label = document.documentElement?.dataset?.theme === "blight" ? "Ruckus" : "Battles";
    return n ? `${label} ${ui.playlist.index + 1}/${n}` : label;
  }
  const n = mixer.playingIds().length;
  return n ? `${n} layer${n === 1 ? "" : "s"}` : "Nothing playing";
}

let nowSig = "";
function renderNow() {
  const ids = mixer.playingIds();
  const sig = ids.join("\0");
  if (sig === nowSig) return;
  nowSig = sig;
  const box = $(".pills");
  if (!ids.length) {
    box.innerHTML = `<span class="empty">Choose a scene, or press any sound.</span>`;
    return;
  }
  box.innerHTML = ids
    .map((id) => {
      const l = layerById(id);
      if (!l) return "";
      return `<button class="pill" data-stop="${id}" title="Stop ${escapeHtml(l.name)}">${icon(l.icon)}${escapeHtml(l.name)}<i aria-hidden="true">×</i></button>`;
    })
    .join("");
}

function paint(opts = {}) {
  renderScenes(opts);
  renderTabs();
  refreshMix();
}

function holdMix() {
  const layers = mixer.snapshot();
  if (!Object.keys(layers).length) return;
  ui.held = {
    scene: ui.scene,
    setting: ui.setting,
    time: ui.time,
    clock: ui.clock,
    place: ui.place,
    musicMood: ui.musicMood,
    playlist: ui.playlist
      ? {
          scene: ui.playlist.scene,
          mood: ui.playlist.mood,
          tracks: [...(ui.playlist.tracks || [])],
          index: ui.playlist.index,
          volume: ui.playlist.volume,
          beds: { ...(ui.playlist.beds || {}) },
          radio: ui.playlist.radio,
          live: ui.playlist.live,
        }
      : null,
    liveStation: mixer.liveStation || selectedStationId(),
    liveRadio: liveWireOn(),
    layers: { ...layers },
  };
}

function syncFade() {
  const btn = $("#fade-out");
  if (!btn) return;
  const live = tableIsSounding();
  const canIn = Boolean(ui.held && Object.keys(ui.held.layers || {}).length);
  btn.disabled = !live && !canIn;
  const label = live ? "Fade out" : "Fade in";
  btn.innerHTML = icon("fade") + label;
  btn.title = live
    ? "Slowly fade the table to silence"
    : "Slowly bring the last mix back";
  btn.setAttribute("aria-label", label);
}

function fadeMix() {
  if (!mixer.ctx) return;
  mixer.resume();
  if (tableIsSounding()) {
    holdMix();
    mixer.stopAll(4.2);
    if (liveWireOn()) liveRadio.fadeStop(4200);
    else liveRadio?.stop();
    ui.scene = null;
    stopPlaylist();
    paint();
    return;
  }
  const held = ui.held;
  if (!held || !Object.keys(held.layers || {}).length) return;
  ui.scene = held.scene;
  if (held.setting) setSetting(held.setting, true);
  if (held.clock != null) setClock(held.clock, true);
  else if (held.time) setTime(held.time, true);
  if (held.place) setPlace(held.place, true);
  if (held.musicMood) ui.musicMood = held.musicMood;
  if (held.playlist) {
    ui.playlist = {
      ...held.playlist,
      beds: { ...(held.playlist.beds || {}) },
      tracks: [...(held.playlist.tracks || [])],
    };
    if (held.playlist.live || held.liveRadio) {
      const st = held.liveStation || selectedStationId();
      if (st) liveRadio?.playStation(st);
    } else {
      armPlaylistTimer();
    }
  } else {
    stopPlaylist();
  }
  mixer.applyScene({ layers: held.layers }, 4.2);
  paint({ scrollActive: true });
}

function applyScene(id) {
  if (mixLocked()) return;
  const scene = findScene(id);
  if (!scene) return;
  if (!mixer.ctx) return;
  mixer.resume();
  ui.scene = scene.id;
  const nextSetting = scene.setting || SCENE_SETTING[scene.id];
  if (nextSetting) setSetting(nextSetting);
  mixer.click();
  holdRadio = true;
  try {
    if (scene.playlist && startPlaylist(scene)) {
      paint({ scrollActive: true });
      return;
    }
    stopPlaylist();
    mixer.applyScene(scene);
    paint({ scrollActive: true });
  } finally {
    window.setTimeout(() => {
      holdRadio = false;
    }, 80);
  }
}

function openSave() {
  const snap = mixer.snapshot();
  if (!Object.keys(snap).length) {
    const count = $("#mix-count");
    if (count) {
      count.textContent = "Play something first";
      window.setTimeout(() => {
        count.textContent = mixCountLabel();
      }, 1400);
    }
    return;
  }
  $("#save-modal").classList.remove("hidden");
  const input = $("#mix-name");
  input.value = "";
  input.focus();
}

function commitSave() {
  const name = $("#mix-name").value.trim() || "Custom mix";
  const snap = mixer.snapshot();
  if (!Object.keys(snap).length) {
    $("#save-modal").classList.add("hidden");
    return;
  }
  const scene = {
    id: "custom-" + Date.now(),
    name,
    blurb: "Saved from the table.",
    icon: "save",
    setting: ui.setting,
    cover: settingSrc(ui.setting, ui.time),
    layers: snap,
  };
  ui.custom.push(scene);
  saveCustom();
  ui.scene = scene.id;
  $("#save-modal").classList.add("hidden");
  paint();
}

function syncPlace() {
  const inside = ui.place === "inside";
  $("#place-out")?.classList.toggle("on", !inside);
  $("#place-in")?.classList.toggle("on", inside);
  $("#place-out")?.setAttribute("aria-pressed", String(!inside));
  $("#place-in")?.setAttribute("aria-pressed", String(inside));
  document.body.dataset.place = ui.place;
}

function setPlace(place, fromNet = false) {
  if (mixLocked() && !fromNet) return;
  ui.place = place === "inside" ? "inside" : "outside";
  localStorage.setItem("hearthsong.place", ui.place);
  mixer.setPlace(ui.place, 0.55);
  syncPlace();
  broadcastMix();
}

function wrapClock(n) {
  n = Math.round(Number(n) || 0) % 1440;
  if (n < 0) n += 1440;
  return n;
}

function periodFromClock(mins) {
  const t = wrapClock(mins);
  if (t >= 21 * 60 || t < 6 * 60) return "night";
  if (t < 11 * 60) return "morning";
  if (t < 17 * 60) return "day";
  return "evening";
}

function clockFromPeriod(period) {
  return { morning: 7 * 60, day: 13 * 60, evening: 18 * 60 + 30, night: 23 * 60 }[period] ?? 13 * 60;
}

function formatClock(mins) {
  const t = wrapClock(mins);
  const h = String(Math.floor(t / 60)).padStart(2, "0");
  const m = String(t % 60).padStart(2, "0");
  return h + ":" + m;
}

function parseClock(value) {
  const hit = String(value || "").match(/^(\d{1,2}):(\d{2})/);
  if (!hit) return null;
  const h = Math.max(0, Math.min(23, Number(hit[1])));
  const m = Math.max(0, Math.min(59, Number(hit[2])));
  return h * 60 + m;
}

function loadClock() {
  const raw = localStorage.getItem("hearthsong.clock");
  if (raw != null && raw !== "") {
    const n = Number(raw);
    if (Number.isFinite(n)) return wrapClock(n);
  }
  const time = HOURS.includes(localStorage.getItem("hearthsong.time"))
    ? localStorage.getItem("hearthsong.time")
    : "day";
  return clockFromPeriod(time);
}

function paintWatch() {
  const mins = wrapClock(ui.clock);
  const digits = formatClock(mins);
  const period = periodFromClock(mins);
  const names = { morning: "Morning", day: "Day", evening: "Evening", night: "Night" };
  const hh = Math.floor(mins / 60);
  const mm = mins % 60;
  const hourDeg = ((hh % 12) + mm / 60) * 30;
  const minDeg = mm * 6;
  $("#watch-hour")?.setAttribute("transform", `rotate(${hourDeg} 32 32)`);
  $("#watch-min")?.setAttribute("transform", `rotate(${minDeg} 32 32)`);
  const dig = $("#watch-digits");
  if (dig) dig.textContent = digits;
  const per = $("#watch-period");
  if (per) per.textContent = names[period] || period;
  const stamp = $("#bn-status-time");
  if (stamp) stamp.textContent = digits;
  const deckClk = $("#net-deck-clk");
  if (deckClk) deckClk.textContent = digits;
  const face = $("#game-watch");
  if (face) {
    face.dataset.period = period;
    face.title = mixLocked()
      ? "In-game time " + digits + " (" + (names[period] || period) + "). The host sets the clock."
      : "In-game time " + digits + " (" + (names[period] || period) + "). Click to set.";
  }
  const locked = mixLocked();
  const slider = $("#watch-slider");
  if (slider) {
    if (document.activeElement !== slider) slider.value = String(mins);
    slider.disabled = locked;
  }
  const input = $("#watch-hhmm");
  if (input) {
    if (document.activeElement !== input) input.value = digits;
    input.disabled = locked;
  }
  $("#watch-back") && ($("#watch-back").disabled = locked);
  $("#watch-fwd") && ($("#watch-fwd").disabled = locked);
  const hint = $("#watch-hint");
  if (hint) {
    hint.textContent = mixLocked()
      ? "The host sets the clock. You still see it."
      : "Slider, type a time, or Morning / Day / Evening / Night. The painting follows.";
  }
}

function setWatchPop(open) {
  const pop = $("#watch-pop");
  const btn = $("#game-watch");
  if (!pop || !btn) return;
  pop.hidden = !open;
  btn.setAttribute("aria-expanded", String(Boolean(open)));
}

function syncTime() {
  for (const hour of HOURS) {
    const btn = $(`#time-${hour}`);
    const on = ui.time === hour;
    btn?.classList.toggle("on", on);
    btn?.setAttribute("aria-pressed", String(on));
  }
  document.body.dataset.time = ui.time;
  paintSky();
  paintWatch();
}

function setClock(mins, fromNet = false) {
  if (mixLocked() && !fromNet) return;
  ui.clock = wrapClock(mins);
  ui.time = periodFromClock(ui.clock);
  localStorage.setItem("hearthsong.clock", String(ui.clock));
  localStorage.setItem("hearthsong.time", ui.time);
  mixer.setTime(ui.time, 0.7);
  syncTime();
  if (!fromNet) broadcastMix();
}

function setTime(time, fromNet = false) {
  setClock(clockFromPeriod(HOURS.includes(time) ? time : "day"), fromNet);
}

function syncSetting() {
  const setting = settingById(ui.setting);
  ui.setting = setting.id;
  document.body.dataset.setting = setting.id;
  const btn = $("#setting-btn");
  if (btn) {
    btn.innerHTML = `${icon(setting.icon)}<span>${setting.name}</span>`;
    btn.setAttribute("aria-label", "Place: " + setting.name);
  }
  const list = $("#setting-list");
  if (list) {
    for (const opt of list.querySelectorAll(".setting-opt")) {
      const on = opt.dataset.setting === setting.id;
      opt.classList.toggle("on", on);
      opt.setAttribute("aria-selected", String(on));
    }
  }
  paintSky();
}

function setSetting(id, fromNet = false) {
  if (mixLocked() && !fromNet) return;
  ui.setting = settingById(id).id;
  localStorage.setItem("hearthsong.setting", ui.setting);
  syncSetting();
  broadcastMix();
}

function renderSettingMenu() {
  const list = $("#setting-list");
  if (!list) return;
  list.innerHTML = settingsOf().map(
    (s) =>
      `<button type="button" class="setting-opt ${ui.setting === s.id ? "on" : ""}" role="option" data-setting="${s.id}" aria-selected="${ui.setting === s.id}">${icon(s.icon)}${s.name}</button>`
  ).join("");
}

function toggleSettingMenu(open) {
  const menu = $("#setting-menu");
  const list = $("#setting-list");
  const btn = $("#setting-btn");
  if (!menu || !list || !btn) return;
  const show = open ?? list.hidden;
  if (!show && list.hidden) return;
  list.hidden = !show;
  menu.classList.toggle("open", show);
  btn.setAttribute("aria-expanded", String(show));
}

let skySig = "";
function paintSky() {
  const sky = weatherOf(mixer);
  const sig = [sky.src, sky.kind, sky.time, sky.setting, sky.indoor ? "1" : "0", sky.label].join("|");
  if (sig === skySig) return;
  skySig = sig;
  const board = $(".sky-board");
  if (board) {
    board.dataset.kind = sky.kind;
    board.dataset.hour = sky.time;
    board.dataset.setting = sky.setting;
    board.classList.toggle("indoor", Boolean(sky.indoor));
  }
  const label = $("#weather-label");
  if (label) label.textContent = sky.label;
  setSkySrc(sky.src, sky.time, sky.kind);
}

function skyFrontBack() {
  const a = $("#weather-art");
  const b = $("#weather-art-b");
  if (!a) return { front: null, back: null };
  if (!b) return { front: a, back: null };
  const even = ui.skySlot % 2 === 0;
  return even ? { front: a, back: b } : { front: b, back: a };
}

let skyFadeTimer = 0;
let skyLoadGen = 0;

function markSkyFade() {
  const board = $(".sky-board");
  if (!board) return;
  board.classList.add("crossfading");
  window.clearTimeout(skyFadeTimer);
  skyFadeTimer = window.setTimeout(() => board.classList.remove("crossfading"), 1450);
}

function setSkySrc(src, hour, kind, tried) {
  const { front, back } = skyFrontBack();
  if (!front) return;
  if (front.getAttribute("data-sky") === src) {
    front.dataset.hour = hour || front.dataset.hour;
    front.dataset.kind = kind || front.dataset.kind;
    return;
  }
  const target = back || front;
  const gen = ++skyLoadGen;
  const next = new Image();
  next.onload = () => {
    if (gen !== skyLoadGen) return;
    target.src = src;
    target.decoding = "async";
    target.setAttribute("data-sky", src);
    if (hour) target.dataset.hour = hour;
    if (kind) target.dataset.kind = kind;
    markSkyFade();
    if (back && back !== front) {
      back.classList.add("on");
      front.classList.remove("on");
      ui.skySlot += 1;
    } else {
      front.classList.add("on");
    }
  };
  next.onerror = () => {
    if (gen !== skyLoadGen) return;
    const step = Number(tried) || 0;
    if (step >= 2) return;
    if (step < 1) {
      const byHour = src.replace(/-(morning|evening|night)\.jpg$/, "-day.jpg");
      if (byHour !== src) {
        setSkySrc(byHour, hour, kind, 1);
        return;
      }
    }
    const byPlace = src.replace(/\/[a-z]+-(morning|day|evening|night)\.jpg$/, "/castle-$1.jpg");
    if (byPlace !== src) setSkySrc(byPlace, hour, kind, 2);
  };
  next.src = src;
}

function initViz() {
  const canvas = $("#viz-sky");
  if (!canvas || canvas.dataset.ready) return;
  canvas.dataset.ready = "1";
  createVisualizer({ canvas, mixer });
  paintSky();
}

async function boot() {
  const gate = $("#gate");
  const status = $("#gate-status");
  const light = $("#light");
  status.textContent = "Lighting the hearth…";
  try {
    await mixer.init();
    liveRadio = createLiveRadio(mixer);
  } catch (err) {
    console.error(err);
    status.textContent = "The browser blocked audio. Click again, or try another browser.";
    $("#gate-bar")?.classList.remove("busy");
    light.disabled = false;
    return;
  }
  mixer.onChange = () => {
    renderNow();
    renderLayers();
    paintSky();
    syncFade();
    broadcastMix();
    tuneRadio();
  };
  mixer.setPlace(ui.place, 0.05);
  mixer.setTime(ui.time, 0.05);
  syncPlace();
  syncTime();
  syncSetting();
  const rawMaster = localStorage.getItem("hearthsong.master");
  if (rawMaster !== null && rawMaster !== "") {
    const storedMaster = Number(rawMaster);
    if (!Number.isNaN(storedMaster) && storedMaster >= 0 && storedMaster <= 1) {
      mixer.setMaster(storedMaster, 0);
      const slider = $("#master");
      slider.value = String(Math.round(storedMaster * 100));
      slider.parentElement.style.setProperty("--p", slider.value + "%");
    }
  }
  paint();
  initViz();
  playIntro();
  hearthReady = true;
  for (const rec of uploads.values()) {
    if (rec.bytes) await mixer.ingestAudio(rec.file || "upload:" + rec.id, rec.bytes);
  }
  for (const item of pendingUploads.splice(0)) {
    registerUpload(item.meta, item.bytes);
  }
  if (pendingMix) applyRemoteMix(pendingMix);
  gate.classList.add("hidden");
  light.blur();
  window.setTimeout(() => {
    if (gate.classList.contains("hidden")) gate.style.display = "none";
  }, 750);
  const sceneParam = params.get("scene") || params.get("preview");
  if (sceneParam && findScene(sceneParam)) applyScene(sceneParam);
}

function currentTheme() {
  return document.documentElement.dataset.theme === "blight" ? "blight" : "hearthsong";
}

function applyTheme(id) {
  const theme = id === "blight" ? "blight" : "hearthsong";
  const prev = document.documentElement.dataset.theme;
  document.documentElement.dataset.theme = theme;
  document.body.dataset.theme = theme;
  try {
    localStorage.setItem("hearthsong.theme", theme);
  } catch {
    /* ignore */
  }
  $("#theme-hearthsong")?.classList.toggle("on", theme === "hearthsong");
  $("#theme-blight")?.classList.toggle("on", theme === "blight");
  syncBestiaryTheme();
  if (prev && prev !== theme) applyWorld(theme);
}

function syncBestiaryTheme() {
  const blight = currentTheme() === "blight";
  const beastBtn = $("#bestiary-toggle");
  const shardBtn = $("#datashard-toggle");
  const kitBtn = $("#kit-toggle");
  if (beastBtn) beastBtn.hidden = blight;
  if (shardBtn) shardBtn.hidden = !blight;
  const corpBtn = $("#corp-toggle");
  if (corpBtn) corpBtn.hidden = !blight;
  if (!blight) corps.closeBoard();
  const npcBtn = $("#npc-toggle");
  if (npcBtn) npcBtn.hidden = !blight;
  if (!blight) npcs.closeBoard();
  const srdNpcBtn = $("#srdnpc-toggle");
  if (srdNpcBtn) srdNpcBtn.hidden = blight;
  if (blight) srdNpcs.closeBoard();
  const godBtn = $("#god-toggle");
  if (godBtn) godBtn.hidden = blight;
  if (blight) gods.closeBoard();
  const loreBtn = $("#lore-toggle");
  if (loreBtn) loreBtn.hidden = !blight;
  if (!blight) lore.closeBoard();
  const gangBtn = $("#gang-toggle");
  if (gangBtn) gangBtn.hidden = !blight;
  if (!blight) gangs.closeBoard();
  const bjBtn = $("#bj-toggle");
  if (bjBtn) bjBtn.hidden = !blight;
  const launchBj = $("#launch-bj");
  if (launchBj) launchBj.hidden = !blight;
  if (!blight) blackjack.close();
  if (kitBtn) {
    kitBtn.hidden = false;
    kitBtn.innerHTML = icon("swords") + (blight ? "Night Market" : "Armory");
  }
  const vendorBtn = $("#vendor-toggle");
  if (vendorBtn) vendorBtn.innerHTML = icon("market") + "Vendors";
  vendors.syncTheme();
  blackjack.paint();
  if (blight) bestiary.closeBoard();
  else datashard.closeBoard();
  kit.syncTheme();
}

function applyWorld(theme) {
  if (theme !== "blight") liveRadio?.stop();
  const live = new Set(liveLayers().map((l) => l.id));
  for (const id of mixer.playingIds()) {
    if (!live.has(id)) mixer.stop(id, 0.55);
  }
  const moods = liveMoods();
  if (ui.musicMood !== "all" && !moods.some((m) => m.id === ui.musicMood)) ui.musicMood = "all";
  const still =
    liveScenes().find((s) => s.id === ui.scene) || ui.custom.find((s) => s.id === ui.scene) || null;
  if (ui.scene && !still) ui.scene = null;
  if (ui.playlist) {
    if (theme === "blight") {
      stopPlaylist();
    } else {
      const tracks = tracksForMood("battle");
      if (tracks.length) ui.playlist = { mood: "battle", tracks, index: 0 };
      else stopPlaylist();
    }
  }
  if (still && mixer.ctx) applyScene(still.id);
  ui.setting = settingById(ui.setting).id;
  localStorage.setItem("hearthsong.setting", ui.setting);
  renderSettingMenu();
  syncSetting();
  paint();
  paintSky();
  syncNetcast();
  paintRadioChip(mixer);
  chars.render();
}

function sceneBedsOrCurrent() {
  const scene = ui.scene ? findScene(ui.scene) : null;
  if (scene) return sceneBeds(scene);
  return ui.playlist?.beds ? { ...ui.playlist.beds } : {};
}

function tuneToStation(id) {
  if (worldOf() !== "blight") return;
  if (!mixer.ctx) mixer.resume();
  mixer.resume();
  const beds = sceneBedsOrCurrent();
  if (isLiveRadio()) {
    const st = liveStationById(id) || defaultLiveForMood(id);
    if (!st) return;
    for (const layer of playingMusic()) mixer.stop(layer.id, 0.35);
    liveRadio?.playStation(st.id);
    ui.musicMood = st.vibe;
    ui.playlist = {
      scene: ui.scene,
      mood: st.vibe,
      tracks: [],
      index: 0,
      volume: ui.playlist?.volume || 0.56,
      beds,
      radio: true,
      live: true,
    };
    mixer.applyScene({ layers: beds });
    ui.openCats.add("music");
    paintRadioChip(mixer);
    paint();
    return;
  }
  liveRadio?.stop();
  const st = libraryStationById(id) || stationForMood(id);
  const mood = st?.id || id;
  const tracks = shuffleIds(tracksForMood(mood));
  if (!tracks.length) return;
  setSelectedStationId(mood);
  mixer.liveMood = mood;
  ui.musicMood = mood;
  ui.playlist = {
    scene: ui.scene,
    mood,
    tracks,
    index: 0,
    volume: ui.playlist?.volume || 0.56,
    beds,
    radio: true,
    live: false,
  };
  const first = tracks[0];
  mixer.applyScene({ layers: { ...beds, [first]: ui.playlist.volume } });
  ui.openCats.add("music");
  armPlaylistTimer();
  paintRadioChip(mixer);
  paint();
}

function stepStation(dir) {
  if (mixLocked()) return;
  const st = currentStation(mixer);
  const next = neighborStation(st?.id, dir);
  if (next) tuneToStation(next.id);
}

function tuneRadio() {
  if (mixLocked()) {
    paintRadioChip(mixer);
    return;
  }
  if (holdRadio) {
    paintRadioChip(mixer);
    return;
  }
  if (worldOf() !== "blight" || !mixer.ctx) {
    paintRadioChip(mixer);
    return;
  }
  if (isLiveRadio()) {
    paintRadioChip(mixer);
    return;
  }
  const live = playingMusic();
  if (!live.length) {
    if (ui.playlist?.radio && !ui.playlist.live) stopPlaylist();
    paintRadioChip(mixer);
    return;
  }
  const mood = live[0].mood;
  if (!stationForMood(mood)) {
    paintRadioChip(mixer);
    return;
  }
  if (ui.playlist?.radio && ui.playlist.mood === mood) {
    const i = ui.playlist.tracks.indexOf(live[0].id);
    if (i >= 0) ui.playlist.index = i;
    paintRadioChip(mixer);
    return;
  }
  const tracks = shuffleIds(tracksForMood(mood));
  if (tracks.length < 2) {
    paintRadioChip(mixer);
    return;
  }
  let idx = tracks.indexOf(live[0].id);
  if (idx < 0) {
    tracks.unshift(live[0].id);
    idx = 0;
  }
  const scene = ui.scene ? findScene(ui.scene) : null;
  ui.playlist = {
    radio: true,
    mood,
    tracks,
    index: idx,
    scene: ui.scene,
    volume: mixer.volumeOf(live[0].id) || 0.56,
    beds: scene ? sceneBeds(scene) : {},
  };
  armPlaylistTimer();
  paintRadioChip(mixer);
}



function paintDisplays(list) {
  const root = $("#display-list");
  if (!root) return;
  const rows = list || [];
  if (!rows.length) {
    root.innerHTML = `<span class="launch-screen-empty">No other screens found.</span>`;
    return;
  }
  root.innerHTML = rows
    .map((d) => {
      const hz = d.hz ? ` · ${d.hz} Hz` : "";
      const on = d.on ? " on" : "";
      return `<button type="button" class="launch-screen-btn${on}" data-display="${escapeHtml(d.name)}"><span class="net-mon-glass" aria-hidden="true"></span><b>${escapeHtml(d.name)}</b><small>${d.w}×${d.h}${hz}</small></button>`;
    })
    .join("");
}

async function bindDisplayPicker() {
  const root = $("#display-list");
  if (!root) return;
  root.addEventListener("click", async (e) => {
    const btn = e.target.closest("[data-display]");
    if (!btn) return;
    e.stopPropagation();
    try {
      const res = await fetch("/api/display", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: btn.dataset.display }),
      });
      const data = await res.json();
      if (data?.displays) paintDisplays(data.displays);
      const d = data?.display;
      if (d && Number.isFinite(d.x) && Number.isFinite(d.y)) {
        try {
          window.moveTo(d.x, d.y);
          window.resizeTo(d.w, d.h);
        } catch {
          /* OS window manager owns fullscreen */
        }
      }
    } catch {
      /* stay */
    }
  });
  try {
    const res = await fetch("/api/displays", { cache: "no-store" });
    const data = await res.json();
    paintDisplays(data.displays || []);
  } catch {
    paintDisplays([]);
  }
}

function showBnPage(page) {
  bnPage = page === "hearthsong" ? "hearthsong" : "start";
  const home = $("#bn-start");
  const view = $("#bn-viewport");
  if (home) home.hidden = bnPage !== "start";
  if (view) view.hidden = bnPage !== "hearthsong";
  syncBnChrome();
}

function syncBnChrome() {
  $("#bn-tab-start")?.classList.toggle("on", bnPage === "start");
  $("#bn-tab-hearthsong")?.classList.toggle("on", bnPage === "hearthsong");
  const url = $("#bn-omnibox");
  if (url && document.activeElement !== url) {
    if (table.state.role === "host" && url.dataset.share) url.value = url.dataset.share;
    else if (table.state.role === "guest" && table.state.hostUrl) {
      url.value = table.state.hostUrl.replace(/^https?:\/\//i, "blightnet://");
    } else {
      url.value = bnPage === "hearthsong" ? "blightnet://table" : "blightnet://start";
    }
  }
  const back = $("#bn-back");
  const fwd = $("#bn-fwd");
  if (back) back.disabled = bnPage === "start";
  if (fwd) fwd.disabled = bnPage === "hearthsong" || !hearthsongOpened;
  document.title = bnPage === "hearthsong" ? "Blightnet — table" : "Blightnet";
  const link = $("#bn-status-link");
  const role = table.state.role === "host" ? "HOST" : table.state.role === "guest" ? "GUEST" : "LOCAL";
  if (link) link.textContent = role;
  const deckLink = $("#net-deck-link");
  if (deckLink) deckLink.textContent = role;
  const page = $("#bn-status-page");
  if (page) page.textContent = bnPage === "hearthsong" ? "TABLE" : "INDEX";
}

function quitBlightnet() {
  try {
    navigator.sendBeacon("/api/quit");
  } catch {
    try {
      fetch("/api/quit", { method: "POST", keepalive: true });
    } catch {
      /* ignore */
    }
  }
  try {
    window.close();
  } catch {
    /* ignore */
  }
}

function submitOmnibox() {
  const raw = ($("#bn-omnibox")?.value || "").trim();
  const low = raw.toLowerCase().replace(/^blighnet:\/\//, "blightnet://");
  if (low === "blightnet://quit" || low === "blightnet://exit") {
    quitBlightnet();
    return;
  }
  if (!raw || low === "blightnet://start" || low === "blightnet:///" || low === "blightnet://") {
    showBnPage("start");
    return;
  }
  if (low.includes("hearthsong") || low.includes("://table")) {
    startHearthsong();
    return;
  }
  joinTable(raw.replace(/^(blightnet|blighnet):\/\//i, ""));
}

function startHearthsong() {
  if (location.protocol === "file:") return;
  const light = $("#light");
  hearthsongOpened = true;
  $("#boot")?.classList.add("hidden");
  showBnPage("hearthsong");
  if (light && !light.disabled) light.click();
}

function playIntro() {
  const root = document.documentElement;
  const reduce = window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches;
  if (reduce) {
    root.classList.remove("unrolling", "unroll");
    root.classList.add("settled");
    return;
  }
  root.classList.add("unrolling");
  requestAnimationFrame(() => {
    requestAnimationFrame(() => root.classList.add("unroll"));
  });
  window.setTimeout(() => {
    root.classList.remove("unrolling", "unroll");
    root.classList.add("settled");
  }, 2600);
}

function bind() {
  if (location.protocol === "file:") {
    const light = $("#light");
    light.disabled = true;
    light.textContent = "Run start.sh first";
    $("#gate").querySelector("p").textContent =
      "Double-click start.sh in this folder (or run it in a terminal). Then open the address it prints — usually http://127.0.0.1:8765";
    const launch = $("#launch-hearthsong");
    if (launch) {
      launch.disabled = true;
      launch.querySelector("small").textContent = "RUN START.SH FIRST";
    }
    $("#table-host") && ($("#table-host").disabled = true);
    $("#table-join") && ($("#table-join").disabled = true);
  }

  applyTheme(currentTheme());
  playBoot();
  if (/(hearthsong|table)/i.test(String(location.hash || ""))) startHearthsong();
  $("#launch-hearthsong")?.addEventListener("click", (e) => {
    e.stopPropagation();
    startHearthsong();
  });
  $("#launch-quit")?.addEventListener("click", (e) => {
    e.stopPropagation();
    quitBlightnet();
  });
  const guide = $("#guide");
  const openGuide = () => {
    if (!guide) return;
    guide.classList.remove("hidden");
    guide.setAttribute("aria-hidden", "false");
  };
  const closeGuide = () => {
    if (!guide) return;
    guide.classList.add("hidden");
    guide.setAttribute("aria-hidden", "true");
  };
  $("#launch-guide")?.addEventListener("click", (e) => {
    e.stopPropagation();
    openGuide();
  });
  $("#guide-close")?.addEventListener("click", (e) => {
    e.stopPropagation();
    closeGuide();
  });
  guide?.addEventListener("click", (e) => {
    if (e.target.id === "guide") closeGuide();
  });
  bindDisplayPicker();
  $("#bn-tab-start")?.addEventListener("click", () => showBnPage("start"));
  $("#bn-tab-hearthsong")?.addEventListener("click", () => startHearthsong());
  $("#bn-back")?.addEventListener("click", () => showBnPage("start"));
  $("#bn-fwd")?.addEventListener("click", () => startHearthsong());
  $("#bn-home")?.addEventListener("click", () => showBnPage("start"));
  $("#bn-close")?.addEventListener("click", () => showBnPage("start"));
  $("#bn-reload")?.addEventListener("click", () => {
    location.reload();
  });
  $("#bn-omnibox")?.addEventListener("keydown", (e) => {
    if (e.key === "Enter") submitOmnibox();
  });
  $("#theme-hearthsong")?.addEventListener("click", () => applyTheme("hearthsong"));
  $("#theme-blight")?.addEventListener("click", () => applyTheme("blight"));

  $("#light").addEventListener("click", () => {
    if ($("#light").disabled) return;
    $("#light").disabled = true;
    $("#gate-progress").hidden = false;
    $("#gate-bar")?.classList.add("busy");
    boot().catch((err) => {
      console.error(err);
      $("#gate-status").textContent = "Something went wrong. Click again.";
      $("#gate-bar")?.classList.remove("busy");
      $("#light").disabled = false;
    });
  });

  document.addEventListener("click", (e) => {
    if (!e.target.closest("#setting-menu")) toggleSettingMenu(false);

    const forget = e.target.closest("[data-forget]");
    if (forget) {
      e.stopPropagation();
      ui.custom = ui.custom.filter((s) => s.id !== forget.dataset.forget);
      saveCustom();
      if (ui.scene === forget.dataset.forget) ui.scene = null;
      paint();
      return;
    }

    const sceneBtn = e.target.closest("[data-scene]");
    if (sceneBtn) {
      applyScene(sceneBtn.dataset.scene);
      return;
    }

    const catBtn = e.target.closest("[data-cat]");
    if (catBtn) {
      const id = catBtn.dataset.cat;
      if (ui.openCats.has(id)) ui.openCats.delete(id);
      else ui.openCats.add(id);
      renderLayers();
      return;
    }

    const queueBtn = e.target.closest("[data-playlist-track]");
    if (queueBtn) {
      const index = Number(queueBtn.dataset.playlistTrack);
      if (Number.isFinite(index)) playPlaylistIndex(index);
      return;
    }

    const moodBtn = e.target.closest("[data-mood-filter]");
    if (moodBtn) {
      ui.musicMood = moodBtn.dataset.moodFilter;
      ui.openCats.add("music");
      renderLayers();
      return;
    }

    const filterBtn = e.target.closest("[data-filter]");
    if (filterBtn) {
      ui.filter = filterBtn.dataset.filter;
      if (ui.filter !== "all" && ui.filter !== "on") ui.openCats.add(ui.filter);
      renderTabs();
      renderLayers();
      return;
    }

    const toggle = e.target.closest("[data-toggle]");
    if (toggle) {
      if (mixLocked()) return;
      mixer.resume();
      const id = toggle.dataset.toggle;
      if (ui.playlist) {
        const idx = ui.playlist.tracks.indexOf(id);
        if (idx >= 0) {
          if (idx === ui.playlist.index && mixer.isPlaying(id)) mixer.stop(id, 0.5);
          else playPlaylistIndex(idx);
          return;
        }
      }
      mixer.toggle(id);
      return;
    }

    const pill = e.target.closest("[data-stop]");
    if (pill) {
      if (mixLocked()) return;
      mixer.stop(pill.dataset.stop, 0.4);
      return;
    }

    const settingOpt = e.target.closest(".setting-opt[data-setting]");
    if (settingOpt) {
      setSetting(settingOpt.dataset.setting);
      toggleSettingMenu(false);
    }
  });

  document.addEventListener("input", (e) => {
    if (e.target.dataset.vol) {
      if (mixLocked()) return;
      const id = e.target.dataset.vol;
      const v = Number(e.target.value) / 100;
      mixer.setVolume(id, v);
      const row = e.target.closest(".row");
      if (row) {
        row.style.setProperty("--p", e.target.value + "%");
        const label = row.querySelector(".vol");
        if (label) label.textContent = e.target.value;
      }
      broadcastMix();
    }
    if (e.target.id === "master") {
      const v = Number(e.target.value) / 100;
      mixer.setMaster(v);
      e.target.parentElement.style.setProperty("--p", e.target.value + "%");
      localStorage.setItem("hearthsong.master", String(v));
    }
    if (e.target.id === "mix-search") {
      ui.search = e.target.value;
      renderLayers();
    }
    if (e.target.id === "scene-search") {
      ui.sceneQuery = e.target.value;
      renderScenes();
    }
  });

  const searchIco = $(".mix-search .search-ico");
  if (searchIco) searchIco.innerHTML = icon("search");
  syncFade();
  $("#stop-all").innerHTML = icon("stop") + "Silence";
  $("#save-mix").innerHTML = icon("save") + "Save this mix";
  const shuffleBtn = $("#shuffle");
  if (shuffleBtn) {
    shuffleBtn.innerHTML = icon("shuffle") + "Shuffle";
    shuffleBtn.addEventListener("click", shuffleMusic);
  }
  $("#credits-btn").innerHTML = icon("info");
  $("#credits-btn").addEventListener("click", () => $("#credits-modal").classList.remove("hidden"));
  $("#credits-close").addEventListener("click", () => $("#credits-modal").classList.add("hidden"));
  $("#credits-modal").addEventListener("click", (e) => {
    if (e.target.id === "credits-modal") $("#credits-modal").classList.add("hidden");
  });
  $("#save-modal").addEventListener("click", (e) => {
    if (e.target.id === "save-modal") $("#save-modal").classList.add("hidden");
  });

  $("#stop-all").addEventListener("click", () => {
    holdMix();
    mixer.stopAll(0.45);
    liveRadio?.stop();
    ui.scene = null;
    stopPlaylist();
    paint();
  });

  $("#fade-out").addEventListener("click", fadeMix);

  $("#save-mix").addEventListener("click", openSave);
  document.addEventListener("keydown", (e) => {
    if (e.key === "/" && !e.metaKey && !e.ctrlKey && !e.altKey) {
      const tag = (e.target && e.target.tagName) || "";
      if (tag !== "INPUT" && tag !== "TEXTAREA") {
        e.preventDefault();
        $("#mix-search")?.focus();
        return;
      }
    }
    if (e.key !== "Escape") return;
    if (chars.cancelAim?.()) {
      e.preventDefault();
      return;
    }
    if ($("#guide") && !$("#guide").classList.contains("hidden")) {
      $("#guide").classList.add("hidden");
      $("#guide").setAttribute("aria-hidden", "true");
      return;
    }
    const tag = (e.target && e.target.tagName) || "";
    const fieldId = (e.target && e.target.id) || "";
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") {
      if (fieldId === "mix-search" && e.target.value) {
        e.target.value = "";
        ui.search = "";
        renderLayers();
        return;
      }
      if (fieldId === "scene-search" && e.target.value) {
        e.target.value = "";
        ui.sceneQuery = "";
        renderScenes();
        return;
      }
      if ((fieldId === "beast-search" || fieldId === "shard-search" || fieldId === "kit-search" || fieldId === "vendor-search" || fieldId === "corp-search" || fieldId === "npc-search" || fieldId === "srdnpc-search" || fieldId === "god-search" || fieldId === "lore-search" || fieldId === "gang-search") && e.target.value) {
        e.target.value = "";
        e.target.dispatchEvent(new Event("input", { bubbles: true }));
        return;
      }
      e.target.blur();
      return;
    }
    if ($("#setting-list") && !$("#setting-list").hidden) {
      toggleSettingMenu(false);
      return;
    }
    if ($("#watch-pop") && !$("#watch-pop").hidden) {
      setWatchPop(false);
      return;
    }
    if (!$("#join-modal")?.classList.contains("hidden")) {
      $("#join-modal").classList.add("hidden");
      return;
    }
    if (!$("#save-modal").classList.contains("hidden")) {
      $("#save-modal").classList.add("hidden");
      return;
    }
    if (!$("#credits-modal")?.classList.contains("hidden")) {
      $("#credits-modal").classList.add("hidden");
      return;
    }
    holdMix();
    mixer.stopAll(0.45);
    liveRadio?.stop();
    ui.scene = null;
    stopPlaylist();
    paint();
  });
  $("#save-cancel").addEventListener("click", () => $("#save-modal").classList.add("hidden"));
  $("#save-confirm").addEventListener("click", commitSave);
  $("#mix-name").addEventListener("keydown", (e) => {
    if (e.key === "Enter") commitSave();
    if (e.key === "Escape") $("#save-modal").classList.add("hidden");
  });

  $("#place-out").addEventListener("click", () => setPlace("outside"));
  $("#place-in").addEventListener("click", () => setPlace("inside"));
  const hourIcons = { morning: "dawn", day: "sun", evening: "dusk", night: "moon" };
  const hourNames = { morning: "Morning", day: "Day", evening: "Evening", night: "Night" };
  for (const hour of HOURS) {
    const btn = $(`#time-${hour}`);
    if (!btn) continue;
    btn.innerHTML = icon(hourIcons[hour]) + hourNames[hour];
    btn.addEventListener("click", () => setTime(hour));
  }
  mixer.time = ui.time;
  mixer.place = ui.place;
  ui.time = periodFromClock(ui.clock);
  paintWatch();
  let watchDrag = false;
  let watchMoved = false;
  const watchFace = $("#game-watch");
  const clockFromPointer = (ev) => {
    const svg = watchFace?.querySelector("svg");
    if (!svg) return ui.clock;
    const r = svg.getBoundingClientRect();
    const x = ev.clientX - (r.left + r.width / 2);
    const y = ev.clientY - (r.top + r.height / 2);
    let deg = (Math.atan2(y, x) * 180) / Math.PI + 90;
    if (deg < 0) deg += 360;
    const mins12 = Math.round((deg / 360) * 12 * 60) % (12 * 60);
    return ui.clock >= 12 * 60 ? mins12 + 12 * 60 : mins12;
  };
  watchFace?.addEventListener("pointerdown", (e) => {
    if (!e.target.closest("svg") || mixLocked()) return;
    watchDrag = true;
    watchMoved = false;
    e.preventDefault();
    watchFace.setPointerCapture?.(e.pointerId);
    setClock(clockFromPointer(e));
  });
  watchFace?.addEventListener("pointermove", (e) => {
    if (!watchDrag) return;
    watchMoved = true;
    setClock(clockFromPointer(e));
  });
  watchFace?.addEventListener("pointerup", () => {
    watchDrag = false;
  });
  watchFace?.addEventListener("click", (e) => {
    e.stopPropagation();
    if (watchMoved) {
      watchMoved = false;
      return;
    }
    const pop = $("#watch-pop");
    setWatchPop(Boolean(pop?.hidden));
  });
  $("#watch-slider")?.addEventListener("input", (e) => setClock(e.target.value));
  $("#watch-hhmm")?.addEventListener("change", (e) => {
    const n = parseClock(e.target.value);
    if (n != null) setClock(n);
  });
  $("#watch-back")?.addEventListener("click", (e) => {
    e.stopPropagation();
    setClock(ui.clock - 15);
  });
  $("#watch-fwd")?.addEventListener("click", (e) => {
    e.stopPropagation();
    setClock(ui.clock + 15);
  });
  document.addEventListener("click", (e) => {
    if (!e.target.closest(".game-watch-wrap")) setWatchPop(false);
  });
  const nameInput = $("#profile-name");
  if (nameInput) {
    nameInput.value = profileName();
    nameInput.addEventListener("change", () => {
      const next = setProfileName(nameInput.value);
      nameInput.value = next;
      table.rename(next);
    });
  }
  paintOwnFace();
  $("#profile-pic-btn")?.addEventListener("click", () => $("#profile-pic-file")?.click());
  $("#profile-pic-file")?.addEventListener("change", async (e) => {
    const file = e.target.files && e.target.files[0];
    e.target.value = "";
    if (!file) return;
    try {
      const dataUrl = await resizeImage(file, 128, 0.8);
      setProfilePic(dataUrl);
      paintOwnFace();
      table.sendProfile(dataUrl);
    } catch {
      pushChat({ sys: true, text: "Could not read that picture." });
    }
  });
  $("#table-host")?.addEventListener("click", hostTable);
  $("#table-join")?.addEventListener("click", () => {
    $("#join-modal")?.classList.remove("hidden");
    $("#join-addr")?.focus();
  });
  $("#table-copy")?.addEventListener("click", async () => {
    const share = $("#table-copy")?.dataset.share || $("#bn-omnibox")?.dataset.share || "";
    const ok = await copyJoinAddress(String(share).replace(/^(blightnet|blighnet):\/\//i, ""));
    pushChat({ sys: true, text: ok ? "Join address copied." : "Could not copy. The address is in the bar." });
  });
  $("#table-leave")?.addEventListener("click", () => {
    table.disconnect();
    const copyBtn = $("#table-copy");
    if (copyBtn) copyBtn.hidden = true;
    pushChat({ sys: true, text: "Left the table." });
  });
  $("#join-cancel")?.addEventListener("click", () => $("#join-modal")?.classList.add("hidden"));
  $("#join-confirm")?.addEventListener("click", () => {
    const addr = $("#join-addr")?.value;
    $("#join-modal")?.classList.add("hidden");
    joinTable(addr);
  });
  $("#join-addr")?.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      const addr = $("#join-addr")?.value;
      $("#join-modal")?.classList.add("hidden");
      joinTable(addr);
    }
    if (e.key === "Escape") $("#join-modal")?.classList.add("hidden");
  });
  $("#join-modal")?.addEventListener("click", (e) => {
    if (e.target.id === "join-modal") $("#join-modal").classList.add("hidden");
  });
  $("#chat-toggle")?.addEventListener("click", () => {
    const panel = $("#chat-panel");
    if (!panel) return;
    panel.hidden = !panel.hidden;
    $("#launcher")?.classList.toggle("chat-open", !panel.hidden);
    if (!panel.hidden) $("#chat-input")?.focus();
  });
  $("#contacts-toggle")?.addEventListener("click", () => {
    const panel = $("#contacts-panel");
    if (!panel) return;
    panel.hidden = !panel.hidden;
    if (!panel.hidden) renderContacts();
  });
  $("#voice-toggle")?.addEventListener("click", () => {
    const on = !table.voice().table;
    table.setTableVoice(on).catch((err) => {
      pushChat({ sys: true, text: voiceError(err) });
      paintVoice();
    });
  });
  $("#voice-accept")?.addEventListener("click", () => {
    table.acceptCall().catch((err) => {
      pushChat({ sys: true, text: voiceError(err) });
      paintVoice();
    });
  });
  $("#voice-decline")?.addEventListener("click", () => table.declineCall());
  $("#voice-hangup")?.addEventListener("click", () => table.hangup());
  const onContactClick = (e) => {
    const add = e.target.closest("[data-add-contact]");
    if (add) {
      e.preventDefault();
      const ok = addContact(add.dataset.addContact, add.dataset.addName);
      if (ok) pushChat({ sys: true, text: "Saved " + (add.dataset.addName || "them") + " to contacts." });
      renderPeers(table.state.peers);
      return;
    }
    const whisper = e.target.closest("[data-whisper]");
    if (whisper) {
      fillWhisper(whisper.dataset.whisper || "");
      return;
    }
    const del = e.target.closest("[data-remove-contact]");
    if (del) {
      removeContact(del.dataset.removeContact);
      return;
    }
    const hang = e.target.closest("[data-voice-hangup]");
    if (hang) {
      table.hangup();
      return;
    }
    const call = e.target.closest("[data-voice-call]");
    if (call) {
      table.startCall(call.dataset.voiceCall).catch((err) => {
        pushChat({ sys: true, text: voiceError(err) });
      });
      return;
    }
    const pic = e.target.closest("[data-media-to]");
    if (pic) {
      mediaSendTo = { id: pic.dataset.mediaTo, name: pic.dataset.mediaName };
      $("#chat-media-file")?.click();
    }
  };
  $("#contacts-panel")?.addEventListener("click", onContactClick);
  chars.bind();
  $("#live-radio-toggle")?.addEventListener("click", () => {
    if (worldOf() !== "blight") return;
    const on = !isLiveRadio();
    setLiveRadio(on);
    const mood = ui.playlist?.mood || (ui.musicMood && ui.musicMood !== "all" ? ui.musicMood : "brutal");
    if (on) {
      for (const layer of playingMusic()) mixer.stop(layer.id, 0.4);
      const st = defaultLiveForMood(mood);
      tuneToStation(st.id);
    } else {
      liveRadio?.stop();
      tuneToStation(mood);
    }
    paintRadioChip(mixer);
    paint();
  });
  $("#station-prev")?.addEventListener("click", () => stepStation(-1));
  $("#station-next")?.addEventListener("click", () => stepStation(1));
  $("#station-pick")?.addEventListener("change", (e) => {
    const id = e.target.value;
    if (id) tuneToStation(id);
  });
  $("#radio-chip")?.addEventListener("click", () => {
    if (worldOf() === "blight") stepStation(1);
  });
  $("#chars-toggle")?.addEventListener("click", () => {
    const panel = $("#chars-panel");
    if (!panel) return;
    panel.hidden = !panel.hidden;
    const open = !panel.hidden;
    document.querySelector(".app")?.classList.toggle("chars-open", open);
    $("#chars-toggle")?.classList.toggle("on", open);
    if (open) chars.render();
  });
  bestiary.bind();
  datashard.bind();
  kit.bind();
  vendors.bind();
  blackjack.bind();
  corps.bind();
  npcs.bind();
  srdNpcs.bind();
  gods.bind();
  lore.bind();
  gangs.bind();
  $("#bestiary-toggle")?.addEventListener("click", () => {
    closeCatalogs("bestiary");
    bestiary.toggle();
  });
  $("#datashard-toggle")?.addEventListener("click", () => {
    closeCatalogs("datashard");
    datashard.toggle();
  });
  $("#kit-toggle")?.addEventListener("click", () => {
    closeCatalogs("kit");
    kit.toggle();
  });
  $("#vendor-toggle")?.addEventListener("click", () => {
    closeCatalogs("vendors");
    vendors.toggle();
  });
  $("#corp-toggle")?.addEventListener("click", () => {
    closeCatalogs("corps");
    corps.toggle();
  });
  $("#npc-toggle")?.addEventListener("click", () => {
    closeCatalogs("npcs");
    npcs.toggle();
  });
  $("#srdnpc-toggle")?.addEventListener("click", () => {
    closeCatalogs("srdNpcs");
    srdNpcs.toggle();
  });
  $("#god-toggle")?.addEventListener("click", () => {
    closeCatalogs("gods");
    gods.toggle();
  });
  $("#gang-toggle")?.addEventListener("click", () => {
    closeCatalogs("gangs");
    gangs.toggle();
  });
  $("#lore-toggle")?.addEventListener("click", () => {
    closeCatalogs("lore");
    lore.toggle();
  });
  const kitOver = (e) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    e.currentTarget.classList.add("kit-drop");
  };
  const kitLeave = (e) => {
    if (e.currentTarget.contains(e.relatedTarget)) return;
    e.currentTarget.classList.remove("kit-drop");
  };
  const sheetDrop = async (e) => {
    e.preventDefault();
    e.currentTarget.classList.remove("kit-drop");
    const kitId = e.dataTransfer.getData("application/x-blight-kit");
    if (kitId) {
      try {
        await kit.ensure();
      } catch {
        return;
      }
      const item = kit.item(kitId);
      if (!item) return;
      const chip = e.target.closest("[data-open-char]");
      chars.receiveKit(item, chip?.dataset.openChar);
      return;
    }
    const godId = e.dataTransfer.getData("application/x-blight-god");
    if (godId) {
      try {
        await gods.ensure();
      } catch {
        return;
      }
      const g = gods.item(godId);
      if (g) chars.receiveDeity(g);
    }
  };
  $("#chars-panel")?.addEventListener("dragover", kitOver);
  $("#chars-panel")?.addEventListener("dragleave", kitLeave);
  $("#chars-panel")?.addEventListener("drop", sheetDrop);
  $("#chars-toggle")?.addEventListener("dragover", (e) => {
    e.preventDefault();
    const panel = $("#chars-panel");
    if (panel?.hidden) {
      panel.hidden = false;
      document.querySelector(".app")?.classList.add("chars-open");
      $("#chars-toggle")?.classList.add("on");
      chars.render();
    }
  });
  document.addEventListener("blight-god-add", async (e) => {
    const id = e.detail?.id;
    if (!id) return;
    await gods.ensure();
    const g = gods.item(id);
    if (g) chars.receiveDeity(g);
  });
  document.addEventListener("blight-kit-add", async (e) => {
    const id = e.detail?.id;
    if (!id) return;
    await kit.ensure();
    const item = kit.item(id);
    if (item) chars.receiveKit(item);
  });
  $("#chat-peers")?.addEventListener("click", onContactClick);
  $("#chat-form")?.addEventListener("submit", (e) => {
    e.preventDefault();
    const input = $("#chat-input");
    const text = input?.value;
    if (table.state.role === "idle") {
      pushChat({ sys: true, text: "Host or join a table to chat." });
      return;
    }
    table.chat(text);
    if (input) input.value = "";
  });
  $("#chat-media-btn")?.addEventListener("click", () => {
    if (table.state.role === "idle") {
      pushChat({ sys: true, text: "Host or join a table to send pictures." });
      return;
    }
    mediaSendTo = null;
    $("#chat-media-file")?.click();
  });
  $("#chat-media-file")?.addEventListener("change", (e) => {
    const file = e.target.files && e.target.files[0];
    e.target.value = "";
    if (file) sendPickedMedia(file);
  });
  maps.bind();
  maps.fillSelect();
  document.addEventListener("dragstart", (e) => {
    if (window._hearthPlacing) {
      e.preventDefault();
      return;
    }
    const kitEl = e.target.closest("[data-kit-id]");
    if (kitEl?.dataset.kitId) {
      e.dataTransfer.setData("application/x-blight-kit", kitEl.dataset.kitId);
      e.dataTransfer.setData("text/plain", kitEl.dataset.kitId);
      e.dataTransfer.effectAllowed = "copy";
      return;
    }
    const godEl = e.target.closest("[data-god-id]");
    if (godEl?.dataset.godId) {
      e.dataTransfer.setData("application/x-blight-god", godEl.dataset.godId);
      e.dataTransfer.effectAllowed = "copy";
    }
    const payload = tokenPayload(e.target);
    if (!payload) {
      if (godEl?.dataset.godId) e.dataTransfer.setData("text/plain", godEl.dataset.godId);
      return;
    }
    if (maps.state.activeId) maps.openBoard();
    if (!maps.state.activeId) {
      if (godEl?.dataset.godId) {
        e.dataTransfer.setData("text/plain", godEl.dataset.godId);
        return;
      }
      e.preventDefault();
      const count = $("#mix-count");
      if (count) {
        count.textContent = "Load a map first";
        window.setTimeout(() => {
          if ($("#mix-search")?.value) return;
          count.textContent = mixCountLabel();
        }, 1400);
      }
      return;
    }
    const json = JSON.stringify(payload);
    e.dataTransfer.setData("application/x-hearth-token", json);
    e.dataTransfer.setData("text/plain", json);
    e.dataTransfer.effectAllowed = "copy";
  });
  document.addEventListener("pointerdown", (e) => {
    if (e.button) return;
    if (e.target.closest(".map-token") || e.target.closest("#map-stage")) return;
    if (e.target.closest("[data-kit-id]")) return;
    const payload = tokenPayload(e.target);
    if (!payload) return;
    const start = { x: e.clientX, y: e.clientY, payload, moved: false };
    let ghost = null;
    const move = (ev) => {
      if (!start.moved && Math.hypot(ev.clientX - start.x, ev.clientY - start.y) < 8) return;
      start.moved = true;
      window._hearthPlacing = true;
      if (maps.state.activeId) maps.openBoard();
      if (!ghost) {
        ghost = document.createElement("img");
        ghost.className = "map-drag-ghost";
        ghost.alt = "";
        ghost.src = start.payload.src || "";
        document.body.appendChild(ghost);
      }
      ghost.style.left = ev.clientX - 28 + "px";
      ghost.style.top = ev.clientY - 28 + "px";
    };
    const up = (ev) => {
      document.removeEventListener("pointermove", move);
      document.removeEventListener("pointerup", up);
      window._hearthPlacing = false;
      if (ghost) ghost.remove();
      if (!start.moved) return;
      if (!maps.state.activeId) {
        const count = $("#mix-count");
        if (count) {
          count.textContent = "Load a map first";
          window.setTimeout(() => {
            if ($("#mix-search")?.value) return;
            count.textContent = mixCountLabel();
          }, 1400);
        }
        return;
      }
      const panel = document.getElementById("chars-panel");
      if (panel && !panel.hidden && start.payload.kind === "god" && start.payload.ref) {
        const pbox = panel.getBoundingClientRect();
        if (ev.clientX >= pbox.left && ev.clientX <= pbox.right && ev.clientY >= pbox.top && ev.clientY <= pbox.bottom) {
          gods.ensure().then(() => {
            const g = gods.item(start.payload.ref);
            if (g) chars.receiveDeity(g);
          });
          window._hearthSkipClick = true;
          return;
        }
      }
      const stage = document.getElementById("map-stage");
      if (!stage) return;
      const box = stage.getBoundingClientRect();
      if (ev.clientX < box.left || ev.clientX > box.right || ev.clientY < box.top || ev.clientY > box.bottom) return;
      maps.dropAt(start.payload, ev.clientX, ev.clientY);
      window._hearthSkipClick = true;
    };
    document.addEventListener("pointermove", move);
    document.addEventListener("pointerup", up);
  });
  document.addEventListener(
    "click",
    (e) => {
      if (!window._hearthSkipClick) return;
      window._hearthSkipClick = false;
      e.preventDefault();
      e.stopPropagation();
    },
    true
  );
  const toolIcons = { pan: "map", draw: "draw", circle: "circle", rect: "rect", erase: "erase" };
  for (const btn of document.querySelectorAll("[data-map-tool]")) {
    const name = toolIcons[btn.dataset.mapTool];
    if (name) btn.innerHTML = icon(name) + btn.textContent.trim();
  }
  $("#map-clear") && ($("#map-clear").innerHTML = icon("stop") + "Clear marks");
  $("#map-toggle")?.addEventListener("click", () => {
    const board = $("#map-board");
    if (!board) return;
    if (board.hidden) {
      maps.openBoard();
      if (maps.state.activeId) maps.show(maps.state.activeId, { keepView: true, silent: true });
    } else maps.closeBoard(table.state.role === "host");
  });
  $("#map-import")?.addEventListener("click", () => $("#map-file")?.click());
  $("#map-file")?.addEventListener("change", (e) => {
    const file = e.target.files && e.target.files[0];
    e.target.value = "";
    if (file) addMapFile(file);
  });
  $("#map-select")?.addEventListener("change", (e) => {
    if (e.target.value) maps.show(e.target.value);
  });
  $("#map-grid")?.addEventListener("click", () => maps.toggleGrid());
  $("#map-fit")?.addEventListener("click", () => {
    maps.fit();
    if (table.state.role === "host") table.sendMap(maps.snapshot());
  });
  $("#map-close")?.addEventListener("click", () => maps.closeBoard(table.state.role === "host"));
  $("#map-toggle") && ($("#map-toggle").innerHTML = icon("map") + "Maps");
  $("#upload-btn")?.addEventListener("click", () => $("#sound-file")?.click());
  $("#sound-file")?.addEventListener("change", (e) => {
    const file = e.target.files && e.target.files[0];
    e.target.value = "";
    if (file) addSoundFile(file);
  });
  $("#launch-update")?.addEventListener("click", () => {
    runAppUpdate();
  });
  $("#table-host").innerHTML = icon("table") + "Host";
  $("#table-join").innerHTML = icon("table") + "Join";
  $("#table-leave") && ($("#table-leave").innerHTML = icon("stop") + "Leave");
  if ($("#table-copy")) $("#table-copy").textContent = "Copy address";
  $("#chat-toggle").innerHTML = icon("chat") + "Chat";
  $("#contacts-toggle") && ($("#contacts-toggle").innerHTML = icon("contacts") + "Contacts");
  $("#voice-toggle") && ($("#voice-toggle").innerHTML = icon("mic") + "Voice");
  paintVoice();
  renderContacts();
  $("#chars-toggle") && ($("#chars-toggle").innerHTML = icon("hero") + "Characters");
  $("#bestiary-toggle") && ($("#bestiary-toggle").innerHTML = icon("bestiary") + "Bestiary");
  $("#datashard-toggle") && ($("#datashard-toggle").innerHTML = icon("shard") + "Datashard");
  $("#corp-toggle") && ($("#corp-toggle").innerHTML = icon("crown") + "Corps");
  $("#npc-toggle") && ($("#npc-toggle").innerHTML = icon("hero") + "Faces");
  $("#srdnpc-toggle") && ($("#srdnpc-toggle").innerHTML = icon("hero") + "NPCs");
  $("#god-toggle") && ($("#god-toggle").innerHTML = icon("bestiary") + "Gods");
  $("#lore-toggle") && ($("#lore-toggle").innerHTML = icon("book") + "Lore");
  $("#gang-toggle") && ($("#gang-toggle").innerHTML = icon("mask") + "Gangs");
  const gangIco = document.querySelector(".gang-search-ico");
  if (gangIco) gangIco.innerHTML = icon("search");
  const loreIco = document.querySelector(".lore-search-ico");
  if (loreIco) loreIco.innerHTML = icon("search");
  const godIco = document.querySelector(".god-search-ico");
  if (godIco) godIco.innerHTML = icon("search");
  const npcIco = document.querySelector(".npc-search-ico");
  if (npcIco) npcIco.innerHTML = icon("search");
  const srdNpcIco = document.querySelector(".srdnpc-search-ico");
  if (srdNpcIco) srdNpcIco.innerHTML = icon("search");
  const corpIco = document.querySelector(".corp-search-ico");
  if (corpIco) corpIco.innerHTML = icon("search");
  $("#kit-toggle") && ($("#kit-toggle").innerHTML = icon("swords") + (currentTheme() === "blight" ? "Night Market" : "Armory"));
  const kitIco = document.querySelector(".kit-search-ico");
  if (kitIco) kitIco.innerHTML = icon("search");
  const vendorIco = document.querySelector(".vendor-search-ico");
  if (vendorIco) vendorIco.innerHTML = icon("search");
  $("#vendor-toggle") && ($("#vendor-toggle").innerHTML = icon("market") + "Vendors");
  $("#bj-toggle") && ($("#bj-toggle").innerHTML = "21");
  const beastIco = document.querySelector("#bestiary-board .beast-search .search-ico");
  if (beastIco) beastIco.innerHTML = icon("search");
  const shardIco = document.querySelector(".shard-search-ico");
  if (shardIco) shardIco.innerHTML = icon("search");
  $("#upload-btn").innerHTML = icon("upload") + "Add sound";
  renderSettingMenu();
  syncPlace();
  syncTime();
  syncSetting();
  $("#setting-btn")?.addEventListener("click", (e) => {
    e.stopPropagation();
    toggleSettingMenu();
  });

  $("#mix-count")?.addEventListener("click", () => {
    ui.filter = "on";
    renderTabs();
    renderLayers();
  });
  $("#master").parentElement.style.setProperty("--p", $("#master").value + "%");
  initViz();

  document.addEventListener("visibilitychange", () => {
    if (!document.hidden) mixer.resume();
  });
  syncBnChrome();
  syncNetcast();
  paintRadioChip(mixer);
  if (!isLocalHostName(location.hostname) && table.state.role === "idle") {
    joinTable(location.origin);
  }
}

bind();
