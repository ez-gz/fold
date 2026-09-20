//! Drill-spot exporter: hero node + one ply of villain responses per hero action, with the
//! deterministic facts the rationale layer needs. Output is JSON (prototype pack format).

use crate::cards::{card_str, grid_cell};
use crate::cfr::{avg_strategy, walk, Ctx, Mode};
use crate::classify::{classify, CLASS_NAMES};
use crate::eval::eval;
use crate::tree::{Act, ActionNode, Node};
use crate::Solved;
use rayon::prelude::*;
use serde::Serialize;


#[derive(Serialize, Clone)]
struct Hist { street: u8, pos: &'static str, label: String }

#[derive(Serialize)]
struct ActOut { label: String, kind: &'static str, amt: f32 }

#[derive(Serialize)]
struct Resp { labels: Vec<String>, kinds: Vec<&'static str>, freq: Vec<f32>, grid: Vec<Vec<f32>>, cls: Vec<Vec<f32>>,
    /// per villain action, percent (0-100) for every combo listed in the spot's `vdetail`, same order
    combo: Vec<Vec<u8>> }

#[derive(Serialize)]
struct Drill { hand: [String; 2], cls: u8, w: f32, strat: Vec<f32>, ev: Vec<f32>, eq: f32, top: f32, eq_cont: Vec<Option<f32>>, src: Vec<Vec<f32>>,
    /// per hero action that villain answers: share of villain's range that [folds & was ahead, folds & was behind,
    /// continues & is ahead, continues & is behind]; "ahead" = more than 50% equity against this exact hand
    vs: Vec<Option<[f32; 4]>> }

#[derive(Serialize)]
struct Spot {
    id: String, board: Vec<String>, street: u8, hero: &'static str, villain: &'static str,
    pot: f32, stack: f32, to_call: f32, line_p: f32, history: Vec<Hist>, actions: Vec<ActOut>,
    vr: Vec<f32>, hr: Vec<f32>, hs: Vec<Vec<f32>>, range_freq: Vec<f32>,
    vclass: Vec<f32>, hclass: Vec<f32>, hclass_strat: Vec<Vec<f32>>,
    resp: Vec<Option<Resp>>, drills: Vec<Drill>,
    /// villain's range split by the board's flush suit (boards with 2+ of one suit): cells for combos that hold
    /// the suit (both cards if suited, at least one otherwise) and for the rest
    #[serde(skip_serializing_if = "Option::is_none")] vsuit: Option<SuitSplit>,
    /// villain's exact combos when the range is narrow enough to name them (weight relative to the fullest combo)
    #[serde(skip_serializing_if = "Option::is_none")] vcombos: Option<Vec<(String, f32)>>,
    /// every listed villain combo with its weight (percent of the fullest combo); `resp[..].combo` lines up with it
    vdetail: Vec<(String, u8)>,
    /// every listed hero combo: [weight, then percent per action]
    hdetail: Vec<(String, Vec<u8>)>,
}

#[derive(Serialize)]
struct SuitSplit { suit: char, with: Vec<f32>, without: Vec<f32> }

#[derive(Serialize)]
struct Step { spot: Spot, line: usize }

#[derive(Serialize)]
struct Hand { hero: &'static str, villain: &'static str, hero_hand: [String; 2], villain_hand: [String; 2], steps: Vec<Step>, tail: Vec<Hist>, board: Vec<String>, end: &'static str, result: f32, villain_top: f32, net: f32 }

/// One fully played line: both players hold real hands, villain samples the solver strategy,
/// hero's scripted action is the solver's most frequent one. The trainer grades each hero step
/// against the full per-action EVs and then continues down this line.
fn play_hand(s: &Solved, idx: usize, rng: &mut Rng) -> Option<Hand> {
    let ctx = &s.ctx;
    let hp = (rng.f() < 0.5) as usize;
    let vp = 1 - hp;
    let h = *rng.pick(&ctx.weights[hp], 1).first()?;
    let hh = ctx.hands[hp][h];
    let vw: Vec<f32> = ctx.hands[vp].iter().zip(&ctx.weights[vp]).map(|(v, w)| if v.0 == hh.0 || v.0 == hh.1 || v.1 == hh.0 || v.1 == hh.1 { 0.0 } else { *w }).collect();
    let v = *rng.pick(&vw, 1).first()?;
    let vh = ctx.hands[vp][v];
    let idx_of = [if hp == 0 { h } else { v }, if hp == 0 { v } else { h }];
    let mut node = &s.root;
    let mut board = s.flop.clone();
    let mut reach = [ctx.weights[0].clone(), ctx.weights[1].clone()];
    let mut hist: Vec<Hist> = Vec::new();
    let mut steps: Vec<Step> = Vec::new();
    let mut v_before = reach.clone();
    let end;
    let fin: [f32; 2];
    loop {
        match node {
            Node::Action(a) => {
                let p = a.player as usize;
                let n = ctx.hands[p].len();
                let strat = avg_strategy(a, n);
                let mine: Vec<f32> = (0..a.actions.len()).map(|x| strat[x * n + idx_of[p]]).collect();
                let pickd = if p == hp {
                    (0..mine.len()).max_by(|x, y| mine[*x].partial_cmp(&mine[*y]).unwrap()).unwrap()
                } else {
                    *rng.pick(&mine, 1).first()?
                };
                if p == hp && a.actions.len() > 1 {
                    let c = Cand { node: a, board: board.clone(), reach: reach.clone(), hist: hist.clone(), p: 1.0 };
                    let spot = build_spot(s, &c, 1000 + idx * 10 + steps.len(), rng, Some(h))?;
                    steps.push(Step { spot, line: pickd });
                }
                if p == vp { v_before = reach.clone(); }
                for k in 0..n { reach[p][k] *= strat[pickd * n + k]; }
                hist.push(Hist { street: a.street, pos: s.form.pos[p], label: act_label(&a.actions[pickd]).0 });
                node = &a.children[pickd];
            }
            Node::Chance { cards, children, .. } => {
                let w: Vec<f32> = cards.iter().map(|c| if [hh.0, hh.1, vh.0, vh.1].contains(c) { 0.0 } else { 1.0 }).collect();
                let i = *rng.pick(&w, 1).first()?;
                for p in 0..2 { for &k in &ctx.card_hands[p][cards[i] as usize] { reach[p][k as usize] = 0.0; } }
                board.push(cards[i]);
                node = &children[i];
            }
            Node::Fold { folder, commit } => { fin = *commit; end = if *folder as usize == hp { "hero_fold" } else { "villain_fold" }; break; }
            Node::Showdown { commit, .. } => { fin = [*commit, *commit]; end = "showdown"; break; }
        }
    }
    if steps.is_empty() { return None; }
    let result = if end == "showdown" {
        let ev = |x: (u8, u8)| eval(&[board[0], board[1], board[2], board[3], board[4], x.0, x.1]);
        let (a, b) = (ev(hh), ev(vh));
        if a > b { 1.0 } else if a == b { 0.5 } else { 0.0 }
    } else if end == "villain_fold" { 1.0 } else { 0.0 };
    let vr = if end == "villain_fold" { &v_before } else { &reach };
    let villain_top = range_rank(s, &board, vp, v, &vr[vp], &vr[hp]);
    let last = steps.last().unwrap().spot.history.len() + 1;
    Some(Hand {
        hero: s.form.pos[hp], villain: s.form.pos[vp],
        hero_hand: [card_str(hh.1), card_str(hh.0)], villain_hand: [card_str(vh.1), card_str(vh.0)],
        tail: hist[last.min(hist.len())..].to_vec(), steps, board: board.iter().map(|c| card_str(*c)).collect(), end, result, villain_top,
        // bb won or lost from the flop on, the preflop pot counted as dead money
        net: r3(result * (s.cfg.start_pot + fin[vp] + fin[hp]) - fin[hp]),
    })
}

/// Share of player `p`'s range (weights `wp`) that has more equity than hand `h` against `wo`.
fn range_rank(s: &Solved, board: &[u8], p: usize, h: usize, wp: &[f32], wo: &[f32]) -> f32 {
    let eq = equity_all(s, board, p, wp, wo);
    let (mut above, mut all) = (0f32, 0f32);
    for g in 0..eq.len() { if eq[g] < 0.0 { continue; } all += wp[g]; if eq[g] > eq[h] { above += wp[g]; } else if eq[g] == eq[h] { above += 0.5 * wp[g]; } }
    r3(above / all.max(1e-9))
}

/// Equity of every hand of player `p` (with weight > 0) against the opponent weights `wo`; -1 where undefined.
pub fn equity_all(s: &Solved, board: &[u8], p: usize, wp: &[f32], wo: &[f32]) -> Vec<f32> {
    let ctx = &s.ctx;
    let deck: Vec<u8> = (0..52u8).filter(|x| !board.contains(x)).collect();
    let mut runouts: Vec<Vec<u8>> = Vec::new();
    match board.len() {
        3 => for i in 0..deck.len() { for j in i + 1..deck.len() { runouts.push([board, &[deck[i], deck[j]]].concat()); } },
        4 => for &d in &deck { runouts.push([board, &[d]].concat()); },
        _ => runouts.push(board.to_vec()),
    }
    let st = |q: usize| -> Vec<Vec<u32>> { runouts.par_iter().map(|b| ctx.hands[q].iter().map(|x| if b.contains(&x.0) || b.contains(&x.1) { 0 } else { eval(&[b[0], b[1], b[2], b[3], b[4], x.0, x.1]) }).collect()).collect() };
    let (sp, so) = (st(p), st(1 - p));
    let eq: Vec<f32> = (0..ctx.hands[p].len()).into_par_iter().map(|g| {
        if wp[g] <= 0.0 { return -1.0; }
        let me = ctx.hands[p][g];
        let (mut num, mut den) = (0f64, 0f64);
        for ri in 0..runouts.len() {
            let a = sp[ri][g]; if a == 0 { continue; }
            for (oi, oh) in ctx.hands[1 - p].iter().enumerate() {
                let (b, w) = (so[ri][oi], wo[oi]);
                if b == 0 || w == 0.0 || oh.0 == me.0 || oh.0 == me.1 || oh.1 == me.0 || oh.1 == me.1 { continue; }
                den += w as f64; num += w as f64 * if a > b { 1.0 } else if a == b { 0.5 } else { 0.0 };
            }
        }
        if den > 0.0 { (num / den) as f32 } else { -1.0 }
    }).collect();
    eq
}

#[derive(Serialize)]
struct FlopFile {
    flop: String, formation: &'static str, pos: [&'static str; 2], opener: &'static str, caller: &'static str, open_size: f32, three_bet: f32, hands: Vec<Hand>, rake: &'static str, exploitability_pct_pot: f32, iterations: u32,
    start_pot: f32, eff_stack: f32, tree: String, ranges: &'static str, class_names: Vec<&'static str>, spots: Vec<Spot>, briefing: Briefing,
}

/// Flop-level summary shown before the first decision: whose board this is and how each range starts.
#[derive(Serialize)]
struct Briefing {
    /// per player [OOP, IP]: range equity, share of range with 75%+ equity, share under 35%, class shares
    equity: [f32; 2], strong: [f32; 2], weak: [f32; 2], classes: [Vec<f32>; 2],
    /// OOP's first action, then IP's action after a check: labels and range frequencies
    first: Vec<(String, f32)>, after_check: Vec<(String, f32)>,
}

pub fn briefing_json(s: &Solved) -> serde_json::Value { serde_json::to_value(briefing(s)).unwrap() }

fn briefing(s: &Solved) -> Briefing {
    let ctx = &s.ctx;
    let w = [&ctx.weights[0], &ctx.weights[1]];
    let (mut equity, mut strong, mut weak) = ([0f32; 2], [0f32; 2], [0f32; 2]);
    let mut classes: [Vec<f32>; 2] = [vec![], vec![]];
    for p in 0..2 {
        let eq = equity_all(s, &s.flop, p, w[p], w[1 - p]);
        let mut tot = 0f32;
        for g in 0..eq.len() { if eq[g] < 0.0 { continue; } tot += w[p][g]; equity[p] += w[p][g] * eq[g]; if eq[g] >= 0.75 { strong[p] += w[p][g]; } if eq[g] < 0.35 { weak[p] += w[p][g]; } }
        equity[p] = r3(equity[p] / tot); strong[p] = r3(strong[p] / tot); weak[p] = r3(weak[p] / tot);
        let cls: Vec<u8> = ctx.hands[p].iter().map(|h| classify(*h, &s.flop)).collect();
        classes[p] = class_share(&cls, w[p], w[p].iter().sum());
    }
    let freq = |a: &ActionNode, reach: &[f32]| -> Vec<(String, f32)> {
        let n = ctx.hands[a.player as usize].len(); let st = avg_strategy(a, n); let tot: f32 = reach.iter().sum();
        (0..a.actions.len()).map(|x| (act_label(&a.actions[x]).0, r3((0..n).map(|h| reach[h] * st[x * n + h]).sum::<f32>() / tot))).collect()
    };
    let (mut first, mut after_check) = (vec![], vec![]);
    if let Node::Action(a) = &s.root {
        first = freq(a, w[a.player as usize]);
        if let Some(ci) = a.actions.iter().position(|x| matches!(x, Act::Check)) {
            if let Node::Action(b) = &a.children[ci] { after_check = freq(b, w[b.player as usize]); }
        }
    }
    Briefing { equity, strong, weak, classes, first, after_check }
}

struct Rng(u64);
impl Rng {
    fn f(&mut self) -> f32 {
        self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17;
        ((self.0 >> 40) as f32 + 0.5) / (1u64 << 24) as f32
    }
    /// weighted sample without replacement (Efraimidis-Spirakis)
    fn pick(&mut self, w: &[f32], k: usize) -> Vec<usize> {
        let mut keys: Vec<(f32, usize)> = w.iter().enumerate().filter(|(_, w)| **w > 0.0)
            .map(|(i, w)| (self.f().ln() / w, i)).collect();
        keys.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        keys.into_iter().take(k).map(|x| x.1).collect()
    }
}

struct Cand<'a> { node: &'a ActionNode, board: Vec<u8>, reach: [Vec<f32>; 2], hist: Vec<Hist>, p: f32 }

fn act_label(a: &Act) -> (String, &'static str, f32) {
    match a {
        Act::Fold => ("Fold".into(), "fold", 0.0),
        Act::Check => ("Check".into(), "check", 0.0),
        Act::Call => ("Call".into(), "call", 0.0),
        Act::Bet(amt, f) => (format!("Bet {:.0}%", f * 100.0), "bet", *amt),
        Act::Raise(amt, to) => (format!("Raise to {:.1}", to), "raise", *amt),
    }
}

fn mass(ctx: &Ctx, s: &Solved, reach: &[Vec<f32>; 2]) -> f32 {
    let m = ctx.valid_mass(0, &reach[1]);
    let z: f32 = m.iter().zip(&reach[0]).map(|(a, b)| a * b).sum();
    let m0 = ctx.valid_mass(0, &ctx.weights[1]);
    let z0: f32 = m0.iter().zip(&ctx.weights[0]).map(|(a, b)| a * b).sum();
    let _ = s;
    z / z0
}

fn collect<'a>(s: &'a Solved, node: &'a Node, board: &mut Vec<u8>, reach: [Vec<f32>; 2], hist: &mut Vec<Hist>,
               chance_p: f32, rng: &mut Rng, out: &mut Vec<Cand<'a>>) {
    let ctx = &s.ctx;
    match node {
        Node::Action(a) => {
            let p = a.player as usize;
            let line_p = mass(ctx, s, &reach);
            // rarer lines are kept too: that is where ranges get narrow enough to name combo by combo
            if line_p < 0.004 { return; }
            out.push(Cand { node: a, board: board.clone(), reach: reach.clone(), hist: hist.clone(), p: line_p * chance_p });
            let n = ctx.hands[p].len();
            let strat = avg_strategy(a, n);
            for (ai, ch) in a.children.iter().enumerate() {
                let mut r = reach.clone();
                for h in 0..n { r[p][h] *= strat[ai * n + h]; }
                hist.push(Hist { street: a.street, pos: s.form.pos[p], label: act_label(&a.actions[ai]).0 });
                collect(s, ch, board, r, hist, chance_p, rng, out);
                hist.pop();
            }
        }
        Node::Chance { cards, children, .. } => {
            if board.len() >= 5 { return; }
            let k = if board.len() == 3 { 3 } else { 2 };
            let w: Vec<f32> = cards.iter().map(|_| 1.0).collect();
            for i in rng.pick(&w, k) {
                if !matches!(children[i], Node::Action(_)) { continue; }
                let c = cards[i];
                let mut r = reach.clone();
                for p in 0..2 { for &hi in &ctx.card_hands[p][c as usize] { r[p][hi as usize] = 0.0; } }
                board.push(c);
                collect(s, &children[i], board, r, hist, chance_p / k as f32, rng, out);
                board.pop();
            }
        }
        _ => {}
    }
}

