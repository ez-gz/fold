# .foldpack v1

One file per content pack. Written by `fold-cli pack --dir <exports> --out <name>.foldpack`, read by
`proto/index.html?pack=<name>.foldpack` (reference loader: `loadFoldpack`) and, later, the iOS app.

| bytes | content |
|---|---|
| 0..8 | ASCII magic `FOLDPK01` |
| 8..12 | `N`: index length, u32 little-endian |
| 12..12+N | index, UTF-8 JSON |
| 12+N.. | flop blobs, back to back |

**Index**: `{ version: 1, ranges: "ranges-v1", tree: "tree-v1", formations: {...}, flops: [...] }`.
`formations[name]` = `{ pos:[OOP,IP], opener, caller, open_size, three_bet, pre:[grid169 OOP, grid169 IP] }`
(`opener` is the last preflop aggressor; in a 3-bet pot `caller` made the original open).
`flops[i]` = `{ flop, formation, offset, length, raw_length, spots, hands, exploitability_pct_pot }`;
`offset` is relative to the first blob byte, so a reader can fetch or decompress one flop at a time.

**Blob**: gzip of the flop's export JSON (`FlopFile` in `solver/src/export.rs`): spots with drills,
played hands, ranges as 169-cell grids (row/col 0 = Ace, suited above the diagonal).

Rules: readers must reject an unknown magic; `version` bumps on any breaking change to the index or the
flop JSON; `ranges` and `tree` identify the solve inputs, and a pack never mixes two values of either.
gzip + JSON was chosen over a bespoke bit layout because it is ~7x smaller than raw JSON already, decodes
with stock APIs on both platforms (DecompressionStream / Compression + JSONDecoder), and keeps the HTML
prototype usable as the reference the app is tested against.

## Additions (2026-09-20, pack-v1 re-export, 39 MB)

Per flop: `briefing` {equity, strong, weak, classes, first, after_check}.
Per drill: `vs` = [fold&ahead, fold&behind, continue&ahead, continue&behind] per action, shares of villain's range ("ahead" = >50% equity vs the exact hero hand).
Per spot: `vsuit` {suit, with[169], without[169]} when the board has two or more of a suit; `vcombos` [[combo, weight]] when villain has 50 or fewer combos at relative weight >= 0.05 (puzzle mode source); `vdetail` / `hdetail` per-combo weights and hero action mix (cell popup); `resp[a].combo` villain response per listed combo.
Export also force-picks up to 6 narrow-range spots per flop (10-50 combos over 6+ grid cells, street > 0). Pool in pack-v1: 388 puzzle spots across all 9 formations.
Trainer-side only (localStorage): `book` (where EV is lost) and `patterns` (direction of each mistake by hand class, context and street).

## Additions (2026-09-23, pack-v1r re-export: river what-if)

Per flop: `rivers` = list of river sweeps, one per turn line that reaches the river in a played hand:
`{ history, board[4], hero, cards[] }`. Each `cards[i]` = `{ card, actor, actions, vr, hr, strat[a][169], freq[a], pot, to_call }`
for the first river decision on that card (cards already on the board are absent; the trainer also hides hero's and
villain's hole cards). `vr`/`hr` are the range grids entering the river, `strat` the actor's strategy per action,
`freq` the range-weighted action frequencies.
Per hand: `rivers` = `{ sweep, ev: [[card, [ev per action]]] }` when hero acts first on the river (`sweep` indexes
`flop.rivers`); hero's exact-hand EV per action on each alternative river, in bb from the hand's start.
Trainer: after the decision score, the river card carries a ⇄ badge; tapping it opens a picker and swaps the river,
showing the actor's play / ranges on that card. The score never changes. ~50 KB gz per flop.
