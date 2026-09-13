import { blightMoney, walletOf, canAfford, charge, credit, formatMoney, walletText } from "./money.js";

const SUITS = [
  { mark: "♠", red: false },
  { mark: "♥", red: true },
  { mark: "♦", red: true },
  { mark: "♣", red: false },
];
const RANKS = ["A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"];
const BANK_KEY = "hearthsong.bjBank";

function esc(s) {
  return String(s ?? "").replace(/[&<>"']/g, (ch) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[ch])
  );
}

function cardValue(rank) {
  if (rank === "A") return 11;
  if (rank === "10" || rank === "J" || rank === "Q" || rank === "K") return 10;
  return Number(rank) || 0;
}

function handTotal(cards) {
  let t = 0;
  let aces = 0;
  for (const c of cards) {
    if (c.hidden) continue;
    t += cardValue(c.rank);
    if (c.rank === "A") aces += 1;
  }
  while (t > 21 && aces > 0) {
    t -= 10;
    aces -= 1;
  }
  return t;
}

function isBlackjack(cards) {
  return cards.length === 2 && !cards.some((c) => c.hidden) && handTotal(cards) === 21;
}

function makeShoe() {
  const shoe = [];
  for (let d = 0; d < 6; d += 1) {
    for (const s of SUITS) {
      for (const rank of RANKS) shoe.push({ rank, suit: s.mark, red: s.red, hidden: false });
    }
  }
  const buf = new Uint32Array(shoe.length);
  crypto.getRandomValues(buf);
  for (let i = shoe.length - 1; i > 0; i -= 1) {
    const j = buf[i] % (i + 1);
    const tmp = shoe[i];
    shoe[i] = shoe[j];
    shoe[j] = tmp;
  }
  return shoe;
}

function defaultBet() {
  return blightMoney() ? 50 : 500;
}

function betSteps() {
  return blightMoney() ? [10, 50, 100, 500] : [100, 500, 1000, 2500];
}

export function createBlackjack(hooks) {
  const state = {
    shoe: [],
    player: [],
    dealer: [],
    bet: defaultBet(),
    ante: defaultBet(),
    phase: "idle",
    msg: "",
    last: "",
  };

  const $ = (sel) => document.querySelector(sel);

  function sheet() {
    const ch = hooks?.active?.();
    if (!ch || hooks?.isRemote?.(ch)) return null;
    return ch;
  }

  function houseKey() {
    return BANK_KEY + (blightMoney() ? ".eb" : ".gp");
  }

  function houseBank() {
    try {
      const n = Number(localStorage.getItem(houseKey()));
      if (Number.isFinite(n) && n >= 0) return n;
    } catch {
      /* ignore */
    }
    return blightMoney() ? 500 : 5000;
  }

  function setHouseBank(n) {
    try {
      localStorage.setItem(houseKey(), String(Math.max(0, Math.round(n))));
    } catch {
      /* ignore */
    }
  }

  function bank() {
    const ch = sheet();
    return ch ? walletOf(ch) : houseBank();
  }

  function take(n) {
    const ch = sheet();
    if (ch) {
      if (!canAfford(ch, n) || !charge(ch, n)) return false;
      hooks.persistMoney?.();
      return true;
    }
    if (houseBank() < n) return false;
    setHouseBank(houseBank() - n);
    return true;
  }

  function give(n) {
    const ch = sheet();
    if (ch) {
      credit(ch, n);
      hooks.persistMoney?.();
      return;
    }
    setHouseBank(houseBank() + n);
  }

  function draw() {
    if (state.shoe.length < 40) state.shoe = makeShoe();
    return state.shoe.pop();
  }

  function cardHtml(c, i) {
    if (c.hidden) {
      return `<div class="bj-card back" style="--i:${i}"><span></span></div>`;
    }
    return `<div class="bj-card${c.red ? " red" : ""}" style="--i:${i}"><b>${esc(c.rank)}</b><i>${esc(c.suit)}</i></div>`;
  }

  function paint() {
    const title = $("#bj-title");
    const kicker = $("#bj-kicker");
    const purse = $("#bj-purse");
    const msg = $("#bj-msg");
    const dealer = $("#bj-dealer-cards");
    const player = $("#bj-player-cards");
    const dt = $("#bj-dealer-total");
    const pt = $("#bj-player-total");
    const betL = $("#bj-bet-label");
    if (title) title.textContent = "21";
    if (kicker) kicker.textContent = "NETDIR://HOUSE · TWENTY-ONE";
    if (purse) {
      const ch = sheet();
      purse.textContent = ch ? (ch.name || "Sheet") + " · " + walletText(ch) : "HOUSE CHIPS · " + formatMoney(bank());
    }
    if (dealer) dealer.innerHTML = state.dealer.map(cardHtml).join("") || `<p class="bj-empty">HOUSE</p>`;
    if (player) player.innerHTML = state.player.map(cardHtml).join("") || `<p class="bj-empty">YOU</p>`;
    if (dt) dt.textContent = state.dealer.some((c) => c.hidden) ? "?" : state.dealer.length ? String(handTotal(state.dealer)) : "—";
    if (pt) pt.textContent = state.player.length ? String(handTotal(state.player)) : "—";
    if (msg) msg.textContent = state.msg || (state.phase === "idle" ? "Ante up. Beat the house to 21." : "");
    if (betL) betL.textContent = formatMoney(state.bet);
    const playing = state.phase === "play";
    const idle = state.phase === "idle" || state.phase === "done";
    $("#bj-deal") && ($("#bj-deal").hidden = !idle);
    $("#bj-hit") && ($("#bj-hit").hidden = !playing);
    $("#bj-stand") && ($("#bj-stand").hidden = !playing);
    $("#bj-double") && ($("#bj-double").hidden = !playing);
    $("#bj-double") && ($("#bj-double").disabled = !(playing && state.player.length === 2 && canBet(state.bet)));
    const stage = $("#bj-stage");
    stage?.classList.toggle("is-win", state.last === "win" || state.last === "bj");
    stage?.classList.toggle("is-lose", state.last === "lose" || state.last === "bust");
    const steps = $("#bj-steps");
    if (steps) {
      steps.innerHTML = betSteps()
        .map((n) => `<button type="button" class="bj-chip${n === state.bet ? " on" : ""}" data-bj-bet="${n}" ${idle ? "" : "disabled"}>${esc(formatMoney(n))}</button>`)
        .join("");
    }
  }

  function canBet(n) {
    return bank() >= n;
  }

  function revealDealer() {
    for (const c of state.dealer) c.hidden = false;
  }

  function finish(kind, msg, payout) {
    revealDealer();
    if (payout > 0) give(payout);
    state.phase = "done";
    state.last = kind;
    state.msg = msg;
    state.bet = state.ante || state.bet;
    paint();
  }

  function dealerPlay() {
    revealDealer();
    while (handTotal(state.dealer) < 17) state.dealer.push(draw());
    const p = handTotal(state.player);
    const d = handTotal(state.dealer);
    if (d > 21) finish("win", "Dealer busts. You take " + formatMoney(state.bet * 2) + ".", state.bet * 2);
    else if (p > d) finish("win", "You win " + formatMoney(state.bet * 2) + ".", state.bet * 2);
    else if (p < d) finish("lose", "House takes it.", 0);
    else finish("push", "Push. Bet returned.", state.bet);
  }

  function deal() {
    if (state.phase === "play") return;
    const bet = state.bet;
    if (!canBet(bet)) {
      state.msg = "Not enough in the purse.";
      paint();
      return;
    }
    if (!take(bet)) {
      state.msg = "Not enough in the purse.";
      paint();
      return;
    }
    state.ante = bet;
    if (state.shoe.length < 40) state.shoe = makeShoe();
    state.player = [draw(), draw()];
    state.dealer = [draw(), { ...draw(), hidden: true }];
    state.last = "";
    state.phase = "play";
    if (isBlackjack(state.player)) {
      const hole = { ...state.dealer[1], hidden: false };
      const dealerBj = isBlackjack([state.dealer[0], hole]);
      if (dealerBj) finish("push", "Both blackjack. Push.", bet);
      else {
        const win = bet + Math.floor((bet * 3) / 2);
        finish("bj", "Blackjack. " + formatMoney(win) + " back.", win);
      }
      return;
    }
    if (isBlackjack([state.dealer[0], { ...state.dealer[1], hidden: false }])) {
      finish("lose", "Dealer blackjack.", 0);
      return;
    }
    state.msg = "Hit, stand, or double.";
    paint();
  }

  function hit() {
    if (state.phase !== "play") return;
    state.player.push(draw());
    const t = handTotal(state.player);
    if (t > 21) finish("bust", "Bust.", 0);
    else {
      state.msg = t === 21 ? "Twenty-one. Stand or wait." : "Hit or stand.";
      paint();
    }
  }

  function stand() {
    if (state.phase !== "play") return;
    dealerPlay();
  }

  function doubleDown() {
    if (state.phase !== "play" || state.player.length !== 2) return;
    const extra = state.ante || state.bet;
    if (!canBet(extra) || !take(extra)) {
      state.msg = "Can't double. Not enough coin.";
      paint();
      return;
    }
    state.bet = extra * 2;
    state.player.push(draw());
    if (handTotal(state.player) > 21) finish("bust", "Bust on the double.", 0);
    else dealerPlay();
  }

  function open() {
    const el = $("#bj-stage");
    if (!el) return;
    el.hidden = false;
    if (state.phase === "idle" && !state.shoe.length) state.shoe = makeShoe();
    if (state.phase !== "play") {
      if (!betSteps().includes(state.bet)) state.bet = defaultBet();
      if (state.bet > bank() && bank() > 0) {
        const steps = betSteps().filter((n) => n <= bank());
        state.bet = steps[steps.length - 1] || defaultBet();
      }
      state.ante = state.bet;
      state.msg = "Ante up. Beat the house to 21.";
    }
    paint();
  }

  function close() {
    const el = $("#bj-stage");
    if (el) el.hidden = true;
  }

  function toggle() {
    const el = $("#bj-stage");
    if (!el || el.hidden) open();
    else close();
  }

  function bind() {
    $("#launch-bj")?.addEventListener("click", (e) => {
      e.stopPropagation();
      open();
    });
    $("#bj-toggle")?.addEventListener("click", () => toggle());
    $("#bj-close")?.addEventListener("click", close);
    $("#bj-deal")?.addEventListener("click", deal);
    $("#bj-hit")?.addEventListener("click", hit);
    $("#bj-stand")?.addEventListener("click", stand);
    $("#bj-double")?.addEventListener("click", doubleDown);
    $("#bj-steps")?.addEventListener("click", (e) => {
      const btn = e.target.closest("[data-bj-bet]");
      if (!btn || state.phase === "play") return;
      state.bet = Number(btn.dataset.bjBet) || defaultBet();
      paint();
    });
    document.addEventListener(
      "keydown",
      (e) => {
        if (e.key !== "Escape") return;
        const el = $("#bj-stage");
        if (!el || el.hidden) return;
        e.preventDefault();
        e.stopImmediatePropagation();
        if (state.phase === "play") return;
        close();
      },
      true
    );
  }

  return { open, close, toggle, bind, paint };
}
