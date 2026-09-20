//! Hand evaluator for 3..=7 cards. Higher value = stronger hand. category = value >> 24.

fn straight_high(mask: u16) -> Option<u32> {
    for h in (4..=12u32).rev() {
        if (mask >> (h - 4)) & 0x1F == 0x1F {
            return Some(h);
        }
    }
    if mask & 0b1_0000_0000_1111 == 0b1_0000_0000_1111 {
        return Some(3);
    }
    None
}

fn top_n(mut mask: u16, n: u32) -> u32 {
    while mask.count_ones() > n {
        mask &= mask - 1;
    }
    mask as u32
}

pub fn eval(cards: &[u8]) -> u32 {
    let mut cnt = [0u8; 13];
    let mut smask = [0u16; 4];
    let mut rmask = 0u16;
    for &c in cards {
        let r = (c >> 2) as usize;
        cnt[r] += 1;
        smask[(c & 3) as usize] |= 1 << r;
        rmask |= 1 << r;
    }
    for s in 0..4 {
        if smask[s].count_ones() >= 5 {
            if let Some(h) = straight_high(smask[s]) {
                return (8 << 24) | h;
            }
            return (5 << 24) | top_n(smask[s], 5);
        }
    }
    let (mut quad, mut trips, mut pairs) = (None, [None; 2], [None; 3]);
    let (mut nt, mut np) = (0, 0);
    for r in (0..13).rev() {
        match cnt[r] {
            4 => quad = Some(r as u32),
            3 => { if nt < 2 { trips[nt] = Some(r as u32); nt += 1; } }
            2 => { if np < 3 { pairs[np] = Some(r as u32); np += 1; } }
            _ => {}
        }
    }
    if let Some(q) = quad {
        let k = top_n(rmask & !(1 << q), 1);
        return (7 << 24) | (q << 13) | k;
    }
    if let Some(t) = trips[0] {
        let p = match (trips[1], pairs[0]) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };
        if let Some(p) = p {
            return (6 << 24) | (t << 4) | p;
        }
    }
    if let Some(h) = straight_high(rmask) {
        return (4 << 24) | h;
    }
    if let Some(t) = trips[0] {
        return (3 << 24) | (t << 13) | top_n(rmask & !(1 << t), 2);
    }
    if let (Some(p1), Some(p2)) = (pairs[0], pairs[1]) {
        let k = top_n(rmask & !(1 << p1) & !(1 << p2), 1);
        return (2 << 24) | (p1 << 17) | (p2 << 13) | k;
    }
    if let Some(p) = pairs[0] {
        return (1 << 24) | (p << 13) | top_n(rmask & !(1 << p), 3);
    }
    top_n(rmask, 5)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn five_card_category_counts() {
        let mut counts = [0u32; 9];
        let mut c = [0u8; 5];
        for a in 0..52 { for b in a + 1..52 { for d in b + 1..52 { for e in d + 1..52 { for f in e + 1..52 {
            c = [a, b, d, e, f];
            counts[(eval(&c) >> 24) as usize] += 1;
        }}}}}
        let _ = c;
        assert_eq!(counts, [1302540, 1098240, 123552, 54912, 10200, 5108, 3744, 624, 40]);
    }
    #[test]
    fn seven_card_category_counts() {
        // Known 7-card frequencies (133,784,560 hands).
        let mut counts = [0u32; 9];
        for a in 0..52u8 { for b in a + 1..52 { for d in b + 1..52 { for e in d + 1..52 { for f in e + 1..52 { for g in f + 1..52 { for h in g + 1..52 {
            counts[(eval(&[a, b, d, e, f, g, h]) >> 24) as usize] += 1;
        }}}}}}}
        assert_eq!(counts, [23294460, 58627800, 31433400, 6461620, 6180020, 4047644, 3473184, 224848, 41584]);
    }
}
