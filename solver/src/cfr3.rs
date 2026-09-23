//! Vector DCFR for three seats. One traversal per seat per iteration; `reach` carries the other seats'
//! reach vectors (the traverser's own slot is unused). After the traverser folds, its value is flat, so
//! the subtree is not entered.

use rayon::prelude::*;
use crate::cfr::{Discount, Mode};
use crate::three::{Ctx3, Pay};
use crate::tree3::{ActionNode3, Node3};

pub type Reach = [Vec<f32>; 3];

fn others(t: usize) -> [usize; 2] { [(t + 1) % 3, (t + 2) % 3] }

fn pay(ctx: &Ctx3, commit: &[f32; 3], t: usize) -> Pay {
    let pot = ctx.start_pot + commit.iter().sum::<f32>();
    Pay { win: pot - commit[t], tie2: pot / 2.0 - commit[t], tie3: pot / 3.0 - commit[t], lose: -commit[t] }
}

fn terminal(ctx: &Ctx3, node: &Node3, t: usize, reach: &Reach) -> Vec<f32> {
    let o = others(t);
    let r = [&reach[o[0]][..], &reach[o[1]][..]];
    match node {
        Node3::FoldWin { winner, commit } => { let p = pay(ctx, commit, t); ctx.flat_value(t, o, r, if *winner as usize == t { p.win } else { p.lose }) }
        Node3::Showdown3 { commit, board_id } => ctx.showdown3_value(*board_id, t, o, r, pay(ctx, commit, t)),
        Node3::Showdown2 { alive, dead, commit, board_id } => {
            let p = pay(ctx, commit, t);
            if *dead as usize == t { return ctx.flat_value(t, o, r, p.lose); }
            let opp = if alive[0] as usize == t { alive[1] } else { alive[0] } as usize;
            ctx.showdown2_value(*board_id, t, [opp, *dead as usize], [&reach[opp], &reach[*dead as usize]], p)
        }
        _ => unreachable!(),
    }
}

fn regret_match(regrets: &[f32], na: usize, n: usize) -> Vec<f32> {
    let mut s = vec![0f32; na * n];
    for h in 0..n {
        let mut sum = 0f32; for a in 0..na { sum += regrets[a * n + h].max(0.0); }
        for a in 0..na { s[a * n + h] = if sum > 0.0 { regrets[a * n + h].max(0.0) / sum } else { 1.0 / na as f32 }; }
    }
    s
}
pub fn avg_strategy(a: &ActionNode3, n: usize) -> Vec<f32> {
    let na = a.actions.len();
    let mut s = vec![0f32; na * n];
    for h in 0..n {
        let mut sum = 0f32; for x in 0..na { sum += a.strat_sum[x * n + h]; }
        for x in 0..na { s[x * n + h] = if sum > 0.0 { a.strat_sum[x * n + h] / sum } else { 1.0 / na as f32 }; }
    }
    s
}

fn deal(ctx: &Ctx3, t: usize, reach: &Reach, c: u8) -> Reach {
    let mut r = reach.clone();
    for &o in &others(t) { for &i in &ctx.card_hands[o][c as usize] { r[o][i as usize] = 0.0; } }
    r
}
fn sum_chance(ctx: &Ctx3, t: usize, cards: &[u8], vals: Vec<Vec<f32>>) -> Vec<f32> {
    let mut out = vec![0f32; ctx.hands[t].len()];
    let scale = 1.0 / (cards.len() as f32 - 6.0);
    for (v, &c) in vals.iter().zip(cards) {
        for (x, y) in out.iter_mut().zip(v) { *x += y * scale; }
        for &i in &ctx.card_hands[t][c as usize] { out[i as usize] -= v[i as usize] * scale; }
    }
    out
}
fn dead(reach: &Reach, t: usize) -> bool { others(t).iter().any(|&o| reach[o].iter().all(|&x| x == 0.0)) }

