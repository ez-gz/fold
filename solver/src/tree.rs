//! Betting tree for one flop. Player 0 = OOP, player 1 = IP. OOP acts first on every street.

use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum Act {
    Fold,
    Check,
    Call,
    /// chips added, fraction of pot
    Bet(f32, f32),
    /// chips added, total street commit after the raise
    Raise(f32, f32),
}

pub struct ActionNode {
    pub player: u8,
    pub street: u8,
    pub actions: Vec<Act>,
    pub children: Vec<Node>,
    pub commit: [f32; 2],
    pub regrets: Vec<f32>,
    pub strat_sum: Vec<f32>,
}

pub enum Node {
    Action(ActionNode),
    Chance { cards: Vec<u8>, children: Vec<Node>, par: bool },
    Fold { folder: u8, commit: [f32; 2] },
    Showdown { commit: f32, board_id: usize },
}

#[derive(Clone)]
pub struct TreeConfig {
    pub start_pot: f32,
    pub eff_stack: f32,
    /// [player][street] -> pot fractions
    pub bets: [[Vec<f32>; 3]; 2],
    pub raises: [[Vec<f32>; 3]; 2],
    pub max_raises: u8,
    /// a bet that would use at least this fraction of the remaining stack becomes all-in
    pub allin_threshold: f32,
}

pub struct Builder<'a> {
    cfg: &'a TreeConfig,
    pub boards: Vec<Vec<u8>>,
    board_ids: HashMap<u64, usize>,
}

struct St {
    board: Vec<u8>,
    commit: [f32; 2],
    street_start: f32,
    raises: u8,
}

impl<'a> Builder<'a> {
    pub fn new(cfg: &'a TreeConfig) -> Self {
        Builder { cfg, boards: Vec::new(), board_ids: HashMap::new() }
    }

    pub fn build(&mut self, flop: &[u8]) -> Node {
        let st = St { board: flop.to_vec(), commit: [0.0; 2], street_start: 0.0, raises: 0 };
        self.action(&st, 0)
    }

    fn sized(&self, want: f32, remaining: f32) -> f32 {
        if want >= remaining * self.cfg.allin_threshold { remaining } else { want }
    }

    fn action(&mut self, st: &St, p: usize) -> Node {
        let cfg = self.cfg;
        let street = st.board.len() - 3;
        let o = 1 - p;
        let to_call = st.commit[o] - st.commit[p];
        let remaining = cfg.eff_stack - st.commit[p];
        let pot = cfg.start_pot + st.commit[0] + st.commit[1];
        let mut actions = Vec::new();
        let mut children = Vec::new();
        if to_call <= 0.0 {
            actions.push(Act::Check);
            children.push(if p == 0 { self.action(st, 1) } else { self.next_street(st) });
            let mut seen: Vec<f32> = Vec::new();
            for &f in &cfg.bets[p][street] {
                let amt = self.sized(f * pot, remaining);
                if amt <= 0.0 || seen.iter().any(|&s| (s - amt).abs() < 1e-3) { continue; }
                seen.push(amt);
                let mut n = St { board: st.board.clone(), commit: st.commit, street_start: st.street_start, raises: 0 };
                n.commit[p] += amt;
                actions.push(Act::Bet(amt, amt / pot));
                children.push(self.action(&n, o));
            }
        } else {
            actions.push(Act::Fold);
            children.push(Node::Fold { folder: p as u8, commit: st.commit });
            let mut n = St { board: st.board.clone(), commit: st.commit, street_start: st.street_start, raises: st.raises };
            n.commit[p] += to_call;
            actions.push(Act::Call);
            children.push(self.next_street(&n));
            if st.raises < cfg.max_raises && remaining > to_call + 1e-3 {
                let mut seen: Vec<f32> = Vec::new();
                for &f in &cfg.raises[p][street] {
                    let by = f * (pot + to_call);
                    let amt = self.sized(to_call + by, remaining).min(remaining);
                    if seen.iter().any(|&s| (s - amt).abs() < 1e-3) { continue; }
                    seen.push(amt);
                    let mut n = St { board: st.board.clone(), commit: st.commit, street_start: st.street_start, raises: st.raises + 1 };
                    n.commit[p] += amt;
                    actions.push(Act::Raise(amt, n.commit[p] - st.street_start));
                    children.push(self.action(&n, o));
                }
            }
        }
        Node::Action(ActionNode {
            player: p as u8, street: street as u8, actions, children, commit: st.commit,
            regrets: Vec::new(), strat_sum: Vec::new(),
        })
    }

    fn next_street(&mut self, st: &St) -> Node {
        if st.board.len() == 5 {
            let mut key = 0u64;
            for &c in &st.board { key |= 1 << c; }
            let id = match self.board_ids.get(&key) {
                Some(&id) => id,
                None => {
                    let id = self.boards.len();
                    self.boards.push(st.board.clone());
                    self.board_ids.insert(key, id);
                    id
                }
            };
            return Node::Showdown { commit: st.commit[0], board_id: id };
        }
        let allin = st.commit[0] >= self.cfg.eff_stack - 1e-3;
        let cards: Vec<u8> = (0..52u8).filter(|c| !st.board.contains(c)).collect();
        let mut children = Vec::with_capacity(cards.len());
        for &c in &cards {
            let mut board = st.board.clone();
            board.push(c);
            let n = St { board, commit: st.commit, street_start: st.commit[0], raises: 0 };
            children.push(if allin { self.next_street(&n) } else { self.action(&n, 0) });
        }
        Node::Chance { cards, children, par: st.board.len() == 3 }
    }
}

/// Returns (action nodes, floats needed) and allocates storage when `alloc` is set.
pub fn size_tree(node: &mut Node, n_hands: [usize; 2], alloc: bool) -> (usize, usize) {
    match node {
        Node::Action(a) => {
            let len = a.actions.len() * n_hands[a.player as usize];
            if alloc {
                a.regrets = vec![0.0; len];
                a.strat_sum = vec![0.0; len];
            }
            let mut t = (1, 2 * len);
            for c in a.children.iter_mut() {
                let s = size_tree(c, n_hands, alloc);
                t = (t.0 + s.0, t.1 + s.1);
            }
            t
        }
        Node::Chance { children, .. } => {
            let mut t = (0, 0);
            for c in children.iter_mut() {
                let s = size_tree(c, n_hands, alloc);
                t = (t.0 + s.0, t.1 + s.1);
            }
            t
        }
        _ => (0, 0),
    }
}
