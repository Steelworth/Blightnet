const BOOT_FILES = {
  power: "audio/boot/power.ogg",
  hdd: "audio/boot/hdd.ogg",
  beep: "audio/boot/beep.ogg",
  modem: "audio/boot/modem.ogg",
  dive: "audio/boot/dive.ogg",
  ice: "audio/boot/ice.ogg",
  tick: "audio/boot/tick.ogg",
  hum: "audio/boot/hum.ogg",
  land: "audio/boot/land.ogg",
};

export function createBootSfx() {
  const live = [];
  const cache = new Map();

  function load(name) {
    let node = cache.get(name);
    if (!node) {
      node = new Audio(`${BOOT_FILES[name]}?hs=3`);
      node.preload = "auto";
      cache.set(name, node);
    }
    return node;
  }

  function play(name, { loop = false, volume = 0.65 } = {}) {
    const proto = load(name);
    const a = proto.cloneNode(true);
    a.loop = loop;
    a.volume = volume;
    live.push(a);
    const p = a.play();
    if (p && p.catch) p.catch(() => {});
    if (!loop) {
      a.addEventListener("ended", () => {
        const i = live.indexOf(a);
        if (i >= 0) live.splice(i, 1);
      });
    }
    return a;
  }

  function tick() {
    const a = play("tick", { volume: 0.28 });
    try {
      a.playbackRate = 0.88 + Math.random() * 0.28;
    } catch {
      /* ignore */
    }
  }

  function warmup() {
    Object.keys(BOOT_FILES).forEach(load);
  }

  function stop() {
    for (const a of live) {
      try {
        a.pause();
        a.currentTime = 0;
      } catch {
        /* ignore */
      }
    }
    live.length = 0;
  }

  return { play, tick, warmup, stop };
}

export function bootLines() {
  const who = (() => {
    try {
      return (localStorage.getItem("hearthsong.profileName") || "RUNNER").trim().toUpperCase() || "RUNNER";
    } catch {
      return "TRAVELLER";
    }
  })();
  return [
    "> POST / unauthorized bios ......... OK",
    "> disk0 spin ....................... OK",
    "> icebreaker.ghost ................. LOADED",
    "> spoof mac 00:DE:AD:C0:DE:00",
    "> jack /dev/neural0",
    "> handshake v.34 ................... OK",
    "> black ice ........................ BYPASSED",
    "> mount /table ..................... OK",
    `> runner id ........................ ${who.slice(0, 18)}`,
    "> shell ............................ ROOT",
  ];
}

export function fillBootTunnel(el) {
  if (!el || el.dataset.ready) return;
  el.dataset.ready = "1";
  let html = "";
  for (let i = 0; i < 18; i++) html += `<i style="--i:${i}"></i>`;
  el.innerHTML = html;
}

export function fillBootStream(el) {
  if (!el || el.dataset.ready) return;
  el.dataset.ready = "1";
  let html = "";
  for (let i = 0; i < 28; i++) {
    const ang = (i / 28) * Math.PI * 2 + (i % 3) * 0.21;
    const dist = 46 + (i % 7) * 7;
    const dx = `${(Math.cos(ang) * dist).toFixed(1)}vmin`;
    const dy = `${(Math.sin(ang) * dist).toFixed(1)}vmin`;
    const delay = (-i * 0.11).toFixed(2);
    const dur = (1.15 + (i % 5) * 0.16).toFixed(2);
    html += `<span style="--dx:${dx};--dy:${dy};animation-delay:${delay}s;animation-duration:${dur}s"></span>`;
  }
  el.innerHTML = html;
}

