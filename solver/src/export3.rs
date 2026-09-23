//! Three-seat drill spots: every action node reached often enough becomes a spot for the acting seat, with
//! the average strategy and the value of each action for every hero hand. Flop trees carry no rivers, so
//! flop-node action values come from the GPU dump (`.flopev.f32`); turn subgames are complete and walked here.

use crate::cards::{card_str, grid_cell};
use crate::cfr::Mode;
use crate::cfr3::{avg_strategy, walk3, Reach};
use crate::classify::classify;
use crate::three::Ctx3;
use crate::tree::Act;
use crate::tree3::{ActionNode3, Node3};

fn others(t: usize) -> [usize; 2] { [(t + 1) % 3, (t + 2) % 3] }
fn r3(x: f32) -> f32 { (x * 1000.0).round() / 1000.0 }

fn act_label(a: &Act) -> (String, &'static str, f32) {
    match a {
        Act::Fold => ("Fold".into(), "fold", 0.0),
        Act::Check => ("Check".into(), "check", 0.0),
        Act::Call => ("Call".into(), "call", 0.0),
        Act::Bet(amt, f) => (format!("Bet {:.0}%", f * 100.0), "bet", *amt),
        Act::Raise(amt, to) => (format!("Raise to {:.1}", to), "raise", *amt),
    }
}

/// Mass of the three seats' reach over card-disjoint triples.
fn mass(ctx: &Ctx3, reach: &Reach) -> f32 {
    let m = ctx.flat_value(0, [1, 2], [&reach[1], &reach[2]], 1.0);
    m.iter().zip(&reach[0]).map(|(a, b)| a * b).sum()
}

fn grid(hands: &[(u8, u8)], w: &[f32], board: &[u8]) -> Vec<f32> {
    let mut g = vec![0f32; 169]; let mut den = vec![0f32; 169];
    for (i, h) in hands.iter().enumerate() { g[grid_cell(*h)] += w[i]; }
    for r1 in 0..52u8 { for r2 in r1 + 1..52 { if !board.contains(&r1) && !board.contains(&r2) { den[grid_cell((r1, r2))] += 1.0; } } }
    g.iter().zip(&den).map(|(x, d)| if *d > 0.0 { r3(x / d) } else { 0.0 }).collect()
}

pub struct Export3<'a> {
    pub ctx: &'a Ctx3,
    pub seats: Vec<String>,
    pub board: Vec<u8>,
    pub stack: f32,
    /// history before the tree root (turn subgames): [street, seat, label]
    pub prefix: Vec<(u8, String, String)>,
    /// GPU flop-node action values, preorder over street-0 action nodes
    pub flopev: Option<Vec<f32>>,
    pub min_p: f32,
    pub id: String,
    pub root_mass: f32,
}

impl<'a> Export3<'a> {
    pub fn run(&self, root: &Node3, reach: Reach, alive: [bool; 3]) -> Vec<serde_json::Value> {
        let mut out = Vec::new(); let mut pos = 0usize;
        let mut hist: Vec<(u8, String, String)> = self.prefix.clone();
        let mut board = self.board.clone();
        self.walk(root, reach, alive, &mut hist, &mut board, &mut pos, &mut out);
        out
    }

