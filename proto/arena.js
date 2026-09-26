/* ---------- Arena: a 6-max table against five bots (docs/SPEC.md 13.8) ----------
   Preflop: every seat plays the v1 charts (solver/src/preflop.rs). Postflop: heads-up on a pack flop the bots sample the
   solved strategy wherever the exact node was exported (spots, played-hand steps, river sweeps); everywhere else, and
   multiway, a hand-strength policy. Hero's decisions are graded where a solve exists (frequency, or EV when the pack
   carries a drill for the exact hand) and filed into the same book / patterns / log as Hands and Drills. */

const AR_RANGES = {
  UTG_OPEN: "55+,A2s+,K8s+,Q9s+,J9s+,T9s,ATo+,KJo+,44:0.5,33:0.3,22:0.3,98s:0.5,87s:0.5,76s:0.5,65s:0.4,K7s:0.5,QJo:0.5,A9o:0.3",
  CO_OPEN: "22+,A2s+,K5s+,Q8s+,J8s+,T8s+,97s+,86s+,76s,65s,54s,A9o+,KTo+,QTo+,JTo,K4s:0.5,Q7s:0.5,J7s:0.5,A8o:0.5,K9o:0.5",
  BTN_OPEN: "22+,A2s+,K2s+,Q4s+,J6s+,T6s+,96s+,85s+,75s+,64s+,53s+,43s,A4o+,K8o+,Q9o+,J9o+,T8o+,98o,Q3s:0.5,Q2s:0.5,J5s:0.5,A3o:0.5,A2o:0.5,K7o:0.5,Q8o:0.5,87o:0.5",
  SB_OPEN: "22+,A2s+,K4s+,Q6s+,J7s+,T7s+,96s+,86s+,75s+,65s,54s,A5o+,K9o+,Q9o+,J9o+,T9o,K3s:0.5,K2s:0.5,Q5s:0.5,A4o:0.5,A3o:0.5,K8o:0.5,98o:0.5",
  BB_3BET_VS_UTG: "QQ+,AKs,AKo:0.6,JJ:0.4,AQs:0.4,A5s:0.6,A4s:0.4,KQs:0.2,87s:0.2,76s:0.2",
  BB_CALL_VS_UTG: "JJ-22,AQs-A2s,K7s+,Q8s+,J8s+,T8s+,97s+,86s+,75s+,65s,54s,AQo,AJo,KQo,JJ:0.6,AQs:0.6,A5s:0.4,A4s:0.6,KQs:0.8,87s:0.8,76s:0.8,AKo:0.4,ATo:0.7,KJo:0.7,QJo:0.5,K6s-K2s,Q7s-Q5s,J7s,T7s,96s,85s,64s,KTo:0.4,A9o:0.3",
  BB_3BET_VS_CO: "JJ+,AQs+,AKo,TT:0.5,AJs:0.5,A5s,A4s,KJs:0.4,KTs:0.4,QJs:0.3,T9s:0.3,98s:0.3,87s:0.3,76s:0.3,AQo:0.4,KQo:0.25",
  BB_CALL_VS_CO: "TT-22,AJs-A2s,K2s+,Q5s+,J7s+,T7s+,96s+,85s+,75s+,64s+,54s,AQo-A8o,KTo+,QTo+,JTo,TT:0.5,AJs:0.5,A5s:0,A4s:0,KJs:0.6,KTs:0.6,QJs:0.7,T9s:0.7,98s:0.7,87s:0.7,76s:0.7,AQo:0.6,KQo:0.75,A7o:0.5,K9o,T9o:0.5,Q4s-Q2s,J6s-J4s:0.5,T6s,95s,A6o-A2o:0.5,Q9o:0.5,J9o:0.5,98o:0.5,K8o:0.4",
  BB_3BET_VS_BTN: "TT+,AQs+,AKo,99:0.5,AJs:0.5,A5s,A4s,A3s:0.5,KTs:0.5,K9s:0.5,QTs:0.5,J9s:0.5,T8s:0.5,97s:0.5,86s:0.5,76s:0.5,65s:0.5,AQo:0.5,AJo:0.3,KQo:0.4,88:0.3,A2s:0.5,K8s:0.5,Q9s:0.5,J8s:0.5,T7s:0.3,ATo:0.3,KJo:0.3,A5o:0.3",
  BB_CALL_VS_BTN: "99-22,AJs-A2s,K2s+,Q3s+,J5s+,T6s+,95s+,85s+,74s+,64s+,53s+,43s,AQo-A2o,K8o+,Q9o+,J9o+,T8o+,98o,87o,99:0.5,AJs:0.5,A5s:0,A4s:0,A3s:0.5,KTs:0.5,K9s:0.5,QTs:0.5,J9s:0.5,T8s:0.5,97s:0.5,86s:0.5,76s:0.5,65s:0.5,AQo:0.5,AJo:0.7,KQo:0.6,Q2s:0.5,K7o:0.5,76o:0.4",
  BB_CALL_VS_SB: "TT-22,AJs-A2s,K2s+,Q2s+,J4s+,T6s+,95s+,85s+,74s+,64s+,53s+,43s,AJo-A2o,K5o+,Q8o+,J8o+,T8o+,97o+,87o,76o,TT:0.5,AJs:0.5,KQs:0.5,KJs:0.6,AJo:0.6,KQo:0.6,A5s:0.5,A4s:0.5,J3s:0.5,T5s:0.5",
  BTN_CALL_VS_CO: "99-22,AJs-A8s,KTs+,QTs+,JTs,T9s,98s,87s,76s,TT:0.5,AQs:0.5,A5s:0.5,A4s:0.5,KQs:0.6,J9s:0.5,65s:0.5,AQo:0.5,AJo:0.5,KQo:0.6",
  BTN_CALL_VS_UTG: "TT-22,AQs-ATs,KTs+,QTs+,JTs,T9s,98s,87s:0.6,76s:0.6,65s:0.4,JJ:0.5,A5s:0.5,AQo:0.5,KQo:0.4",
  BTN_CALL_3BET: "JJ-44,AQs-A9s,A5s,A4s,K9s+,Q9s+,J9s+,T8s+,97s+,87s,76s,65s,AQo,KQo,QQ:0.3,33:0.5,22:0.5,AKs:0.2,A8s:0.5,54s:0.5,AJo:0.5,KJo:0.4,AKo:0.3",
  SB_3BET_VS_BTN: "99+,ATs+,A5s,A4s,KTs+,QTs+,JTs,T9s,AJo+,KQo,88:0.5,77:0.5,A9s:0.5,A3s:0.5,K9s:0.5,J9s:0.5,98s:0.5,87s:0.5,76s:0.5,65s:0.5,ATo:0.4,KJo:0.5",
  BTN_3BET_VS_CO: "JJ+,AQs+,AKo,A5s,A4s,TT:0.5,AJs:0.5,A3s:0.4,KQs:0.5,KJs:0.4,KTs:0.4,QJs:0.3,JTs:0.3,T9s:0.3,98s:0.3,87s:0.3,76s:0.4,65s:0.4,AQo:0.6,AJo:0.2,KQo:0.3",
  CO_CALL_3BET: "JJ-55,AQs-ATs,KTs+,QTs+,JTs,T9s,98s,QQ:0.3,44:0.5,AKs:0.2,A5s:0.6,A4s:0.4,87s:0.6,76s:0.6,65s:0.4,AQo:0.7,KQo:0.5,AKo:0.3",
  // beyond the pack's charts: hand-written and tight, so the bots do not spew in 4-bet pots
  FOURBET: "QQ+,AKs,AKo:0.7,A5s:0.5,A4s:0.3", COLD_4BET: "KK+,AKs:0.3", COLD_CALL_3BET: "QQ,JJ:0.5,AKs,AQs:0.5",
  JAM_VS_4BET: "KK+,AKs:0.5", CALL_VS_4BET: "QQ,AKo,AKs:0.5,JJ:0.5,AQs:0.3", CALL_JAM: "QQ+,AKs,AKo:0.5",
};
const AR_GRIDS = {};
// "55+,A2s+,KTs-K8s,AQo:0.5" -> 169 weights (row/col 0 = Ace, suited above the diagonal); later tokens override earlier ones
function parseRange(str){
  const g = new Float32Array(169), ri = c => R.indexOf(c), set = (a, b, suited, w) => { const hi = Math.min(a, b), lo = Math.max(a, b); g[hi === lo || suited ? hi * 13 + lo : lo * 13 + hi] = w; };
  str.split(",").forEach(tok => {
    tok = tok.trim(); if (!tok) return; let w = 1, m; if ((m = tok.match(/^(.*):([\d.]+)$/))){ tok = m[1]; w = +m[2]; }
    if ((m = tok.match(/^([AKQJT2-9])\1(\+?)$/))){ const r = ri(m[1]); for (let x = m[2] ? 0 : r; x <= r; x++) set(x, x, false, w); return; }
    if ((m = tok.match(/^([AKQJT2-9])\1-([AKQJT2-9])\2$/))){ const a = ri(m[1]), b = ri(m[2]); for (let x = Math.min(a, b); x <= Math.max(a, b); x++) set(x, x, false, w); return; }
    if ((m = tok.match(/^([AKQJT2-9])([AKQJT2-9])([so])(\+?)$/))){ const a = ri(m[1]), b = ri(m[2]), s = m[3] === "s"; if (m[4]) for (let x = a + 1; x <= b; x++) set(a, x, s, w); else set(a, b, s, w); return; }
    if ((m = tok.match(/^([AKQJT2-9])([AKQJT2-9])([so])-\1([AKQJT2-9])\3$/))){ const a = ri(m[1]), b = ri(m[2]), c = ri(m[4]), s = m[3] === "s"; for (let x = Math.min(b, c); x <= Math.max(b, c); x++) set(a, x, s, w); return; }
    console.warn("range token", tok);
  });
  return g;
}
const rangeGrid = name => AR_GRIDS[name] || (AR_GRIDS[name] = parseRange(AR_RANGES[name]));

