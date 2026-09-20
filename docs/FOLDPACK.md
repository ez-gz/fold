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
