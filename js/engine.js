import { LAYERS, BLIGHT_LAYERS } from "./catalog.js";

const WHEN_DAY = new Set([
  "tavern_jig", "harvest_dance", "village_fair", "royal_court", "road_theme",
  "elven_glade", "sacred_hymn", "lord_land", "achaidh", "call_adventure",
  "pippin", "blue_feather", "heroic_age", "maestro",
  "drizzle", "light_rain", "breeze", "mountain_air", "canopy_wind",
  "songbirds", "morning_birds", "jungle_birds", "river_birds", "forest_birds",
  "rooster", "chickens", "bees", "farmyard", "cattle", "goats", "geese",
  "eagle", "gulls",
  "market", "town_square", "street", "fair", "church_bells", "garden", "barn",
  "river_dawn", "european_forest", "rainforest", "leaves", "waterfall_woods",
  "with_the_sea", "shores_avalon", "beach", "shore", "shore_birds", "sea_wind", "splash",
]);

const WHEN_NIGHT = new Set([
  "dark_rite", "oppressive_gloom", "hidden_past", "darkest_child", "ghost_story",
  "darkling", "ossuary", "interloper", "mystery", "chamber", "ritual_song",
  "the_descent", "wizards_tower", "lost_time", "morgana",
  "night_rain", "fog", "howling", "rumble", "distant_storm", "winter_wind",
  "owls", "owl_woods", "crickets", "cicadas", "bats", "wolves", "fox",
  "swamp_bugs", "purring",
  "night_woods", "night_forest", "scary_woods", "horror", "tomb", "graveyard",
  "torch", "desert_night", "campfire", "campfire_wind", "fireplace", "clock",
  "sewers", "drips", "twilight_jungle",
  "water_prelude", "floating_cities", "underwater", "diving", "deep_hum", "sinking", "bubbles",
]);

export class Mixer {
  constructor() {
    this.ctx = null;
    this.master = null;
    this.analyser = null;
    this.buffers = new Map();
    this.layers = new Map();
    this.playing = new Set();
    this.masterVolume = 0.85;
    this.place = "outside";
    this.time = "day";
    this.onChange = () => {};
    this._inflight = new Map();
    this._emitTimer = null;
    this._mediaNode = null;
    this.liveMood = null;
  }

  async init() {
    if (this.ctx) {
      if (this.ctx.state === "suspended") await this.ctx.resume();
      return;
    }
    const Ctx = window.AudioContext || window.webkitAudioContext;
    try {
      this.ctx = new Ctx({ latencyHint: "playback" });
    } catch {
      this.ctx = new Ctx();
    }
    const master = this.ctx.createGain();
    master.gain.value = this.masterVolume;
    const compressor = this.ctx.createDynamicsCompressor();
    compressor.threshold.value = -20;
    compressor.knee.value = 16;
    compressor.ratio.value = 1.7;
    compressor.attack.value = 0.01;
    compressor.release.value = 0.25;
    const analyser = this.ctx.createAnalyser();
    analyser.fftSize = 256;
    analyser.smoothingTimeConstant = 0.72;
    const bus = this.ctx.createGain();
    const air = this.ctx.createBiquadFilter();
    air.type = "lowpass";
    air.frequency.value = 18000;
    air.Q.value = 0.4;
    const tilt = this.ctx.createBiquadFilter();
    tilt.type = "highshelf";
    tilt.frequency.value = 2800;
    tilt.gain.value = 1.6;
    bus.connect(air);
    air.connect(tilt);
    tilt.connect(master);
    master.connect(compressor);
    compressor.connect(analyser);
    analyser.connect(this.ctx.destination);
    this.master = master;
    this.bus = bus;
    this.air = air;
    this.tilt = tilt;
    this.analyser = analyser;

    const room = this.ctx.createDelay(0.5);
    room.delayTime.value = 0.066;
    const fb = this.ctx.createGain();
    fb.gain.value = 0.18;
    room.connect(fb);
    fb.connect(room);
    room.connect(bus);
    this.room = room;
    this.roomFb = fb;

    for (const spec of LAYERS) this.addLayer(spec);
    for (const spec of BLIGHT_LAYERS) this.addLayer(spec);
    this._applySky(0.05);
    await this.resume();
  }