const AR_ORDER_PRE = ["UTG", "HJ", "CO", "BTN", "SB", "BB"], AR_ORDER_POST = ["SB", "BB", "UTG", "HJ", "CO", "BTN"];
const AR_OPEN = { UTG: 2.5, HJ: 2.5, CO: 2.5, BTN: 2.5, SB: 3 };
const AR = { stacks: {}, seatOf: 0, net: 0, n: 0, graded: 0, ok: 0, hand: null, busy: false };

/* ---------- preflop policy: (raise, call) probabilities for a seat from the charts ---------- */
function prePolicy(H, seat, cell){
  const lv = H.raises.length, op = lv ? H.raises[lv - 1].seat : null, w = name => rangeGrid(name)[cell];
  if (lv === 0) return seat === "BB" ? { raise: 0, call: 1, label: "check" } : { raise: seat === "HJ" ? (w("UTG_OPEN") + w("CO_OPEN")) / 2 : w(seat + "_OPEN"), call: 0, label: "first in" };
  if (lv === 1){
    const early = op === "UTG" || op === "HJ";
    if (seat === "BB") return { raise: early ? w("BB_3BET_VS_UTG") : op === "CO" ? w("BB_3BET_VS_CO") : w("BB_3BET_VS_BTN"), call: early ? w("BB_CALL_VS_UTG") : op === "CO" ? w("BB_CALL_VS_CO") : op === "BTN" ? w("BB_CALL_VS_BTN") : w("BB_CALL_VS_SB"), label: "facing an open" };
    if (seat === "SB") return { raise: w("SB_3BET_VS_BTN") * (op === "BTN" ? 1 : op === "CO" ? 0.8 : 0.6), call: 0, label: "facing an open" };
    if (seat === "BTN") return { raise: w("BTN_3BET_VS_CO") * (op === "CO" ? 1 : 0.7), call: op === "CO" ? w("BTN_CALL_VS_CO") : w("BTN_CALL_VS_UTG"), label: "facing an open" };
    return { raise: w("BTN_3BET_VS_CO") * 0.7, call: w("BTN_CALL_VS_UTG") * 0.8, label: "facing an open" };
  }
  const me = H.raises.some(r => r.seat === seat), mine = H.raises[H.raises.length - 2] && H.raises[H.raises.length - 2].seat === seat;
  if (lv === 2) return mine ? { raise: w("FOURBET"), call: seat === "BTN" ? w("BTN_CALL_3BET") : w("CO_CALL_3BET"), label: "facing a 3-bet" } : { raise: w("COLD_4BET"), call: w("COLD_CALL_3BET"), label: "facing a 3-bet" };
  if (lv === 3) return mine ? { raise: w("JAM_VS_4BET"), call: w("CALL_VS_4BET"), label: "facing a 4-bet" } : { raise: 0, call: w("COLD_4BET"), label: "facing a 4-bet" };
  return { raise: 0, call: me ? w("CALL_JAM") : w("JAM_VS_4BET"), label: "facing all-in" };
}
// raise size in bb (total this street): pack sizes where a pack formation could follow, else 3x IP / 4x+1 OOP, 2.3x for 4-bets
function preRaiseTo(H, seat){
  const lv = H.raises.length, stack = AR.stacks[seat] + H.commit[seat];
  if (lv === 0) return AR_OPEN[seat];
  const last = H.raises[lv - 1].to;
  if (lv === 1) return Math.min(stack, seat === "SB" || seat === "BB" ? (H.raises[0].seat === "SB" ? 12 : 11) + H.callers.length : 3 * last + H.callers.length);
  if (lv === 2) return Math.min(stack, Math.round(2.3 * last * 2) / 2);
  return stack;
}

