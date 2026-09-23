//! Betting tree for three seats on one flop. Seats act in index order every street (0 first).
//! A folded seat drops out of the action but its range keeps removing cards. Equal stacks, no side pots.

use std::collections::HashMap;
use crate::tree::{Act, TreeConfig};

pub struct ActionNode3 {
    pub player: u8,
    pub street: u8,
    pub actions: Vec<Act>,
    pub children: Vec<Node3>,
    pub commit: [f32; 3],
    pub regrets: Vec<f32>,
    pub strat_sum: Vec<f32>,
}

pub enum Node3 {
    Action(ActionNode3),
    Chance { cards: Vec<u8>, children: Vec<Node3>, par: bool },
    /// everyone else folded
    FoldWin { winner: u8, commit: [f32; 3] },
    /// two seats reach showdown, `dead` folded earlier
    Showdown2 { alive: [u8; 2], dead: u8, commit: [f32; 3], board_id: usize },
    Showdown3 { commit: [f32; 3], board_id: usize },
}

pub struct Builder3<'a> {
    cfg: &'a TreeConfig,
    /// raise cap per street (overrides cfg.max_raises); the flop solve drops river raises to fit in memory
    pub max_raises: [u8; 3],
    pub boards: Vec<Vec<u8>>,
    board_ids: HashMap<u64, usize>,
}

#[derive(Clone)]
struct St {
    board: Vec<u8>,
    commit: [f32; 3],
    alive: [bool; 3],
    /// acted since the last bet or raise this street (all-in seats count as acted)
    acted: [bool; 3],
    street_start: f32,
    raises: u8,
}