export function playBoot({ onDone } = {}) {
  const boot = document.getElementById("boot");
  const log = document.getElementById("boot-log");
  const launcher = document.getElementById("launcher");
  const iceCore = document.getElementById("boot-ice-core");
  const iceTrace = document.getElementById("boot-ice-trace");
  const hudDeck = document.getElementById("boot-hud-deck");
  const hudTrace = document.getElementById("boot-hud-trace");
  const hudLink = document.getElementById("boot-hud-link");
  const hudIce = document.getElementById("boot-hud-ice");
  const reduce = window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches;
  const sfx = createBootSfx();
  const timers = [];
  const later = (fn, ms) => {
    const id = window.setTimeout(fn, ms);
    timers.push(id);
    return id;
  };
  let skip = () => {};
  const finish = () => {
    timers.forEach((id) => {
      window.clearTimeout(id);
      window.clearInterval(id);
    });
    document.removeEventListener("click", skip);
    document.removeEventListener("keydown", skip);
    sfx.stop();
    boot?.classList.add("hidden");
    boot?.setAttribute("aria-hidden", "true");
    launcher?.classList.remove("hidden");
    launcher?.setAttribute("aria-hidden", "false");
    onDone?.();
  };
  skip = finish;
  if (!boot || reduce) {
    finish();
    return;
  }

  fillBootTunnel(document.getElementById("boot-tunnel"));
  fillBootStream(document.getElementById("boot-stream"));
  sfx.warmup();

  const setHud = (deck, link, ice) => {
    if (hudDeck) hudDeck.textContent = deck;
    if (hudLink) hudLink.textContent = link;
    if (hudIce) hudIce.textContent = ice;
  };
  const setCore = (title, sub) => {
    if (iceCore) iceCore.textContent = title;
    if (iceTrace) iceTrace.textContent = sub;
  };

  let trace = 0;
  const traceId = window.setInterval(() => {
    if (!boot || boot.classList.contains("hidden")) return;
    trace += 0.31 + Math.random() * 0.72;
    if (hudTrace) hudTrace.textContent = trace.toFixed(2);
  }, 160);
  timers.push(traceId);

  boot.dataset.phase = "dark";
  sfx.play("hum", { loop: true, volume: 0.2 });
  sfx.play("power", { volume: 0.7 });
  later(() => sfx.play("hdd", { volume: 0.46 }), 180);
  later(() => sfx.play("beep", { volume: 0.38 }), 920);

  later(() => {
    if (boot.classList.contains("hidden")) return;
    boot.dataset.phase = "jack";
    setCore("JACK IN", "NEURAL UPLINK · SPOOFING TRACE");
    setHud("OFF-GRID", "JACKING", "SCAN");
    sfx.play("dive", { volume: 0.55 });
    sfx.play("modem", { volume: 0.4 });
  }, 1600);

  later(() => {
    if (boot.classList.contains("hidden")) return;
    boot.dataset.phase = "dive";
    setCore("DIVING", "NETDIR://LOCAL · PACKET STREAM");
    setHud("GHOST", "LIVE", "AHEAD");
  }, 4000);

  later(() => {
    if (boot.classList.contains("hidden")) return;
    boot.dataset.phase = "ice";
    setCore("BLACK ICE", "WALL AHEAD · BREAKER ARMED");
    setHud("GHOST", "LIVE", "HIT");
    sfx.play("ice", { volume: 0.5 });
  }, 7200);

  later(() => {
    if (boot.classList.contains("hidden")) return;
    boot.dataset.phase = "breach";
    setCore("BREACH", "GATE OPEN · RIDING THE PIPE");
    setHud("ROOT", "TUNNEL", "BYPASSED");
    sfx.play("dive", { volume: 0.42 });
  }, 9400);

  later(() => {
    if (boot.classList.contains("hidden")) return;
    boot.dataset.phase = "mark";
    setHud("ROOT", "SHELL", "CLEAR");
    sfx.play("land", { volume: 0.58 });
  }, 11200);

  later(() => {
    if (boot.classList.contains("hidden")) return;
    boot.dataset.phase = "shell";
    const lines = bootLines();
    let i = 0;
    const type = () => {
      if (!boot || boot.classList.contains("hidden")) return;
      if (i >= lines.length) {
        later(finish, 700);
        return;
      }
      if (log) log.textContent += (log.textContent ? "\n" : "") + lines[i];
      sfx.tick();
      i += 1;
      later(type, 150 + Math.random() * 80);
    };
    type();
  }, 12600);

  document.addEventListener("click", skip);
  document.addEventListener("keydown", skip);
}