/* ---------- postflop: solved nodes from the pack ---------- */
function nodeIndex(f){
  if (f._nodes) return f._nodes;
  const idx = new Map(), key = (board, hist) => board.join("") + "|" + hist.map(h => h.street + h.pos + h.label).join(",");
  const addSpot = s => { const raw = s._raw ? s._raw.history : s.history, labels = s._raw ? s._raw.actions : s.actions.map(a => a.label), k = key(s.board, raw);
    if (!idx.has(k)) idx.set(k, { actor: s.hero, actions: s.actions.map((a, i) => ({ label: labels[i], kind: a.kind, amt: a.amt })), strat: s.hs, spot: s }); };
  f.spots.forEach(addSpot); (f.hands || []).forEach(h => h.steps.forEach(st => addSpot(st.spot)));
  (f.rivers || []).forEach(sw => sw.cards.forEach(c => { const k = key([...sw.board, c.card], sw.history); if (!idx.has(k)) idx.set(k, { actor: c.actor, actions: c.actions, strat: c.strat, spot: null }); }));
  return f._nodes = idx;
}
function solvedNode(H){
  if (!H.f) return null;
  return nodeIndex(H.f).get(H.board.join("") + "|" + H.post.map(h => h.street + h.pos + h.label).join(",")) || null;
}

