# Three-way pots

## Status
- `solver/src/three.rs`: exact vector-form terminal values for three players (3-way showdown, heads-up showdown
  with a folded third player who still removes cards, fold terminals). Verified against brute force; ~1 ms per
  showdown at 1326 x 3 hands. Trick: mass of card-disjoint pairs from two strength bands is
  A1 A2 - sum_c C1[c] C2[c] + sum_h w1(h) w2(h), restricted to hands avoiding hero's two cards.
- Next: `tree3.rs` (three-seat betting tree, folded seats drop out, one bet size and one raise per street),
  `cfr3.rs` (vector DCFR over three reach vectors), `measure`/`solve` subcommands, then the GPU port.

## Decisions
- Spots first: CO open, BTN call, BB call (most common 3-way pot); SB/BB defend vs open + BTN call next.
  Squeezed and 3-bet multiway pots wait.
- Coarse tree: one size per street, one raise. Three-seat trees are ~10x bigger; frequencies matter more
  than the exact size in multiway play.
- "Solved" means each seat's best response against the other two is within target; no Nash guarantee with
  three players, CFR converges in practice on these games.
- Equal stacks: no side pots.
