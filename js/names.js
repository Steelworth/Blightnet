let lists = null;
let loading = null;

function pick(rows) {
  const n = rows?.length || 0;
  if (!n) return "";
  return rows[Math.floor(Math.random() * n)];
}

export async function loadNames() {
  if (lists) return lists;
  if (!loading) {
    loading = fetch("data/names.json", { cache: "force-cache" })
      .then((res) => {
        if (!res.ok) throw new Error("names");
        return res.json();
      })
      .then((data) => {
        lists = {
          male: Array.isArray(data.male) ? data.male : [],
          female: Array.isArray(data.female) ? data.female : [],
          last: Array.isArray(data.last) ? data.last : [],
        };
        return lists;
      })
      .catch(() => {
        loading = null;
        return { male: [], female: [], last: [] };
      });
  }
  return loading;
}

export async function randomPersonName(gender) {
  const data = await loadNames();
  const firsts = gender === "female" ? data.female : data.male;
  const first = pick(firsts);
  const last = pick(data.last);
  if (!first || !last) return "";
  return `${first} ${last}`;
}