/* ---------- postflop fallback policy: hand strength buckets ---------- */
function strengthOf(hand, board){
  const ft = fineType(hand, board);
  if (/Two pair|Set|Straight$|Flush$|Full house/.test(ft)) return "monster";
  if (/Overpair|Top pair/.test(ft)) return "top";
  if (/Second pair|Low pair|Pocket pair|Underpair/.test(ft)) return "weak";
  if (/flush draw|Open-ended/i.test(ft)) return "draw";
  if (/Gutshot|overcards|Ace high/.test(ft)) return "gut";
  return "air";
}
// returns an action {kind, label, amt}: amt = this seat's street commit after the action; label = raw pack-style label
function policyAct(H, seat){
  const st = strengthOf(H.cards[seat], H.board), toCall = H.maxCommit - H.commit[seat], pot = H.pot + Object.values(H.commit).reduce((a, b) => a + b, 0);
  const others = H.alive.filter(p => p !== seat).length, multi = others > 1, agg = H.aggressor === seat, river = H.street === 2, r = Math.random();
  const cap = AR.stacks[seat] + H.commit[seat], r1 = x => Math.round(x * 10) / 10;
  const bet = pc => ({ kind: "bet", label: `Bet ${pc}%`, amt: Math.min(cap, r1(pot * pc / 100)) });
  const raise = () => { const to = Math.min(cap, r1(H.maxCommit * 2.6 + pot * 0.5)); return { kind: "raise", label: `Raise to ${to}`, amt: to }; };
  const call = { kind: "call", label: "Call", amt: Math.min(cap, H.maxCommit) }, check = { kind: "check", label: "Check", amt: H.commit[seat] }, fold = { kind: "fold", label: "Fold", amt: H.commit[seat] };
  if (toCall <= 0){
    if (st === "monster") return r < (multi ? 0.85 : 0.75) ? bet(66) : check;
    if (st === "top") return r < (multi ? 0.45 : 0.6) ? bet(river ? 66 : 50) : check;
    if (st === "weak") return r < 0.15 && !river ? bet(33) : check;
    if (st === "draw") return river ? (r < 0.25 ? bet(66) : check) : (r < 0.45 ? bet(66) : check);
    if (st === "gut") return !river && agg && H.street === 0 && r < (multi ? 0.3 : 0.55) ? bet(33) : (river && !multi && r < 0.2 ? bet(66) : check);
    return agg && !river && r < (multi ? 0.2 : 0.4) ? bet(33) : (river && !multi && r < 0.18 ? bet(66) : check);
  }
  const price = toCall / (pot + toCall), big = toCall > pot * 0.6, allin = toCall >= AR.stacks[seat];
  if (st === "monster") return r < (allin ? 1 : 0.55) ? call : raise();
  if (st === "top") return big ? (r < (river ? 0.45 : 0.6) ? call : fold) : (r < 0.9 ? call : raise());
  if (st === "weak") return big ? (r < 0.2 ? call : fold) : (r < (river ? 0.5 : 0.7) ? call : fold);
  if (st === "draw") return river ? fold : (price < 0.36 ? (r < 0.85 ? call : raise()) : (r < 0.35 ? call : fold));
  if (st === "gut") return !river && price < 0.25 && r < 0.5 ? call : fold;
  return fold;
}