  async resume() {
    if (!this.ctx) return;
    if (this.ctx.state === "suspended") {
      try {
        await this.ctx.resume();
      } catch {
        /* browser may still block until a click */
      }
    }
  }

  attachMedia(el) {
    if (!this.ctx || !el) return;
    if (this._mediaNode) return;
    const src = this.ctx.createMediaElementSource(el);
    const gain = this.ctx.createGain();
    gain.gain.value = 0.7;
    src.connect(gain);
    gain.connect(this.bus || this.master);
    this._mediaNode = src;
  }

  urlsFor(layer) {
    if (layer.spec.files) return layer.spec.files;
    if (layer.spec.file) return [layer.spec.file];
    return [];
  }

  async load(urls, onProgress) {
    const total = urls.length;
    let done = 0;
    const queue = [...urls];
    const workers = Array.from({ length: Math.min(8, queue.length || 1) }, async () => {
      while (queue.length) {
        const url = queue.shift();
        await this._loadOne(url);
        done += 1;
        onProgress?.(done, total);
      }
    });
    await Promise.all(workers);
  }

  addLayer(spec) {
    if (!spec?.id) return null;
    const existing = this.layers.get(spec.id);
    if (existing) {
      existing.spec = { ...existing.spec, ...spec };
      return existing;
    }
    const layer = {
      spec,
      gain: null,
      filter: null,
      tone: null,
      presence: null,
      dry: null,
      wet: null,
      volume: 0.6,
      playing: false,
      sources: [],
      timer: null,
      gen: 0,
    };
    this.layers.set(spec.id, layer);
    return layer;
  }

  _wire(layer) {
    if (!layer || layer.gain || !this.ctx) return layer;
    const gain = this.ctx.createGain();
    gain.gain.value = 0;
    const filter = this.ctx.createBiquadFilter();
    filter.type = "lowpass";
    filter.frequency.value = 14000;
    filter.Q.value = 0.45;
    const tone = this.ctx.createBiquadFilter();
    tone.type = "highshelf";
    tone.frequency.value = 2600;
    tone.gain.value = 0;
    const presence = this.ctx.createGain();
    presence.gain.value = 1;
    const dry = this.ctx.createGain();
    dry.gain.value = 1;
    const wet = this.ctx.createGain();
    wet.gain.value = 0.04;
    gain.connect(filter);
    filter.connect(tone);
    tone.connect(presence);
    presence.connect(dry);
    dry.connect(this.bus);
    presence.connect(wet);
    wet.connect(this.room);
    layer.gain = gain;
    layer.filter = filter;
    layer.tone = tone;
    layer.presence = presence;
    layer.dry = dry;
    layer.wet = wet;
    this._applyWorld(layer, 0.05);
    return layer;
  }

  async ingestAudio(url, arrayBuffer) {
    if (!this.ctx || !url || !arrayBuffer) return false;
    if (this.buffers.has(url)) return true;
    try {
      const buf = await this.ctx.decodeAudioData(arrayBuffer.slice(0));
      this.buffers.set(url, buf);
      return true;
    } catch (err) {
      console.warn("ingest", url, err);
      return false;
    }
  }

  async _loadOne(url, tries = 3) {
    if (!url || this.buffers.has(url)) return true;
    if (url.startsWith("upload:")) return false;
    if (this._inflight.has(url)) return this._inflight.get(url);
    const job = (async () => {
      let lastErr;
      for (let i = 0; i < tries; i++) {
        try {
          const fetchUrl = url.includes("?") ? url : url + "?hs=2";
          const res = await fetch(fetchUrl);
          if (!res.ok) throw new Error("HTTP " + res.status);
          const raw = await res.arrayBuffer();
          const buf = await this.ctx.decodeAudioData(raw.slice(0));
          this.buffers.set(url, buf);
          return true;
        } catch (err) {
          lastErr = err;
          await new Promise((r) => setTimeout(r, 140 * (i + 1)));
        }
      }
      console.warn("Could not load", url, lastErr);
      return false;
    })();
    this._inflight.set(url, job);
    try {
      return await job;
    } finally {
      this._inflight.delete(url);
    }
  }

