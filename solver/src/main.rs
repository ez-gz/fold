mod cards;
mod cfr;
mod classify;
mod eval;
mod export;
mod import;
mod preflop;
mod range;
mod tree;

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

fn solve(form: &'static Formation, flop_s: &str, max_iters: u32, target_pct: f32, rake: bool) -> Solved {
    let flop = cards::parse_board(flop_s);
    let cfg = tree_config(form);
    let (rake_pct, rake_cap) = if rake { (0.05, 1.0) } else { (0.0, 0.0) };
    let mut hands = [Vec::new(), Vec::new()];
    let mut weights = [Vec::new(), Vec::new()];
    for (p, r) in form.ranges.iter().enumerate() {
        for (h, w) in range::parse_range(r) {
            if flop.contains(&h.0) || flop.contains(&h.1) { continue; }
            hands[p].push(h);
            weights[p].push(w);
        }
    }
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
            let mut hands: [Vec<(u8, u8)>; 2] = [Vec::new(), Vec::new()];
            let mut weights: [Vec<f32>; 2] = [Vec::new(), Vec::new()];
            for (p, r) in form.ranges.iter().enumerate() {
                for (h, w) in range::parse_range(r) {
                    if flop.contains(&h.0) || flop.contains(&h.1) { continue; }
                    hands[p].push(h);
                    weights[p].push(w);
                }
            }
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
            let (e, ev0, ev1) = exploitability(&s.ctx, &s.root);
            s.expl = e;
            s.iters = get("--iters", "0").parse().unwrap();
            eprintln!("  imported: exploitability (Rust best response) {:.4} bb = {:.3}% pot   EV OOP {:.3} IP {:.3}", e, 100.0 * e / s.cfg.start_pot, ev0, ev1);
            export::export(&s, false, get("--seed", "1").parse().unwrap(), &get("--out", &format!("out/{}_{}.json", form.key, args[3])));
        }
        Some("ranges") => for f in FORMATIONS.iter() {
            let pct = |r: &str| range::parse_range(r).iter().map(|e| e.1).sum::<f32>() / 13.26;
            println!("{:10} {:>4} {:5.1}%   {:>4} {:5.1}%", f.key, f.pos[0], pct(f.ranges[0]), f.pos[1], pct(f.ranges[1]));
        },
        Some("pack") => export::pack(&get("--dir", "out"), &get("--out", "../proto/pack.js")),
        _ => eprintln!("usage: fold-cli solve <btn_bb|co_bb|sb_bb|utg_btn> <flop> [--iters N] [--target PCT] [--rake 0|1] | pack"),
    }
}