/* ---------- table engine ---------- */
function arenaStart(){ arenaDeal(); }
function arenaStop(){ const H = AR.hand; AR.hand = null; if (H && H.resolve){ const r = H.resolve; H.resolve = null; r({ kind: "fold", label: "Fold", amt: 0 }); } }   // unblock a run waiting on the hero so AR.busy clears
function shuffled(){ const d = []; for (const r of R) for (const s of "shdc") d.push(r + s); for (let i = d.length - 1; i > 0; i--){ const j = Math.floor(Math.random() * (i + 1)); [d[i], d[j]] = [d[j], d[i]]; } return d; }
function arenaDeal(){
  arenaStop();   // unwind any hand still running (a run parked on the hero's click would hold AR.busy forever)
  SEATS.forEach(p => AR.stacks[p] = 100);   // top-up every hand: equal stacks, so no side pots
  AR.seatOf = (AR.seatOf + 1) % 6; const hero = SEATS[AR.seatOf];
  const deck = shuffled(), cards = {}; SEATS.forEach(p => cards[p] = [deck.pop(), deck.pop()]);
  const H = { hero, cards, deck, board: [], street: -1, pot: 0, commit: {}, maxCommit: 0, alive: [...SEATS], allin: [], raises: [], callers: [], aggressor: null, post: [], f: null, log: [], done: false, res: [] };
  SEATS.forEach(p => H.commit[p] = 0); H.commit.SB = 0.5; H.commit.BB = 1; AR.stacks.SB -= 0.5; AR.stacks.BB -= 1; H.maxCommit = 1;
  AR.hand = H; arenaRender(); arenaRun();
}
const arDelay = () => AR.fast ? 0 : 380 + Math.random() * 320;
async function bettingRound(H, order){
  let i = 0, acted = new Set();
  while (true){
    const live = H.alive.filter(p => !H.allin.includes(p));
    if (H.alive.length === 1 || live.length === 0) break;
    if (live.every(p => acted.has(p) && H.commit[p] === H.maxCommit)) break;
    const p = order[i % order.length]; i++;
    if (!H.alive.includes(p) || H.allin.includes(p) || (acted.has(p) && H.commit[p] === H.maxCommit)) continue;
    const a = await arenaDecide(p); if (AR.hand !== H) return false; acted.add(p);
    if (a.kind === "bet" || a.kind === "raise" || a.kind === "open"){ acted = new Set([p]); if (H.street >= 0) H.aggressor = p; }
  }
  return true;
}
async function arenaRun(){
  const H = AR.hand; if (!H) return;
  if (AR.busy){ AR.pending = true; return; }   // an older hand is still unwinding; it restarts us when it exits
  AR.busy = true; AR.pending = false;
  try {
    if (!await bettingRound(H, AR_ORDER_PRE)) return;
    for (let st = 0; st < 3 && H.alive.length > 1; st++){
      H.street = st; SEATS.forEach(p => { H.pot += H.commit[p]; H.commit[p] = 0; }); H.maxCommit = 0;
      if (st === 0) H.aggressor = H.raises.length ? H.raises[H.raises.length - 1].seat : null;
      if (st === 0) arenaFlop(H); else H.board.push(H.deck.pop());
      arenaRender(); await sleep(AR.fast ? 0 : 500); if (AR.hand !== H) return;
      if (H.alive.filter(p => !H.allin.includes(p)).length < 2) continue;
      if (!await bettingRound(H, AR_ORDER_POST)) return;
    }
    arenaFinish();
  } finally { AR.busy = false; if (AR.pending){ AR.pending = false; if (AR.hand && !AR.hand.done) arenaRun(); } }
}
// the flop: when the preflop line is one the pack solved (heads-up, pack sizes), most deals use one of that
// formation's solved flops (cards not already dealt), so the postflop has a solve behind it; otherwise random
function arenaFormation(H){
  if (H.alive.length !== 2) return null;
  const lv = H.raises.length; if (lv < 1 || lv > 2) return null;
  const opener = H.raises[lv - 1].seat, caller = H.alive.find(p => p !== opener);
  const name = lv === 1 ? `${opener} vs ${caller}` : `${caller} vs ${opener}, 3-bet pot`, F = PACK.formations[name]; if (!F) return null;
  if (Math.abs(H.raises[0].to - F.open_size) > 0.01 || (lv === 2 && Math.abs(H.raises[1].to - F.three_bet) > 0.01)) return null;
  return name;
}
function arenaFlop(H){
  H.f = null; H.post = [];
  const name = arenaFormation(H), dealt = new Set(SEATS.flatMap(p => H.cards[p]));
  const cands = name ? PACK.flops.filter(f => f.formation === name && Math.abs(f.start_pot - H.pot) < 0.05 && !f.flop.match(/../g).some(c => dealt.has(c))) : [];
  if (cands.length && Math.random() < 0.85){ const f = cands[Math.floor(Math.random() * cands.length)], cs = f.flop.match(/../g); H.deck = H.deck.filter(c => !cs.includes(c)); H.board.push(...cs); H.f = f; }
  else H.board.push(H.deck.pop(), H.deck.pop(), H.deck.pop());
}
async function arenaDecide(p){
  const H = AR.hand; arenaRender(p);
  if (p === H.hero) return new Promise(res => { H.resolve = res; arenaButtons(); });
  await sleep(arDelay()); if (AR.hand !== H) return { kind: "fold" };
  const a = botAct(p); arenaApply(p, a); return a;
}
function botAct(p){
  const H = AR.hand, cell = cellOf(H.cards[p]), cap = AR.stacks[p] + H.commit[p];
  if (H.street < 0){
    const pol = prePolicy(H, p, cell), r = Math.random(), toCall = H.maxCommit - H.commit[p];
    if (r < pol.raise) return { kind: H.raises.length ? "raise" : "open", label: "", amt: preRaiseTo(H, p) };
    if (r < pol.raise + pol.call) return toCall > 0 ? { kind: "call", label: "Call", amt: Math.min(cap, H.maxCommit) } : { kind: "check", label: "Check", amt: H.commit[p] };
    return toCall > 0 ? { kind: "fold", label: "Fold", amt: H.commit[p] } : { kind: "check", label: "Check", amt: H.commit[p] };
  }
  const node = solvedNode(H);
  if (node && node.actor === p){
    const w = node.actions.map((a, k) => node.strat[k] ? node.strat[k][cell] : 0), tot = w.reduce((x, y) => x + y, 0);
    if (tot > 0.02){ let u = Math.random() * tot, k = 0; while (k < w.length - 1 && (u -= w[k]) > 0) k++; const a = node.actions[k];
      return { kind: a.kind, label: a.label, amt: a.kind === "call" ? Math.min(cap, H.maxCommit) : a.kind === "check" || a.kind === "fold" ? H.commit[p] : Math.min(cap, a.amt), solved: true }; }
  }
  return policyAct(H, p);
}
function arenaApply(p, a){
  const H = AR.hand, prev = H.commit[p];
  if (a.kind === "fold"){ H.alive = H.alive.filter(x => x !== p); }
  else { const to = Math.min(a.amt, AR.stacks[p] + prev); AR.stacks[p] -= to - prev; H.commit[p] = to; if (to > H.maxCommit) H.maxCommit = to; if (AR.stacks[p] <= 0.001) H.allin.push(p);
    if (H.street < 0){ if (a.kind === "open" || a.kind === "raise"){ H.raises.push({ seat: p, to }); H.callers = []; } else if (a.kind === "call") H.callers.push(p); } }
  const n = H.raises.length, label = a.kind === "open" ? bbf(H.commit[p]) : a.kind === "raise" && H.street < 0 ? `${n === 2 ? "3-bet" : n === 3 ? "4-bet" : "raise"} ${bbf(H.commit[p])}` : a.kind === "call" ? `call${H.street < 0 ? "" : " " + bbf(H.commit[p] - prev)}` : a.kind === "bet" ? `bet ${bbf(H.commit[p])}` : a.kind === "raise" ? `raise ${bbf(H.commit[p])}` : a.kind;
  H.log.push({ street: H.street, pos: p, label, kind: a.kind === "open" ? "raise" : a.kind, allin: H.allin.includes(p) });
  if (H.street >= 0) H.post.push({ street: H.street, pos: p, label: a.kind === "bet" || a.kind === "raise" ? a.label : a.kind === "call" ? "Call" : a.kind === "check" ? "Check" : "Fold" });
  arenaRender();
}