  async _ensure(layer) {
    const urls = this.urlsFor(layer);
    const results = await Promise.all(urls.map((u) => this._loadOne(u)));
    return results.some(Boolean);
  }

  _hasAudio(layer) {
    return this.urlsFor(layer).some((u) => this.buffers.has(u));
  }

  setMaster(value, fade = 0.05) {
    this.masterVolume = value;
    if (!this.master) return;
    this._ramp(this.master, value, fade);
  }

  spaceOf(spec) {
    if (spec.space) return spec.space;
    if (spec.category === "music") return "both";
    if (spec.category === "weather") return "out";
    if (spec.category === "animals") {
      if (["cats", "hounds", "cattle", "rooster", "farmyard", "purring", "donkeys", "chickens", "goats"].includes(spec.id)) {
        return "both";
      }
      return "out";
    }
    const indoor = new Set([
      "tavern_hall", "dungeon", "cavern", "temple", "forge", "torch", "magic",
      "market", "kitchen", "library", "sewers", "drips", "fireplace",
      "crowded_pub", "tomb", "clock", "horror", "big_fire",
    ]);
    if (spec.id === "church_bells") return "both";
    if (["underwater", "diving", "deep_hum", "sinking", "bubbles"].includes(spec.id)) return "both";
    return indoor.has(spec.id) ? "in" : "out";
  }

  whenOf(spec) {
    if (spec.when) return spec.when;
    if (WHEN_DAY.has(spec.id)) return "day";
    if (WHEN_NIGHT.has(spec.id)) return "night";
    return "both";
  }

  setPlace(place, fade = 0.5) {
    this.place = place === "inside" ? "inside" : "outside";
    if (!this.ctx) return;
    for (const layer of this.layers.values()) this._applyWorld(layer, fade);
  }

  setTime(time, fade = 0.7) {
    const allowed = ["morning", "day", "evening", "night"];
    this.time = allowed.includes(time) ? time : "day";
    if (!this.ctx) return;
    this._applySky(fade);
    for (const layer of this.layers.values()) this._applyWorld(layer, fade);
  }

  _isDark() {
    return this.time === "night" || this.time === "evening";
  }

  _applySky(fade) {
    if (!this.ctx || !this.air || !this.tilt || !this.room || !this.roomFb) return;
    const night = this._isDark();
    this._rampParam(this.air.frequency, night ? 6800 : 18000, fade);
    this._rampParam(this.tilt.gain, night ? -3.8 : 1.8, fade);
    this._rampParam(this.room.delayTime, night ? 0.096 : 0.062, fade);
    this._ramp(this.roomFb, night ? 0.3 : 0.16, fade);
  }

  _applyWorld(layer, fade) {
    if (!layer.filter || !this.ctx) return;
    const indoor = this.place === "inside";
    const night = this._isDark();
    const space = this.spaceOf(layer.spec);
    const when = this.whenOf(layer.spec);
    let freq = 14000;
    let dry = 1;
    let wet = 0.04;
    if (indoor) {
      if (space === "out") {
        freq = 1650;
        dry = 0.52;
        wet = 0.1;
      } else if (space === "in") {
        freq = 4200;
        dry = 1;
        wet = 0.28;
      } else {
        freq = 7200;
        dry = 1;
        wet = 0.16;
      }
    } else if (space === "out") {
      freq = 18000;
      dry = 1;
      wet = 0.03;
    } else if (space === "in") {
      freq = 1800;
      dry = 0.48;
      wet = 0.08;
    } else {
      freq = 16000;
      dry = 1;
      wet = 0.05;
    }
    if (night) {
      freq = Math.min(freq, freq * 0.72 + 900);
      wet += 0.06;
    }
    let presence = 1;
    let shelf = night ? -3.6 : 2.2;
    if (when === "day" && night) {
      presence = 0.36;
      shelf = -7.5;
    } else if (when === "night" && !night) {
      presence = 0.4;
      shelf = -1.2;
    } else if (when === "day" && !night) {
      presence = 1.08;
      shelf = 2.8;
    } else if (when === "night" && night) {
      presence = 1.12;
      shelf = -2.4;
    }
    this._rampParam(layer.filter.frequency, freq, fade);
    this._ramp(layer.dry, dry, fade);
    this._ramp(layer.wet, Math.min(0.45, wet), fade);
    if (layer.tone) this._rampParam(layer.tone.gain, shelf, fade);
    if (layer.presence) this._ramp(layer.presence, presence, fade);
  }

