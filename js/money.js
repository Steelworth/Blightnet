import { worldOf } from "./catalog.js";

export function blightMoney() {
  return worldOf() === "blight";
}

export function parsePrice(raw) {
  const s = String(raw || "").toLowerCase().replace(/,/g, "");
  const m = s.match(/([\d]*\.?[\d]+)/);
  const n = m ? Number(m[1]) : 0;
  if (!Number.isFinite(n) || n < 0) return 0;
  if (/eb/.test(s)) return Math.round(n);
  if (/pp/.test(s)) return Math.round(n * 1000);
  if (/gp/.test(s)) return Math.round(n * 100);
  if (/ep/.test(s)) return Math.round(n * 50);
  if (/sp/.test(s)) return Math.round(n * 10);
  if (/cp/.test(s)) return Math.round(n);
  return blightMoney() ? Math.round(n) : Math.round(n * 100);
}

export function copperOf(ch) {
  return Math.max(
    0,
    Math.round(
      (Number(ch?.pp) || 0) * 1000 +
        (Number(ch?.gp) || 0) * 100 +
        (Number(ch?.ep) || 0) * 50 +
        (Number(ch?.sp) || 0) * 10 +
        (Number(ch?.cp) || 0)
    )
  );
}

export function setCopper(ch, value) {
  let n = Math.max(0, Math.round(Number(value) || 0));
  ch.pp = Math.floor(n / 1000);
  n %= 1000;
  ch.gp = Math.floor(n / 100);
  n %= 100;
  ch.ep = 0;
  ch.sp = Math.floor(n / 10);
  ch.cp = n % 10;
}

export function ebOf(ch) {
  return Math.max(0, Math.round(Number(ch?.eb) || 0));
}

export function setEb(ch, value) {
  ch.eb = Math.max(0, Math.round(Number(value) || 0));
}

export function walletOf(ch) {
  return blightMoney() ? ebOf(ch) : copperOf(ch);
}

export function formatMoney(amount) {
  const n = Math.max(0, Math.round(Number(amount) || 0));
  if (blightMoney()) return n.toLocaleString("en-US") + " eb";
  if (n >= 100) {
    const gp = n / 100;
    return (Number.isInteger(gp) ? String(gp) : gp.toFixed(2).replace(/0+$/, "").replace(/\.$/, "")) + " gp";
  }
  if (n >= 10) return Math.floor(n / 10) + " sp";
  return n + " cp";
}

export function walletText(ch) {
  return formatMoney(walletOf(ch));
}

export function canAfford(ch, amount) {
  return walletOf(ch) >= Math.max(0, Math.round(Number(amount) || 0));
}

export function charge(ch, amount) {
  const n = Math.max(0, Math.round(Number(amount) || 0));
  if (walletOf(ch) < n) return false;
  if (blightMoney()) setEb(ch, ebOf(ch) - n);
  else setCopper(ch, copperOf(ch) - n);
  return true;
}

export function credit(ch, amount) {
  const n = Math.max(0, Math.round(Number(amount) || 0));
  if (blightMoney()) setEb(ch, ebOf(ch) + n);
  else setCopper(ch, copperOf(ch) + n);
}

export function sellPrice(costRaw) {
  const full = parsePrice(costRaw);
  if (full <= 0) return 0;
  return Math.max(1, Math.floor(full / 2));
}
