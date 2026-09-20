//! Card / combo primitives. card = rank*4 + suit, rank 0 = deuce .. 12 = ace.

pub const RANKS: &[u8] = b"23456789TJQKA";
pub const SUITS: &[u8] = b"cdhs";

pub fn parse_card(s: &str) -> u8 {
    let b = s.as_bytes();
    let r = RANKS.iter().position(|&x| x == b[0]).expect("bad rank") as u8;
    let su = SUITS.iter().position(|&x| x == b[1]).expect("bad suit") as u8;
    r * 4 + su
}

pub fn card_str(c: u8) -> String {
    format!("{}{}", RANKS[(c >> 2) as usize] as char, SUITS[(c & 3) as usize] as char)
}

pub fn parse_board(s: &str) -> Vec<u8> {
    let s: String = s.chars().filter(|c| !c.is_whitespace() && *c != ',').collect();
    (0..s.len() / 2).map(|i| parse_card(&s[i * 2..i * 2 + 2])).collect()
}

/// Cell in the 13x13 chart: row/col 0 = ace. Suited above the diagonal, offsuit below.
pub fn grid_cell(h: (u8, u8)) -> usize {
    let (r1, r2) = ((h.0 >> 2) as usize, (h.1 >> 2) as usize);
    let (hi, lo) = if r1 >= r2 { (r1, r2) } else { (r2, r1) };
    let suited = (h.0 & 3) == (h.1 & 3);
    if hi == lo || suited {
        (12 - hi) * 13 + (12 - lo)
    } else {
        (12 - lo) * 13 + (12 - hi)
    }
}
