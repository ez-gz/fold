# Fold — Engineering Spec (v0)

Solver-backed NLHE cash trainer for intermediate/advanced players. iOS, offline-first.
Priority order: **accuracy > trainer feel > everything else**. Minimal feature set.

Status: planning (rev 2). Date: 2026-09-19.

---

## 1. Product scope

### In (v1)
- NLHE cash, 6-max and 9-max, 100bb effective (40bb/200bb later).
- **Two rake settings: 0% (home game — the default) and 5% cap 1bb.** Each is a full, separate
  solution set (rake changes preflop ranges, which changes every postflop solve).
- **Trainer**: rapid-fire spot drills. See a spot, pick an action, get instant grading
  (EV loss in bb + GTO frequency) and a rationale. Filter by position, street, pot type, texture.
- **Play mode**: play hands vs. bots — GTO bot and human archetypes.
- **Rationale**: pre-computed, templated from solver facts, and **range-first** (§8): every
  explanation answers "what is villain's range, and what does my action do to it?"
  LLM on-demand is a later add-on.

### Out (v1)
Tournaments/ICM, PLO, hand-history import, range editor, custom solves, accounts, social.

### Bet-size menu (discrete, per situation)
| Node | Sizes |
|---|---|
| Preflop open | 2.5bb (SB 3bb) |
| 3bet / 4bet | IP 3x / OOP 4x; 4bet ~2.3x; 5bet jam |
| Flop | 33%, 50% (SRP); 33%, 75% (3BP); raise 3x |
| Turn | 50%, 100%, 150% overbet |
| River | 33%, 75%, 150%, all-in when SPR < 1.5 |

Two to three sizes per node is the cap. More sizes barely change EV but multiply tree size and
make grading ambiguous. The menu is config (`configs/trees/*.toml`), not code.

---

## 2. What "accurate" means — read this first

This constrains the whole design, so it's stated plainly:

1. **Heads-up postflop is solvable to arbitrary precision.** Given fixed ranges and a bet menu,
   DCFR converges to a Nash equilibrium; we can measure exploitability exactly. Target:
   **< 0.3% of pot** (better than typical commercial library solves at 0.5–1%).
2. **Preflop with 6–9 players is not "solved" by anyone.** There's no Nash convergence guarantee
   for >2 players, and the tree needs card/action abstraction. MCCFR-family methods produce
   strategies that are empirically strong and stable, which is what every commercial product
   (GTO Wizard, Monker, HRC) ships. We do the same, and validate by stability across seeds
   and agreement with published ranges — not by an exploitability number.
3. **Multiway postflop is out of scope for grading.** v1 trainer spots are heads-up postflop
   only (which is ~75–80% of real postflop play at these stakes). Multiway pots exist in play
   mode, where bots use a heuristic/NN policy and the hand is *not graded*. We never display a
   grade we can't back.
4. **Mixed strategies must be graded on EV, not on frequency.** A 30%-frequency action with
   0.00bb EV loss is *correct*. Grade = EV loss vs. best action; frequency shown as context.
   This is the single most common accuracy bug in trainers.

Rule: **every graded spot ships with its solve's exploitability and tree config hash.** If it
isn't in the pack, it isn't graded.

---

## 3. Architecture

```
 OFFLINE (your machine / gpubox / cloud burst)              ON DEVICE (iOS)
┌───────────────────────────────────────────────┐     ┌──────────────────────────────┐
│ solver-core ──► solve farm ──► raw solutions   │     │ FoldKit (Swift package)      │
│   (DCFR HU postflop,           (per flop,      │     │  ├ PackReader (mmap)         │
│    MCCFR preflop)               parquet/bin)   │     │  ├ Engine (rules, eval)      │
│                        │                       │     │  ├ Grader (EV loss)          │
│                        ▼                       │     │  ├ Bots (GTO + archetypes)   │
│                 packer ──► .foldpack ──────────┼──►  │  └ Rationale (templates)     │
│                        │                       │ CDN │ App (SwiftUI trainer / play) │
│                 rationale-gen ──► facts + text │     └──────────────────────────────┘
│                 verifier (exploitability, diff)│
└───────────────────────────────────────────────┘
```

No runtime server in v1. Packs are static files: bundle a starter pack, download the rest
from a CDN/object store. A server only appears if/when on-demand LLM rationale ships (§9).

