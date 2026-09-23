//! Three-player terminal values in vector form with exact card removal.
//!
//! For hero hand t and two other players with reach vectors r1, r2, every terminal value is a sum over
//! ordered pairs (h1, h2) with h1, h2 and t pairwise card-disjoint, weighted r1(h1) r2(h2), times a payoff
//! that depends only on which strength band (weaker / equal / stronger than t, or "any") each hand is in.
//! Mass of disjoint pairs from two bands = A1 A2 - sum_c C1[c] C2[c] + sum_h w1(h) w2(h) (inclusion-exclusion
//! over shared cards: a pair conflicts iff it shares a card; identical combos are counted twice by the
//! per-card sum). Restricting to hands that avoid t's two cards is the same correction one level down.

use crate::eval::eval;
use rayon::prelude::*;


/// Accumulated mass of one player's hands in one strength band.
#[derive(Clone)]
pub struct Acc {
    pub a: f32,
    pub c: [f32; 52],
    /// weight of the exact combo (x, y), symmetric
    pub wk: Vec<f32>,
    hs: Vec<((u8, u8), f32)>,
}
impl Acc {
    fn new() -> Acc { Acc { a: 0.0, c: [0.0; 52], wk: vec![0.0; 52 * 52], hs: Vec::new() } }
    fn clear(&mut self) { self.a = 0.0; self.c = [0.0; 52]; for (h, _) in self.hs.drain(..) { self.wk[h.0 as usize * 52 + h.1 as usize] = 0.0; self.wk[h.1 as usize * 52 + h.0 as usize] = 0.0; } }
    fn add(&mut self, h: (u8, u8), w: f32) {
        self.a += w; self.c[h.0 as usize] += w; self.c[h.1 as usize] += w;
        self.wk[h.0 as usize * 52 + h.1 as usize] += w; self.wk[h.1 as usize * 52 + h.0 as usize] += w; self.hs.push((h, w));
    }
    #[inline] fn w(&self, x: usize, y: usize) -> f32 { self.wk[x * 52 + y] }
}

/// Mass of ordered pairs (h1 in acc1, h2 in acc2) that are card-disjoint from each other and from hero's (a, b).
/// `ident` = sum over identical combos h of p.w(h) q.w(h); see `ident`.
pub fn pair_mass(p: &Acc, q: &Acc, ident: f32, a: usize, b: usize) -> f32 {
    let pa = p.a - p.c[a] - p.c[b] + p.w(a, b);
    let qa = q.a - q.c[a] - q.c[b] + q.w(a, b);
    let mut x = 0f32; // sum over cards c (not a, b) of restricted per-card masses
    let mut i = 0f32; // identical combos avoiding a and b
    for c in 0..52 {
        if c == a || c == b { continue; }
        let pc = p.c[c] - p.w(c, a) - p.w(c, b);
        let qc = q.c[c] - q.w(c, a) - q.w(c, b);
        x += pc * qc;
        i += p.w(c, a) * q.w(c, a) + p.w(c, b) * q.w(c, b); // identical combos that touch a or b, to subtract
    }
    pa * qa - x + (ident - i - p.w(a, b) * q.w(a, b))
}
pub fn ident(p: &Acc, q: &Acc) -> f32 {
    let (s, o) = if p.hs.len() <= q.hs.len() { (p, q) } else { (q, p) };
    s.hs.iter().map(|(h, w)| w * o.w(h.0 as usize, h.1 as usize)).sum()
}

pub struct Ctx3 {
    pub hands: [Vec<(u8, u8)>; 3],
    /// per river board, per player: (strength, hand idx) ascending, board-conflicting hands removed
    pub sd: Vec<[Vec<(u32, u16)>; 3]>,
    pub start_pot: f32,
}

/// Payoffs for hero, per outcome band of the two others.
#[derive(Clone, Copy)]
pub struct Pay {
    pub win: f32,      // hero best
    pub tie2: f32,     // hero ties one, beats the other
    pub tie3: f32,     // all three tie
    pub lose: f32,     // someone beats hero
}

