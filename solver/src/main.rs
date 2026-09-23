mod cards;
mod cfr;
mod cfr3;
mod classify;
mod eval;
mod export;
mod import;
mod preflop;
mod range;
mod three;
mod tree;
mod tree3;

use cfr::{cfr, exploitability, Ctx, Discount};
use std::time::Instant;
use tree::{size_tree, Builder, Node, TreeConfig};

use preflop::*;

pub struct Formation {
    pub key: &'static str,
    pub name: &'static str,
    pub pos: [&'static str; 2],
    pub ranges: [&'static str; 2],
    pub opener: usize,
    pub open_size: f32,
    /// size the 3-bet was made to, 0 in single-raised pots; `opener` is then the 3-bettor
    pub three_bet: f32,
    pub start_pot: f32,
    pub eff_stack: f32,
}

pub const FORMATIONS: [Formation; 9] = [
    Formation { key: "btn_bb", name: "BTN vs BB", pos: ["BB", "BTN"], ranges: [BB_CALL_VS_BTN, BTN_OPEN], opener: 1, open_size: 2.5, three_bet: 0.0, start_pot: 5.5, eff_stack: 97.5 },
    Formation { key: "co_bb", name: "CO vs BB", pos: ["BB", "CO"], ranges: [BB_CALL_VS_CO, CO_OPEN], opener: 1, open_size: 2.5, three_bet: 0.0, start_pot: 5.5, eff_stack: 97.5 },
    Formation { key: "utg_bb", name: "UTG vs BB", pos: ["BB", "UTG"], ranges: [BB_CALL_VS_UTG, UTG_OPEN], opener: 1, open_size: 2.5, three_bet: 0.0, start_pot: 5.5, eff_stack: 97.5 },
    Formation { key: "sb_bb", name: "SB vs BB", pos: ["SB", "BB"], ranges: [SB_OPEN, BB_CALL_VS_SB], opener: 0, open_size: 3.0, three_bet: 0.0, start_pot: 6.0, eff_stack: 97.0 },
    Formation { key: "co_btn", name: "CO vs BTN", pos: ["CO", "BTN"], ranges: [CO_OPEN, BTN_CALL_VS_CO], opener: 0, open_size: 2.5, three_bet: 0.0, start_pot: 6.5, eff_stack: 97.5 },
    Formation { key: "utg_btn", name: "UTG vs BTN", pos: ["UTG", "BTN"], ranges: [UTG_OPEN, BTN_CALL_VS_UTG], opener: 0, open_size: 2.5, three_bet: 0.0, start_pot: 6.5, eff_stack: 97.5 },
    Formation { key: "btn_bb_3b", name: "BTN vs BB, 3-bet pot", pos: ["BB", "BTN"], ranges: [BB_3BET_VS_BTN, BTN_CALL_3BET], opener: 0, open_size: 2.5, three_bet: 11.0, start_pot: 22.5, eff_stack: 89.0 },
    Formation { key: "btn_sb_3b", name: "BTN vs SB, 3-bet pot", pos: ["SB", "BTN"], ranges: [SB_3BET_VS_BTN, BTN_CALL_3BET], opener: 0, open_size: 2.5, three_bet: 11.0, start_pot: 23.0, eff_stack: 89.0 },
    Formation { key: "co_btn_3b", name: "CO vs BTN, 3-bet pot", pos: ["CO", "BTN"], ranges: [CO_CALL_3BET, BTN_3BET_VS_CO], opener: 1, open_size: 2.5, three_bet: 7.5, start_pot: 16.5, eff_stack: 92.5 },
];

/// Bet tree v1 (locked for the first real batch; any change means re-solving everything).
/// Single-raised: aggressor flop 33/100, turn 66, river 50/125; caller bets 66 when checked to,
/// and an out-of-position caller may lead the flop for 33. 3-bet pots: aggressor 33/66, 66, 50/100.
/// One raise (60% pot) per street; a bet using 67%+ of the remaining stack becomes all-in.
pub const TREE_VERSION: &str = "tree-v1";
pub fn tree_config(f: &Formation) -> TreeConfig {
    let v = |x: &[f32]| x.to_vec();
    if std::env::var("FOLD_TREE").as_deref() == Ok("pre") {
        // coarse tree for measuring per-hand flop EVs that feed the preflop solve: one size, one raise
        let one = || [v(&[0.66]), v(&[0.66]), v(&[0.66])];
        let r = || [v(&[0.6]), v(&[0.6]), v(&[0.6])];
        return TreeConfig { start_pot: f.start_pot, eff_stack: f.eff_stack, bets: [one(), one()], raises: [r(), r()], max_raises: 1, allin_threshold: 0.67 };
    }
    if std::env::var("FOLD_TREE").as_deref() == Ok("ref") {
        // cross-check tree, chosen so an external solver can build the identical game:
        // one bet size everywhere, 60% raises with no cap, bets only become all-in when they exceed the stack
        let one = || [v(&[0.5]), v(&[0.5]), v(&[0.5])];
        let r = || [v(&[0.6]), v(&[0.6]), v(&[0.6])];
        return TreeConfig { start_pot: f.start_pot, eff_stack: f.eff_stack, bets: [one(), one()], raises: [r(), r()], max_raises: 99, allin_threshold: 1.0 };
    }
    let agg = if f.three_bet > 0.0 { [v(&[0.33, 0.66]), v(&[0.66]), v(&[0.5, 1.0])] } else { [v(&[0.33, 1.0]), v(&[0.66]), v(&[0.5, 1.25])] };
    let lead = if f.opener == 1 && f.three_bet == 0.0 { v(&[0.33]) } else if f.opener == 1 { v(&[]) } else { v(&[0.66]) };
    let caller = [lead, v(&[0.66]), v(&[0.66])];
    let bets = if f.opener == 1 { [caller, agg] } else { [agg, caller] };
    TreeConfig {
        start_pot: f.start_pot, eff_stack: f.eff_stack, bets,
        raises: [[v(&[0.6]), v(&[0.6]), v(&[0.6])], [v(&[0.6]), v(&[0.6]), v(&[0.6])]],
        max_raises: 1, allin_threshold: 0.67,
    }
}

pub struct Solved {
    pub ctx: Ctx,
    pub root: Node,
    pub cfg: TreeConfig,
    pub flop: Vec<u8>,
    pub form: &'static Formation,
    pub expl: f32,
    pub iters: u32,
}

/// Hands and weights per player. FOLD_EPS gives every hand outside the range a tiny weight, so its EV is
/// measured without moving the equilibrium (used to feed the preflop solve).
fn load_hands(form: &Formation, flop: &[u8]) -> ([Vec<(u8, u8)>; 2], [Vec<f32>; 2]) {
    let eps: f32 = std::env::var("FOLD_EPS").ok().and_then(|x| x.parse().ok()).unwrap_or(0.0);
    let (mut hands, mut weights) = ([Vec::new(), Vec::new()], [Vec::new(), Vec::new()]);
    for (p, r) in form.ranges.iter().enumerate() {
        let mut all = std::collections::BTreeMap::<(u8, u8), f32>::new();
        let only: Option<usize> = std::env::var("FOLD_EPS_P").ok().and_then(|x| x.parse().ok());   // extend one player only (memory)
        if eps > 0.0 && only.map_or(true, |o| o == p) { for a in 0..52u8 { for b in a + 1..52 { all.insert((a, b), eps); } } }
        // FOLD_RANGE0 / FOLD_RANGE1 replace a player's range (preflop iteration rounds)
        let over = std::env::var(format!("FOLD_RANGE{p}")).ok();
        for (h, w) in range::parse_range(over.as_deref().unwrap_or(r)) { all.insert(h, w.max(eps)); }
        for (h, w) in all {
            if flop.contains(&h.0) || flop.contains(&h.1) { continue; }
            hands[p].push(h); weights[p].push(w);
        }
    }
    (hands, weights)
}

/// Per-class root EV and equity for both players under the average strategy: [weight, sum w*EV, sum w*equity] per cell.
fn class_cells(s: &Solved, flop: &str) -> serde_json::Value {
    let mut players = Vec::new();
    for t in 0..2 {
        let wo = s.ctx.weights[1 - t].clone();
        let cfv = cfr::walk(&s.ctx, &s.root, t, &wo, cfr::Mode::Average);
        let vm = s.ctx.valid_mass(t, &wo);
        let eq = export::equity_all(s, &s.flop, t, &s.ctx.weights[t], &wo);
        let mut cells = vec![[0f64; 3]; 169];
        for h in 0..s.ctx.hands[t].len() {
            if vm[h] <= 1e-6 || eq[h] < 0.0 { continue; }
            // unweighted within the class here: eps-weight hands must count as much as in-range ones
            let c = cards::grid_cell(s.ctx.hands[t][h]);
            cells[c][0] += 1.0; cells[c][1] += (cfv[h] / vm[h]) as f64; cells[c][2] += eq[h] as f64;
        }
        players.push(cells.iter().map(|c| c.to_vec()).collect::<Vec<_>>());
    }
    serde_json::json!({ "formation": s.form.key, "flop": flop, "pot": s.cfg.start_pot, "stack": s.cfg.eff_stack, "opener": s.form.opener, "three_bet": s.form.three_bet,
        "exploitability_pct_pot": 100.0 * s.expl / s.cfg.start_pot, "iterations": s.iters, "cells": players })
}

fn solve(form: &'static Formation, flop_s: &str, max_iters: u32, target_pct: f32, rake: bool) -> Solved {
    let flop = cards::parse_board(flop_s);
    let cfg = tree_config(form);
    let (rake_pct, rake_cap) = if rake { (0.05, 1.0) } else { (0.0, 0.0) };
    let (hands, weights) = load_hands(form, &flop);
    let t0 = Instant::now();
    let mut b = Builder::new(&cfg);
    let mut root = b.build(&flop);
    let nh = [hands[0].len(), hands[1].len()];
    let (nodes, floats) = size_tree(&mut root, nh, false);
    eprintln!("[{flop_s}] hands OOP/IP {}/{}  action nodes {}  memory {:.2} GB  river boards {}",
        nh[0], nh[1], nodes, floats as f64 * 4.0 / 1e9, b.boards.len());
    size_tree(&mut root, nh, true);
    let ctx = Ctx::new(hands, weights, &b.boards, cfg.start_pot, rake_pct, rake_cap);
    eprintln!("  built in {:.1}s", t0.elapsed().as_secs_f32());
    let mut expl = f32::MAX;
    let mut it = 0;
    while it < max_iters {
        it += 1;
        let d = Discount::at(it);
        for t in 0..2 {
            let reach = ctx.weights[1 - t].clone();
            cfr(&ctx, &mut root, t, &reach, &d);
        }
        if it % 25 == 0 || it == max_iters {
            let (e, ev0, ev1) = exploitability(&ctx, &root);
            expl = e;
            eprintln!("  iter {it:4}  expl {:.4} bb = {:.3}% pot   EV OOP {:.3} IP {:.3}   {:.0}s",
                e, 100.0 * e / cfg.start_pot, ev0, ev1, t0.elapsed().as_secs_f32());
            if 100.0 * e / cfg.start_pot < target_pct { break; }
        }
    }
    Solved { ctx, root, cfg, flop, form, expl, iters: it }
}

/// DCFR loop for a three-seat tree, then an optional strategy dump of nodes with street < `keep_below`.
fn run3(ctx: &three::Ctx3, root: &mut tree3::Node3, pot: f32, stack: f32, caps: &[u8], board_s: &str, max_iters: u32, target: f32, dump: &str, keep_below: u8) {
    let t0 = Instant::now();
    let mut it = 0;
    while it < max_iters {
        it += 1;
        let d = Discount::at(it);
        for t in 0..3 { let reach: cfr3::Reach = [ctx.weights[0].clone(), ctx.weights[1].clone(), ctx.weights[2].clone()]; cfr3::cfr3(ctx, root, t, &reach, &d); }
        if it % 25 == 0 || it == max_iters {
            let (g, ev) = cfr3::exploitability3(ctx, root);
            eprintln!("  iter {it:4}  expl {g:.4} bb = {:.3}% pot   EV {:.3} {:.3} {:.3}   {:.0}s", 100.0 * g / pot, ev[0], ev[1], ev[2], t0.elapsed().as_secs_f32());
            if 100.0 * g / pot < target { break; }
        }
    }
    if dump.is_empty() { return; }
    let mut idx = Vec::new(); let mut bytes: Vec<u8> = Vec::new();
    fn pre(n: &tree3::Node3, ctx: &three::Ctx3, idx: &mut Vec<serde_json::Value>, bytes: &mut Vec<u8>, line: &mut Vec<String>, keep_below: u8) {
        match n {
            tree3::Node3::Action(a) => {
                if a.street >= keep_below { return; }
                let nh = ctx.hands[a.player as usize].len();
                let s = cfr3::avg_strategy(a, nh);
                idx.push(serde_json::json!({ "line": line.join(" "), "player": a.player, "street": a.street, "actions": a.actions.iter().map(|x| format!("{x:?}")).collect::<Vec<_>>(), "commit": a.commit, "offset": bytes.len() }));
                for v in &s { bytes.extend_from_slice(&v.to_le_bytes()); }
                for (k, c) in a.children.iter().enumerate() { line.push(format!("{}:{}", a.player, k)); pre(c, ctx, idx, bytes, line, keep_below); line.pop(); }
            }
            tree3::Node3::Chance { children, cards, .. } => { for (c, ch) in cards.iter().zip(children) { line.push(format!("c{c}")); pre(ch, ctx, idx, bytes, line, keep_below); line.pop(); } }
            _ => {}
        }
    }
    pre(root, ctx, &mut idx, &mut bytes, &mut Vec::new(), keep_below);
    std::fs::write(format!("{dump}.f32"), &bytes).unwrap();
    std::fs::write(format!("{dump}.json"), serde_json::to_vec(&serde_json::json!({ "board": board_s, "pot": pot, "stack": stack, "raises": caps, "hands": ctx.hands.iter().map(|h| h.iter().map(|x| [x.0, x.1]).collect::<Vec<_>>()).collect::<Vec<_>>(), "weights": ctx.weights, "nodes": idx })).unwrap()).unwrap();
    eprintln!("dumped {} nodes (street < {keep_below}) -> {dump}.f32/.json", idx.len());
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let get = |k: &str, d: &str| args.iter().position(|a| a == k).map(|i| args[i + 1].clone()).unwrap_or(d.to_string());
    match args.get(1).map(String::as_str) {
        Some("solve") => {
            let form = FORMATIONS.iter().find(|f| f.key == args[2]).expect("unknown formation");
            let flop = args[3].clone();
            let rake = get("--rake", "0") != "0";
            let s = solve(form, &flop, get("--iters", "300").parse().unwrap(), get("--target", "0.3").parse().unwrap(), rake);
            let out = get("--out", &format!("out/{}_{}_{}.json", form.key, flop, if rake { "r5" } else { "r0" }));
            export::export(&s, rake, get("--seed", "1").parse().unwrap(), &out);
        }
        Some("spec") => {
            // problem description for the GPU solver: hands, weights, tree config, 7-card strengths per runout
            let form = FORMATIONS.iter().find(|f| f.key == args[2]).expect("unknown formation");
            let flop = cards::parse_board(&args[3]);
            let dir = get("--out", "../gpu/spec");
            std::fs::create_dir_all(&dir).unwrap();
            let cfg = tree_config(form);
            let (hands, weights) = load_hands(form, &flop);
            let deck: Vec<u8> = (0..52u8).filter(|c| !flop.contains(c)).collect();
            let mut bin: Vec<u8> = Vec::new();
            for &t in &deck { for &r in &deck { for p in 0..2 { for h in &hands[p] {
                let v = if t == r || [t, r].contains(&h.0) || [t, r].contains(&h.1) { 0 } else { eval::eval(&[flop[0], flop[1], flop[2], t, r, h.0, h.1]) };
                bin.extend_from_slice(&v.to_le_bytes());
            }}}}
            let name = format!("{}_{}", form.key, args[3]);
            std::fs::write(format!("{dir}/{name}.bin"), bin).unwrap();
            let j = serde_json::json!({
                "formation": form.key, "flop": flop, "deck": deck, "pos": form.pos,
                "hands": hands.iter().map(|hs| hs.iter().map(|h| [h.0, h.1]).collect::<Vec<_>>()).collect::<Vec<_>>(),
                "weights": weights, "start_pot": cfg.start_pot, "eff_stack": cfg.eff_stack, "bets": cfg.bets, "raises": cfg.raises,
                "max_raises": cfg.max_raises, "allin_threshold": cfg.allin_threshold,
            });
            std::fs::write(format!("{dir}/{name}.json"), j.to_string()).unwrap();
            eprintln!("wrote {dir}/{name}.json + .bin");
        }
        Some("spec3") => {
            // three-seat problem for gpu/solver3.py: spec3 <flop> <r0> <r1> <r2> --out DIR --name NAME [--pot 8] [--stack 97.5] [--raises 1,1,0]
            let flop = cards::parse_board(&args[2]);
            let (pot, stack): (f32, f32) = (get("--pot", "8.0").parse().unwrap(), get("--stack", "97.5").parse().unwrap());
            let caps: Vec<u8> = get("--raises", "1,1,0").split(',').map(|x| x.parse().unwrap()).collect();
            let dir = get("--out", "../gpu/spec3"); std::fs::create_dir_all(&dir).unwrap();
            let mut hands: [Vec<(u8, u8)>; 3] = [Vec::new(), Vec::new(), Vec::new()];
            let mut weights: [Vec<f32>; 3] = [Vec::new(), Vec::new(), Vec::new()];
            for p in 0..3 { for (h, w) in range::parse_range(&args[3 + p]) { if flop.contains(&h.0) || flop.contains(&h.1) { continue; } hands[p].push(h); weights[p].push(w); } }
            assert!(hands.iter().all(|h| !h.is_empty()), "every seat needs a non-empty range");
            let deck: Vec<u8> = (0..52u8).filter(|c| !flop.contains(c)).collect();
            let mut bin: Vec<u8> = Vec::new();
            for &t in &deck { for &r in &deck { for p in 0..3 { for h in &hands[p] {
                let v = if t == r || [t, r].contains(&h.0) || [t, r].contains(&h.1) { 0 } else { eval::eval(&[flop[0], flop[1], flop[2], t, r, h.0, h.1]) };
                bin.extend_from_slice(&v.to_le_bytes());
            }}}}
            let name = get("--name", &format!("three_{}", args[2]));
            std::fs::write(format!("{dir}/{name}.bin"), bin).unwrap();
            let j = serde_json::json!({
                "formation": name, "flop": flop, "deck": deck, "seats": 3,
                "hands": hands.iter().map(|hs| hs.iter().map(|h| [h.0, h.1]).collect::<Vec<_>>()).collect::<Vec<_>>(),
                "weights": weights, "start_pot": pot, "eff_stack": stack, "bet": 0.66, "raise": 0.6, "max_raises": caps, "allin_threshold": 0.67,
            });
            std::fs::write(format!("{dir}/{name}.json"), j.to_string()).unwrap();
            eprintln!("wrote {dir}/{name}.json + .bin ({} MB)", std::fs::metadata(format!("{dir}/{name}.bin")).unwrap().len() / 1_000_000);
        }
        Some("import") => {
            // load a GPU-solved average strategy, verify it with our own best response, then export as usual
            let form = FORMATIONS.iter().find(|f| f.key == args[2]).expect("unknown formation");
            let mut s = solve(form, &args[3], 0, 0.0, false);
            let bytes = std::fs::read(&args[4]).expect("strategy file");
            let nh = [s.ctx.hands[0].len(), s.ctx.hands[1].len()];
            let mut deck_pos = [0usize; 52];
            for (i, c) in (0..52u8).filter(|c| !s.flop.contains(c)).enumerate() { deck_pos[c as usize] = i; }
            let mut shapes = Vec::new();
            import::shapes(&s.root, nh, &mut 0, &mut shapes);
            assert_eq!(shapes.last().map(|x| x.0 + x.1).unwrap(), bytes.len(), "strategy file size mismatch");
            import::fill(&mut s.root, &bytes, &shapes, &mut 0, [0, 0], &deck_pos, nh);
            let patch = get("--brief-into", "");
            if !patch.is_empty() {
                // only add the flop briefing to an export that already exists (no best response, no re-sampling)
                let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&patch).unwrap()).unwrap();
                v["briefing"] = export::briefing_json(&s);
                std::fs::write(get("--out", &patch), serde_json::to_string(&v).unwrap()).unwrap();
                return;
            }
            // --expl <bb>: reuse a best-response result already verified for this strategy file (re-exports only)
            let known: f32 = get("--expl", "-1").parse().unwrap();
            let (e, ev0, ev1) = if known >= 0.0 { (known, 0.0, 0.0) } else { exploitability(&s.ctx, &s.root) };
            s.expl = e;
            s.iters = get("--iters", "0").parse().unwrap();
            eprintln!("  imported: exploitability (Rust best response) {:.4} bb = {:.3}% pot   EV OOP {:.3} IP {:.3}", e, 100.0 * e / s.cfg.start_pot, ev0, ev1);
            export::export(&s, false, get("--seed", "1").parse().unwrap(), &get("--out", &format!("out/{}_{}.json", form.key, args[3])));
        }
        Some("refcheck") => {
            // solve the cross-check tree and print what an external solver can be compared on
            let form = FORMATIONS.iter().find(|f| f.key == args[2]).expect("unknown formation");
            let s = solve(form, &args[3], get("--iters", "1000").parse().unwrap(), get("--target", "0.1").parse().unwrap(), false);
            let (_, ev0, ev1) = exploitability(&s.ctx, &s.root);
            let Node::Action(a) = &s.root else { panic!() };
            let n = s.ctx.hands[0].len();
            let strat = cfr::avg_strategy(a, n);
            let total: f32 = s.ctx.weights[0].iter().sum();
            let freq: Vec<f32> = (0..a.actions.len()).map(|x| (0..n).map(|h| s.ctx.weights[0][h] * strat[x * n + h]).sum::<f32>() / total).collect();
            let hands: serde_json::Map<String, serde_json::Value> = (0..n).map(|h| {
                let c = s.ctx.hands[0][h];
                (format!("{}{}", cards::card_str(c.1), cards::card_str(c.0)), serde_json::json!((0..a.actions.len()).map(|x| strat[x * n + h]).collect::<Vec<f32>>()))
            }).collect();
            let class_range = |r: &str| { let mut m = std::collections::BTreeMap::<usize, (f32, f32)>::new();
                for (h, w) in range::parse_range(r) { let e = m.entry(cards::grid_cell(h)).or_insert((0.0, 0.0)); e.0 += w; e.1 += 1.0; }
                m.iter().map(|(c, (w, k))| { let (row, col) = (c / 13, c % 13); let rk = |i: usize| cards::RANKS[12 - i] as char;
                    let name = if row == col { format!("{}{}", rk(row), rk(row)) } else if row < col { format!("{}{}s", rk(row), rk(col)) } else { format!("{}{}o", rk(col), rk(row)) };
                    format!("{}:{:.3}", name, w / k) }).collect::<Vec<_>>().join(",") };
            println!("{}", serde_json::json!({ "formation": form.key, "flop": args[3], "start_pot": s.cfg.start_pot, "eff_stack": s.cfg.eff_stack,
                "range_oop": class_range(form.ranges[0]), "range_ip": class_range(form.ranges[1]),
                "exploitability_pct_pot": 100.0 * s.expl / s.cfg.start_pot, "ev_oop": ev0, "ev_ip": ev1, "root_freq": freq, "root_hands": hands }));
        }
        Some("calib") => {
            // per-class root EV and equity for both players from a cached strategy: input to the preflop model
            let form = FORMATIONS.iter().find(|f| f.key == args[2]).expect("unknown formation");
            let mut s = solve(form, &args[3], 0, 0.0, false);
            let bytes = std::fs::read(&args[4]).expect("strategy file");
            let nh = [s.ctx.hands[0].len(), s.ctx.hands[1].len()];
            let mut deck_pos = [0usize; 52];
            for (i, c) in (0..52u8).filter(|c| !s.flop.contains(c)).enumerate() { deck_pos[c as usize] = i; }
            let mut shapes = Vec::new();
            import::shapes(&s.root, nh, &mut 0, &mut shapes);
            import::fill(&mut s.root, &bytes, &shapes, &mut 0, [0, 0], &deck_pos, nh);
            println!("{}", class_cells(&s, &args[3]));
        }
        Some("preeq") => {
            // Monte Carlo preflop all-in equity, 169 x 169 classes (row beats column), plus the number of non-conflicting combo pairs
            use rayon::prelude::*;
            let n: usize = get("--samples", "10000").parse().unwrap();
            let mut combos: Vec<Vec<(u8, u8)>> = vec![Vec::new(); 169];
            for a in 0..52u8 { for b in a + 1..52 { combos[cards::grid_cell((a, b))].push((a, b)); } }
            let rows: Vec<(Vec<f32>, Vec<f32>)> = (0..169usize).into_par_iter().map(|i| {
                let mut x = (i as u64 + 1) * 0x9E3779B97F4A7C15;
                let mut rnd = move || { x ^= x << 13; x ^= x >> 7; x ^= x << 17; x };
                let (mut eq, mut cnt) = (vec![0f32; 169], vec![0f32; 169]);
                for j in 0..169 {
                    let pairs: Vec<((u8, u8), (u8, u8))> = combos[i].iter().flat_map(|&a| combos[j].iter().filter(move |&&b| a.0 != b.0 && a.0 != b.1 && a.1 != b.0 && a.1 != b.1).map(move |&b| (a, b))).collect();
                    cnt[j] = pairs.len() as f32;
                    if pairs.is_empty() { continue; }
                    let mut win = 0f64;
                    for _ in 0..n {
                        let (a, b) = pairs[(rnd() % pairs.len() as u64) as usize];
                        let mut board = [0u8; 5]; let mut k = 0;
                        while k < 5 { let c = (rnd() % 52) as u8; if c == a.0 || c == a.1 || c == b.0 || c == b.1 || board[..k].contains(&c) { continue; } board[k] = c; k += 1; }
                        let (ea, eb) = (eval::eval(&[board[0], board[1], board[2], board[3], board[4], a.0, a.1]), eval::eval(&[board[0], board[1], board[2], board[3], board[4], b.0, b.1]));
                        win += if ea > eb { 1.0 } else if ea == eb { 0.5 } else { 0.0 };
                    }
                    eq[j] = (win / n as f64) as f32;
                }
                (eq, cnt)
            }).collect();
            println!("{}", serde_json::json!({ "eq": rows.iter().map(|r| r.0.clone()).collect::<Vec<_>>(), "pairs": rows.iter().map(|r| r.1.clone()).collect::<Vec<_>>() }));
        }
        Some("solve3") => {
            // three-seat CPU solve: solve3 <board> <range0> <range1> <range2> [--pot 8] [--stack 97.5] [--iters N] [--target pct]
            let board = cards::parse_board(&args[2]);
            let (pot, stack): (f32, f32) = (get("--pot", "8.0").parse().unwrap(), get("--stack", "97.5").parse().unwrap());
            let v = |x: &[f32]| x.to_vec();
            let one = || [v(&[0.66]), v(&[0.66]), v(&[0.66])];
            let r = || [v(&[0.6]), v(&[0.6]), v(&[0.6])];
            let cfg = TreeConfig { start_pot: pot, eff_stack: stack, bets: [one(), one()], raises: [r(), r()], max_raises: 1, allin_threshold: 0.67 };
            let caps: Vec<u8> = get("--raises", "1,1,1").split(',').map(|x| x.parse().unwrap()).collect();
            let mut hands: [Vec<(u8, u8)>; 3] = [Vec::new(), Vec::new(), Vec::new()];
            let mut weights: [Vec<f32>; 3] = [Vec::new(), Vec::new(), Vec::new()];
            for p in 0..3 { for (h, w) in range::parse_range(&args[3 + p]) { if board.contains(&h.0) || board.contains(&h.1) { continue; } hands[p].push(h); weights[p].push(w); } }
            assert!(hands.iter().all(|h| !h.is_empty()), "every seat needs a non-empty range");
            let t0 = Instant::now();
            let mut b = tree3::Builder3::new(&cfg); b.max_raises = [caps[0], caps[1], caps[2]];
            let mut root = b.build(&board);
            let nh = [hands[0].len(), hands[1].len(), hands[2].len()];
            let (nodes, floats) = tree3::size_tree3(&mut root, nh, true);
            eprintln!("[{}] hands {:?}  action nodes {}  memory {:.2} GB  river boards {}", args[2], nh, nodes, floats as f64 * 4.0 / 1e9, b.boards.len());
            let ctx = three::Ctx3::new(hands, weights, &b.boards, pot);
            eprintln!("  built in {:.1}s", t0.elapsed().as_secs_f32());
            run3(&ctx, &mut root, pot, stack, &caps, &args[2], get("--iters", "300").parse().unwrap(), get("--target", "0.5").parse().unwrap(), &get("--dump", ""), 2);
        }
        Some("resolve3") => {
            // turn re-solve from a flop dump: resolve3 <dump-prefix> [--entry N --card Xy] [--iters] [--target] [--dump out]; no --entry lists entries
            let j: serde_json::Value = serde_json::from_slice(&std::fs::read(format!("{}.json", args[2])).unwrap()).unwrap();
            let bytes = std::fs::read(format!("{}.f32", args[2])).unwrap();
            let board = cards::parse_board(j["board"].as_str().unwrap());
            let (pot, stack) = (j["pot"].as_f64().unwrap() as f32, j["stack"].as_f64().unwrap() as f32);
            let caps: Vec<u8> = j["raises"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as u8).collect();
            let hands: [Vec<(u8, u8)>; 3] = [0, 1, 2].map(|p| j["hands"][p].as_array().unwrap().iter().map(|h| (h[0].as_u64().unwrap() as u8, h[1].as_u64().unwrap() as u8)).collect());
            let weights: [Vec<f32>; 3] = [0, 1, 2].map(|p| j["weights"][p].as_array().unwrap().iter().map(|x| x.as_f64().unwrap() as f32).collect());
            let v = |x: &[f32]| x.to_vec();
            let one = || [v(&[0.66]), v(&[0.66]), v(&[0.66])];
            let r = || [v(&[0.6]), v(&[0.6]), v(&[0.6])];
            let cfg = TreeConfig { start_pot: pot, eff_stack: stack, bets: [one(), one()], raises: [r(), r()], max_raises: 1, allin_threshold: 0.67 };
            let mut b = tree3::Builder3::new(&cfg); b.max_raises = [caps[0], caps[1], caps[2]];
            let mut root = b.build(&board);
            let nh = [hands[0].len(), hands[1].len(), hands[2].len()];
            // load flop + turn strategies in dump order (preorder, rivers skipped); river nodes get no storage
            fn load(n: &mut tree3::Node3, nh: [usize; 3], bytes: &[u8], pos: &mut usize) {
                match n {
                    tree3::Node3::Action(a) => {
                        if a.street == 2 { return; }
                        let len = a.actions.len() * nh[a.player as usize];
                        a.strat_sum = (0..len).map(|i| f32::from_le_bytes(bytes[*pos + 4 * i..*pos + 4 * i + 4].try_into().unwrap())).collect();
                        *pos += 4 * len;
                        for c in a.children.iter_mut() { load(c, nh, bytes, pos); }
                    }
                    tree3::Node3::Chance { children, .. } => { for c in children.iter_mut() { load(c, nh, bytes, pos); } }
                    _ => {}
                }
            }
            // GPU dumps (solver3.py) store each turn shape-node as [49 cards, A, H] in one block; rebuild the per-card order here
            fn load_gpu(n: &mut tree3::Node3, nh: [usize; 3], bytes: &[u8], pos: &mut usize) {
                fn sizes(n: &tree3::Node3, nh: [usize; 3], out: &mut Vec<usize>) {
                    match n {
                        tree3::Node3::Action(a) => { if a.street == 2 { return; } out.push(a.actions.len() * nh[a.player as usize]); for c in &a.children { sizes(c, nh, out); } }
                        tree3::Node3::Chance { children, .. } => { for c in children { sizes(c, nh, out); } }
                        _ => {}
                    }
                }
                fn fill(n: &mut tree3::Node3, nh: [usize; 3], bytes: &[u8], base: usize, offs: &[usize], nd: usize, d: usize, k: &mut usize) {
                    match n {
                        tree3::Node3::Action(a) => {
                            if a.street == 2 { return; }
                            let len = a.actions.len() * nh[a.player as usize];
                            let p = base + 4 * (offs[*k] + d * len); *k += 1;
                            a.strat_sum = (0..len).map(|i| f32::from_le_bytes(bytes[p + 4 * i..p + 4 * i + 4].try_into().unwrap())).collect();
                            for c in a.children.iter_mut() { fill(c, nh, bytes, base, offs, nd, d, k); }
                        }
                        tree3::Node3::Chance { children, .. } => { for c in children.iter_mut() { fill(c, nh, bytes, base, offs, nd, d, k); } }
                        _ => {}
                    }
                }
                match n {
                    tree3::Node3::Action(a) => {
                        let len = a.actions.len() * nh[a.player as usize];
                        a.strat_sum = (0..len).map(|i| f32::from_le_bytes(bytes[*pos + 4 * i..*pos + 4 * i + 4].try_into().unwrap())).collect();
                        *pos += 4 * len;
                        for c in a.children.iter_mut() { load_gpu(c, nh, bytes, pos); }
                    }
                    tree3::Node3::Chance { children, .. } => {
                        let nd = children.len();
                        let mut sz = Vec::new(); sizes(&children[0], nh, &mut sz);
                        let mut offs = Vec::with_capacity(sz.len()); let mut acc = 0; for &s in &sz { offs.push(acc); acc += nd * s; }
                        for d in 0..nd { let mut k = 0; fill(&mut children[d], nh, bytes, *pos, &offs, nd, d, &mut k); }
                        *pos += 4 * acc;
                    }
                    _ => {}
                }
            }
            let mut pos = 0;
            if args.iter().any(|a| a == "--gpu") { load_gpu(&mut root, nh, &bytes, &mut pos); } else { load(&mut root, nh, &bytes, &mut pos); }
            assert_eq!(pos, bytes.len(), "dump size mismatch");
            let ctx = three::Ctx3 { hands: hands.clone(), weights: weights.clone(), card_hands: [vec![], vec![], vec![]], sd: vec![], start_pot: pot };
            let entries = cfr3::turn_entries(&ctx, &root);
            let entry = get("--entry", "");
            if entry.is_empty() {
                let mut rows: Vec<(usize, f32)> = entries.iter().enumerate().map(|(i, e)| (i, e.3[0].iter().sum::<f32>() * e.3[1].iter().sum::<f32>() * e.3[2].iter().sum::<f32>())).collect();
                let tot: f32 = rows.iter().map(|x| x.1).sum(); rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
                for (i, m) in rows.iter().take(25) { let e = &entries[*i]; println!("{i:3}  {:5.1}%  commit {:?} alive {:?}  line {}", 100.0 * m / tot, e.1, e.2, e.0); }
                return;
            }
            let e = &entries[entry.parse::<usize>().unwrap()];
            let card = cards::parse_board(&get("--card", ""))[0];
            let mut tb = board.clone(); tb.push(card);
            let mut w = e.3.clone();
            for p in 0..3 { for (i, h) in hands[p].iter().enumerate() { if h.0 == card || h.1 == card { w[p][i] = 0.0; } } }
            eprintln!("entry {} line [{}] commit {:?} alive {:?} turn {}", entry, e.0, e.1, e.2, get("--card", ""));
            let mut b2 = tree3::Builder3::new(&cfg); b2.max_raises = [1, 1, 1];
            let mut sub = b2.build_from(&tb, e.1, e.2);
            let (nodes, floats) = tree3::size_tree3(&mut sub, nh, true);
            eprintln!("  subgame action nodes {nodes}  memory {:.2} GB  river boards {}", floats as f64 * 4.0 / 1e9, b2.boards.len());
            let ctx2 = three::Ctx3::new(hands, w, &b2.boards, pot);
            run3(&ctx2, &mut sub, pot, stack, &[1, 1, 1], &tb.iter().map(|&c| cards::card_str(c)).collect::<String>(), get("--iters", "300").parse().unwrap(), get("--target", "0.5").parse().unwrap(), &get("--dump", ""), 3);
        }
        Some("measure") => {
            // solve on the CPU (use with FOLD_TREE=pre FOLD_EPS=..) and print per-class EVs; no strategy file needed
            let form = FORMATIONS.iter().find(|f| f.key == args[2]).expect("unknown formation");
            let s = solve(form, &args[3], get("--iters", "300").parse().unwrap(), get("--target", "1.0").parse().unwrap(), false);
            println!("{}", class_cells(&s, &args[3]));
        }
        Some("classrange") => {
            // one player's range as explicit per-class weights: "AA:1.000,AKs:0.500,..."
            let form = FORMATIONS.iter().find(|f| f.key == args[2]).expect("unknown formation");
            let mut m = std::collections::BTreeMap::<usize, (f32, f32)>::new();
            for (h, w) in range::parse_range(form.ranges[args[3].parse::<usize>().unwrap()]) { let e = m.entry(cards::grid_cell(h)).or_insert((0.0, 0.0)); e.0 += w; e.1 += if h.0 / 4 == h.1 / 4 { 6.0 } else if h.0 % 4 == h.1 % 4 { 4.0 } else { 12.0 } / if h.0 / 4 == h.1 / 4 { 6.0 } else if h.0 % 4 == h.1 % 4 { 4.0 } else { 12.0 }; }
            let rk = |i: usize| cards::RANKS[12 - i] as char;
            println!("{}", m.iter().map(|(c, (w, k))| { let (row, col) = (c / 13, c % 13);
                let name = if row == col { format!("{}{}", rk(row), rk(row)) } else if row < col { format!("{}{}s", rk(row), rk(col)) } else { format!("{}{}o", rk(col), rk(row)) };
                format!("{}:{:.3}", name, w / k) }).collect::<Vec<_>>().join(","));
        }
        Some("ranges") => for f in FORMATIONS.iter() {
            let pct = |r: &str| range::parse_range(r).iter().map(|e| e.1).sum::<f32>() / 13.26;
            println!("{:10} {:>4} {:5.1}%   {:>4} {:5.1}%", f.key, f.pos[0], pct(f.ranges[0]), f.pos[1], pct(f.ranges[1]));
        },
        Some("pack") => export::pack(&get("--dir", "out"), &get("--out", "../proto/pack.js")),
        _ => eprintln!("usage: fold-cli solve <btn_bb|co_bb|sb_bb|utg_btn> <flop> [--iters N] [--target PCT] [--rake 0|1] | pack"),
    }
}