### Language choices
- **Solver core + packer: Rust** (decided). The hot loop is dense float math over
  1326-combo vectors; Rust gives SIMD, no GC pauses on large trees, and the best open-source
  reference implementations are Rust.
- **Pipeline orchestration / CLI glue: Go or plain shell+Make** — not performance-relevant.
- **iOS: Swift / SwiftUI**, iOS 17+. Core logic in a pure-Swift package (`FoldKit`) with no UI
  deps so it's testable from the command line.

### Build vs. borrow the solver
`b-inary/postflop-solver` (Rust, DCFR, reportedly faster than Pio) is the best open reference,
but it's **AGPL** and development is suspended. Using it offline to *generate data* is a grey
area worth a lawyer's 10 minutes if this goes commercial. Plan: write our own core (HU postflop
DCFR is ~3–4k lines and well documented), use postflop-solver and TexasSolver purely as
**cross-validation oracles** in the verifier. **Open decision D2.**

---

## 4. Solver pipeline

### 4.1 Preflop (multiway, 6-max and 9-max)
- Algorithm: external-sampling MCCFR with DCFR-style discounting, linear averaging.
- Postflop during preflop solve: **card-abstracted** (EMD k-means buckets on equity
  histograms, ~200/street) with a reduced postflop bet menu, *or* — stretch — a learned value
  net for depth-limited leaves (the approach GTO Wizard shipped for 9-player preflop in Feb 2026).
  Start with abstraction; it's known to work and needs no training infra.
- Rake: two configs, 0% (default) and 5% cap 1bb — materially changes SB/BB defend ranges
  and 3bet-vs-call decisions, so each gets its own preflop solve and postflop set.
- Output: for every reachable preflop action sequence → 169-hand-class strategy + EV.
  Tiny: whole preflop tree for both table sizes is < 5 MB packed.
- Prune: only keep lines with reach > 0.5%; limps and cold-4bet trees collapse to a handful.
- Validation: 3 seeds must agree within 2% frequency on hands with >5% reach; sanity diff
  against published charts (RangeConverter/GTOW free ranges) as a smoke test, not ground truth.

### 4.2 Postflop (heads-up)
- Algorithm: **DCFR** (α=1.5, β=0, γ=2), vectorized over hands, suit-isomorphism on the flop,
  no card abstraction (full 1326 combos). Track recent river-solving variants (HS-DCFR,
  arXiv 2605.19928) as a drop-in once the baseline is verified.
- Inputs: the two ranges from the preflop solution for each **formation**.
- Formations v1 (6-max; 9-max adds UTG/MP variants but many collapse onto the same ranges):
  SRP: BTN/CO/MP/UTG vs BB, SB vs BB, BTN vs SB-call. 3BP: BB/SB vs BTN, BTN vs CO, etc.
  4BP: two representative. ≈ **20 formations**.
- Flops: **95-flop weighted subset** (published subsets reproduce full-1755 aggregate
  frequencies within ~1%) for v1; expand to 184 later. Weights carried into drill sampling.
