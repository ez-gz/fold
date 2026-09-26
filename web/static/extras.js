// Web-only layer on top of proto/index.html (injected by build.py): first-run explainer, the You sheet
// (account, sync, invite, install, reset) and optional cloud sync. No typed input anywhere: sign-in is one-tap OAuth.
// The app itself never depends on this file — without config.json keys everything stays on-device, exactly as before.
(() => {
  const $h = html => { const t = document.createElement("template"); t.innerHTML = html.trim(); return t.content.firstChild; };
  const LS = localStorage, SKIP = new Set(["fold.seen", "fold.syncedAt"]);
  const snapshot = () => { const o = {}; for (let i = 0; i < LS.length; i++) { const k = LS.key(i); if (k.startsWith("fold.") && !SKIP.has(k)) o[k] = LS.getItem(k); } return o; };
  const weight = d => { let n = 0; try { n += JSON.parse(d["fold.puzzle"] || "{}").n || 0; const b = JSON.parse(d["fold.book"] || "{}"); for (const k in b) if (k.startsWith("c:")) n += b[k].n || 0; } catch (e) {} return n; };
  const sheet = (title, body) => { const o = $h(`<div class="overlay"><div class="sheet"><h3>${title}<span class="x">Close</span></h3></div></div>`); o.firstChild.appendChild(body); o.onclick = e => { if (e.target === o || e.target.classList.contains("x")) o.remove(); }; document.body.appendChild(o); return o; };
  const css = document.createElement("style"); css.textContent = `
    .you{margin-left:8px;width:30px;height:30px;border-radius:50%;border:1px solid var(--line);background:var(--panel2);color:var(--text);font-size:15px;flex:none;cursor:pointer;padding:0}
    header .chips{gap:5px}header .chip{padding-left:10px;padding-right:10px}.you{margin-left:4px}
    header:has(.sbadge) .you{display:none}#stats{cursor:pointer}   /* once there is a score badge, the badge is the You button (keeps the header on one line at 375px) */
    .you.in{border-color:var(--accent);color:var(--accent)}
    .xrow{display:flex;gap:8px;align-items:center;justify-content:space-between;padding:12px 0;border-top:1px solid var(--line);font-size:14px}
    .xrow small{display:block;color:var(--dim);font-size:12px;margin-top:2px}
    .xbtn{border:0;border-radius:12px;padding:10px 14px;font-weight:700;font-size:14px;background:var(--panel2);color:var(--text);cursor:pointer;white-space:nowrap}
    .xbtn.pri{background:var(--accent);color:#06110c}.xbtn.warn{color:var(--bad)}.xbtn.wide{width:100%;height:48px;margin-top:8px;font-size:15px}
    .xbtn.apple{background:#fff;color:#000}.xbtn.google{background:#fff;color:#1f1f1f}
    .intro p{color:var(--dim);font-size:14px;margin:6px 0 0}.intro .m{padding:10px 0;border-top:1px solid var(--line)}.intro b{color:var(--text)}`;
  document.head.appendChild(css);

  /* ---------- cloud sync (optional) ---------- */
  let sb = null, user = null, pushT = null;
  // Returning from the sign-in redirect: take the one-time code out of the URL right now, before the app rewrites the
  // address bar for share links (that rewrite used to wipe the sign-in token, so the session was never picked up).
  const authCode = (() => { const q = new URLSearchParams(location.search), c = q.get("code"); if (!c && !q.get("error")) return null; ["code", "error", "error_code", "error_description"].forEach(k => q.delete(k)); const s = q.toString(); history.replaceState(null, "", location.pathname + (s ? "?" + s : "") + location.hash); return c; })();
  const loadScript = src => new Promise((ok, no) => { const s = document.createElement("script"); s.src = src; s.onload = ok; s.onerror = no; document.head.appendChild(s); });
  async function initAuth(){
    let cfg; try { cfg = await (await fetch("config.json")).json(); } catch (e) { return; }
    if (!cfg.supabaseUrl || !cfg.supabaseAnonKey) return;
    await loadScript("https://cdn.jsdelivr.net/npm/@supabase/supabase-js@2");
    sb = supabase.createClient(cfg.supabaseUrl, cfg.supabaseAnonKey, { auth: { flowType: "pkce", detectSessionInUrl: false, persistSession: true, autoRefreshToken: true } }); sb._providers = cfg.providers || ["apple", "google"];
    sb.auth.onAuthStateChange((_e, s) => { const was = user; user = s ? s.user : null; btn.classList.toggle("in", !!user); if (user && !was) reconcile(); });
    if (authCode) { const { error } = await sb.auth.exchangeCodeForSession(authCode); if (error) console.error("sign-in failed", error.message); }
    // come back to the hand or puzzle the visitor was on before the sign-in redirect
    const back = sessionStorage.getItem("fold.back"); if (back) { sessionStorage.removeItem("fold.back"); if (!/^#[hp]=/.test(location.hash)) history.replaceState(null, "", location.pathname + location.search + back); }
  }
  async function reconcile(){ // first contact after sign-in: whichever side has more answered spots wins, then keep pushing
    const { data } = await sb.from("progress").select("data").eq("user_id", user.id).maybeSingle(), local = snapshot();
    if (data && weight(data.data) > weight(local)) { for (const k in data.data) LS.setItem(k, data.data[k]); location.reload(); } else push();
  }
  async function push(){ if (!sb || !user) return; await sb.from("progress").upsert({ user_id: user.id, data: snapshot(), updated_at: new Date().toISOString() }); try { LS.setItem("fold.syncedAt", Date.now()); } catch (e) {} }
  const rawSet = LS.setItem.bind(LS);
  Storage.prototype.setItem = function (k, v) { rawSet(k, v); if (this === LS && user && k.startsWith("fold.") && !SKIP.has(k)) { clearTimeout(pushT); pushT = setTimeout(push, 4000); } };
  addEventListener("visibilitychange", () => { if (document.visibilityState === "hidden" && pushT) { clearTimeout(pushT); pushT = null; push(); } });
  const signIn = p => { if (/^#[hp]=/.test(location.hash)) sessionStorage.setItem("fold.back", location.hash); sb.auth.signInWithOAuth({ provider: p, options: { redirectTo: location.origin + location.pathname } }); };

  /* ---------- the You sheet ---------- */
  const btn = $h(`<button class="you" aria-label="You">☺</button>`);
  function youSheet(){
    const b = $h(`<div></div>`), n = weight(snapshot());
    if (!sb) b.appendChild($h(`<div class="xrow" style="border:0"><div>Saved on this device<small>${n} answers so far. Sign-in and sync across devices is coming.</small></div></div>`));
    else if (!user) { b.appendChild($h(`<div style="font-size:14px;color:var(--dim)">Save your ${n ? n + " answers" : "progress"} and pick up on any device. One tap, nothing to type.</div>`));
      sb._providers.forEach(p => { const x = $h(`<button class="xbtn wide ${p}">Continue with ${p[0].toUpperCase() + p.slice(1)}</button>`); x.onclick = () => signIn(p); b.appendChild(x); }); }
    else { const r = $h(`<div class="xrow" style="border:0"><div>${user.email || "Signed in"}<small>Progress syncs automatically</small></div><button class="xbtn">Sign out</button></div>`);
      r.querySelector("button").onclick = async () => { await push(); await sb.auth.signOut(); o.remove(); }; b.appendChild(r); }
    const beg = (typeof state !== "undefined" && state.beginner), lv = $h(`<div class="xrow"><div>Level: <b>${beg ? "Beginner" : "Advanced"}</b><small>${beg ? "Two questions per decision, one answer, plain words. Switch for sizes, frequencies and ranges." : "Sizes, frequencies, ranges and the full why. Switch for the distilled version."}</small></div><button class="xbtn">${beg ? "Go advanced" : "Go beginner"}</button></div>`);
    lv.querySelector("button").onclick = () => { o.remove(); if (typeof setBeginner === "function") setBeginner(!beg); }; b.appendChild(lv);
    const inv = $h(`<div class="xrow"><div>Invite a friend<small>Send them the app, or share any hand with the yellow button</small></div><button class="xbtn pri">Invite</button></div>`);
    inv.querySelector("button").onclick = async () => { const url = location.origin + location.pathname, text = "Solver-backed poker trainer I've been using. Free, works on your phone."; try { navigator.share ? await navigator.share({ title: "Fold", text, url }) : await navigator.clipboard.writeText(text + " " + url); } catch (e) {} }; b.appendChild(inv);
    if (!matchMedia("(display-mode: standalone)").matches && !navigator.standalone) b.appendChild($h(`<div class="xrow"><div>Add to your home screen<small>${/iPhone|iPad/.test(navigator.userAgent) ? "Tap the Share icon in Safari, then “Add to Home Screen”" : "Open the browser menu, then “Install app” or “Add to Home screen”"}. Opens full screen and works offline.</small></div></div>`));
    const how = $h(`<div class="xrow"><div>How Fold works</div><button class="xbtn">Show</button></div>`); how.querySelector("button").onclick = () => { o.remove(); intro(); }; b.appendChild(how);
    const rs = $h(`<div class="xrow"><div>Reset my progress<small>Clears scores and leaks${user ? " here and in your account" : " on this device"}</small></div><button class="xbtn warn">Reset</button></div>`);
    rs.querySelector("button").onclick = e => { const x = e.target; if (x.textContent === "Reset") { x.textContent = "Tap again to confirm"; setTimeout(() => x.textContent = "Reset", 3000); return; } Object.keys(snapshot()).forEach(k => LS.removeItem(k)); (user ? push() : Promise.resolve()).then(() => location.reload()); }; b.appendChild(rs);
    if (user) { const del = $h(`<div class="xrow"><div>Delete my account<small>Removes your saved progress from our servers and signs you out</small></div><button class="xbtn warn">Delete</button></div>`);
      del.querySelector("button").onclick = async e => { const x = e.target; if (x.textContent === "Delete") { x.textContent = "Tap again to confirm"; setTimeout(() => x.textContent = "Delete", 3000); return; } await sb.from("progress").delete().eq("user_id", user.id); await sb.rpc("delete_me").catch(() => {}); await sb.auth.signOut(); location.reload(); }; b.appendChild(del); }
    b.appendChild($h(`<div style="padding-top:14px;border-top:1px solid var(--line);text-align:center;font-size:12px"><a href="privacy.html" style="color:var(--dim)">Privacy &amp; Terms</a></div>`));
    const o = sheet("You", b);
  }
  btn.onclick = youSheet;

  /* ---------- first run ---------- */
  function intro(){
    const b = $h(`<div class="intro"><p>Every answer is graded against a solver. After each one you see <b>why</b>, built around what your opponent can hold.</p>
      <div class="m"><b>Hands</b><p>Play a full hand street by street, then review it.</p></div>
      <div class="m"><b>Drills</b><p>Single decisions, fast. Filter by street or focus on your leaks.</p></div>
      ${(typeof state !== "undefined" && state.beginner) ? `<div class="m"><b>Beginner mode is on</b><p>Two questions per decision: what do they have, and what do I do about it. Switch to advanced any time from this sheet.</p></div>` : `<div class="m"><b>Puzzle</b><p>Your opponent's range is narrow. Name the exact hands they can have.</p></div>`}
      <div class="m"><b>Leaks</b><p>Where you lose the most, tracked as you play.</p></div>
      <button class="xbtn pri wide">Start</button><p style="text-align:center;font-size:12px;margin-top:10px">Free. No account needed. <a href="privacy.html" style="color:var(--dim)">Privacy &amp; Terms</a></p></div>`);
    const o = sheet("Welcome to Fold", b); b.querySelector("button").onclick = () => o.remove();
    try { rawSet("fold.seen", "1"); } catch (e) {}
  }

  addEventListener("DOMContentLoaded", () => {
    const hd = document.querySelector("header"); if (hd) hd.appendChild(btn); const st = document.getElementById("stats"); if (st) st.onclick = youSheet;
    // a visitor arriving on a shared link goes straight to the challenge; everyone else gets the explainer once
    if (!LS.getItem("fold.seen") && !/^#[hp]=/.test(location.hash)) intro();
    initAuth();
  });
})();