/* ---------- hero ---------- */
function arenaButtons(){
  const H = AR.hand, p = H.hero, cell = cellOf(H.cards[p]), toCall = H.maxCommit - H.commit[p], stack = AR.stacks[p], cap = stack + H.commit[p], box = $(`<div class="actions" id="actions"></div>`), r1 = x => Math.round(x * 10) / 10;
  let opts = [], grade = null;
  if (H.street < 0){
    const pol = prePolicy(H, p, cell), to = preRaiseTo(H, p), fold = Math.max(0, 1 - pol.raise - pol.call), n = H.raises.length;
    if (toCall > 0) opts.push({ kind: "fold", label: "Fold", amt: H.commit[p], freq: fold });
    opts.push(toCall > 0 ? { kind: "call", label: `Call ${bbf(Math.min(toCall, stack))}`, amt: Math.min(H.maxCommit, cap), freq: pol.call } : { kind: "check", label: "Check", amt: H.commit[p], freq: pol.call });
    if (toCall < stack) opts.push({ kind: n ? "raise" : "open", label: to >= cap ? `All-in ${bbf(stack)}` : `${n === 0 ? "Raise to" : n === 1 ? "3-bet to" : n === 2 ? "4-bet to" : "Raise to"} ${bbf(to)}`, amt: to, freq: pol.raise });
    grade = { kind: "chart", concept: `Preflop: ${pol.label}` };
  } else {
    const node = solvedNode(H), pot = H.pot + Object.values(H.commit).reduce((a, b) => a + b, 0);
    if (node && node.actor === p && node.actions.some((a, k) => node.strat[k] && node.strat[k][cell] > 0)){
      const tot = node.actions.reduce((t, a, k) => t + (node.strat[k] ? node.strat[k][cell] : 0), 0);
      opts = node.actions.map((a, k) => ({ kind: a.kind, label: a.kind === "call" ? `Call ${bbf(toCall)}` : a.kind === "bet" ? `Bet ${bbf(a.amt)} (${a.label.replace(/\D/g, "")}%)` : a.kind === "raise" ? `Raise to ${bbf(a.amt)}` : a.label, amt: a.kind === "call" ? Math.min(cap, H.maxCommit) : a.kind === "check" || a.kind === "fold" ? H.commit[p] : Math.min(cap, a.amt), raw: a.label, freq: (node.strat[k] ? node.strat[k][cell] : 0) / tot }));
      grade = { kind: "solved", node };
    } else {
      if (toCall > 0) opts.push({ kind: "fold", label: "Fold", amt: H.commit[p] });
      opts.push(toCall > 0 ? { kind: "call", label: `Call ${bbf(Math.min(toCall, stack))}`, amt: Math.min(H.maxCommit, cap) } : { kind: "check", label: "Check", amt: H.commit[p] });
      if (stack > toCall){
        if (toCall > 0){ const to = Math.min(cap, r1(H.maxCommit * 2.6 + pot * 0.5)); opts.push({ kind: "raise", label: `Raise to ${bbf(to)}`, amt: to, raw: `Raise to ${to}` }); }
        else [33, 66, 100].forEach(pc => { const amt = Math.min(cap, r1(pot * pc / 100)); if (amt < cap) opts.push({ kind: "bet", label: `Bet ${bbf(amt)} (${pc}%)`, amt, raw: `Bet ${pc}%` }); });
        if (!opts.some(o => o.amt >= cap)) opts.push({ kind: toCall > 0 ? "raise" : "bet", label: `All-in ${bbf(stack)}`, amt: cap, raw: toCall > 0 ? `Raise to ${cap}` : "Bet allin" });
      }
    }
  }
  opts.forEach(o => {
    const kinds = opts.filter(x => x.kind === o.kind), bi = kinds.indexOf(o), color = o.kind === "open" ? kindColor("bet", 0) : kindColor(o.kind, bi);
    const b = $(`<button class="act" style="background:${color}"><div class="fill"></div><span>${o.label}</span><span class="sub"></span></button>`);
    b.onclick = () => arenaHeroAct(o, opts, grade, box); box.appendChild(b);
  });
  main.appendChild(box);
}
function arenaHeroAct(o, opts, grade, box){
  const H = AR.hand; if (H.done || !H.resolve) return;
  const a = { kind: o.kind, label: o.raw || o.label, amt: o.amt };
  let verdict = null;
  if (grade && grade.kind === "chart") verdict = arenaGradeFreq(o, opts, grade.concept, "Preflop", null);
  else if (grade && grade.kind === "solved"){
    const s = grade.node.spot, hh = H.cards[H.hero], d = s && s.drills.find(x => (x.hand[0] === hh[0] && x.hand[1] === hh[1]) || (x.hand[0] === hh[1] && x.hand[1] === hh[0]));
    if (s && d){ const i = opts.indexOf(o), gr = window.grade(s, d, i); track(H.f, s, d, gr); trackErr(H.f, s, d, i, gr); logDecision(H.f, s, d, i, gr); AR.graded++; if (gr.g === "Best" || gr.g === "Good") AR.ok++;
      verdict = { g: gr.g, text: `${gr.g} · ${gr.loss < 0.005 ? "no EV lost" : "−" + gr.loss.toFixed(2) + "bb"} · solver: ${opts.map((x, k) => `${x.label.split(" (")[0].toLowerCase()} ${pct(d.strat[k])}`).join(", ")}` }; }
    else verdict = arenaGradeFreq(o, opts, s ? concept(H.f, s) : `${STREET_NAMES[H.street]} decision`, STREET_NAMES[H.street], s);
  }
  [...box.querySelectorAll(".act")].forEach((b, k) => { b.disabled = true; if (opts[k].freq != null){ b.querySelector(".sub").textContent = pct(opts[k].freq); b.querySelector(".fill").style.width = (100 * opts[k].freq) + "%"; } if (opts[k] === o) b.style.outline = "2px solid #fff"; });
  if (verdict){ H.res.push(verdict); box.insertAdjacentHTML("afterend", `<div class="toast" style="color:${verdict.g === "Best" || verdict.g === "Good" ? "var(--accent)" : verdict.g === "Mistake" ? "var(--bad)" : "var(--warn)"}">${verdict.text}</div>`); }
  const res = H.resolve; H.resolve = null;
  setTimeout(() => { if (AR.hand === H) arenaApply(H.hero, a); res(a); }, verdict ? 1000 : 150);   // always resolve, so an abandoned hand's run can exit and free AR.busy
}
// frequency grading when the pack has the node but not the exact hand (or the chart preflop): filed with a flat loss estimate
function arenaGradeFreq(o, opts, conceptName, streetName, s){
  const f = o.freq; if (f == null) return null;
  const g = f >= 0.25 ? "Best" : f >= 0.05 ? "Mixed" : "Mistake", p = g === "Best" ? 0 : g === "Mixed" ? 1 : 4, cls = s ? arenaClass() : null;
  const book = store.get("book", {}), keys = ["c:" + conceptName, "s:" + streetName]; if (cls) keys.push("h:" + cls);
  keys.forEach(k => { const e = book[k] || (book[k] = { n: 0, p: 0, bb: 0, ok: 0 }); e.n++; e.p += p; if (g === "Best") e.ok++; }); store.set("book", book);
  const log = store.get("log", []); log.push({ t: Date.now(), k: AR.hand.f ? AR.hand.f._k : null, sid: null, hand: AR.hand.cards[AR.hand.hero].join(""), c: conceptName, st: AR.hand.street, cls, ft: null, ctx: null, err: g === "Mistake" ? "off the chart" : null, p, bb: null, role: null, mode: "arena", graded: "freq" }); if (log.length > 3000) log.splice(0, log.length - 3000); store.set("log", log);
  AR.graded++; if (g === "Best") AR.ok++;
  const best = opts.reduce((b, x) => (x.freq || 0) > (b.freq || 0) ? x : b, opts[0]), nm = x => x.label.split(" (")[0].toLowerCase();
  return { g, text: g === "Best" ? `Fine · solver ${pct(f)} here` : g === "Mixed" ? `Rare · solver ${pct(f)}, mostly ${nm(best)} ${pct(best.freq)}` : `Off the chart · solver ${nm(best)} ${pct(best.freq)}` };
}
function arenaClass(){ const H = AR.hand, ft = fineType(H.cards[H.hero], H.board), c = coarseType(ft); return { "Top pair": "Top pair / overpair", "Overpair": "Top pair / overpair", "Weak pair": "Middle / weak pair", "Two pair+": "Two pair+", "Flush draw": "Strong draw", "Straight draw": "Gutshot / overcards", "Air": ft === "Ace high" ? "Ace high" : "Air" }[c] || null; }

