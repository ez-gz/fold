# Three-way pots

## Status
- `solver/src/three.rs`: exact vector-form terminal values for three players (3-way showdown, heads-up showdown
  with a folded third player who still removes cards, fold terminals). Verified against brute force; ~1 ms per
  showdown at 1326 x 3 hands. Trick: mass of card-disjoint pairs from two strength bands is
  A1 A2 - sum_c C1[c] C2[c] + sum_h w1(h) w2(h), restricted to hands avoiding hero's two cards.
- `tree3.rs` (three-seat tree, per-street raise caps, `build_from` for turn subgames), `cfr3.rs` (vector DCFR,
  `exploitability3`, `turn_entries`), CLI: `solve3 <board> <r0> <r1> <r2> --raises f,t,r --dump X` (keeps flop+turn
  strategies), `resolve3 <dump> [--entry N --card Xy --dump Y]` (turn re-solve with full raises), `spec3` for the GPU.
- `gpu/solver3.py`: batched three-seat DCFR (same structure as solver.py; showdown chunked over turn cards).
- Pilot: `solver/three/co_btn_bb_Kh7d4d` (Mac CPU, 19 GB, raises 1,1,0).

## Memory (CO open, BTN call, BB call; 360/200/300 combos; one size; f32 regrets + strategy sums)
| raises allowed (flop, turn, river) | action nodes | memory |
|---|---|---|
| 1,1,1 | 5.4M | 26.8 GB |
| 1,1,0 | 3.7M | 16.8 GB |
| 1,0,0 | 2.3M | 10.5 GB |
| 0,0,0 | 0.85M | 3.9 GB |
Flop + turn nodes are 117 MB of that; rivers are the rest, driven by the number of histories entering the river
(flop line x turn card x turn line x river card), not by the river tree itself. Re-solving rivers inside the loop
is far too slow (~70k river states per iteration), so river storage stays and the tree is solved in two stages:
1. Flop solve on the GPU with raises capped to fit 8 GB in f16 (1,0,0 = 5.3 GB, or 0,0,0 = 2 GB).
2. Turn re-solves, one per turn card (27 GB / 49 = 0.55 GB each), with raises everywhere, from the flop
   strategy's reach. Stored artifact: flop + turn strategies; rivers re-solved on demand at export.
Suit isomorphism helps only two-tone (~1.7x) and monotone (~3x) flops; rainbow flops have no symmetry.

## Decisions
- Spots first: CO open, BTN call, BB call (most common 3-way pot); SB/BB defend vs open + BTN call next.
  Squeezed and 3-bet multiway pots wait.
- Coarse tree: one size per street, one raise. Three-seat trees are ~10x bigger; frequencies matter more
  than the exact size in multiway play.
- "Solved" means each seat's best response against the other two is within target; no Nash guarantee with
  three players, CFR converges in practice on these games.
- Equal stacks: no side pots.
