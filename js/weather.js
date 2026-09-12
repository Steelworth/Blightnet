import { settingById, settingSrc } from "./places.js";

const KIND = {
  rain: "rain",
  downpour: "rain",
  drizzle: "rain",
  light_rain: "rain",
  forest_rain: "rain",
  night_rain: "rain",
  heavy_rain: "rain",
  jungle_rain: "rain",
  roof_rain: "rain",
  metal_roof: "rain",
  thunder_rain: "storm",
  thunder: "storm",
  distant_storm: "storm",
  rumble: "storm",
  evil_storm: "storm",
  forest_storm: "storm",
  storm_wind: "storm",
  gale: "storm",
  blizzard: "snow",
  hail: "snow",
  winter_wind: "snow",
  fog: "fog",
  wind: "wind",
  howling: "wind",
  breeze: "wind",
  canopy_wind: "wind",
  mountain_air: "wind",
  sea_wind: "wind",
};

const RANK = { storm: 5, rain: 4, snow: 3, fog: 2, wind: 1, clear: 0 };

const LABELS = {
  clear: "Clear skies",
  rain: "Rain",
  storm: "Storm",
  snow: "Snow",
  fog: "Fog",
  wind: "Wind",
};

export const HOURS = ["morning", "day", "evening", "night"];

export function hourOf(mixer) {
  const fromBody = document.body?.dataset?.time;
  const fromMixer = mixer?.time;
  if (HOURS.includes(fromBody)) return fromBody;
  if (HOURS.includes(fromMixer)) return fromMixer;
  return "day";
}

export function settingIdOf() {
  const fromBody = document.body?.dataset?.setting;
  return settingById(fromBody).id;
}

export function weatherOf(mixer) {
  const ids = typeof mixer.playingIds === "function" ? mixer.playingIds() : [];
  let kind = "clear";
  let score = -1;
  for (const id of ids) {
    const k = KIND[id];
    if (!k) continue;
    const vol = mixer.volumeOf?.(id) ?? 0.6;
    const s = (RANK[k] || 0) * 10 + vol;
    if (s > score) {
      score = s;
      kind = k;
    }
  }
  const time = hourOf(mixer);
  const setting = settingById(settingIdOf());
  const weatherLabel = LABELS[kind] || "Clear skies";
  return {
    kind,
    time,
    setting: setting.id,
    indoor: Boolean(setting.indoor),
    label: kind === "clear" ? setting.name : `${setting.name} · ${weatherLabel}`,
    src: settingSrc(setting.id, time),
  };
}