/* ---------- showdown ---------- */
function rank7(cards){
  const rk = cards.map(c => 12 - R.indexOf(c[0])), su = {}; cards.forEach(c => (su[c[1]] = su[c[1]] || []).push(12 - R.indexOf(c[0])));
  const cnt = {}; rk.forEach(r => cnt[r] = (cnt[r] || 0) + 1);
  const straightHigh = rs => { const u = [...new Set(rs)].sort((a, b) => b - a); if (u.includes(12)) u.push(-1); for (let i = 0; i + 4 < u.length; i++) if (u[i] - u[i + 4] === 4) return u[i]; return -1; };
  const fl = Object.values(su).find(v => v.length >= 5);
  if (fl){ const sh = straightHigh(fl); if (sh >= 0) return [8, sh]; return [5, ...fl.sort((a, b) => b - a).slice(0, 5)]; }
  const by = Object.keys(cnt).map(Number).sort((a, b) => cnt[b] - cnt[a] || b - a), c0 = cnt[by[0]], c1 = by[1] != null ? cnt[by[1]] : 0;
  if (c0 === 4) return [7, by[0], Math.max(...by.filter(r => r !== by[0]))];
  if (c0 === 3 && c1 >= 2) return [6, by[0], by[1]];
  const sh = straightHigh(rk); if (sh >= 0) return [4, sh];
  if (c0 === 3) return [3, by[0], ...by.slice(1, 3)];
  if (c0 === 2 && c1 === 2) return [2, by[0], by[1], Math.max(...by.filter(r => r !== by[0] && r !== by[1]))];
  if (c0 === 2) return [1, by[0], ...by.slice(1, 4)];
  return [0, ...by.slice(0, 5)];
}
const cmpRank = (a, b) => { for (let i = 0; i < Math.max(a.length, b.length); i++){ const d = (a[i] || 0) - (b[i] || 0); if (d) return d; } return 0; };
const RANK_NAMES = ["high card", "a pair", "two pair", "three of a kind", "a straight", "a flush", "a full house", "quads", "a straight flush"];
function arenaFinish(){
  const H = AR.hand; H.done = true; SEATS.forEach(p => { H.pot += H.commit[p]; H.commit[p] = 0; });
  let winners = H.alive, note = "";
  if (H.alive.length > 1){
    while (H.board.length < 5) H.board.push(H.deck.pop());   // run it out when everyone is all-in
    const ranks = {}; H.alive.forEach(p => ranks[p] = rank7([...H.cards[p], ...H.board]));
    winners = H.alive.filter(p => H.alive.every(q => cmpRank(ranks[p], ranks[q]) >= 0));
    note = `${winners.join(" & ")} ${winners.length > 1 ? "split" : "wins"} with ${RANK_NAMES[ranks[winners[0]][0]]}`;
  } else note = `${winners[0]} takes it`;
  const share = H.pot / winners.length; winners.forEach(p => AR.stacks[p] += share);
  const net = AR.stacks[H.hero] - 100; AR.net += net; AR.n++; H.net = net; H.note = note; H.showdown = H.alive.length > 1;
  arenaRender(); renderStats();
}