fn r3(x: f32) -> f32 { (x * 1000.0).round() / 1000.0 }

/// Aggregate per-hand values into 169 chart cells, weighted by `w`, normalised by `den` per cell.
fn grid(hands: &[(u8, u8)], w: &[f32], den: &[f32]) -> Vec<f32> {
    let mut g = vec![0f32; 169];
    for (i, h) in hands.iter().enumerate() { g[grid_cell(*h)] += w[i]; }
    g.iter().zip(den).map(|(x, d)| if *d > 0.0 { r3(x / d) } else { 0.0 }).collect()
}

fn class_share(cls: &[u8], w: &[f32], total: f32) -> Vec<f32> {
    let mut v = vec![0f32; 7];
    for (i, c) in cls.iter().enumerate() { v[*c as usize] += w[i]; }
    v.iter().map(|x| r3(x / total.max(1e-9))).collect()
}

fn build_spot(s: &Solved, c: &Cand, idx: usize, rng: &mut Rng, forced: Option<usize>) -> Option<Spot> {
    let ctx = &s.ctx;
    let a = c.node;
    let (hp, vp) = (a.player as usize, 1 - a.player as usize);
    let (nh, nv, na) = (ctx.hands[hp].len(), ctx.hands[vp].len(), a.actions.len());
    let strat = avg_strategy(a, nh);
    let (hreach, vreach) = (&c.reach[hp], &c.reach[vp]);
    let vtotal: f32 = vreach.iter().sum();
    let htotal: f32 = hreach.iter().sum();

    // per-action EV for every hero hand (chips, fold = 0)
    let vm = ctx.valid_mass(hp, vreach);
    let evs: Vec<Vec<f32>> = a.children.iter().map(|ch| {
        let cfv = walk(ctx, ch, hp, vreach, Mode::Average);
        (0..nh).map(|h| if vm[h] > 1e-6 { cfv[h] / vm[h] + a.commit[hp] } else { 0.0 }).collect()
    }).collect();

    // EV is linear in villain's reach, so masking the reach to one hand class gives the exact share of
    // each action's EV that comes from that class: src[class][action][hero hand], summing to the EV.
    let vcls_pre: Vec<u8> = ctx.hands[vp].iter().map(|h| classify(*h, &c.board)).collect();
    let jobs: Vec<(usize, usize)> = (0..7).flat_map(|k| (0..na).map(move |x| (k, x))).collect();
    let parts: Vec<Vec<f32>> = jobs.par_iter().map(|&(k, x)| {
        let masked: Vec<f32> = (0..nv).map(|v| if vcls_pre[v] as usize == k { vreach[v] } else { 0.0 }).collect();
        let vmk = ctx.valid_mass(hp, &masked);
        let cfv = walk(ctx, &a.children[x], hp, &masked, Mode::Average);
        (0..nh).map(|h| if vm[h] > 1e-6 { (cfv[h] + a.commit[hp] * vmk[h]) / vm[h] } else { 0.0 }).collect()
    }).collect();

    let avail = |hands: &[(u8, u8)]| -> Vec<f32> {
        let mut g = vec![0f32; 169];
        for r1 in 0..52u8 { for r2 in r1 + 1..52 {
            if c.board.contains(&r1) || c.board.contains(&r2) { continue; }
            g[grid_cell((r1, r2))] += 1.0;
        }}
        let _ = hands;
        g
    };
    let av = avail(&ctx.hands[vp]);
    let vcls: Vec<u8> = ctx.hands[vp].iter().map(|h| classify(*h, &c.board)).collect();
    let hcls: Vec<u8> = ctx.hands[hp].iter().map(|h| classify(*h, &c.board)).collect();

    // combos worth listing one by one (the cell popup): at least 2% of the fullest combo, not blocked by the board
    let listed = |hands: &[(u8, u8)], w: &[f32]| -> Vec<usize> {
        let mx = w.iter().cloned().fold(0f32, f32::max).max(1e-9);
        (0..hands.len()).filter(|&i| w[i] / mx >= 0.02 && !c.board.contains(&hands[i].0) && !c.board.contains(&hands[i].1)).collect()
    };
    let (vlist, hlist) = (listed(&ctx.hands[vp], vreach), listed(&ctx.hands[hp], hreach));
    let pc = |x: f32| (100.0 * x).round().clamp(0.0, 100.0) as u8;
    let vmx = vreach.iter().cloned().fold(0f32, f32::max).max(1e-9);
    let hmx = hreach.iter().cloned().fold(0f32, f32::max).max(1e-9);
    let vdetail: Vec<(String, u8)> = vlist.iter().map(|&i| (format!("{}{}", card_str(ctx.hands[vp][i].1), card_str(ctx.hands[vp][i].0)), pc(vreach[i] / vmx))).collect();
    let hdetail: Vec<(String, Vec<u8>)> = hlist.iter().map(|&i| (format!("{}{}", card_str(ctx.hands[hp][i].1), card_str(ctx.hands[hp][i].0)),
        std::iter::once(pc(hreach[i] / hmx)).chain((0..na).map(|x| pc(strat[x * nh + i]))).collect())).collect();
    // villain responses one ply down
    let mut resp = Vec::new();
    let mut cont_reach: Vec<Option<Vec<f32>>> = Vec::new();
    for ch in &a.children {
        match ch {
            Node::Action(v) if v.player as usize == vp => {
                let vs = avg_strategy(v, nv);
                let nva = v.actions.len();
                let vcell = { let mut g = vec![0f32; 169]; for (i, h) in ctx.hands[vp].iter().enumerate() { g[grid_cell(*h)] += vreach[i]; } g };
                let mut r = Resp { labels: vec![], kinds: vec![], freq: vec![], grid: vec![], cls: vec![], combo: vec![] };
                let mut cont = vec![0f32; nv];
                for x in 0..nva {
                    let w: Vec<f32> = (0..nv).map(|h| vreach[h] * vs[x * nv + h]).collect();
                    let (l, k, _) = act_label(&v.actions[x]);
                    if k != "fold" { for h in 0..nv { cont[h] += w[h]; } }
                    r.labels.push(l); r.kinds.push(k);
                    r.freq.push(r3(w.iter().sum::<f32>() / vtotal));
                    r.grid.push(grid(&ctx.hands[vp], &w, &vcell));
                    r.cls.push(class_share(&vcls, &w, vtotal));
                    r.combo.push(vlist.iter().map(|&h| (100.0 * vs[x * nv + h]).round() as u8).collect());
                }
                resp.push(Some(r));
                cont_reach.push(Some(cont));
            }
            _ => { resp.push(None); cont_reach.push(None); }
        }
    }

    // runout strength tables for equity
    let mut runouts: Vec<Vec<u8>> = Vec::new();
    let deck: Vec<u8> = (0..52u8).filter(|x| !c.board.contains(x)).collect();
    match c.board.len() {
        3 => for i in 0..deck.len() { for j in i + 1..deck.len() { runouts.push([&c.board[..], &[deck[i], deck[j]]].concat()); } },
        4 => for &d in &deck { runouts.push([&c.board[..], &[d]].concat()); },
        _ => runouts.push(c.board.clone()),
    }
    let strength = |p: usize| -> Vec<Vec<u32>> {
        runouts.par_iter().map(|b| ctx.hands[p].iter().map(|h| {
            if b.contains(&h.0) || b.contains(&h.1) { 0 } else { eval(&[b[0], b[1], b[2], b[3], b[4], h.0, h.1]) }
        }).collect()).collect()
    };
    let (sh, sv) = (strength(hp), strength(vp));
    let equity = |h: usize, w: &[f32]| -> f32 {
        let hand = ctx.hands[hp][h];
        let (mut num, mut den) = (0f64, 0f64);
        for (ri, _) in runouts.iter().enumerate() {
            let me = sh[ri][h];
            if me == 0 { continue; }
            for (vi, vh) in ctx.hands[vp].iter().enumerate() {
                let (x, wv) = (sv[ri][vi], w[vi]);
                if x == 0 || wv == 0.0 || vh.0 == hand.0 || vh.0 == hand.1 || vh.1 == hand.0 || vh.1 == hand.1 { continue; }
                den += wv as f64;
                num += wv as f64 * if me > x { 1.0 } else if me == x { 0.5 } else { 0.0 };
            }
        }
        if den > 0.0 { r3((num / den) as f32) } else { 0.0 }
    };

    // where each hand sits in hero's own range here: share of the range with more equity
    let heq: Vec<f32> = (0..nh).into_par_iter().map(|h| if hreach[h] > 0.0 && vm[h] > 1e-6 { equity(h, vreach) } else { -1.0 }).collect();
    let top_of = |h: usize| -> f32 {
        let (mut above, mut all) = (0f32, 0f32);
        for g in 0..nh {
            if heq[g] < 0.0 { continue; }
            all += hreach[g];
            if heq[g] > heq[h] { above += hreach[g]; } else if heq[g] == heq[h] { above += 0.5 * hreach[g]; }
        }
        r3(above / all.max(1e-9))
    };

    // drills: sample hero hands, favouring in-range hands with a real decision
    let spot_pot = s.cfg.start_pot + a.commit[0] + a.commit[1];
    let pick_w: Vec<f32> = (0..nh).map(|h| {
        if hreach[h] < 0.05 || vm[h] <= 1e-6 { return 0.0; }
        // favour hands where the options really differ in EV (pure mixes are ties by definition)
        let (lo, hi) = (0..na).fold((f32::MAX, f32::MIN), |m, x| (m.0.min(evs[x][h]), m.1.max(evs[x][h])));
        hreach[h] * (0.25 + ((hi - lo) / (0.08 * spot_pot)).min(1.0))
    }).collect();
    let mut drills = Vec::new();
    // stratify by hand class so every spot drills value hands, bluff-catchers, draws and air
    let mut picked: Vec<usize> = Vec::new();
    for k in 0..7u8 {
        let w: Vec<f32> = (0..nh).map(|h| if hcls[h] == k { pick_w[h] } else { 0.0 }).collect();
        picked.extend(rng.pick(&w, 2));
    }
    if let Some(h) = forced { picked = if vm[h] > 1e-6 { vec![h] } else { vec![] }; }
    for h in picked {
        let hand = ctx.hands[hp][h];
        // equity of every villain hand against this exact hero hand
        let ahead: Vec<bool> = (0..nv).into_par_iter().map(|vi| {
            let vh = ctx.hands[vp][vi];
            if vreach[vi] <= 0.0 || vh.0 == hand.0 || vh.0 == hand.1 || vh.1 == hand.0 || vh.1 == hand.1 { return false; }
            let (mut win, mut n) = (0f32, 0f32);
            for ri in 0..runouts.len() { let (me, x) = (sh[ri][h], sv[ri][vi]); if me == 0 || x == 0 { continue; } n += 1.0; win += if x > me { 1.0 } else if x == me { 0.5 } else { 0.0 }; }
            n > 0.0 && win / n > 0.5
        }).collect();
        let vs: Vec<Option<[f32; 4]>> = cont_reach.iter().map(|cr| cr.as_ref().map(|cont| {
            let (mut q, mut tot) = ([0f32; 4], 0f32);
            for vi in 0..nv {
                let vh = ctx.hands[vp][vi];
                if vreach[vi] <= 0.0 || vh.0 == hand.0 || vh.0 == hand.1 || vh.1 == hand.0 || vh.1 == hand.1 { continue; }
                let (c, f) = (cont[vi], (vreach[vi] - cont[vi]).max(0.0)); tot += vreach[vi];
                if ahead[vi] { q[0] += f; q[2] += c; } else { q[1] += f; q[3] += c; }
            }
            [r3(q[0] / tot.max(1e-9)), r3(q[1] / tot.max(1e-9)), r3(q[2] / tot.max(1e-9)), r3(q[3] / tot.max(1e-9))]
        })).collect();
        drills.push(Drill {
            hand: [card_str(hand.1), card_str(hand.0)], cls: hcls[h], w: r3(hreach[h]),
            strat: (0..na).map(|x| r3(strat[x * nh + h])).collect(),
            ev: (0..na).map(|x| r3(evs[x][h])).collect(),
            eq: heq[h].max(0.0), top: top_of(h),
            eq_cont: cont_reach.iter().map(|cr| cr.as_ref().map(|w| equity(h, w))).collect(),
            src: (0..na).map(|x| (0..7).map(|k| r3(parts[k * na + x][h])).collect()).collect(),
            vs,
        });
    }
    if drills.is_empty() { return None; }

    let hcell = { let mut g = vec![0f32; 169]; for (i, h) in ctx.hands[hp].iter().enumerate() { g[grid_cell(*h)] += hreach[i]; } g };
    let hs: Vec<Vec<f32>> = (0..na).map(|x| {
        let w: Vec<f32> = (0..nh).map(|h| hreach[h] * strat[x * nh + h]).collect();
        grid(&ctx.hands[hp], &w, &hcell)
    }).collect();
    let range_freq: Vec<f32> = (0..na).map(|x| r3((0..nh).map(|h| hreach[h] * strat[x * nh + h]).sum::<f32>() / htotal)).collect();
    let mut hclass_strat = vec![vec![0f32; na]; 7];
    let mut hclass_mass = vec![0f32; 7];
    for h in 0..nh {
        hclass_mass[hcls[h] as usize] += hreach[h];
        for x in 0..na { hclass_strat[hcls[h] as usize][x] += hreach[h] * strat[x * nh + h]; }
    }
    for k in 0..7 { for x in 0..na { hclass_strat[k][x] = r3(hclass_strat[k][x] / hclass_mass[k].max(1e-9)); } }

    let pot = s.cfg.start_pot + a.commit[0] + a.commit[1];
    let vsuit = {
        let mut n = [0u8; 4]; for c in &c.board { n[(*c % 4) as usize] += 1; }
        let (si, cnt) = n.iter().enumerate().max_by_key(|x| *x.1).map(|(i, k)| (i as u8, *k)).unwrap();
        if cnt >= 2 {
            let holds = |h: &(u8, u8)| { let (a, b) = (h.0 % 4 == si, h.1 % 4 == si); if h.0 % 4 == h.1 % 4 { a } else { a || b } };
            let part = |want: bool| -> Vec<f32> {
                let (mut g, mut den) = (vec![0f32; 169], vec![0f32; 169]);
                for (i, h) in ctx.hands[vp].iter().enumerate() {
                    if holds(h) != want || c.board.contains(&h.0) || c.board.contains(&h.1) { continue; }
                    g[grid_cell(*h)] += vreach[i]; den[grid_cell(*h)] += 1.0;
                }
                g.iter().zip(&den).map(|(x, d)| if *d > 0.0 { r3(x / d) } else { -1.0 }).collect()
            };
            Some(SuitSplit { suit: card_str(si).chars().nth(1).unwrap(), with: part(true), without: part(false) })
        } else { None }
    };
    let vcombos = {
        let mx = vreach.iter().cloned().fold(0f32, f32::max).max(1e-9);
        let mut v: Vec<(String, f32)> = ctx.hands[vp].iter().enumerate()
            .filter(|(i, h)| vreach[*i] / mx >= 0.05 && !c.board.contains(&h.0) && !c.board.contains(&h.1))
            .map(|(i, h)| (format!("{}{}", card_str(h.1), card_str(h.0)), r3(vreach[i] / mx))).collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        if v.len() <= 50 { Some(v) } else { None }
    };
    Some(Spot {
        id: format!("{}-{}", s.flop.iter().map(|c| card_str(*c)).collect::<String>(), idx),
        board: c.board.iter().map(|c| card_str(*c)).collect(),
        street: a.street, hero: s.form.pos[hp], villain: s.form.pos[vp],
        pot: r3(pot), stack: r3(s.cfg.eff_stack - a.commit[hp]), to_call: r3(a.commit[vp] - a.commit[hp]),
        line_p: r3(c.p), history: c.hist.clone(),
        actions: a.actions.iter().map(|x| { let (label, kind, amt) = act_label(x); ActOut { label, kind, amt: r3(amt) } }).collect(),
        vr: grid(&ctx.hands[vp], vreach, &av), hr: grid(&ctx.hands[hp], hreach, &av), hs, range_freq,
        vclass: class_share(&vcls, vreach, vtotal), hclass: class_share(&hcls, hreach, htotal), hclass_strat,
        resp, drills, vsuit, vcombos, vdetail, hdetail,
    })
}

