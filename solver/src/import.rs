//! Loads the GPU solver's average strategy (gpu/solver.py --dump) into the per-card Rust tree.
//! File layout: shape action nodes in preorder, each float16 [49^street, actions, hands].

use crate::tree::Node;

fn f16(b: u16) -> f32 {
    let (s, e, m) = ((b >> 15) as u32, ((b >> 10) & 31) as u32, (b & 1023) as u32);
    let v = if e == 0 { m as f32 * 2f32.powi(-24) } else { (1.0 + m as f32 / 1024.0) * 2f32.powi(e as i32 - 15) };
    if s == 1 { -v } else { v }
}

/// (byte offset, byte length, actions) per shape node; chance nodes contribute their first child only.
pub fn shapes(node: &Node, nh: [usize; 2], off: &mut usize, out: &mut Vec<(usize, usize, usize)>) {
    match node {
        Node::Action(a) => {
            let len = 49usize.pow(a.street as u32) * a.actions.len() * nh[a.player as usize] * 2;
            out.push((*off, len, a.actions.len()));
            *off += len;
            for c in &a.children { shapes(c, nh, off, out); }
        }
        Node::Chance { children, .. } => shapes(&children[0], nh, off, out),
        _ => {}
    }
}

pub fn fill(node: &mut Node, bytes: &[u8], sh: &[(usize, usize, usize)], id: &mut usize, tr: [usize; 2], deck_pos: &[usize; 52], nh: [usize; 2]) {
    match node {
        Node::Action(a) => {
            let (off, _, na) = sh[*id];
            *id += 1;
            let n = nh[a.player as usize];
            let block = match a.street { 0 => 0, 1 => tr[0], _ => tr[0] * 49 + tr[1] };
            let base = off + block * na * n * 2;
            for i in 0..na * n {
                a.strat_sum[i] = f16(u16::from_le_bytes([bytes[base + 2 * i], bytes[base + 2 * i + 1]]));
            }
            for c in a.children.iter_mut() { fill(c, bytes, sh, id, tr, deck_pos, nh); }
        }
        Node::Chance { cards, children, par } => {
            let start = *id;
            let mut end = start;
            for (c, ch) in cards.iter().zip(children.iter_mut()) {
                *id = start;
                let tr2 = if *par { [deck_pos[*c as usize], 0] } else { [tr[0], deck_pos[*c as usize]] };
                fill(ch, bytes, sh, id, tr2, deck_pos, nh);
                end = *id;
            }
            *id = end;
        }
        _ => {}
    }
}