impl<'a> Builder3<'a> {
    pub fn new(cfg: &'a TreeConfig) -> Self { Builder3 { cfg, max_raises: [cfg.max_raises; 3], boards: Vec::new(), board_ids: HashMap::new() } }

    pub fn build(&mut self, flop: &[u8]) -> Node3 {
        let st = St { board: flop.to_vec(), commit: [0.0; 3], alive: [true; 3], acted: [false; 3], street_start: 0.0, raises: 0 };
        self.next_actor(&st, 2)
    }

    fn sized(&self, want: f32, remaining: f32) -> f32 { if want >= remaining * self.cfg.allin_threshold { remaining } else { want } }
    fn allin(&self, st: &St, p: usize) -> bool { st.commit[p] >= self.cfg.eff_stack - 1e-3 }
    fn can_act(&self, st: &St, p: usize) -> bool { st.alive[p] && !self.allin(st, p) }

    /// Continue the street after seat `last` acted: the next seat that still owes an action, else the next street.
    fn next_actor(&mut self, st: &St, last: usize) -> Node3 {
        for k in 1..=3 {
            let p = (last + k) % 3;
            if self.can_act(st, p) && !st.acted[p] { return self.action(st, p); }
        }
        self.next_street(st)
    }

    fn action(&mut self, st: &St, p: usize) -> Node3 {
        let cfg = self.cfg;
        let street = st.board.len() - 3;
        // seat-specific sizes: the config has two seat slots; the first-to-act seat uses slot 0, others slot 1
        let slot = if p == 0 { 0 } else { 1 };
        let max_commit = st.commit.iter().cloned().fold(0.0, f32::max);
        let to_call = max_commit - st.commit[p];
        let remaining = cfg.eff_stack - st.commit[p];
        let pot = cfg.start_pot + st.commit.iter().sum::<f32>();
        let mut actions = Vec::new();
        let mut children = Vec::new();
        let aggress = |st: &St, p: usize, amt: f32, raises: u8| { let mut n = st.clone(); n.commit[p] += amt; n.acted = [false; 3]; n.acted[p] = true; n.raises = raises; n };
        if to_call <= 1e-6 {
            actions.push(Act::Check);
            let mut n = st.clone(); n.acted[p] = true;
            children.push(self.next_actor(&n, p));
            let mut seen: Vec<f32> = Vec::new();
            for &f in &cfg.bets[slot][street] {
                let amt = self.sized(f * pot, remaining);
                if amt <= 0.0 || seen.iter().any(|&s| (s - amt).abs() < 1e-3) { continue; }
                seen.push(amt);
                let n = aggress(st, p, amt, 0);
                actions.push(Act::Bet(amt, amt / pot));
                children.push(self.next_actor(&n, p));
            }
        } else {
            actions.push(Act::Fold);
            let mut n = st.clone(); n.alive[p] = false; n.acted[p] = true;
            let alive: Vec<usize> = (0..3).filter(|&q| n.alive[q]).collect();
            children.push(if alive.len() == 1 { Node3::FoldWin { winner: alive[0] as u8, commit: n.commit } } else { self.next_actor(&n, p) });
            let mut n = st.clone(); n.commit[p] += to_call.min(remaining); n.acted[p] = true;
            actions.push(Act::Call);
            children.push(self.next_actor(&n, p));
            if st.raises < self.max_raises[street] && remaining > to_call + 1e-3 {
                let mut seen: Vec<f32> = Vec::new();
                for &f in &cfg.raises[slot][street] {
                    let by = f * (pot + to_call);
                    let amt = self.sized(to_call + by, remaining).min(remaining);
                    if seen.iter().any(|&s| (s - amt).abs() < 1e-3) { continue; }
                    seen.push(amt);
                    let n = aggress(st, p, amt, st.raises + 1);
                    actions.push(Act::Raise(amt, n.commit[p] - st.street_start));
                    children.push(self.next_actor(&n, p));
                }
            }
        }
        Node3::Action(ActionNode3 { player: p as u8, street: street as u8, actions, children, commit: st.commit, regrets: Vec::new(), strat_sum: Vec::new() })
    }

    fn next_street(&mut self, st: &St) -> Node3 {
        if st.board.len() == 5 {
            let mut key = 0u64; for &c in &st.board { key |= 1 << c; }
            let id = match self.board_ids.get(&key) { Some(&id) => id, None => { let id = self.boards.len(); self.boards.push(st.board.clone()); self.board_ids.insert(key, id); id } };
            let alive: Vec<u8> = (0..3u8).filter(|&q| st.alive[q as usize]).collect();
            return if alive.len() == 3 { Node3::Showdown3 { commit: st.commit, board_id: id } }
                else { Node3::Showdown2 { alive: [alive[0], alive[1]], dead: (0..3u8).find(|q| !st.alive[*q as usize]).unwrap(), commit: st.commit, board_id: id } };
        }
        let actors = (0..3).filter(|&q| self.can_act(st, q)).count();
        let cards: Vec<u8> = (0..52u8).filter(|c| !st.board.contains(c)).collect();
        let mut children = Vec::with_capacity(cards.len());
        for &c in &cards {
            let mut board = st.board.clone(); board.push(c);
            let ss = st.commit.iter().cloned().fold(0.0, f32::max);
            let n = St { board, commit: st.commit, alive: st.alive, acted: [false; 3], street_start: ss, raises: 0 };
            children.push(if actors <= 1 { self.next_street(&n) } else { self.next_actor(&n, 2) });
        }
        Node3::Chance { cards, children, par: st.board.len() == 3 }
    }
}

/// (action nodes, floats needed); allocates regret and strategy storage when `alloc` is set.
pub fn size_tree3(node: &mut Node3, n_hands: [usize; 3], alloc: bool) -> (usize, usize) {
    match node {
        Node3::Action(a) => {
            let len = a.actions.len() * n_hands[a.player as usize];
            if alloc { a.regrets = vec![0.0; len]; a.strat_sum = vec![0.0; len]; }
            let mut t = (1, 2 * len);
            for c in a.children.iter_mut() { let s = size_tree3(c, n_hands, alloc); t = (t.0 + s.0, t.1 + s.1); }
            t
        }
        Node3::Chance { children, .. } => {
            let mut t = (0, 0);
            for c in children.iter_mut() { let s = size_tree3(c, n_hands, alloc); t = (t.0 + s.0, t.1 + s.1); }
            t
        }
        _ => (0, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn count(max_raises: u8, allin: f32, sizes: usize) -> (usize, usize) {
        let v = |x: &[f32]| x.to_vec();
        let one = || if sizes == 1 { [v(&[0.66]), v(&[0.66]), v(&[0.66])] } else { [v(&[0.33, 1.0]), v(&[0.66]), v(&[0.5, 1.25])] };
        let r = || [v(&[0.6]), v(&[0.6]), v(&[0.6])];
        let cfg = TreeConfig { start_pot: 8.0, eff_stack: 97.5, bets: [one(), one()], raises: [r(), r()], max_raises, allin_threshold: allin };
        let mut b = Builder3::new(&cfg);
        let mut root = b.build(&[3, 17, 30]);
        size_tree3(&mut root, [360, 200, 300], false)
    }
    #[test]
    fn no_river_raises() {
        let v = |x: &[f32]| x.to_vec();
        let one = || [v(&[0.66]), v(&[0.66]), v(&[0.66])];
        let r = || [v(&[0.6]), v(&[0.6]), v(&[0.6])];
        let cfg = TreeConfig { start_pot: 8.0, eff_stack: 97.5, bets: [one(), one()], raises: [r(), r()], max_raises: 1, allin_threshold: 0.67 };
        let mut b = Builder3::new(&cfg); b.max_raises = [1, 1, 0];
        let mut root = b.build(&[3, 17, 30]);
        let (n, f) = size_tree3(&mut root, [360, 200, 300], false);
        eprintln!("raises on flop+turn only: nodes {n}, {:.2} GB f32", f as f64 * 4.0 / 1e9);
    }
    fn by_street(node: &Node3, n: [usize; 3], acc: &mut [usize; 3]) {
        match node {
            Node3::Action(a) => { acc[a.street as usize] += 2 * a.actions.len() * n[a.player as usize]; for c in &a.children { by_street(c, n, acc); } }
            Node3::Chance { children, .. } => { for c in children { by_street(c, n, acc); } }
            _ => {}
        }
    }
    #[test]
    fn street_split() {
        let v = |x: &[f32]| x.to_vec();
        let one = || [v(&[0.66]), v(&[0.66]), v(&[0.66])];
        let r = || [v(&[0.6]), v(&[0.6]), v(&[0.6])];
        let cfg = TreeConfig { start_pot: 8.0, eff_stack: 97.5, bets: [one(), one()], raises: [r(), r()], max_raises: 1, allin_threshold: 0.67 };
        let mut b = Builder3::new(&cfg);
        let root = b.build(&[3, 17, 30]);
        let mut acc = [0; 3]; by_street(&root, [360, 200, 300], &mut acc);
        eprintln!("floats by street: flop {:.3} GB, turn {:.3} GB, river {:.2} GB", acc[0] as f64 * 4e-9, acc[1] as f64 * 4e-9, acc[2] as f64 * 4e-9);
    }
    #[test]
    fn sizes() {
        for (mr, al, sz) in [(1, 0.67, 1), (0, 0.67, 1), (1, 0.5, 1), (0, 0.5, 1), (1, 0.67, 2)] {
            let (n, f) = count(mr, al, sz);
            eprintln!("raises {mr} allin {al} sizes {sz}: nodes {n}, {:.1} GB f32", f as f64 * 4.0 / 1e9);
        }
    }
}
