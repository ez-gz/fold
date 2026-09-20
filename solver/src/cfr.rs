//! Vector-form DCFR over the full (unabstracted) hand ranges, plus best response / EV walks.

use crate::eval::eval;
use crate::tree::{ActionNode, Node};
use rayon::prelude::*;

pub const NONE: u32 = u32::MAX;

pub struct Ctx {
    pub hands: [Vec<(u8, u8)>; 2],
    pub weights: [Vec<f32>; 2],
    /// index of the identical combo in the other player's hand list
    pub same: [Vec<u32>; 2],
    /// [player][card] -> hand indices containing that card
    pub card_hands: [Vec<Vec<u32>>; 2],
    /// per river board: [player] -> (strength, hand idx) ascending, board-conflicting hands removed
    pub sd: Vec<[Vec<(u32, u16)>; 2]>,
    pub start_pot: f32,
    pub rake_pct: f32,
    pub rake_cap: f32,
}

impl Ctx {
    pub fn new(hands: [Vec<(u8, u8)>; 2], weights: [Vec<f32>; 2], boards: &[Vec<u8>], start_pot: f32, rake_pct: f32, rake_cap: f32) -> Ctx {
        let mut same = [Vec::new(), Vec::new()];
        let mut card_hands = [vec![Vec::new(); 52], vec![Vec::new(); 52]];
        for p in 0..2 {
            same[p] = hands[p].iter().map(|h| hands[1 - p].iter().position(|g| g == h).map_or(NONE, |i| i as u32)).collect();
            for (i, h) in hands[p].iter().enumerate() {
                card_hands[p][h.0 as usize].push(i as u32);
                card_hands[p][h.1 as usize].push(i as u32);
            }
        }
        let sd: Vec<[Vec<(u32, u16)>; 2]> = boards.par_iter().map(|b| {
            let table = |p: usize| {
                let mut v: Vec<(u32, u16)> = hands[p].iter().enumerate()
                    .filter(|(_, h)| !b.contains(&h.0) && !b.contains(&h.1))
                    .map(|(i, h)| {
                        let mut c = [0u8; 7];
                        c[..5].copy_from_slice(b);
                        c[5] = h.0; c[6] = h.1;
                        (eval(&c), i as u16)
                    }).collect();
                v.sort_unstable();
                v
            };
            [table(0), table(1)]
        }).collect();
        Ctx { hands, weights, same, card_hands, sd, start_pot, rake_pct, rake_cap }
    }

    fn rake(&self, pot: f32) -> f32 {
        (pot * self.rake_pct).min(self.rake_cap)
    }

    /// For each hand of `t`: total opponent reach that does not share a card with it.
    pub fn valid_mass(&self, t: usize, reach: &[f32]) -> Vec<f32> {
        let o = 1 - t;
        let mut cs = [0f32; 52];
        let mut total = 0f32;
        for (i, h) in self.hands[o].iter().enumerate() {
            let w = reach[i];
            total += w;
            cs[h.0 as usize] += w;
            cs[h.1 as usize] += w;
        }
        self.hands[t].iter().enumerate().map(|(i, h)| {
            let s = self.same[t][i];
            let sw = if s == NONE { 0.0 } else { reach[s as usize] };
            total - cs[h.0 as usize] - cs[h.1 as usize] + sw
        }).collect()
    }

    fn fold_value(&self, folder: usize, commit: &[f32; 2], t: usize, reach: &[f32]) -> Vec<f32> {
        let pay = if folder == t {
            -commit[t]
        } else {
            self.start_pot + commit[folder] - self.rake(self.start_pot + 2.0 * commit[folder])
        };
        let mut v = self.valid_mass(t, reach);
        for x in v.iter_mut() { *x *= pay; }
        v
    }

