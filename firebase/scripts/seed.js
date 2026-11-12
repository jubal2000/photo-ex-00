// Seeds the local Firestore emulator with demo catalogue data and two demo users
// who have tradable cards. Usage (emulators running):  node firebase/scripts/seed.js
const PROJECT = "demo-photo-exchanger";
const BASE = `http://127.0.0.1:8080/v1/projects/${PROJECT}/databases/(default)/documents`;
const H = { "Content-Type": "application/json", Authorization: "Bearer owner" }; // emulator admin bypass

const enc = (v) =>
  typeof v === "string" ? { stringValue: v }
  : typeof v === "boolean" ? { booleanValue: v }
  : Number.isInteger(v) ? { integerValue: String(v) }
  : Array.isArray(v) ? { arrayValue: { values: v.map(enc) } }
  : { doubleValue: v };
const fields = (o) => Object.fromEntries(Object.entries(o).map(([k, v]) => [k, enc(v)]));

async function put(path, data) {
  const r = await fetch(`${BASE}/${path}`, { method: "PATCH", headers: H, body: JSON.stringify({ fields: fields(data) }) });
  if (!r.ok) throw new Error(`${path}: ${await r.text()}`);
}

const now = Math.floor(Date.now() / 1000);
const meta = (name) => ({ name, nameLower: name.toLowerCase(), createdBy: "seed", createdAt: now });
const svg = (label, c1, c2) =>
  "data:image/svg+xml;utf8," +
  encodeURIComponent(`<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 55 85'><defs><linearGradient id='g' x1='0' y1='0' x2='1' y2='1'><stop offset='0' stop-color='${c1}'/><stop offset='1' stop-color='${c2}'/></linearGradient></defs><rect width='55' height='85' fill='url(#g)'/><text x='27.5' y='48' font-size='9' font-family='sans-serif' text-anchor='middle' fill='white'>${label}</text></svg>`);

const catalog = {
  artists: [["a_nova", "NOVA"], ["a_lumen", "LUMEN"]],
  categories: [["c_nova_al1", "a_nova", "1st Album"], ["c_nova_pop", "a_nova", "Pop-up Store"], ["c_lumen_al1", "a_lumen", "Debut Album"]],
  series: [["s_nova_al1_a", "c_nova_al1", "Ver. A"], ["s_nova_al1_b", "c_nova_al1", "Ver. B"], ["s_nova_pop_1", "c_nova_pop", "Lucky Draw"], ["s_lumen_al1_a", "c_lumen_al1", "Standard"]],
  members: ["Aria", "Bea", "Cleo", "Dana"],
};
const colors = [["#ff7ab6", "#7a6bff"], ["#ffb36b", "#ff5f8a"], ["#4fd1c5", "#5b6bff"], ["#a3e635", "#14b8a6"]];

async function main() {
  for (const [id, name] of catalog.artists) await put(`artists/${id}`, meta(name));
  const catArtist = {};
  for (const [id, artistId, name] of catalog.categories) {
    catArtist[id] = artistId;
    await put(`categories/${id}`, { ...meta(name), artistId });
  }
  const cards = [];
  for (const [sid, categoryId, sname] of catalog.series) {
    const artistId = catArtist[categoryId];
    await put(`series/${sid}`, { ...meta(sname), artistId, categoryId });
    for (const [i, m] of catalog.members.entries()) {
      const id = `${sid}_${i}`;
      const card = { ...meta(`${m} ${sname}`), artistId, categoryId, seriesId: sid, imageUrl: svg(m, ...colors[i]) };
      await put(`cards/${id}`, card);
      cards.push({ id, ...card });
    }
  }

  const users = [
    { uid: "demo_minji", nickname: "민지", greeting: "NOVA 포카 교환해요 💜", fav: ["a_nova"], own: [0, 1, 2, 4, 5, 8, 9] },
    { uid: "demo_yuki", nickname: "Yuki", greeting: "LUMEN & NOVA collector", fav: ["a_lumen", "a_nova"], own: [3, 6, 7, 10, 12, 13, 14] },
  ];
  for (const u of users) {
    await put(`users/${u.uid}`, { nickname: u.nickname, greeting: u.greeting, photoUrl: "", favoriteArtists: u.fav });
    for (const idx of u.own) {
      const c = cards[idx];
      await put(`users/${u.uid}/holdings/${c.id}`, {
        ownerUid: u.uid, cardId: c.id, artistId: c.artistId, categoryId: c.categoryId, seriesId: c.seriesId,
        count: 2, tradable: 1, public: true, visible: true,
      });
    }
  }
  console.log(`seeded ${catalog.artists.length} artists, ${catalog.categories.length} categories, ${catalog.series.length} series, ${cards.length} cards, ${users.length} users`);
}
main().catch((e) => { console.error(e); process.exit(1); });