pub fn export(s: &Solved, rake: bool, seed: u64, out: &str) {
    let mut rng = Rng(0x9E3779B97F4A7C15 ^ seed.wrapping_mul(0xD1B54A32D192ED03));
    let mut cands = Vec::new();
    let reach = [s.ctx.weights[0].clone(), s.ctx.weights[1].clone()];
    collect(s, &s.root, &mut s.flop.clone(), reach, &mut Vec::new(), 1.0, &mut rng, &mut cands);
    let mut chosen: Vec<usize> = Vec::new();
    for (street, k) in [(0u8, 12usize), (1, 12), (2, 12)] {
        let w: Vec<f32> = cands.iter().map(|c| if c.node.street == street && c.node.actions.len() > 1 && c.p >= 0.02 { c.p } else { 0.0 }).collect();
        chosen.extend(rng.pick(&w, k));
    }
    // a forced share of compressed-range spots for the puzzle mode: villain is down to 10-50 combos spread over
    // at least 6 different hands (so not just AA/KK), on the turn or river
    let narrow = |c: &Cand| -> bool {
        if c.node.street == 0 || c.node.actions.len() < 2 { return false; }
        let vr = &c.reach[1 - c.node.player as usize];
        let mx = vr.iter().cloned().fold(0f32, f32::max); if mx <= 0.0 { return false; }
        let hands = &s.ctx.hands[1 - c.node.player as usize];
        let (mut n, mut cells) = (0, std::collections::HashSet::new());
        for (i, w) in vr.iter().enumerate() { if w / mx >= 0.05 && !c.board.contains(&hands[i].0) && !c.board.contains(&hands[i].1) { n += 1; cells.insert(grid_cell(hands[i])); } }
        (10..=50).contains(&n) && cells.len() >= 6
    };
    let nw: Vec<f32> = cands.iter().enumerate().map(|(i, c)| if !chosen.contains(&i) && narrow(c) { c.p } else { 0.0 }).collect();
    eprintln!("  narrow-range candidates: {} ({} on the river) of {}", nw.iter().filter(|w| **w > 0.0).count(), cands.iter().zip(&nw).filter(|(c, w)| **w > 0.0 && c.node.street == 2).count(), cands.len());
    chosen.extend(rng.pick(&nw, 6));
    chosen.sort_unstable();
    eprintln!("  exporting {} of {} candidate nodes", chosen.len(), cands.len());
    let spots: Vec<Spot> = chosen.iter().enumerate().filter_map(|(i, &ci)| build_spot(s, &cands[ci], i, &mut rng, None)).collect();
    let hands: Vec<Hand> = (0..40).filter_map(|i| play_hand(s, i, &mut rng)).collect();
    eprintln!("  {} played hands", hands.len());
    let file = FlopFile {
        flop: s.flop.iter().map(|c| card_str(*c)).collect(),
        formation: s.form.name, pos: s.form.pos, opener: s.form.pos[s.form.opener], caller: s.form.pos[1 - s.form.opener], open_size: s.form.open_size, three_bet: s.form.three_bet, hands, rake: if rake { "5% cap 1bb" } else { "0%" },
        exploitability_pct_pot: r3(100.0 * s.expl / s.cfg.start_pot), iterations: s.iters,
        start_pot: s.cfg.start_pot, eff_stack: s.cfg.eff_stack,
        tree: format!("bets {:?} raises {:?} max_raises {}", s.cfg.bets, s.cfg.raises, s.cfg.max_raises),
        ranges: "PLACEHOLDER hand-written approximations of 100bb charts", class_names: CLASS_NAMES.to_vec(), spots, briefing: briefing(s),
    };
    std::fs::create_dir_all(std::path::Path::new(out).parent().unwrap()).unwrap();
    std::fs::write(out, serde_json::to_string(&file).unwrap()).unwrap();
    eprintln!("  wrote {out}");
}