pub fn cfr3(ctx: &Ctx3, node: &mut Node3, t: usize, reach: &Reach, d: &Discount) -> Vec<f32> {
    if dead(reach, t) { return vec![0.0; ctx.hands[t].len()]; }
    match node {
        Node3::Chance { cards, children, par } => {
            let vals: Vec<Vec<f32>> = if *par {
                children.par_iter_mut().zip(cards.par_iter()).map(|(ch, &c)| cfr3(ctx, ch, t, &deal(ctx, t, reach, c), d)).collect()
            } else {
                children.iter_mut().zip(cards.iter()).map(|(ch, &c)| cfr3(ctx, ch, t, &deal(ctx, t, reach, c), d)).collect()
            };
            sum_chance(ctx, t, cards, vals)
        }
        Node3::Action(a) => {
            let (na, p) = (a.actions.len(), a.player as usize);
            let n = ctx.hands[p].len();
            let strat = regret_match(&a.regrets, na, n);
            if p == t {
                let mut util = vec![0f32; n];
                let mut cfvs = Vec::with_capacity(na);
                for (ai, ch) in a.children.iter_mut().enumerate() {
                    let v = if matches!(a.actions[ai], crate::tree::Act::Fold) {
                        let o = others(t); ctx.flat_value(t, o, [&reach[o[0]], &reach[o[1]]], -a.commit[t])
                    } else { cfr3(ctx, ch, t, reach, d) };
                    for h in 0..n { util[h] += strat[ai * n + h] * v[h]; }
                    cfvs.push(v);
                }
                for ai in 0..na { for h in 0..n { let r = &mut a.regrets[ai * n + h]; *r = *r * if *r > 0.0 { d.pos } else { d.neg } + cfvs[ai][h] - util[h]; } }
                util
            } else {
                // both other traversers accumulate here (a folded traverser never enters); the factor of two
                // in nodes both can reach is constant per node and drops out when the average is normalised
                for ai in 0..na { for h in 0..n { let s = &mut a.strat_sum[ai * n + h]; *s = *s * d.strat + reach[p][h] * strat[ai * n + h]; } }
                let mut out = vec![0f32; ctx.hands[t].len()];
                for (ai, ch) in a.children.iter_mut().enumerate() {
                    let mut r = reach.clone();
                    for h in 0..n { r[p][h] = reach[p][h] * strat[ai * n + h]; }
                    let v = cfr3(ctx, ch, t, &r, d);
                    for (x, y) in out.iter_mut().zip(&v) { *x += y; }
                }
                out
            }
        }
        term => terminal(ctx, term, t, reach),
    }
}

/// Values for `t` when the others play their average strategies and `t` plays a best response or its average.
pub fn walk3(ctx: &Ctx3, node: &Node3, t: usize, reach: &Reach, mode: Mode) -> Vec<f32> {
    if dead(reach, t) { return vec![0.0; ctx.hands[t].len()]; }
    match node {
        Node3::Chance { cards, children, par } => {
            let vals: Vec<Vec<f32>> = if *par {
                children.par_iter().zip(cards.par_iter()).map(|(ch, &c)| walk3(ctx, ch, t, &deal(ctx, t, reach, c), mode)).collect()
            } else {
                children.iter().zip(cards.iter()).map(|(ch, &c)| walk3(ctx, ch, t, &deal(ctx, t, reach, c), mode)).collect()
            };
            sum_chance(ctx, t, cards, vals)
        }
        Node3::Action(a) => {
            let (na, p) = (a.actions.len(), a.player as usize);
            let n = ctx.hands[p].len();
            let strat = avg_strategy(a, n);
            if p == t {
                let vals: Vec<Vec<f32>> = a.children.iter().enumerate().map(|(ai, ch)| if matches!(a.actions[ai], crate::tree::Act::Fold) {
                    let o = others(t); ctx.flat_value(t, o, [&reach[o[0]], &reach[o[1]]], -a.commit[t]) } else { walk3(ctx, ch, t, reach, mode) }).collect();
                (0..n).map(|h| if mode == Mode::BestResponse { (0..na).map(|ai| vals[ai][h]).fold(f32::MIN, f32::max) } else { (0..na).map(|ai| strat[ai * n + h] * vals[ai][h]).sum() }).collect()
            } else {
                let mut out = vec![0f32; ctx.hands[t].len()];
                for (ai, ch) in a.children.iter().enumerate() {
                    let mut r = reach.clone();
                    for h in 0..n { r[p][h] = reach[p][h] * strat[ai * n + h]; }
                    let v = walk3(ctx, ch, t, &r, mode);
                    for (x, y) in out.iter_mut().zip(&v) { *x += y; }
                }
                out
            }
        }
        term => terminal(ctx, term, t, reach),
    }
}

/// (mean best-response gain over the three seats in chips per hand dealt, EV per seat under the average strategy)
pub fn exploitability3(ctx: &Ctx3, root: &Node3) -> (f32, [f32; 3]) {
    let mut gain = 0f64; let mut ev = [0f32; 3];
    for t in 0..3 {
        let reach: Reach = [ctx.weights[0].clone(), ctx.weights[1].clone(), ctx.weights[2].clone()];
        let o = others(t);
        let mass = ctx.flat_value(t, o, [&reach[o[0]], &reach[o[1]]], 1.0);
        let z: f64 = mass.iter().zip(&ctx.weights[t]).map(|(m, w)| (*m * *w) as f64).sum();
        let v: Vec<f64> = [Mode::BestResponse, Mode::Average].iter().map(|&m| walk3(ctx, root, t, &reach, m).iter().zip(&ctx.weights[t]).map(|(c, w)| (*c * *w) as f64).sum::<f64>() / z).collect();
        gain += v[0] - v[1]; ev[t] = v[1] as f32;
    }
    ((gain / 3.0) as f32, ev)
}