    fn showdown_value(&self, commit: f32, board_id: usize, t: usize, reach: &[f32]) -> Vec<f32> {
        let o = 1 - t;
        let (tt, to) = (&self.sd[board_id][t], &self.sd[board_id][o]);
        let pot = self.start_pot + 2.0 * commit;
        let rake = self.rake(pot);
        let (win, lose, tie) = (pot - rake - commit, -commit, (pot - rake) / 2.0 - commit);
        let mut out = vec![0f32; self.hands[t].len()];
        let mut wins = vec![0f32; self.hands[t].len()];
        // ascending: mass of strictly weaker opponent hands
        let (mut j, mut sum, mut cs) = (0usize, 0f32, [0f32; 52]);
        for &(s, hi) in tt.iter() {
            while j < to.len() && to[j].0 < s {
                let ho = to[j].1 as usize;
                let (w, h) = (reach[ho], self.hands[o][ho]);
                sum += w; cs[h.0 as usize] += w; cs[h.1 as usize] += w;
                j += 1;
            }
            let h = self.hands[t][hi as usize];
            wins[hi as usize] = sum - cs[h.0 as usize] - cs[h.1 as usize];
        }
        // descending: mass of strictly stronger opponent hands
        let (mut j, mut sum, mut cs) = (to.len(), 0f32, [0f32; 52]);
        let mut total_cs = [0f32; 52];
        let mut total = 0f32;
        for &(_, ho) in to.iter() {
            let (w, h) = (reach[ho as usize], self.hands[o][ho as usize]);
            total += w; total_cs[h.0 as usize] += w; total_cs[h.1 as usize] += w;
        }
        for &(s, hi) in tt.iter().rev() {
            while j > 0 && to[j - 1].0 > s {
                let ho = to[j - 1].1 as usize;
                let (w, h) = (reach[ho], self.hands[o][ho]);
                sum += w; cs[h.0 as usize] += w; cs[h.1 as usize] += w;
                j -= 1;
            }
            let h = self.hands[t][hi as usize];
            let l = sum - cs[h.0 as usize] - cs[h.1 as usize];
            let sm = self.same[t][hi as usize];
            let sw = if sm == NONE { 0.0 } else { reach[sm as usize] };
            let valid = total - total_cs[h.0 as usize] - total_cs[h.1 as usize] + sw;
            let w = wins[hi as usize];
            out[hi as usize] = w * win + l * lose + (valid - w - l) * tie;
        }
        out
    }
}

pub struct Discount { pub pos: f32, pub neg: f32, pub strat: f32 }

impl Discount {
    pub fn at(iter: u32) -> Discount {
        let t = iter as f32;
        let a = t.powf(1.5);
        Discount { pos: a / (a + 1.0), neg: 0.5, strat: (t / (t + 1.0)).powi(2) }
    }
}

fn regret_match(regrets: &[f32], na: usize, n: usize) -> Vec<f32> {
    let mut s = vec![0f32; na * n];
    for h in 0..n {
        let mut sum = 0f32;
        for a in 0..na { sum += regrets[a * n + h].max(0.0); }
        for a in 0..na {
            s[a * n + h] = if sum > 0.0 { regrets[a * n + h].max(0.0) / sum } else { 1.0 / na as f32 };
        }
    }
    s
}

pub fn avg_strategy(a: &ActionNode, n: usize) -> Vec<f32> {
    let na = a.actions.len();
    let mut s = vec![0f32; na * n];
    for h in 0..n {
        let mut sum = 0f32;
        for x in 0..na { sum += a.strat_sum[x * n + h]; }
        for x in 0..na {
            s[x * n + h] = if sum > 0.0 { a.strat_sum[x * n + h] / sum } else { 1.0 / na as f32 };
        }
    }
    s
}

fn deal(ctx: &Ctx, o: usize, reach: &[f32], c: u8) -> Vec<f32> {
    let mut r = reach.to_vec();
    for &i in &ctx.card_hands[o][c as usize] { r[i as usize] = 0.0; }
    r
}

fn sum_chance(ctx: &Ctx, t: usize, cards: &[u8], vals: Vec<Vec<f32>>) -> Vec<f32> {
    let mut out = vec![0f32; ctx.hands[t].len()];
    let scale = 1.0 / (cards.len() as f32 - 4.0);
    for (v, &c) in vals.iter().zip(cards) {
        for (x, y) in out.iter_mut().zip(v) { *x += y * scale; }
        for &i in &ctx.card_hands[t][c as usize] { out[i as usize] -= v[i as usize] * scale; }
    }
    out
}