  _rampParam(param, value, fade) {
    if (!this.ctx || !param) return;
    const now = this.ctx.currentTime;
    const current = Number.isFinite(param.value) ? param.value : value;
    try {
      param.cancelScheduledValues(now);
      param.setValueAtTime(current, now);
      param.linearRampToValueAtTime(value, now + Math.max(0.08, fade));
    } catch {
      param.value = value;
    }
  }

  async start(id, volume, fade = 1.2) {
    const layer = this.layers.get(id);
    if (!layer || !this.ctx) return;
    this._wire(layer);
    await this.resume();
    layer.volume = volume;
    if (layer.playing) {
      if (this._hasAudio(layer)) this._ramp(layer.gain, layer.volume, fade);
      this._emit();
      return;
    }
    if (layer.timer) {
      clearTimeout(layer.timer);
      layer.timer = null;
    }
    const gen = ++layer.gen;
    layer.playing = true;
    this.playing.add(id);
    this._emit();
    const ok = await this._ensure(layer);
    if (layer.gen !== gen || !layer.playing) return;
    if (!ok || !this._hasAudio(layer)) {
      layer.playing = false;
      this.playing.delete(id);
      this._emit();
      return;
    }
    if (layer.sources.length) {
      this._ramp(layer.gain, layer.volume, fade);
      this._emit();
      return;
    }
    this._killSources(layer);
    this._ramp(layer.gain, 0, 0);
    this._ramp(layer.gain, layer.volume, fade);
    if (layer.spec.type === "scatter") {
      this._scatterOnce(layer);
      this._armScatter(layer, false);
    } else {
      this._startLoop(layer);
    }
    this._trimBuffers();
    this._emit();
  }

  stop(id, fade = 0.9) {
    const layer = this.layers.get(id);
    if (!layer || !layer.playing) return;
    layer.playing = false;
    this.playing.delete(id);
    layer.gen = (layer.gen || 0) + 1;
    if (layer.timer) {
      clearTimeout(layer.timer);
      layer.timer = null;
    }
    this._ramp(layer.gain, 0, fade);
    const gen = layer.gen;
    window.setTimeout(() => {
      if (layer.gen !== gen || layer.playing) return;
      this._killSources(layer);
      this._trimBuffers();
    }, fade * 1000 + 40);
    this._emit();
  }

  _killSources(layer) {
    for (const src of layer.sources) {
      try {
        src.stop();
      } catch {
        /* already stopped */
      }
    }
    layer.sources = [];
  }

  stopAll(fade = 0.7) {
    for (const [id, layer] of this.layers) {
      if (layer.playing) this.stop(id, fade);
    }
  }

  setVolume(id, volume, fade = 0.05) {
    const layer = this.layers.get(id);
    if (!layer) return;
    layer.volume = volume;
    if (layer.playing) this._ramp(layer.gain, volume, fade);
  }

  toggle(id) {
    const layer = this.layers.get(id);
    if (!layer || !this.ctx) return;
    if (layer.playing) this.stop(id, 0.5);
    else this.start(id, layer.volume || 0.6, 0.6);
  }

  applyScene(scene, fade = 1.85) {
    const wanted = scene.layers || {};
    for (const [id, layer] of this.layers) {
      if (!(id in wanted) && layer.playing) this.stop(id, fade);
    }
    for (const [id, vol] of Object.entries(wanted)) {
      if (!this.layers.has(id)) continue;
      const n = Number(vol);
      if (n > 0) this.start(id, Math.min(1, n), fade);
      else if (this.layers.get(id).playing) this.stop(id, fade);
    }
  }

