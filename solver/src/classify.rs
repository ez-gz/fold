//! Coarse made-hand / draw buckets used by the rationale layer. Ordered strong -> weak.

use crate::eval::eval;

pub const CLASS_NAMES: [&str; 7] = [
    "Two pair+", "Top pair / overpair", "Middle / weak pair", "Strong draw",
    "Gutshot / overcards", "Ace high", "Air",
];

fn has_straight(mask: u16) -> bool {
    (0..=8).any(|s| (mask >> s) & 0x1F == 0x1F) || mask & 0b1_0000_0000_1111 == 0b1_0000_0000_1111
}

fn straight_outs(mask: u16) -> u32 {
    if has_straight(mask) { return 0; }
    (0..13).filter(|r| mask & (1 << r) == 0 && has_straight(mask | (1 << r))).count() as u32
}

pub fn classify(h: (u8, u8), board: &[u8]) -> u8 {
    let mut all = board.to_vec();
    all.push(h.0);
    all.push(h.1);
    let (v, vb) = (eval(&all), eval(board));
    let (cat, bcat) = (v >> 24, vb >> 24);
    let (r1, r2) = (h.0 >> 2, h.1 >> 2);
    let mut br: Vec<u8> = board.iter().map(|c| c >> 2).collect();
    br.sort_unstable_by(|a, b| b.cmp(a));
    br.dedup();
    let pair_logic = |p: u8| -> u8 {
        if r1 == r2 { if p > br[0] { 1 } else { 2 } } else if p == br[0] { 1 } else { 2 }
    };
    let hero_pair = if r1 == r2 { Some(r1) } else if br.contains(&r1) { Some(r1) } else if br.contains(&r2) { Some(r2) } else { None };
    match cat {
        4.. => { if board.len() == 5 && v == vb { return 6; } return 0; }
        3 if bcat < 3 => return 0,
        2 if bcat == 0 => return 0,
        2 if bcat == 1 => { if let Some(p) = hero_pair { return pair_logic(p); } }
        1 if bcat == 0 => { if let Some(p) = hero_pair { return pair_logic(p); } }
        _ => {}
    }
    // no made hand beyond the board: draws, then high cards
    if board.len() < 5 {
        for s in 0..4u8 {
            let n = all.iter().filter(|c| *c & 3 == s).count();
            if n == 4 && (h.0 & 3 == s || h.1 & 3 == s) { return 3; }
        }
        let mask = |cs: &[u8]| cs.iter().fold(0u16, |m, c| m | 1 << (c >> 2));
        let outs = straight_outs(mask(&all)).saturating_sub(straight_outs(mask(board)));
        if outs >= 2 { return 3; }
        if outs == 1 || (r1 > br[0] && r2 > br[0]) { return 4; }
    }
    if r1 == 12 || r2 == 12 { 5 } else { 6 }
}