    fn walk(&self, n: &Node3, reach: Reach, alive: [bool; 3], hist: &mut Vec<(u8, String, String)>, board: &mut Vec<u8>, pos: &mut usize, out: &mut Vec<serde_json::Value>) {
        let ctx = self.ctx;
        match n {
            Node3::Action(a) => {
                let hp = a.player as usize; let nh = ctx.hands[hp].len();
                let line_p = mass(ctx, &reach) / self.root_mass;
                let strat = avg_strategy(a, nh);
                let flop_vals = if a.street == 0 { self.flopev.as_ref().map(|v| { let len = a.actions.len() * nh; let s = v[*pos..*pos + len].to_vec(); *pos += len; s }) } else { None };
                if line_p >= self.min_p && (a.street > 0 || flop_vals.is_some()) {
                    out.push(self.spot(a, &reach, alive, hist, board, line_p, &strat, flop_vals.as_deref(), out.len()));
                }
                for (ai, ch) in a.children.iter().enumerate() {
                    let mut r = reach.clone();
                    for h in 0..nh { r[hp][h] *= strat[ai * nh + h]; }
                    let mut al = alive; if matches!(a.actions[ai], Act::Fold) { al[hp] = false; }
                    hist.push((a.street, self.seats[hp].clone(), act_label(&a.actions[ai]).0));
                    self.walk(ch, r, al, hist, board, pos, out);
                    hist.pop();
                }
            }
            Node3::Chance { cards, children, .. } => {
                for (c, ch) in cards.iter().zip(children) {
                    let mut r = reach.clone();
                    for p in 0..3 { for (i, h) in ctx.hands[p].iter().enumerate() { if h.0 == *c || h.1 == *c { r[p][i] = 0.0; } } }
                    board.push(*c);
                    self.walk(ch, r, alive, hist, board, pos, out);
                    board.pop();
                }
            }
            _ => {}
        }
    }

    fn spot(&self, a: &ActionNode3, reach: &Reach, alive: [bool; 3], hist: &[(u8, String, String)], board: &[u8], line_p: f32, strat: &[f32], flop_vals: Option<&[f32]>, k: usize) -> serde_json::Value {
        let ctx = self.ctx;
        let hp = a.player as usize; let nh = ctx.hands[hp].len(); let na = a.actions.len();
        let o = others(hp);
        let vm = ctx.flat_value(hp, o, [&reach[o[0]], &reach[o[1]]], 1.0);
        let evs: Vec<Vec<f32>> = (0..na).map(|x| {
            let cfv: Vec<f32> = match (flop_vals, &a.actions[x]) {
                (Some(v), _) => v[x * nh..(x + 1) * nh].to_vec(),
                (None, Act::Fold) => ctx.flat_value(hp, o, [&reach[o[0]], &reach[o[1]]], -a.commit[hp]),
                (None, _) => walk3(ctx, &a.children[x], hp, reach, Mode::Average),
            };
            (0..nh).map(|h| if vm[h] > 1e-6 { cfv[h] / vm[h] + a.commit[hp] } else { 0.0 }).collect()
        }).collect();
        let pot = ctx.start_pot + a.commit.iter().sum::<f32>();
        let max_commit = a.commit.iter().cloned().fold(0f32, f32::max);
        let to_call = max_commit - a.commit[hp];
        let mx = reach[hp].iter().cloned().fold(0f32, f32::max).max(1e-9);
        let drills: Vec<serde_json::Value> = (0..nh).filter(|&h| reach[hp][h] / mx >= 0.02 && vm[h] > 1e-6 && !board.contains(&ctx.hands[hp][h].0) && !board.contains(&ctx.hands[hp][h].1)).map(|h| {
            let hd = ctx.hands[hp][h];
            serde_json::json!({ "hand": [card_str(hd.0), card_str(hd.1)], "cls": classify(hd, board), "w": r3(reach[hp][h] / mx),
                "strat": (0..na).map(|x| r3(strat[x * nh + h])).collect::<Vec<_>>(), "ev": (0..na).map(|x| r3(evs[x][h])).collect::<Vec<_>>() })
        }).collect();
        serde_json::json!({
            "id": format!("{}-{}", self.id, k), "seats": self.seats, "hero": hp, "alive": alive,
            "board": board.iter().map(|&c| card_str(c)).collect::<Vec<_>>(), "street": a.street,
            "pot": r3(pot), "stack": r3(self.stack - max_commit), "to_call": r3(to_call), "line_p": r3(line_p),
            "history": hist.iter().map(|(s, p, l)| serde_json::json!({ "street": s, "pos": p, "label": l })).collect::<Vec<_>>(),
            "actions": a.actions.iter().map(|x| { let (label, kind, amt) = act_label(x); serde_json::json!({ "label": label, "kind": kind, "amt": r3(amt) }) }).collect::<Vec<_>>(),
            "ranges": (0..3).map(|p| grid(&ctx.hands[p], &reach[p], board)).collect::<Vec<_>>(),
            "drills": drills,
        })
    }
}