impl Ctx3 {
    pub fn new(hands: [Vec<(u8, u8)>; 3], boards: &[Vec<u8>], start_pot: f32) -> Ctx3 {
        let sd = boards.par_iter().map(|b| {
            let table = |p: usize| {
                let mut v: Vec<(u32, u16)> = hands[p].iter().enumerate()
                    .filter(|(_, h)| !b.contains(&h.0) && !b.contains(&h.1))
                    .map(|(i, h)| { let mut c = [0u8; 7]; c[..5].copy_from_slice(b); c[5] = h.0; c[6] = h.1; (eval(&c), i as u16) }).collect();
                v.sort_unstable(); v
            };
            [table(0), table(1), table(2)]
        }).collect();
        Ctx3 { hands, sd, start_pot }
    }

    fn total(&self, p: usize, reach: &[f32], board: Option<&[u8]>) -> Acc {
        let mut t = Acc::new();
        for (i, h) in self.hands[p].iter().enumerate() {
            if let Some(b) = board { if b.contains(&h.0) || b.contains(&h.1) { continue; } }
            if reach[i] != 0.0 { t.add(*h, reach[i]); }
        }
        t
    }

    /// Terminal where no showdown happens: hero gets `pay` against every valid (h1, h2) pair.
    pub fn flat_value(&self, t: usize, others: [usize; 2], reach: [&[f32]; 2], pay: f32) -> Vec<f32> {
        let (p, q) = (self.total(others[0], reach[0], None), self.total(others[1], reach[1], None)); let id = ident(&p, &q);
        self.hands[t].iter().map(|h| pay * pair_mass(&p, &q, id, h.0 as usize, h.1 as usize)).collect()
    }

    /// Showdown between hero and `others[0]`; `others[1]` folded earlier but still removes cards.
    pub fn showdown2_value(&self, board_id: usize, t: usize, others: [usize; 2], reach: [&[f32]; 2], pay: Pay) -> Vec<f32> {
        self.showdown_generic(board_id, t, others, reach, pay, false)
    }
    /// Three-way showdown.
    pub fn showdown3_value(&self, board_id: usize, t: usize, others: [usize; 2], reach: [&[f32]; 2], pay: Pay) -> Vec<f32> {
        self.showdown_generic(board_id, t, others, reach, pay, true)
    }

