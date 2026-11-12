/**
 * Cloud Functions for PhotoCard Exchange.
 *
 *  - authSession  : sign-in hand-off between the hosted auth page and the app.
 *  - kakaoToken   : Kakao OAuth code -> Firebase custom token (Firebase has no Kakao provider).
 *  - acceptTrade  : atomically moves cards between both users when a trade is accepted.
 */
const { onRequest } = require("firebase-functions/v2/https");
const { setGlobalOptions } = require("firebase-functions/v2");
const admin = require("firebase-admin");

admin.initializeApp();
setGlobalOptions({ region: "asia-northeast3", maxInstances: 10 });

const db = admin.firestore();
const SESSION_TTL_MS = 5 * 60 * 1000;
const SESSION_RE = /^[a-f0-9]{32}$/;

async function verify(req) {
  const h = req.get("Authorization") || "";
  const m = h.match(/^Bearer (.+)$/);
  if (!m) throw Object.assign(new Error("unauthenticated"), { code: 401 });
  return admin.auth().verifyIdToken(m[1]);
}

function fail(res, e) {
  const code = e.code && Number.isInteger(e.code) ? e.code : 500;
  res.status(code).json({ error: { message: e.message } });
}

/**
 * POST {session, refreshToken} with Bearer idToken  (from the hosted auth page)
 * GET  ?session=<id>                                 (polled by the app; single use)
 */
exports.authSession = onRequest({ cors: true }, async (req, res) => {
  try {
    if (req.method === "POST") {
      const decoded = await verify(req);
      const { session, refreshToken } = req.body || {};
      if (!SESSION_RE.test(session || "") || !refreshToken) {
        return res.status(400).json({ error: { message: "bad request" } });
      }
      await db.collection("authSessions").doc(session).set({
        uid: decoded.uid,
        refreshToken,
        expiresAt: Date.now() + SESSION_TTL_MS,
      });
      return res.json({ ok: true });
    }
    if (req.method === "GET") {
      const session = String(req.query.session || "");
      if (!SESSION_RE.test(session)) return res.status(400).json({ error: { message: "bad request" } });
      const ref = db.collection("authSessions").doc(session);
      const out = await db.runTransaction(async (tx) => {
        const snap = await tx.get(ref);
        if (!snap.exists) return null;
        tx.delete(ref); // single use
        const d = snap.data();
        return d.expiresAt > Date.now() ? d : null;
      });
      if (!out) return res.status(404).json({ error: { message: "pending" } });
      return res.json({ refreshToken: out.refreshToken });
    }
    res.status(405).end();
  } catch (e) {
    fail(res, e);
  }
});

/**
 * POST {code, redirectUri} -> {customToken}
 * Requires env KAKAO_REST_API_KEY (and optional KAKAO_CLIENT_SECRET) in functions/.env
 */
exports.kakaoToken = onRequest({ cors: true }, async (req, res) => {
  try {
    const { code, redirectUri } = req.body || {};
    if (!code || !redirectUri) return res.status(400).json({ error: { message: "bad request" } });
    const params = new URLSearchParams({
      grant_type: "authorization_code",
      client_id: process.env.KAKAO_REST_API_KEY || "",
      redirect_uri: redirectUri,
      code,
    });
    if (process.env.KAKAO_CLIENT_SECRET) params.set("client_secret", process.env.KAKAO_CLIENT_SECRET);
    const tokRes = await fetch("https://kauth.kakao.com/oauth/token", {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded;charset=utf-8" },
      body: params,
    });
    const tok = await tokRes.json();
    if (!tok.access_token) return res.status(401).json({ error: { message: "kakao token failed" } });

    const meRes = await fetch("https://kapi.kakao.com/v2/user/me", {
      headers: { Authorization: `Bearer ${tok.access_token}` },
    });
    const me = await meRes.json();
    if (!me.id) return res.status(401).json({ error: { message: "kakao profile failed" } });

    const uid = `kakao:${me.id}`;
    const profile = (me.kakao_account && me.kakao_account.profile) || {};
    const update = {};
    if (profile.nickname) update.displayName = profile.nickname;
    if (profile.profile_image_url) update.photoURL = profile.profile_image_url;
    try {
      await admin.auth().updateUser(uid, update);
    } catch (e) {
      if (e.code === "auth/user-not-found") await admin.auth().createUser({ uid, ...update });
      else throw e;
    }
    const customToken = await admin.auth().createCustomToken(uid, { provider: "kakao" });
    res.json({ customToken });
  } catch (e) {
    fail(res, e);
  }
});

/**
 * POST {tradeId} with Bearer idToken of the receiver.
 * Validates both sides still have enough tradable copies and swaps them atomically.
 */
exports.acceptTrade = onRequest({ cors: true }, async (req, res) => {
  try {
    const decoded = await verify(req);
    const { tradeId } = req.body || {};
    if (!tradeId) return res.status(400).json({ error: { message: "bad request" } });

    await db.runTransaction(async (tx) => {
      const tradeRef = db.collection("trades").doc(tradeId);
      const trade = (await tx.get(tradeRef)).data();
      if (!trade) throw Object.assign(new Error("not found"), { code: 404 });
      if (trade.toUid !== decoded.uid) throw Object.assign(new Error("forbidden"), { code: 403 });
      if (trade.status !== "pending") throw Object.assign(new Error("not pending"), { code: 409 });

      const moves = [
        ...(trade.offered || []).map((i) => ({ ...i, from: trade.fromUid, to: trade.toUid })),
        ...(trade.requested || []).map((i) => ({ ...i, from: trade.toUid, to: trade.fromUid })),
      ];

      // Read everything first (transactions require reads before writes).
      const reads = await Promise.all(
        moves.map(async (m) => {
          const fromRef = db.doc(`users/${m.from}/holdings/${m.cardId}`);
          const toRef = db.doc(`users/${m.to}/holdings/${m.cardId}`);
          const cardRef = db.doc(`cards/${m.cardId}`);
          const [f, t, c] = await Promise.all([tx.get(fromRef), tx.get(toRef), tx.get(cardRef)]);
          return { m, fromRef, toRef, f: f.data(), t: t.data(), c: c.data() };
        })
      );

      for (const r of reads) {
        const qty = Number(r.m.qty) || 0;
        if (!r.f || (r.f.tradable || 0) < qty || (r.f.count || 0) < qty) {
          throw Object.assign(new Error(`insufficient cards: ${r.m.cardId}`), { code: 409 });
        }
      }

      for (const r of reads) {
        const qty = Number(r.m.qty) || 0;
        const newCount = r.f.count - qty;
        if (newCount <= 0) tx.delete(r.fromRef);
        else tx.update(r.fromRef, { count: newCount, tradable: Math.min(r.f.tradable - qty, newCount) });

        if (r.t) {
          tx.update(r.toRef, { count: (r.t.count || 0) + qty });
        } else {
          const c = r.c || {};
          tx.set(r.toRef, {
            ownerUid: r.m.to,
            cardId: r.m.cardId,
            artistId: c.artistId || "",
            categoryId: c.categoryId || "",
            seriesId: c.seriesId || "",
            count: qty,
            tradable: 0,
            public: true,
            visible: true,
          });
        }
      }
      tx.update(tradeRef, { status: "accepted", acceptedAt: Date.now() });
    });
    res.json({ ok: true });
  } catch (e) {
    fail(res, e);
  }
});