/* ---------- render ---------- */
function arenaRender(acting){
  const H = AR.hand; if (!H) return;
  const pill = (pos, txt, kind) => `<span class="pl ${kind} ${pos === H.hero ? "me" : ""}"><b>${pos}</b>${txt}</span>`;
  const rows = []; H.log.forEach(h => { if (!rows.length || rows[rows.length - 1][0] !== h.street) rows.push([h.street, []]); rows[rows.length - 1][1].push(pill(h.pos, h.label + (h.allin ? " all-in" : ""), h.kind === "fold" ? "folded" : h.kind === "check" ? "chk" : h.kind)); });
  const hist = rows.map(([st, ps]) => `<div class="hrow ${st === H.street && !H.done ? "now" : "past"}"><span class="st">${st < 0 ? "Pre" : STREETS[st]}</span><div class="pls">${ps.join('<span class="arr">›</span>')}</div></div>`).join("");
  const potAll = H.pot + Object.values(H.commit).reduce((a, b) => a + b, 0), toCall = Math.max(0, H.maxCommit - H.commit[H.hero]);
  const seat = p => { const last = [...H.log].reverse().find(h => h.pos === p && h.street === H.street), dead = !H.alive.includes(p), cls = dead ? "folded" : last && (last.kind === "raise" || last.kind === "bet") ? "raise" : last && last.kind === "call" ? "call" : "";
    return `<div class="seat ${cls} ${acting === p ? "act-now" : ""}" data-p="${p}">${p === H.hero ? "<em>YOU</em>" : ""}${p}<small>${bbf(AR.stacks[p])}</small>${H.commit[p] && !H.done ? `<i class="bet">${bbf(H.commit[p]).replace("bb", "")}</i>` : ""}</div>`; };
  const villains = H.done && H.showdown ? H.alive.filter(p => p !== H.hero) : [];
  const el = $(`<div class="table">
    <div class="meta"><span>${H.done ? H.note : H.f ? "solved flop" : H.street >= 0 ? "unsolved board · bots on policy" : "6-max · 100bb"}</span><span>${H.done ? (H.net >= 0 ? "+" : "") + bbf(H.net) : ""}</span></div>
    <div class="seats arena">${SEATS.map(seat).join("")}</div>
    <div class="hist">${hist}</div>
    <div class="board">${H.board.map(c => cardEl(c)).join("")}</div>
    <div class="pot">Pot <b>${bbf(potAll)}</b>${toCall > 0 && !H.done && acting === H.hero ? ` · <b>${bbf(toCall)}</b> to call` : ""}</div>
    <div class="hero"><div class="who">You<b>${H.hero}</b></div>${H.cards[H.hero].map(c => cardEl(c)).join("")}${villains.map(p => `<div class="who" style="margin-left:6px"><b>${p}</b></div>${H.cards[p].map(c => cardEl(c)).join("")}`).join("")}</div>
  </div>`);
  main.innerHTML = ""; main.appendChild(el);
  if (H.done){
    const col = r => r.g === "Best" || r.g === "Good" ? "var(--accent)" : r.g === "Mistake" ? "var(--bad)" : "var(--warn)";
    main.appendChild($(`<div class="sheet"><h3>Session</h3><div class="kpis"><div class="kpi"><b style="color:${AR.net >= 0 ? "var(--accent)" : "var(--bad)"}">${(AR.net >= 0 ? "+" : "") + bbf(AR.net)}</b><span>net · ${AR.n} hands</span></div><div class="kpi"><b>${AR.n ? (100 * AR.net / AR.n).toFixed(0) : 0}</b><span>bb / 100</span></div><div class="kpi"><b>${AR.graded ? pct(AR.ok / AR.graded) : "–"}</b><span>on chart · ${AR.graded} graded</span></div></div>${H.res.length ? `<div class="bars">${H.res.map(r => `<div style="font-size:12px;color:${col(r)}">${r.text}</div>`).join("")}</div>` : `<p style="color:var(--dim);font-size:12px;margin:0">No graded decision this hand.</p>`}</div>`));
    const nx = $(`<button class="next">Next hand →</button>`); nx.onclick = () => { if (!AR.busy) arenaDeal(); }; main.appendChild(nx);
  }
}
