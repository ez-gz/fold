//! Range string parser: "22+,A2s+,KTo+,JJ-99,AKs:0.5". Later tokens override earlier ones.

use crate::cards::RANKS;

fn rank(ch: u8) -> usize {
    RANKS.iter().position(|&x| x == ch).expect("bad rank in range")
}

fn set(out: &mut Vec<((u8, u8), f32)>, hi: usize, lo: usize, kind: u8, w: f32) {
    for s1 in 0..4u8 {
        for s2 in 0..4u8 {
            let (c1, c2) = (hi as u8 * 4 + s1, lo as u8 * 4 + s2);
            if c1 == c2 { continue; }
            if hi == lo && s1 >= s2 { continue; }
            let suited = s1 == s2;
            if hi != lo && ((kind == b's' && !suited) || (kind == b'o' && suited)) { continue; }
            let h = if c1 < c2 { (c1, c2) } else { (c2, c1) };
            if let Some(e) = out.iter_mut().find(|e| e.0 == h) { e.1 = w; } else { out.push((h, w)); }
        }
    }
}

pub fn parse_range(s: &str) -> Vec<((u8, u8), f32)> {
    let mut out = Vec::new();
    for tok in s.split(',').map(str::trim).filter(|t| !t.is_empty()) {
        let (body, w) = match tok.split_once(':') {
            Some((b, w)) => (b, w.parse::<f32>().expect("bad weight")),
            None => (tok, 1.0),
        };
        let b = body.as_bytes();
        if let Some((a, z)) = body.split_once('-') {
            let (a, z) = (a.as_bytes(), z.as_bytes());
            if a[0] == a[1] {
                assert!(z[0] == z[1], "bad dash range: {tok}");
                let (x, y) = (rank(a[0]), rank(z[0]));
                for r in x.min(y)..=x.max(y) { set(&mut out, r, r, 0, w); }
            } else {
                // "ATs-A6s": same high card, same suitedness
                assert!(a[0] == z[0] && a.len() == 3 && z.len() == 3 && a[2] == z[2], "bad dash range: {tok}");
                let (x, y) = (rank(a[1]), rank(z[1]));
                for l in x.min(y)..=x.max(y) { set(&mut out, rank(a[0]), l, a[2], w); }
            }
            continue;
        }
        let plus = b[b.len() - 1] == b'+';
        let core = if plus { &b[..b.len() - 1] } else { b };
        let (hi, lo) = (rank(core[0]), rank(core[1]));
        let kind = if core.len() > 2 { core[2] } else { 0 };
        if hi == lo {
            let top = if plus { 12 } else { hi };
            for r in hi..=top { set(&mut out, r, r, 0, w); }
        } else {
            let (hi, lo) = (hi.max(lo), hi.min(lo));
            let top = if plus { hi - 1 } else { lo };
            for l in lo..=top { set(&mut out, hi, l, kind, w); }
        }
    }
    out.retain(|e| e.1 > 0.0);
    out.sort_by_key(|e| e.0);
    out
}