  snapshot() {
    const layers = {};
    for (const [id, layer] of this.layers) {
      if (layer.playing) layers[id] = layer.volume;
    }
    return layers;
  }

  isPlaying(id) {
    return Boolean(this.layers.get(id)?.playing);
  }

  volumeOf(id) {
    return this.layers.get(id)?.volume ?? 0.6;
  }

  playingIds() {
    return [...this.playing];
  }

  click() {
    if (!this.ctx || !this.master) return;
    try {
      const osc = this.ctx.createOscillator();
      const gain = this.ctx.createGain();
      osc.type = "sine";
      osc.frequency.value = 784;
      gain.gain.setValueAtTime(0.03, this.ctx.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.0001, this.ctx.currentTime + 0.09);
      osc.connect(gain);
      gain.connect(this.master);
      osc.start();
      osc.stop(this.ctx.currentTime + 0.1);
    } catch {
      /* click is cosmetic */
    }
  }

  _emit() {
    if (this._emitTimer) return;
    this._emitTimer = window.setTimeout(() => {
      this._emitTimer = null;
      this.onChange();
    }, 20);
  }

  _playingUrls() {
    const urls = new Set();
    for (const layer of this.layers.values()) {
      if (layer.playing || layer.sources.length) this.urlsFor(layer).forEach((u) => urls.add(u));
    }
    return urls;
  }

  _trimBuffers() {
    const keep = this._playingUrls();
    const max = 18;
    if (this.buffers.size <= max) return;
    for (const url of [...this.buffers.keys()]) {
      if (this.buffers.size <= max) break;
      if (!keep.has(url)) this.buffers.delete(url);
    }
  }

  _ramp(gainNode, value, fade) {
    if (!this.ctx || !gainNode) return;
    const now = this.ctx.currentTime;
    const g = gainNode.gain;
    const target = Math.max(0, value);
    try {
      g.cancelScheduledValues(now);
      const current = Number.isFinite(g.value) ? g.value : target;
      g.setValueAtTime(current, now);
      const dt = Math.max(0.03, fade);
      if (Math.abs(current - target) < 0.0005) g.setValueAtTime(target, now);
      else g.linearRampToValueAtTime(target, now + dt);
    } catch {
      g.value = target;
    }
  }

  _startLoop(layer) {
    const url = this.urlsFor(layer).find((u) => this.buffers.has(u));
    const buf = url ? this.buffers.get(url) : null;
    if (!buf) return;
    try {
      const src = this.ctx.createBufferSource();
      src.buffer = buf;
      src.loop = true;
      src.connect(layer.gain);
      src.start();
      layer.sources.push(src);
      src.onended = () => {
        layer.sources = layer.sources.filter((s) => s !== src);
      };
    } catch (err) {
      console.warn("loop", layer.spec.id, err);
    }
  }

  _scatterOnce(layer) {
    const files = (layer.spec.files || []).filter((u) => this.buffers.has(u));
    if (!files.length) return;
    const url = files[Math.floor(Math.random() * files.length)];
    const buf = this.buffers.get(url);
    if (!buf) return;
    try {
      const src = this.ctx.createBufferSource();
      src.buffer = buf;
      src.connect(layer.gain);
      src.start();
      layer.sources.push(src);
      src.onended = () => {
        layer.sources = layer.sources.filter((s) => s !== src);
      };
    } catch (err) {
      console.warn("scatter", layer.spec.id, err);
    }
  }

  _armScatter(layer, immediate) {
    if (!layer.playing) return;
    const spec = layer.spec;
    const min = Number(spec.minGap);
    const max = Number(spec.maxGap);
    const lo = Number.isFinite(min) ? min : 4;
    const hi = Number.isFinite(max) && max > lo ? max : lo + 8;
    const wait = immediate ? 400 : (lo + Math.random() * (hi - lo)) * 1000;
    layer.timer = window.setTimeout(() => {
      if (!layer.playing) return;
      this._scatterOnce(layer);
      this._armScatter(layer, false);
    }, wait);
  }
}
