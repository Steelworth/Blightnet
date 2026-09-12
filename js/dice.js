function clamp(n, a, b) {
  return Math.max(a, Math.min(b, n));
}

function esc(value) {
  return String(value ?? "").replace(/[&<>"']/g, (ch) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[ch])
  );
}

export function parseDice(str) {
  const s = String(str || "");
  const m = s.match(/(\d+)\s*[dD]\s*(\d+)/);
  const bonusMatch = s.match(/([+-]\s*\d+)(?!d)/i);
  const bonus = bonusMatch ? Number(bonusMatch[1].replace(/\s/g, "")) : 0;
  if (!m) {
    const n = parseInt(s, 10);
    return { n: 0, sides: 0, bonus: Number.isFinite(n) ? n : 0, raw: s };
  }
  return { n: Number(m[1]) || 0, sides: Number(m[2]) || 0, bonus, raw: s };
}

export function rollDice(spec) {
  const { n, sides, bonus } = spec;
  let total = bonus;
  for (let i = 0; i < n; i++) total += 1 + Math.floor(Math.random() * Math.max(1, sides));
  return total;
}

export function maxDice(spec) {
  return spec.n * spec.sides + spec.bonus;
}

export function rollPercent() {
  return Math.floor(Math.random() * 101);
}

function clampPct(n) {
  const v = Number(n);
  if (!Number.isFinite(v)) return 0;
  return Math.max(0, Math.min(100, Math.round(v)));
}

export function resolveLuck(mode, extra = {}) {
  const kind = mode === "adv" || mode === "dis" ? mode : "";
  const givenA = extra.pctA != null ? clampPct(extra.pctA) : extra.pct != null ? clampPct(extra.pct) : null;
  const givenB = extra.pctB != null ? clampPct(extra.pctB) : null;
  const a = givenA != null ? givenA : rollPercent();
  if (!kind) return { pct: a, pctA: a, pctB: null, adv: "" };
  const b = givenB != null ? givenB : rollPercent();
  return {
    pct: kind === "adv" ? Math.max(a, b) : Math.min(a, b),
    pctA: a,
    pctB: b,
    adv: kind,
  };
}

export function gradePercent(pct) {
  if (pct <= 0) return "critical failure";
  if (pct >= 100) return "critical success";
  if (pct >= 70) return "success";
  if (pct >= 40) return "mixed";
  return "failure";
}

export function rollFormula(formula) {
  const spec = parseDice(formula);
  if (!spec.n) return Math.max(0, spec.bonus);
  return Math.max(0, rollDice(spec));
}

export function scaleDamage(formula, pct) {
  const rolled = rollFormula(formula);
  if (pct <= 0) return Math.max(1, rolled);
  if (pct >= 100) return Math.max(1, rolled * 2);
  const scaled = Math.round(rolled * (pct / 100));
  if (pct >= 40) return Math.max(1, scaled);
  return scaled;
}

export function outcomeDamage(formula, pct) {
  const grade = gradePercent(pct);
  if (!formula) return { damage: null, taken: false, grade };
  if (grade === "critical success") return { damage: Math.max(1, rollFormula(formula) * 2), taken: false, grade };
  if (grade === "success") {
    const rolled = rollFormula(formula);
    return { damage: Math.max(1, Math.round(rolled * (pct / 100))), taken: false, grade };
  }
  if (grade === "critical failure") return { damage: Math.max(1, rollFormula(formula)), taken: true, grade };
  return { damage: null, taken: false, grade };
}

export function createDice() {
  const lines = [];
  let timer = 0;
  let flick = 0;
  let hideTimer = 0;
  let hackTimer = 0;
  let onShare = null;
  let onRecord = null;
  let onAim = null;
  let luck = "";

  function logEl() {
    return document.getElementById("char-log");
  }

  function whoLabel(row) {
    const who = row.who || "Character";
    const player = row.player || "";
    if (player && player !== who) return `${player} · ${who}`;
    return who;
  }

  function paint() {
    const el = logEl();
    if (!el) return;
    if (!lines.length) {
      el.innerHTML = `<p>Click a name to show it to the table. Click <b>Roll</b> for a 0–100 check. <b>Adv</b> / <b>Dis</b> above roll twice (higher / lower). Shift+Roll is advantage, Alt+Roll is disadvantage. Attacks and heals ask for a token on the map. Click <b>?</b> if you want the dice explained.</p>`;
      return;
    }
    el.innerHTML = lines
      .slice(-40)
      .map((row) => {
        if (row.info) {
          const body = row.text ? `<span class="log-desc">${esc(row.text)}</span>` : "";
          return `<p class="log-note"><b>${esc(whoLabel(row))}</b> shows <b>${esc(row.action)}</b>${body}</p>`;
        }
        const cls = row.grade === "critical success" ? "crit-ok" : row.grade === "critical failure" ? "crit-fail" : "";
        const dmg = Number.isFinite(row.damage)
          ? row.taken
            ? ` <span class="dmg taken">${esc(row.damage)} dmg taken</span>`
            : row.heal
              ? ` <span class="dmg heal">${esc(row.damage)} healed${row.targetName ? " → " + esc(row.targetName) : ""}</span>`
              : ` <span class="dmg">${esc(row.damage)} dmg${row.targetName ? " → " + esc(row.targetName) : ""}</span>`
          : row.targetName
            ? ` <span class="log-desc">vs ${esc(row.targetName)}</span>`
            : "";
        const hack = row.hack ? ` <span class="hack-tag">hack</span>` : "";
        const death = row.deathSave ? ` <span class="hack-tag">death save</span>` : "";
        const luckTag =
          row.adv === "adv" || row.adv === "dis"
            ? ` <span class="luck-tag ${esc(row.adv)}">${row.adv === "adv" ? "adv" : "dis"} ${esc(row.pctA ?? row.pct)}${row.pctB != null ? " / " + esc(row.pctB) : ""}</span>`
            : "";
        return `<p class="${cls}"><b>${esc(whoLabel(row))}</b> · ${esc(row.action)} · ${esc(row.pct)}% ${esc(row.grade)}${luckTag}${hack}${death}${dmg}</p>`;
      })
      .join("");
    el.scrollTop = el.scrollHeight;
  }

  function animate(pct, extra = {}) {
    const stage = document.getElementById("dice-stage");
    const pip = document.getElementById("dice-pip");
    if (!stage || !pip) return;
    window.clearInterval(flick);
    window.clearTimeout(hideTimer);
    stage.hidden = false;
    stage.classList.toggle("adv", extra.adv === "adv");
    stage.classList.toggle("dis", extra.adv === "dis");
    pip.textContent = "—";
    let n = 0;
    flick = window.setInterval(() => {
      if (extra.adv === "adv" || extra.adv === "dis") {
        pip.innerHTML = `<b>${Math.floor(Math.random() * 101)}</b><small>${Math.floor(Math.random() * 101)}</small>`;
      } else {
        pip.textContent = String(Math.floor(Math.random() * 101));
      }
      n += 1;
      if (n > 8) {
        window.clearInterval(flick);
        flick = 0;
        if ((extra.adv === "adv" || extra.adv === "dis") && extra.pctB != null) {
          pip.innerHTML = `<b>${esc(pct)}</b><small>${esc(extra.pctA)} / ${esc(extra.pctB)}</small>`;
        } else {
          pip.textContent = String(pct);
        }
        hideTimer = window.setTimeout(() => {
          stage.hidden = true;
          stage.classList.remove("adv", "dis");
        }, extra.adv ? 1100 : 700);
      }
    }, 55);
  }

  function paintLuck() {
    const root = document.getElementById("chars-panel") || document;
    for (const btn of root.querySelectorAll("[data-luck]")) {
      btn.classList.toggle("on", parseLuck(btn.dataset.luck) === luck);
    }
    const wrap = document.getElementById("char-log-wrap");
    wrap?.classList.toggle("luck-adv", luck === "adv");
    wrap?.classList.toggle("luck-dis", luck === "dis");
  }

  function setLuck(mode) {
    luck = mode === "adv" || mode === "dis" ? mode : "";
    paintLuck();
  }

  function keyLuck(e) {
    const shift = Boolean(e?.shiftKey);
    const alt = Boolean(e?.altKey);
    if (shift && alt) return "";
    if (shift) return "adv";
    if (alt) return "dis";
    return null;
  }

  function parseLuck(v) {
    return v === "adv" || v === "dis" ? v : "";
  }

  function playHack(name, extra = {}) {
    const stage = document.getElementById("breach-stage") || document.getElementById("hack-stage");
    if (!stage) return;
    const label = stage.querySelector(".hack-name") || document.getElementById("hack-name");
    const kicker = stage.querySelector(".hack-kicker");
    const ok = stage.querySelector(".hack-ok");
    if (label) label.textContent = name || "Quickhack";
    if (kicker) kicker.textContent = extra.victim ? "YOU ARE BEING HACKED" : "NET ATTACK";
    if (ok) ok.textContent = extra.victim ? "ICE IN" : "BREACH";
    stage.classList.toggle("victim", Boolean(extra.victim));
    stage.hidden = false;
    stage.classList.remove("on");
    void stage.offsetWidth;
    stage.classList.add("on");
    window.clearTimeout(hackTimer);
    hackTimer = window.setTimeout(() => {
      stage.hidden = true;
      stage.classList.remove("on", "victim");
    }, 1900);
  }

  function pushLine(row, { animateRoll = false, share = false, hackFx = false, silent = false } = {}) {
    lines.push(row);
    if (lines.length > 80) lines.splice(0, lines.length - 80);
    if (animateRoll) animate(row.pct, row);
    if (hackFx) playHack(row.action);
    if (!silent) {
      window.clearTimeout(timer);
      timer = window.setTimeout(paint, 80);
      paint();
    }
    if (share) onShare?.(row);
  }

  function note(who, title, text, extra = {}) {
    const row = {
      who: who || "Character",
      action: String(title || "look").slice(0, 80),
      text: String(text || "").slice(0, 800),
      info: true,
      player: extra.player || "",
      at: Date.now(),
    };
    pushLine(row, { share: true });
    return row;
  }

  function record(who, action, extra = {}) {
    const pair = resolveLuck(extra.adv, extra);
    const pct = pair.pct;
    const grade = extra.grade || gradePercent(pct);
    const hit = grade === "success" || grade === "critical success";
    let damage = extra.damage;
    let taken = Boolean(extra.taken);
    if (extra.formula && damage == null) {
      const out = outcomeDamage(extra.formula, pct);
      damage = out.damage;
      taken = out.taken;
    }
    if (grade === "mixed" || grade === "failure") {
      damage = null;
      taken = false;
    }
    const heal = Boolean(extra.heal);
    const deathSave = Boolean(extra.deathSave);
    if (heal) taken = false;
    if (heal && !hit) damage = null;
    if (deathSave) {
      damage = null;
      taken = false;
    }
    const row = {
      who,
      action,
      pct,
      grade,
      damage: damage != null ? damage : null,
      taken,
      heal,
      hack: Boolean(extra.hack),
      deathSave,
      adv: pair.adv,
      pctA: pair.pctA,
      pctB: pair.pctB,
      targetId: extra.targetId || "",
      targetName: extra.targetName || "",
      targetOwner: extra.targetOwner || "",
      tokenId: extra.tokenId || "",
      sheetId: extra.sheetId || "",
      sheetOwner: extra.sheetOwner || "",
      player: extra.player || "",
      at: Date.now(),
    };
    const hackOk = Boolean(extra.hack) && hit;
    pushLine(row, { animateRoll: !hackOk, share: true, hackFx: hackOk && !extra.hackSilent });
    onRecord?.(row);
    return row;
  }

  function ingest(row, silent = false) {
    if (!row) return;
    if (row.info || row.type === "log") {
      pushLine(
        {
          who: row.who || "Character",
          action: row.action || "look",
          text: String(row.text || "").slice(0, 800),
          info: true,
          player: row.player || "",
          from: row.from || "",
          at: row.at || row.ts || Date.now(),
        },
        { animateRoll: false, share: false, hackFx: false, silent }
      );
      return;
    }
    if (row.pct == null || row.pct === "") return;
    const pct = clampPct(row.pct);
    const dmg = row.damage == null || row.damage === "" ? null : Number(row.damage);
    pushLine(
      {
        who: row.who || "Character",
        action: row.action || "roll",
        pct,
        grade: row.grade || gradePercent(pct),
        damage: Number.isFinite(dmg) ? dmg : null,
        taken: Boolean(row.taken),
        heal: Boolean(row.heal),
        hack: Boolean(row.hack),
        deathSave: Boolean(row.deathSave),
        adv: row.adv === "adv" || row.adv === "dis" ? row.adv : "",
        pctA: row.pctA == null || row.pctA === "" ? null : clampPct(row.pctA),
        pctB: row.pctB == null || row.pctB === "" ? null : clampPct(row.pctB),
        targetId: row.targetId || "",
        targetName: row.targetName || "",
        targetOwner: row.targetOwner || "",
        tokenId: row.tokenId || "",
        sheetId: row.sheetId || "",
        sheetOwner: row.sheetOwner || "",
        player: row.player || "",
        from: row.from || "",
        at: row.at || row.ts || Date.now(),
      },
      { animateRoll: false, share: false, hackFx: false, silent }
    );
  }

  function ingestMany(rows) {
    if (!Array.isArray(rows) || !rows.length) return;
    for (const row of rows) ingest(row, true);
    paint();
  }

  function clear() {
    lines.length = 0;
    paint();
  }

  function setShare(fn) {
    onShare = typeof fn === "function" ? fn : null;
  }

  function setOnRecord(fn) {
    onRecord = typeof fn === "function" ? fn : null;
  }

  function setOnAim(fn) {
    onAim = typeof fn === "function" ? fn : null;
  }

  function bind(root) {
    root?.addEventListener("click", (e) => {
      if (e.target.closest("#char-log-clear")) {
        clear();
        return;
      }
      const luckBtn = e.target.closest("[data-luck]");
      if (luckBtn) {
        e.preventDefault();
        e.stopPropagation();
        const next = parseLuck(luckBtn.dataset.luck);
        setLuck(luck === next ? "" : next);
        return;
      }
      const show = e.target.closest("[data-show]");
      if (show && !e.target.closest("[data-roll]")) {
        e.preventDefault();
        const who = show.dataset.who || "Character";
        let title = show.dataset.showTitle || show.textContent || "look";
        let text = show.dataset.showText || "";
        const attack = show.closest(".attack-row");
        if (attack) {
          const bits = [...attack.querySelectorAll("input")].map((n) => n.value.trim());
          title = bits[0] || title;
          text = [bits[0] && `Name ${bits[0]}`, bits[1] && `bonus ${bits[1]}`, bits[2] && `damage ${bits[2]}`]
            .filter(Boolean)
            .join(". ");
        }
        note(who, title.trim(), text.trim());
        return;
      }
      const btn = e.target.closest("[data-roll]");
      if (!btn) return;
      e.preventDefault();
      const spec = btn.dataset.roll || "";
      const who = btn.dataset.who || "Character";
      let formula = btn.dataset.dmg || "";
      const label = btn.dataset.label || spec;
      const heal = Boolean(btn.dataset.heal);
      const hack = Boolean(btn.dataset.hack);
      const deathSave = Boolean(btn.dataset.deathSave);
      if (heal && !formula) formula = "1d8";
      if (deathSave) formula = "";
      const extra = { formula, hack, heal, deathSave };
      extra.sheetId = btn.dataset.sheet || extra.targetId || "";
      extra.sheetOwner = btn.dataset.owner || extra.targetOwner || "";
      extra._keyLuck = keyLuck(e);
      if (deathSave) {
        extra.targetId = btn.dataset.sheet || "";
        extra.targetOwner = btn.dataset.owner || "";
        extra.targetName = who;
      }
      const go = (aim) => {
        if (aim === false) return;
        if (aim && typeof aim === "object") Object.assign(extra, aim);
        extra.adv = extra._keyLuck != null ? extra._keyLuck : luck;
        record(who, label, extra);
      };
      if (onAim && !deathSave && (formula || hack || heal)) {
        Promise.resolve(onAim(btn, extra)).then(go).catch(() => go(null));
        return;
      }
      go(null);
    });
    paintLuck();
  }

  return { record, note, ingest, ingestMany, clear, paint, bind, setShare, setOnRecord, setOnAim, playHack, setLuck, paintLuck, lines };
}