- Budget: 20 formations × 95 flops × 2 rake settings = **3,800 solves** (est. 2–4 days on one
  big instance; solve 0% rake first, it's the default). Per rake setting: 1,900 solves. At ~5–15 CPU-min each on a 32-core box to
  0.3% pot: **mid hundreds of dollars total (estimate).** gpubox is not the
  right tool (CFR is memory-bandwidth-bound; CPU wins unless we write CUDA kernels).
- Stop criterion: exploitability < 0.3% pot, computed by exact best response. Logged per solve.

### 4.3 Verifier (the accuracy gate — owns CI)
- Exact best-response exploitability for every HU solve.
- Cross-check: 20 random spots solved in an independent oracle solver; per-hand EV must match
  within 0.5% pot.
- Known-answer tests: Kuhn, Leduc (closed-form equilibria), river toy games with analytic solutions.
- Hand evaluator: exhaustively checked against all 2,598,960 5-card hands' category counts.
- **Pack round-trip**: after quantization, re-compute exploitability *of the packed strategy*.
  Compression that costs > 0.1% pot fails the build.

---

## 5. Compression → `.foldpack`

Raw solve ≈ 0.5–2 GB per flop. Target on device: **~1–3 MB per flop, < 500 MB total**, starter
bundle < 80 MB.

Pipeline, in order of payoff:
1. **Prune** nodes with reach < 0.1% and everything below them (fallback: nearest-sibling policy,
   flagged ungraded).
2. **Suit isomorphism** — store canonical boards/hands only.
3. **Quantize** strategy to u8 (1/255 steps; renormalize), EV to i16 in 0.01bb. Drop actions
   < 1% and renormalize (verify round-trip per §4.3).
4. **Layout** struct-of-arrays per node → long runs of 0/255 (most hands are pure) → **zstd
   with a trained dictionary**. Expect 10–20x on top of quantization.
5. **Trainer-first option**: the trainer only needs *sampled spots*, not full trees. A "drill
   pack" of ~200k curated decision points (full range strategy + EVs at that node only) is
   ~100 MB and covers months of training. Full trees needed only for play-mode GTO bot.

Format: single mmap-able file. Header (magic, version, tree-config hash, solver git SHA,
exploitability) → node index (perfect hash on canonical action-line + board) → zstd frames
per flop subtree, independently decompressible. Spec lives in `docs/FOLDPACK.md`; the Rust
writer and Swift reader are both tested against shared golden files.

### 5.1 Neural approximation — considered; runs as a gated experiment (WS-I)

Two different ideas get called "Deep CFR"; they have very different risk:

- **Deep CFR proper** (Brown et al. 2019; SD-CFR, DREAM, DeepDCFR): a net *replaces the regret
  tables during solving*. It exists for games too big to tabulate. Its error sources are sampling
  noise, net capacity and replay-buffer effects, and published results are on reduced games
  (Flop Hold'em), measured in mbb/g vs. other bots — not the <0.3%-pot exploitability tabular
  DCFR gives us on HU postflop. **For HU postflop we can tabulate, so tabular wins on accuracy.**
  Where it *is* relevant: multiway preflop (WS-C leaf values) and multiway postflop bot play —
  places where tabular is infeasible and we're ungraded anyway.
- **Supervised distillation of our own tabular solves**: train a small net
  (board, action line, ranges-or-formation id, hand) → (strategy, EV per action). This is the
  promising one, because **we hold ground truth and can measure the net's error exactly**:
  plug the net's strategy into the real tree and compute its exploitability; compare its EVs
  to the table's per spot.

Upside if it works: generalizes to all 1,755 flops and unseen lines, app shrinks to tens of MB,
unlimited spots. Risk: grading needs *EVs* accurate to ~1% pot to separate "Good" from
"Inaccuracy", and close mixed spots — exactly the instructive ones — are where nets are worst.

Decision: **tables are the source of truth for v1 grading; the net is an experiment with a
hard acceptance gate**, runnable as soon as M2 data exists:
1. Net-strategy exploitability (exact, in-tree) < 1% pot on held-out flops.
2. Grade agreement with table ≥ 99% on held-out spots; zero Best↔Mistake flips.
3. Per-spot fallback: ship a 1-bit "net within tolerance" mask so bad spots use tables or
   are excluded.
If it passes, the net serves unsolved flops and play-mode; tables keep the curated drill set.
Training runs on gpubox (this *is* a good GPU workload). Start: ~1–5M param MLP/transformer
over 1326-hand range tensors, CoreML on device.

---

## 6. Bots

- **GTO bot**: samples from the pack strategy. Off-tree bet sizes → map to nearest size via
  pseudo-harmonic action translation. Multiway postflop → heuristic policy (equity + position
  + SPR rules), ungraded.
- **Archetypes** = parametric distortions of the GTO strategy, not separate solves:

  | Archetype | Distortion |
  |---|---|
  | Nit | fold more vs aggression, under-bluff, tight preflop (scale VPIP ~0.6x) |
  | Calling station | fold freq × 0.5, raise freq × 0.5, calls wide |
  | LAG / maniac | bluff freq × 1.8, over-3bets, barrels too much |
  | TAG reg | near-GTO, under-bluffs rivers, over-folds to raises |
  | Passive fish | limps, min-bets, rarely raises without nuts |

  Implementation: logit-space bias per action class, conditioned on hand-strength bucket, so
  e.g. the station's *value* hands still raise. Parameters tuned so aggregate stats
  (VPIP/PFR/3B/AF/WTSD) hit published population numbers for each type.
- **Exploit trainer (v1.1)**: node-lock the archetype's strategy, re-solve hero's max-exploit
  response offline, ship as separate packs → "vs. Station" drills are *also* solver-backed.
  This is the feature that differentiates from existing trainers; keep the pipeline ready for it.

---

## 7. iOS app

### Trainer interaction (the core loop; target < 4s per spot)
- One spot = one full-screen **card**: board, hero hand, positions, pot/stacks, and a compact
  action-history strip. No table graphic in drill mode — it's noise.
- **Input**: large thumb-zone action buttons along the bottom arc (Fold / Check-Call / each bet
  size), plus gesture shortcuts: swipe left = fold, swipe right = call/check, swipe up = bet
  (release height picks size, with haptic detents per size). Buttons are primary; gestures are
  the power-user accelerator. Decide by prototype — **open decision D3**.
- **Feedback** (< 100ms, all local): card flashes/tints by grade, haptic (success / warning /
  error), EV-loss number animates in. Grades: Best (0), Good (< 0.05bb… scaled to pot),
  Inaccuracy, Mistake, Blunder. Thresholds as % of pot, in config.
- **Reveal sheet** (pull up or tap) — shown on correct answers too, in a lighter form:
  1. Verdict line: EV per action for your hand, solver frequency bar.
  2. **Range lens** (the centerpiece): villain's range *before* your action, and what it
     becomes *after each option* — side-by-side 13×13 grids or a hand-class bar
     (nuts / strong / medium / draws / air) that animates between "vs 33%" and "vs pot".
     Shows at a glance: small bet keeps in the worse pairs and floats we beat; big bet folds
     them and leaves us against a strong, narrow range.
  3. Rationale: 2–3 sentences built on that comparison (§8) + a "what would change it" line.
  Swipe away → next spot.
- **Range-first habit**: optional "range check" drill variant — before acting, tap villain's
  rough range shape (capped / uncapped, strong / medium / weak-heavy); graded against the
  real range. Trains the question "what is their range and what do I do about it" directly.
- UX is iterated, not specified: WS-E ships an on-device prototype in week 1 and we tune
  time-to-answer, time-to-insight and reveal density from real use.
- Session summary: EV lost/100 spots, leak breakdown by tag (e.g. "over-folding BB vs c-bet on
  low boards").
- **Spot sampling**: weighted by reach × flop weight, biased toward (a) close-but-meaningful
  decisions, (b) user's past mistakes (simple Leitner/SM-2 spaced repetition on spot *tags*,
  not individual spots). Exclude trivial spots (best action > 0.5 pot better than second).
- Visual: dark, high-contrast, big type, spring animations, heavy haptics. SwiftUI +
  `matchedGeometryEffect`/`PhaseAnimator`; Metal only if a specific effect needs it.

### Modules
```
ios/
  FoldKit/            Swift package, no UIKit/SwiftUI
    Engine/           cards, eval (lookup-table 7-card), game state, legal actions
    Pack/             mmap reader, zstd, node lookup, canonicalization
    Grader/           EV loss, grade bands, tags
    Bots/             GTO sampler, archetype distortion, action translation
    Rationale/        template renderer over fact records
    Progress/         SwiftData models, SRS scheduler
  FoldApp/            SwiftUI: Trainer, Play, Review, Settings, PackManager
```

---

## 8. Rationale

Pre-compute, in two layers:

1. **Facts** (offline, deterministic, per drill spot): hero equity vs. range, range vs. range
   equity, nut advantage (share of top-10% hands per player), hand class (top pair good kicker,
   OESD + backdoor…), blocker effects (how hero's cards shift villain's value:bluff ratio),
   EV per action, MDF / pot odds, **action-conditioned villain response** (below), how hand class as a whole plays at this node, board-texture
   tags. Stored as a ~100-byte record alongside the spot.
   **Action-conditioned villain response** is the core record: for *each* hero option (check,
   33%, pot…), villain's fold/call/raise split by hand class, the continuing range's
   composition, hero's equity vs. that continuing range, and fold equity. All read straight
   from the solved child nodes. It also records hero's *range-wide* plan at the node (which
   hand classes take which size) so text can say why this hand belongs in the small-bet range.
   ⇒ Pack requirement: a drill spot stores the hero node **plus one ply of villain responses**
   per action (roughly 3–4x spot size; still fine for the ~100 MB drill pack).
2. **Text**: templates keyed on (decision type × dominant facts). "You fold too much here:
   you need 29% equity and have 38% against a range that's bluffing 41% of the time. Your
   A♠ blocks their nut flushes." Deterministic, offline, never wrong about numbers because
   numbers are slotted in from facts.
   Sizing example: "Betting 33% keeps in the hands you beat — villain continues with 71% of
   range including all their Qx and pocket pairs, and you have 64% equity against it. Pot
   folds those out (continues 38%: mostly Kx+, sets, draws) and you're down to 47%. Your hand
   wants worse hands to call, so small wins by 0.4bb. Big sizing here belongs to your
   nutted hands and the bluffs that benefit from folds."
   Same structure for correct answers, one sentence shorter — reinforcing *why* it was right.

Later (v1.x): offline batch LLM pass that rewrites template text for the top ~50k spots, with
the fact record as the *only* permitted source of claims and an automated checker that every
number in the output appears in the record. On-demand "ask why" chat = thin server proxying to
Claude with the fact record + node strategy as context. Not in v1.

---

## 9. Repo layout

```
fold/
  docs/            SPEC.md, FOLDPACK.md, decisions/
  configs/         trees/*.toml, formations.toml, flops_95.csv, archetypes.toml, grades.toml
  solver/          Rust workspace
    crates/core        cards, eval, isomorphism, ranges        (WS-A)
    crates/postflop    HU DCFR + best response                 (WS-B)
    crates/preflop     multiway MCCFR + abstraction            (WS-C)
    crates/pack        .foldpack writer/reader, quantizer      (WS-D)
    crates/facts       rationale fact extraction               (WS-G)
    crates/verify      exploitability, oracles, round-trip     (WS-B/D)
    bin/fold-cli       solve | farm | pack | verify | inspect
  ios/             FoldKit + FoldApp                           (WS-E, WS-F)
  testdata/        golden packs, known-answer games, fixture spots
```

---

## 10. Parallel workstreams

The two contracts that unblock everything are **the `.foldpack` format** and **a fixture pack**.
Do those first (week 1), then six streams run independently.

| WS | Deliverable | Depends on | Notes |
|---|---|---|---|
| **0. Contracts** | `FOLDPACK.md`, tree configs, hand-made **fixture pack** (fake-but-valid strategies for 1 formation × 3 flops), spot/fact JSON schema | — | 2–3 days. Blocks D, E, F, G. |
| **A. Core** | cards, evaluator, suit isomorphism, range parsing — Rust *and* Swift twins with shared test vectors | — | Small; start immediately. |
| **B. Postflop solver** | HU DCFR, best response, exploitability, toy-game tests, oracle cross-check | A | Critical path for real data. Until C lands, feed it published preflop ranges. |
| **C. Preflop solver** | bucketing, multiway MCCFR, 6-max then 9-max ranges | A | Longest research risk. Independent of B until formations are finalized. |
| **D. Packer** | quantize/prune/zstd writer, Swift reader, round-trip exploitability gate | 0, (B for real data) | Develop against fixture + first real solve. |
| **E. Trainer UI** | card loop, input prototype(s), feedback, reveal sheet, range grid, session summary | 0 (fixture pack) | Fully unblocked by fixtures — can be polished before a single real solve exists. |
| **F. FoldKit + bots** | engine, grader, GTO sampler, archetypes, play mode | 0, A | Archetype tuning harness runs bot-vs-bot sims for stat targets. |
| **G. Rationale** | fact extractor, template library, number-consistency checker | 0, B | Template writing can start from the schema alone. |
| **I. Neural distillation (experiment)** | train/eval harness, in-tree exploitability of net strategy, grade-agreement report, CoreML export | M2 data | Gated per §5.1; off the critical path. |
| **H. Solve farm** | job runner, resumable, per-solve manifest, cost tracking; then the real 1,900-solve run | B, C, D | Mostly ops; last. |

### Milestones
- **M1 – Walking skeleton**: fixture pack → Swift reader → one drill card graded on device. (WS 0, A, D-reader, E-minimal)
- **M2 – First truth**: one real formation (BTN vs BB SRP), 95 flops, solved + verified + packed; trainer running on real data with template rationale.
- **M3 – 6-max complete**: our own preflop ranges, all formations, spot sampling + SRS, session summary.
- **M4 – Play mode**: GTO + archetype bots.
- **M5 – 9-max + exploit packs + LLM rationale polish.** TestFlight.

---

## 11. Open decisions

- ~~D1~~ **Resolved: Rust.**
- **D2** Own solver (recommended) vs. build on AGPL postflop-solver. Matters only if commercial.
- ~~D3~~ **Resolved: buttons first**, gestures as accelerators; iterate UX from an on-device prototype.
- ~~D4~~ **Resolved: 100bb; 0% rake (default) and 5% cap 1bb.**
- **D5** Trainer-first "drill pack" (~100 MB, recommended for M2–M3) vs. full trees on device from day one.

## 12. Risks

| Risk | Mitigation |
|---|---|
| Preflop multiway quality is unverifiable by exploitability | Seed-stability gate, published-range diff, and M2 doesn't depend on it (uses published ranges) |
| Solver bugs produce confident wrong answers | Known-answer games, exact best response, independent oracle cross-check, all in CI |
| Quantization/pruning silently degrades strategy | Round-trip exploitability gate on the *packed* artifact |
| Pack size balloons with 9-max formations | Formation dedup by range similarity; drill packs instead of full trees |
| Rationale text states something false | Text is templated from facts; LLM output gated by number-consistency checker |
| Trainer feels sluggish | All grading local + mmap; pre-decompress next 5 spots; 100ms feedback budget as a perf test |

## References
- postflop-solver (Rust, DCFR, AGPL): https://github.com/b-inary/postflop_solver
- OpenSolver (Rust, DCFR): https://github.com/JoakimMich/opensolver
- GTO Wizard multiway preflop (NN + depth-limited re-solving): https://blog.gtowizard.com/introducing-multiway-preflop-solving/
- Real-Time Parallel CFR / HS-DCFR: https://arxiv.org/pdf/2605.19928
- Depth-Limited Solving (Brown et al.): https://arxiv.org/pdf/1805.08195
- Deep CFR overview (variants, error sources): https://www.emergentmind.com/topics/deep-cfr
- Discounted CFR (Brown & Sandholm 2019); Pluribus (Science 2019) for multiway MCCFR + abstraction

## 13. Prototype findings and decisions (rev 3)

- **D5 resolved: drill packs first.** Fuller solutions are an experiment, scoped by likelihood rather than completeness.
- **Viable-line pruning.** Postflop solves already start from chart ranges, so off-chart preflop lines are never solved. The same idea applies postflop: the exporter weights spot selection by the probability the line is reached under the solved strategy and drops lines under 2% reach. A "fuller" pack is this with the threshold lowered, not a full tree dump.
- **No typed input anywhere.** Every interaction is a tap, drag, swipe or selector. The range check is a 13x13 chart the player paints by dragging, with preset chips (pairs, suited Ax, broadway...). Next iteration: start from the previous street's range and have the player remove hands.
- **Backbone prototype** lives in `solver/` (single crate for now) and `proto/` (one HTML file). See `proto/README.md`. It replaces the fixture-pack step of WS-0: the JSON the exporter writes is the first draft of the drill-spot schema.

### 13.1 Trainer loop after first playtest (rev 4)

- **Primary mode is a played hand**, not an isolated spot: preflop action animates across the six seats (watch-only), hero picks up on the flop from a random seat and plays every decision to the end of the hand.
- **One tracked line.** Villain holds a real hand and samples the solved strategy; hero's scripted action is the solver's most frequent one. A different choice is graded immediately against the full per-action EVs, then the hand continues down the scripted line. This keeps pack size linear in hand length.
- **Scoring.** Hand score = 10 x (EV you took - EV of worst options) / (EV of best - EV of worst), summed over the hand's decisions; decisions whose options are within 0.05bb are free. Picking the worse of two +EV options is punished by the share of available EV missed, not the raw bb. Scored on decisions, never on the runout.
- **Hand percentile.** The "My plan" lens outlines hero's hand and shows where it ranks in hero's own range on this board (by equity vs villain's range): "Top 18%".
- **Replay.** After the score, each decision can be reopened with the range lens and rationale: villain's range before, and after every option hero had.
- **Bluff rationale rule.** A bet is compared against the EV of checking, never against zero; the text must say what the hand wins by checking and that folded-out hands are mostly ones it already beats.
- **3-way pots:** required by the product, not supported by the 2-player solver. Needs the multiway MCCFR path (no exploitability proof); grading there must be labelled approximate. Open design item.
- **Value source ("where does my EV come from").** EV is linear in villain's reach, so re-walking an action with villain's range masked to one hand class gives that class's exact share of the action's EV; the shares sum to the EV. Stored per drill as `src[action][class]`, shown by default as a diverging bar chart (best action vs the player's or runner-up) plus one templated sentence. Per-cell (169) resolution is the follow-up: cheap on turn/river, ~25x the export cost on flops.
- **Range check starts from villain's preflop range** (pack carries per-formation 169-grids); the player narrows it with board-aware group buttons, a top-X%-of-their-range slider, and drag painting. Preflop action is always shown in the hand history.
- **Backlog from playtest (not critical):** on the range chart, mark which of villain's hands are bluffs and which hands hero currently beats. Needs per-cell data the export does not carry yet: hero's equity vs each villain cell (cheap), and a bluff tag for villain betting hands (equity vs hero's range below a threshold). Same per-cell pass as the value-source overlay. The written analysis already names example hands that fold/continue, taken from the response grids.

### 13.2 First real batch (rev 5)

- **Preflop ranges v1** (`solver/src/preflop.rs`): hand-authored to the consensus shape of solver output for 6-max 100bb, with mixed boundary hands. Not our own preflop solve and not copied from a chart; replacing them with a real preflop solve stays open.
- **Bet tree v1** (`tree_config`, locked): single-raised aggressor 33/100, 66, 50/125; caller 66 when checked to, and an out-of-position caller may lead the flop for 33; 3-bet pots 33/66, 66, 50/100; one 60% raise per street; all-in at 67% of stack.
- **Matchups**: BTN/CO/UTG vs BB, SB vs BB, CO vs BTN, UTG vs BTN, and 3-bet pots BB vs BTN, SB vs BTN, BTN vs CO.
- **Flops**: 72, picked by `gpu/pick_flops.py` in proportion to texture frequency (high card x suit pattern x paired/connected/semi/dry), 8 per matchup.
- **Pipeline**: `gpu/run_batch.sh` on the box, `gpu/collect_batch.sh` on the Mac (Rust best-response check, export, strategy kept in `solver/cache/` so exports can be redone without re-solving), packed as `.foldpack` (docs/FOLDPACK.md).
- **Dropped**: suit isomorphism. It only helps when two suits are interchangeable on the flop (two-tone, monotone, some paired boards), never on rainbow flops, so the average gain is ~1.3x, not worth the risk now.

### 13.3 Trainer backlog (playtest notes)

- **Prefer hands that play all three streets.** Hands that end on an early fold teach less; try dealing only (or mostly) hands that reach the river, either by filtering in `nextHand` or by biasing `play_hand` in the export. Check it does not bias the lesson: folding early is sometimes the right play and should still appear occasionally.
- Range check inside Hands mode; a forced share of "facing a check-raise" spots; pot-scaled tie threshold; drop absurd folds from the score baseline; per-street / per-hand-type leak summary.

### 13.4 Preflop solve: status and the design that should work (rev 6)

- `preflop/solve_preflop.py` (two-player subgames over 169 classes, fitted share-of-pot curves) runs, but its ranges are not usable: the curves are only measured on hands inside the ranges we solved with, and the decisions we need are exactly about hands at or beyond those boundaries. Fitted realization cannot extrapolate there (3-bets and cold calls come out far too wide, opens too tight).
- **Measured-EV design.** Solve flops with every out-of-range hand added at a tiny weight (`FOLD_EPS=0.02`, one player at a time with `FOLD_EPS_P` to fit 8 GB) on a coarse tree (`FOLD_TREE=pre`). A tiny weight does not move the equilibrium (checked: root EV 2.300 vs 2.303), so the solve yields the true flop EV of *every* hand against the current ranges. Average over ~50 frequency-weighted flops per matchup and side, feed those EVs into the preflop subgames in place of fitted realization, update ranges, repeat until stable.
- **Cost.** ~1-2 GPU-minutes per run; 9 matchups x 2 sides x 50 flops is 15-30 GPU-hours per outer round, 2-3 rounds expected, and opening ranges additionally need the seat pairs we do not train yet.

### 13.5 Learning-tool additions (2026-09-20)

- Every decision is tagged with its situation (c-bet, facing a c-bet, second barrel, delayed c-bet, probe, facing a raise, ...). Results are kept per situation, street, hand type and role; the Leaks sheet shows average EV lost for each, and Focus drills one situation or the three costliest.
- Played hands mostly go to the river (80%), some stop earlier so folding out villain still shows up.
- Each flop export has a `briefing`: range equity, share of range at 75%+ and under 35% equity, class shares, and the opening frequencies for OOP and for IP after a check. `fold-cli import ... --brief-into <export.json>` adds it to an existing export without a best-response pass.

### 13.6 Possible solver speed-ups (noted 2026-09-20, none started)

- More flops at lower precision for the preflop measurement rounds: per-class noise comes from the 25-flop sample (0.2-0.5bb), not from solving to 1% of pot. Half the iterations on twice the flops is better for the same time. Start on a fresh round, not mid-loop.
- Half precision on the GPU for the showdown sweeps (keep regret and strategy sums in 32-bit); maybe 1.3-1.6x on the 2080. Re-run the reference cross-check afterwards.
- Batch the many small per-node GPU calls (CUDA graphs or fused per-street kernels). Profile one solve first: 97% utilisation does not show whether launch overhead or memory bandwidth is the limit.
- Warm-start each preflop round from the previous round's strategy. Needs regrets saved as well as the average strategy.
- Done: the Mac side of the measurement rounds is one-sided (FOLD_EPS_P=0) like the GPU.
- Not worth it: suit isomorphism (about 1.3x, nothing on rainbow flops); river bucketing (costs accuracy).

### 13.7 Preflop solve plan (revised 2026-09-22; running as `preflop/auto_it.sh`)
Audit of the earlier version: its arithmetic did not close. Measured cost is ~70-120 s per one-sided `pre`-tree run
(r4/r5 logs), so 9 formations x 2 sides x 40 flops x 3 rounds was ~54 GPU-hours before the bigger two-size tree, and the
fp16/CUDA-graph speed-ups it leaned on were never measured. Pruning hand classes saves nothing: one FOLD_EPS solve
yields all 169 classes at once, so the cost is formations x sides x flops x rounds only.

What runs instead (`preflop/iterate.py`, ranges in `preflop/it/r<k>/<role>.rng`, one file per role):
- **Chart anchoring.** Round-1 ranges (v1, chart-shaped; the converged r6 BB defence for BB vs BTN/CO/UTG) are the
  prior. Hands the prior plays (weight 1) or folds (weight 0) only move when the measured margin contradicts the prior
  by more than 0.3bb; the rest is the free band and takes a 50/50 damped best response. Locks are re-checked every
  round from the measurements, so a wrong prior unlocks itself; dominance violations are counted as a sanity gate.
- **Openers** get EV(open) chained over the seats behind them (P(all fold) x 1.5, 3-bets cost the open, calls pay the
  measured flop EV minus the open), with exact card removal (`preeq.json` pairs). Unmeasured legs (HJ/CO/SB behind
  UTG/CO) use the BTN measurements and v1 ranges as proxies. **Callers**: call when flop EV > cost to call, capped by
  1 - v1 3-bet weight. 3-bet ranges stay v1 this pass.
- **Sides:** one-sided only. Two-sided sb_bb fits (7.6 GB) but is 11x slower per flop (smoke 2026-09-22), not worth it.
  BB vs BTN/CO/UTG is frozen at r6, so those formations measure the opener side only.
- **Schedule:** round 1 = 225 runs (btn_bb/co_bb/utg_bb opener side, sb_bb both, co_btn/utg_btn both; 25 stratified
  flops each, `gpu/batch_it_r1.txt`), ~5 GPU-hours. Round 2 only for roles whose gap is >= 2 points, max round 3.
  Everything resumes: `gpu/run_it.sh` skips finished flops, the collector and `auto_it.sh` can be restarted with the
  running round number, and the auto loop only (re)starts the GPU queue when the box has had no jobs for 10 minutes,
  so the main session's GPU work always wins. Mac CPU measuring is opt-in (`FOLD_MAC=1`).
- Adoption criteria unchanged: gap < 2.0 and BB defence vs CO within ~5 points of raked charts + rake adjustment.
  Not covered: HJ ranges, multiway/squeezes, the two-size tree bias (0.07-0.11bb, could be an offset later), 3-bet pots.
