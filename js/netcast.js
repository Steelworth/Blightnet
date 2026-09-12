const CORPS = [
  { id: "arasaka", ticker: "ARSK", name: "ARASAKA", base: 486 },
  { id: "militech", ticker: "MLTC", name: "MILITECH", base: 412 },
  { id: "kang-tao", ticker: "KNG", name: "KANG TAO", base: 198 },
  { id: "night-corp", ticker: "NCRP", name: "NIGHT CORP", base: 265 },
  { id: "petrochem", ticker: "PTRC", name: "PETROCHEM", base: 154 },
  { id: "biotechnica", ticker: "BIO", name: "BIOTECHNICA", base: 221 },
  { id: "trauma-team", ticker: "TT", name: "TRAUMA TEAM", base: 318 },
  { id: "netwatch", ticker: "NTW", name: "NETWATCH", base: 177 },
  { id: "zetatech", ticker: "ZTT", name: "ZETATECH", base: 92 },
  { id: "orbital-air", ticker: "ORB", name: "ORBITAL AIR", base: 241 },
  { id: "kiroshi", ticker: "KIR", name: "KIROSHI", base: 134 },
  { id: "sovoil", ticker: "SOV", name: "SOVOIL", base: 119 },
];

const HEADLINES = [
  "Arasaka Tower lights stay on through another Council session. No one asked why.",
  "Militech convoy rolls Santo Domingo. NUSA flags, local bullets.",
  "Kang Tao demo in Westbrook: the smartgun bowed first.",
  "Night Corp delays the maglev again. The rails are 'undergoing personality work.'",
  "Petrochem CHOOH2 shortage blamed on a Biotechnica vat that learned to drink.",
  "Biotechnica recall: do not eat the steak if it remembers you.",
  "Trauma Team gold card wait times down. Street wait times unchanged.",
  "NetWatch reminds runners the Blackwall is not a suggestion.",
  "Zetatech ships a deck that 'almost' doesn't cook the user.",
  "Orbital Air ticket lottery: one seat, forty thousand applicants, three funerals.",
  "Kiroshi clinic in Japantown booked through next year. Bring your old eyes as a deposit.",
  "SovOil tanker 'redefines' the Megadocks shoreline. Petrochem sends flowers.",
  "IEC brownout in Watson scheduled for 'grid meditation.'",
  "Tsunami Defense test-fires over Pacifica. Pacifica notices.",
  "Kendachi waiting list now includes a waiting list.",
  "Constitutional Arms warehouse in Arroyo 'walked off' overnight.",
  "Arasaka and Militech deny a Fourth Corporate War reunion tour.",
  "Night Corp board photographed once. The photo developed blank.",
  "Mayor's office thanks Night Corp for the lights. Night Corp thanks no one.",
  "Fixer market reports a surplus of very used Kiroshi optics.",
];

const state = {
  prices: CORPS.map((c) => ({ ...c, price: c.base, d: 0 })),
  news: 0,
  timer: 0,
};

function fmt(n) {
  return n.toFixed(2);
}

function tickStocks() {
  for (const row of state.prices) {
    const shock = (Math.random() - 0.48) * row.base * 0.012;
    row.d = shock;
    row.price = Math.max(1.1, row.price + shock);
  }
  const tape = document.getElementById("stock-tape");
  if (!tape) return;
  if (!tape.dataset.ready) {
    tape.innerHTML = state.prices
      .map((row) => `<button type="button" class="stock-chip" data-corp="${row.id}" title="${row.name}"></button>`)
      .join("");
    tape.dataset.ready = "1";
  }
  for (const row of state.prices) {
    const el = tape.querySelector(`[data-corp="${row.id}"]`);
    if (!el) continue;
    const up = row.d >= 0;
    el.className = "stock-chip " + (up ? "up" : "dn");
    el.innerHTML = `<b>${row.ticker}</b> ${fmt(row.price)} ${up ? "▲" : "▼"}`;
  }
}

function tickNews() {
  const el = document.getElementById("news-wire");
  if (!el) return;
  state.news = (state.news + 1) % HEADLINES.length;
  el.textContent = "WIRE · " + HEADLINES[state.news];
}

export function syncNetcast() {
  const hud = document.getElementById("net-hud");
  const blight = document.documentElement?.dataset?.theme === "blight";
  if (hud) hud.hidden = !blight;
  if (!blight) {
    if (state.timer) {
      window.clearInterval(state.timer);
      state.timer = 0;
    }
    const tape = document.getElementById("stock-tape");
    if (tape) delete tape.dataset.ready;
    return;
  }
  tickStocks();
  if (!state.timer) {
    tickNews();
    state.timer = window.setInterval(() => {
      tickStocks();
      if (Math.random() < 0.35) tickNews();
    }, 2200);
  }
}