    fn showdown_generic(&self, board_id: usize, t: usize, others: [usize; 2], reach: [&[f32]; 2], pay: Pay, three: bool) -> Vec<f32> {
        let tt = &self.sd[board_id][t];
        let tabs = [&self.sd[board_id][others[0]], &self.sd[board_id][others[1]]];
        let n_bands = if three { 2 } else { 1 };
        // per opponent: weaker (prefix), equal (band), and total
        let mut wk = [Acc::new(), Acc::new()];
        let mut eq = [Acc::new(), Acc::new()];
        let mut tot = [Acc::new(), Acc::new()];
        for o in 0..2 { for &(_, hi) in tabs[o].iter() { let w = reach[o][hi as usize]; if w != 0.0 { tot[o].add(self.hands[others[o]][hi as usize], w); } } }
        let mut out = vec![0f32; self.hands[t].len()];
        let mut ptr = [0usize; 2];
        let mut k = 0;
        while k < tt.len() {
            let s = tt[k].0;
            let mut k2 = k; while k2 < tt.len() && tt[k2].0 == s { k2 += 1; }
            for o in 0..n_bands {
                // move everything strictly weaker than s into wk, rebuild eq with the equal band
                while ptr[o] < tabs[o].len() && tabs[o][ptr[o]].0 < s {
                    let hi = tabs[o][ptr[o]].1 as usize; let w = reach[o][hi]; if w != 0.0 { wk[o].add(self.hands[others[o]][hi], w); } ptr[o] += 1;
                }
                eq[o].clear();
                let mut j = ptr[o];
                while j < tabs[o].len() && tabs[o][j].0 == s { let hi = tabs[o][j].1 as usize; let w = reach[o][hi]; if w != 0.0 { eq[o].add(self.hands[others[o]][hi], w); } j += 1; }
            }
            let i_tt = ident(&tot[0], &tot[1]);
            let (i_ww, i_ee, i_we, i_ew) = if three { (ident(&wk[0], &wk[1]), ident(&eq[0], &eq[1]), ident(&wk[0], &eq[1]), ident(&eq[0], &wk[1])) } else { (0.0, 0.0, 0.0, 0.0) };
            let (i_wt, i_et) = if three { (0.0, 0.0) } else { (ident(&wk[0], &tot[1]), ident(&eq[0], &tot[1])) };
            for &(_, hi) in &tt[k..k2] {
                let h = self.hands[t][hi as usize]; let (a, b) = (h.0 as usize, h.1 as usize);
                let v = if three {
                    // union band "not stronger" = weaker + equal, built by summing pair masses:
                    // pairs(N1,N2) = pairs(W1,W2) + pairs(W1,E2) + pairs(E1,W2) + pairs(E1,E2)
                    let valid = pair_mass(&tot[0], &tot[1], i_tt, a, b);
                    let win = pair_mass(&wk[0], &wk[1], i_ww, a, b);
                    let eqeq = pair_mass(&eq[0], &eq[1], i_ee, a, b);
                    let eqw = pair_mass(&wk[0], &eq[1], i_we, a, b) + pair_mass(&eq[0], &wk[1], i_ew, a, b);
                    let lose = valid - win - eqeq - eqw;
                    win * pay.win + eqw * pay.tie2 + eqeq * pay.tie3 + lose * pay.lose
                } else {
                    let valid = pair_mass(&tot[0], &tot[1], i_tt, a, b);
                    let win = pair_mass(&wk[0], &tot[1], i_wt, a, b);
                    let tie = pair_mass(&eq[0], &tot[1], i_et, a, b);
                    win * pay.win + tie * pay.tie2 + (valid - win - tie) * pay.lose
                };
                out[hi as usize] = v;
            }
            k = k2;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn disjoint(x: (u8, u8), y: (u8, u8)) -> bool { x.0 != y.0 && x.0 != y.1 && x.1 != y.0 && x.1 != y.1 }
    fn lcg(s: &mut u64) -> f32 { *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); ((*s >> 33) as f32) / (1u64 << 31) as f32 }
    fn setup() -> (Ctx3, [Vec<f32>; 3], Vec<u8>) {
        let mut all = Vec::new(); for a in 0..52u8 { for b in a + 1..52 { all.push((a, b)); } }
        let mut s = 7u64;
        let pick = |s: &mut u64, n: usize| -> Vec<(u8, u8)> { let mut v: Vec<(u8, u8)> = all.iter().cloned().filter(|_| lcg(s) < 0.4).collect(); v.truncate(n); v };
        let hands = [pick(&mut s, 220), pick(&mut s, 200), pick(&mut s, 180)];
        let board = vec![3u8, 17, 30, 44, 8]; // 2s 6d 9h Kc 4s
        let reach = [0, 1, 2].map(|p| hands[p].iter().map(|_| lcg(&mut s)).collect::<Vec<f32>>());
        (Ctx3::new(hands, &[board.clone()], 10.0), reach, board)
    }
    fn strength(b: &[u8], h: (u8, u8)) -> u32 { let mut c = [0u8; 7]; c[..5].copy_from_slice(b); c[5] = h.0; c[6] = h.1; eval(&c) }
    fn close(x: &[f32], y: &[f32]) { for (i, (p, q)) in x.iter().zip(y).enumerate() { assert!((p - q).abs() <= 1e-3 * (1.0 + q.abs()), "hand {i}: {p} vs {q}"); } }

    #[test]
    fn showdown3_timing() {
        let mut all = Vec::new(); for a in 0..52u8 { for b in a + 1..52 { all.push((a, b)); } }
        let hands = [all.clone(), all.clone(), all.clone()];
        let board = vec![3u8, 17, 30, 44, 8];
        let ctx = Ctx3::new(hands, &[board], 10.0);
        let r: Vec<f32> = (0..1326).map(|i| 0.5 + (i % 7) as f32 * 0.1).collect();
        let pay = Pay { win: 7.0, tie2: 2.0, tie3: 0.5, lose: -3.0 };
        let t0 = std::time::Instant::now();
        for _ in 0..10 { ctx.showdown3_value(0, 0, [1, 2], [&r, &r], pay); }
        eprintln!("showdown3 full 1326x3: {:.2} ms each", t0.elapsed().as_secs_f64() * 100.0);
    }

    #[test]
    fn flat_matches_bruteforce() {
        let (ctx, r, _) = setup();
        let v = ctx.flat_value(0, [1, 2], [&r[1], &r[2]], 3.0);
        let bf: Vec<f32> = ctx.hands[0].iter().map(|&h| { let mut s = 0f32; for (i, &h1) in ctx.hands[1].iter().enumerate() { if !disjoint(h, h1) { continue; } for (j, &h2) in ctx.hands[2].iter().enumerate() { if disjoint(h, h2) && disjoint(h1, h2) { s += r[1][i] * r[2][j]; } } } 3.0 * s }).collect();
        close(&v, &bf);
    }

    #[test]
    fn showdown3_matches_bruteforce() {
        let (ctx, r, b) = setup();
        let pay = Pay { win: 7.0, tie2: 2.0, tie3: 0.5, lose: -3.0 };
        let v = ctx.showdown3_value(0, 0, [1, 2], [&r[1], &r[2]], pay);
        let bf: Vec<f32> = ctx.hands[0].iter().map(|&h| {
            if b.contains(&h.0) || b.contains(&h.1) { return 0.0; }
            let st = strength(&b, h); let mut s = 0f32;
            for (i, &h1) in ctx.hands[1].iter().enumerate() { if !disjoint(h, h1) || b.contains(&h1.0) || b.contains(&h1.1) { continue; } let s1 = strength(&b, h1);
                for (j, &h2) in ctx.hands[2].iter().enumerate() { if !disjoint(h, h2) || !disjoint(h1, h2) || b.contains(&h2.0) || b.contains(&h2.1) { continue; } let s2 = strength(&b, h2);
                    let w = r[1][i] * r[2][j];
                    let p = if s1 > st || s2 > st { pay.lose } else if s1 == st && s2 == st { pay.tie3 } else if s1 == st || s2 == st { pay.tie2 } else { pay.win };
                    s += w * p; } }
            s }).collect();
        close(&v, &bf);
    }

    #[test]
    fn showdown2_with_dead_player_matches_bruteforce() {
        let (ctx, r, b) = setup();
        let pay = Pay { win: 7.0, tie2: 2.0, tie3: 0.0, lose: -3.0 };
        let v = ctx.showdown2_value(0, 2, [0, 1], [&r[0], &r[1]], pay);
        let bf: Vec<f32> = ctx.hands[2].iter().map(|&h| {
            if b.contains(&h.0) || b.contains(&h.1) { return 0.0; }
            let st = strength(&b, h); let mut s = 0f32;
            for (i, &h1) in ctx.hands[0].iter().enumerate() { if !disjoint(h, h1) || b.contains(&h1.0) || b.contains(&h1.1) { continue; } let s1 = strength(&b, h1);
                for (j, &h2) in ctx.hands[1].iter().enumerate() { if !disjoint(h, h2) || !disjoint(h1, h2) || b.contains(&h2.0) || b.contains(&h2.1) { continue; }
                    let p = if s1 > st { pay.lose } else if s1 == st { pay.tie2 } else { pay.win };
                    s += r[0][i] * r[1][j] * p; } }
            s }).collect();
        close(&v, &bf);
    }
}