pub fn pack(dir: &str, out: &str) {
    let mut files: Vec<_> = std::fs::read_dir(dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |x| x == "json")).collect();
    files.sort();
    let bodies: Vec<String> = files.iter().map(|p| std::fs::read_to_string(p).unwrap()).collect();
    // preflop ranges per formation as 169-cell grids [OOP, IP], so the trainer can start a range read from them
    let mut forms = serde_json::Map::new();
    for f in crate::FORMATIONS.iter() {
        let grids: Vec<Vec<f32>> = f.ranges.iter().map(|r| {
            let mut g = vec![0f32; 169];
            for (h, w) in crate::range::parse_range(r) { g[grid_cell(h)] += w; }
            g.iter().enumerate().map(|(i, x)| r3(x / if i % 14 == 0 { 6.0 } else if i / 13 < i % 13 { 4.0 } else { 12.0 })).collect()
        }).collect();
        forms.insert(f.name.to_string(), serde_json::json!({ "pos": f.pos, "opener": f.pos[f.opener], "caller": f.pos[1 - f.opener], "open_size": f.open_size, "three_bet": f.three_bet, "pre": grids }));
    }
    if out.ends_with(".foldpack") {
        // .foldpack v1 -- see docs/FOLDPACK.md. "FOLDPK01", u32 LE index length, JSON index, then one gzip(JSON) blob per flop.
        use std::io::Write;
        let (mut blobs, mut entries) = (Vec::<u8>::new(), Vec::new());
        for body in &bodies {
            let v: serde_json::Value = serde_json::from_str(body).unwrap();
            let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
            gz.write_all(body.as_bytes()).unwrap();
            let z = gz.finish().unwrap();
            entries.push(serde_json::json!({ "flop": v["flop"], "formation": v["formation"], "offset": blobs.len(), "length": z.len(), "raw_length": body.len(),
                "spots": v["spots"].as_array().map_or(0, |a| a.len()), "hands": v["hands"].as_array().map_or(0, |a| a.len()), "exploitability_pct_pot": v["exploitability_pct_pot"] }));
            blobs.extend(z);
        }
        let index = serde_json::json!({ "version": 1, "ranges": crate::preflop::VERSION, "tree": crate::TREE_VERSION, "formations": forms, "flops": entries }).to_string();
        let mut file = Vec::from(*b"FOLDPK01");
        file.extend((index.len() as u32).to_le_bytes()); file.extend(index.as_bytes()); file.extend(&blobs);
        std::fs::write(out, &file).unwrap();
        eprintln!("packed {} flops -> {} ({:.1} MB, {:.1} MB raw)", files.len(), out, file.len() as f64 / 1e6, bodies.iter().map(|b| b.len()).sum::<usize>() as f64 / 1e6);
        return;
    }
    let js = format!("window.FOLD_PACK = {{\"version\":0,\"formations\":{},\"flops\":[{}]}};\n", serde_json::Value::Object(forms), bodies.join(","));
    std::fs::write(out, &js).unwrap();
    eprintln!("packed {} flops -> {} ({:.1} MB)", files.len(), out, js.len() as f64 / 1e6);
}