pub fn cfr(ctx: &Ctx, node: &mut Node, t: usize, reach: &[f32], d: &Discount) -> Vec<f32> {
    if reach.iter().all(|&x| x == 0.0) {
        return vec![0.0; ctx.hands[t].len()];
    }
    match node {
        Node::Fold { folder, commit } => ctx.fold_value(*folder as usize, commit, t, reach),
        Node::Showdown { commit, board_id } => ctx.showdown_value(*commit, *board_id, t, reach),
        Node::Chance { cards, children, par } => {
            let o = 1 - t;
            let vals: Vec<Vec<f32>> = if *par {
                children.par_iter_mut().zip(cards.par_iter()).map(|(ch, &c)| cfr(ctx, ch, t, &deal(ctx, o, reach, c), d)).collect()
            } else {
                children.iter_mut().zip(cards.iter()).map(|(ch, &c)| cfr(ctx, ch, t, &deal(ctx, o, reach, c), d)).collect()
            };
            sum_chance(ctx, t, cards, vals)
        }
        Node::Action(a) => {
            let (na, p) = (a.actions.len(), a.player as usize);
            let n = ctx.hands[p].len();
            let strat = regret_match(&a.regrets, na, n);
            if p == t {
                let mut util = vec![0f32; n];
                let mut cfvs = Vec::with_capacity(na);
                for (ai, ch) in a.children.iter_mut().enumerate() {
                    let v = cfr(ctx, ch, t, reach, d);
                    for h in 0..n { util[h] += strat[ai * n + h] * v[h]; }
                    cfvs.push(v);
                }
                for ai in 0..na {
                    for h in 0..n {
                        let r = &mut a.regrets[ai * n + h];
                        *r = *r * if *r > 0.0 { d.pos } else { d.neg } + cfvs[ai][h] - util[h];
                    }
                }
                util
            } else {
                for ai in 0..na {
                    for h in 0..n {
                        let s = &mut a.strat_sum[ai * n + h];
                        *s = *s * d.strat + reach[h] * strat[ai * n + h];
                    }
                }
                let mut out = vec![0f32; ctx.hands[t].len()];
                for (ai, ch) in a.children.iter_mut().enumerate() {
                    let r: Vec<f32> = (0..n).map(|h| reach[h] * strat[ai * n + h]).collect();
                    let v = cfr(ctx, ch, t, &r, d);
                    for (x, y) in out.iter_mut().zip(&v) { *x += y; }
                }
                out
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Mode { BestResponse, Average }

/// Counterfactual values for `t` when the opponent plays the average strategy and `t` plays
/// either a best response or its own average strategy.
pub fn walk(ctx: &Ctx, node: &Node, t: usize, reach: &[f32], mode: Mode) -> Vec<f32> {
    if reach.iter().all(|&x| x == 0.0) {
        return vec![0.0; ctx.hands[t].len()];
    }
    match node {
        Node::Fold { folder, commit } => ctx.fold_value(*folder as usize, commit, t, reach),
        Node::Showdown { commit, board_id } => ctx.showdown_value(*commit, *board_id, t, reach),
        Node::Chance { cards, children, par } => {
            let o = 1 - t;
            let vals: Vec<Vec<f32>> = if *par {
                children.par_iter().zip(cards.par_iter()).map(|(ch, &c)| walk(ctx, ch, t, &deal(ctx, o, reach, c), mode)).collect()
            } else {
                children.iter().zip(cards.iter()).map(|(ch, &c)| walk(ctx, ch, t, &deal(ctx, o, reach, c), mode)).collect()
            };
            sum_chance(ctx, t, cards, vals)
        }
        Node::Action(a) => {
            let (na, p) = (a.actions.len(), a.player as usize);
            let n = ctx.hands[p].len();
            let strat = avg_strategy(a, n);
            if p == t {
                let vals: Vec<Vec<f32>> = a.children.iter().map(|ch| walk(ctx, ch, t, reach, mode)).collect();
                let mut out = vec![0f32; n];
                for h in 0..n {
                    out[h] = if mode == Mode::BestResponse {
                        (0..na).map(|ai| vals[ai][h]).fold(f32::MIN, f32::max)
                    } else {
                        (0..na).map(|ai| strat[ai * n + h] * vals[ai][h]).sum()
                    };
                }
                out
            } else {
                let mut out = vec![0f32; ctx.hands[t].len()];
                for (ai, ch) in a.children.iter().enumerate() {
                    let r: Vec<f32> = (0..n).map(|h| reach[h] * strat[ai * n + h]).collect();
                    let v = walk(ctx, ch, t, &r, mode);
                    for (x, y) in out.iter_mut().zip(&v) { *x += y; }
                }
                out
            }
        }
    }
}

/// (exploitability in chips, EV of player 0, EV of player 1), all per hand dealt.
pub fn exploitability(ctx: &Ctx, root: &Node) -> (f32, f32, f32) {
    let mut v = [[0f64; 2]; 2];
    let mut z = 0f64;
    for t in 0..2 {
        let reach = &ctx.weights[1 - t];
        let mass = ctx.valid_mass(t, reach);
        z = mass.iter().zip(&ctx.weights[t]).map(|(m, w)| (*m * *w) as f64).sum();
        for (k, mode) in [Mode::BestResponse, Mode::Average].into_iter().enumerate() {
            let cfv = walk(ctx, root, t, reach, mode);
            v[t][k] = cfv.iter().zip(&ctx.weights[t]).map(|(c, w)| (*c * *w) as f64).sum::<f64>() / z;
        }
    }
    let _ = z;
    (((v[0][0] - v[0][1]) + (v[1][0] - v[1][1])) as f32 / 2.0, v[0][1] as f32, v[1][1] as f32)
}
