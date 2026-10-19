# Applying the poker-reasoning rubric to Fold — concrete next steps

Source: [`poker-reasoning-rubric.md`](./poker-reasoning-rubric.md), checked against the
actual code in `proto/index.html`. Every recommendation below is anchored to a real
function so it's buildable rather than aspirational. Ordered by impact × how little new
infrastructure it needs.

## First: what's already built that matches the rubric

Worth stating plainly so nothing below gets mistaken for new ground. Fold already
implements more of this rubric than a blank-slate read would suggest:

- **Range-first teaching is already the core loop.** `thinkFirst()` asks "Whose range is
  stronger?" *before* revealing the answer — exactly the Socratic move the rubric's §12
  argues for. `trackReason("Range read", ok)` already logs whether the range read was
  correct, separately from whether the action was correct.
- **Capped / polarized / linear are already computed**, not just described in theory.
  `rangeFacts()` derives `capped`, `shape` ("polarized" / "linear"), `best` and `worst`
  from the solver's class weights, and `rangeDuel()` renders them as tappable tags with
  one-line explanations. The rubric's §4 is effectively already shipped as UI.
- **A 10-reason taxonomy already exists** (`REASONS`: Value, Bluff, Semi-bluff, Protect,
  Trap, Pot control, Free card, Give up, Bluff-catch, Price) and every decision gets
  tagged via `reasonOf()`. That's close to the rubric's 5-bucket hand taxonomy, just
  action-shaped instead of hand-shaped — see §1 for the one real gap in it.
- **Mistake direction is already tagged.** `errKind()` labels *how* an answer was wrong
  ("fold too much", "size too big", "bet when checking is best"). Adjacent to, but not
  the same as, the rubric's logic/calibration split — §3 is the actual gap.
- **"Carry the thread" is implicit in `concept()`**, which already names streets by what
  preceded them (second barrel, third barrel, delayed c-bet, turn probe) rather than
  treating each street as fresh.
- **Blockers are already de-emphasized.** `blocker` appears only in a code comment, never
  in explanation copy. That's correct per §10 — see §5 for keeping it that way.

So these recommendations are deltas, not a rebuild.

---

## 1. Give "showdown value" its own identity (biggest single gap)

**The gap.** `reasonOf()`'s check branch currently collapses showdown value into
"Pot control" (`eq >= 0.35`) or "Give up" below it. The rubric's §5 is the most
developed original framework in the source material: showdown value as a *third*
category with its own job (just get to showdown — don't bet it for "protection"), its
own river call/fold heuristics, and a higher bar for turning into a bluff. Right now the
app has no concept of "this hand's only job is to show up at showdown." The nearest
label, "Pot control", implies a different intent — *deliberately keep the pot small* —
rather than *don't touch it*.

**Concrete change.**
- Split the check branch in `reasonOf()`: add a `"Showdown value"` reason distinct from
  `"Pot control"`. Roughly, no-draw hands in the `eq ∈ [0.35, 0.5)` band with no real
  protection need get "Showdown value".
- Add the definition alongside the others, e.g.
  `"Showdown value": ["🪑", "Wins sometimes if it gets to showdown. Not strong enough to bet — don't touch it."]`
  (copy needs a pass; the distinction is the point).
- This sharpens `whyPlay()`'s check branch immediately, since a showdown-value hand and a
  genuine pot-control hand currently share one copy template.

**Second-order payoff.** Once "Showdown value" is a tagged reason it becomes queryable
like every other reason — `worstConcepts` / `worstTypes` / the leak tracking all key off
`store.get("book")`. A player who habitually bets their showdown-value hands (the
rubric's named #1 mistake: betting "to protect" or "to find out") becomes a detectable,
nameable leak for the first time instead of being invisible inside "Pot control".

## 2. A standalone showdown-value drill pack

**The gap.** Nothing in drill selection filters for "hand has showdown value, villain
checked back or used a small bet, what now" — the richest, most differentiated teaching
module in the source material has no dedicated practice surface.

**Concrete change.** Once §1 lands, add it as a filterable concept the same way
`leakPlan()` / `trainFocus` already filter by concept string (`focusObj`). The pack pulls
turn/river spots where `reasonOf(...) === "Showdown value"` is correct, and uses the
rubric's river call/fold heuristics as that pack's explanation template: lean call at
≤60% pot, lean call when the size is exactly double the previous one, lean fold at >60%
pot or when the absolute size is big for the stakes.

## 3. Logic mistake vs. calibration mistake — genuinely new, nearly free

**The gap.** `errKind()` says *what* was wrong (direction) but not *why* — whether the
player misjudged the range, or read it correctly and still acted wrong. The rubric's §7
names this the cleanest split for actionable feedback. The useful part: **the app already
has the exact signal needed.** `thinkFirst()` asks "whose range is stronger?" and logs it
via `trackReason("Range read", ok)` *before* the player picks an action.

**Concrete change.** When both signals exist for one decision, tag the miss:
- **Calibration mistake** — range read was wrong. The fix is reps and exposure at reading
  sizing and lines, not decision logic.
- **Logic mistake** — range read was right, action still wrong. The fix is the decision
  rule itself (e.g. correctly spotting a polarized range but still sizing wrong, which is
  exactly the inversion §4 flags as a common nameable error).

This only requires joining two things already tracked — `store.get("reasons")` for range-read
correctness and `store.get("book")` / the log for action correctness — at logging time in
`logDecision()`. No new player input, no new UI, just a richer `err` field. The Leaks
sheet could then distinguish "you're reading ranges fine but not acting on them" from
"you're misjudging ranges" — two completely different prescriptions from the same raw
mistake count.

**Caveat.** This only covers decisions where `thinkFirst()` actually ran (beginner mode,
first decision of a spot — see the guard in `renderActions`). It's a sample, not full
coverage, and strictly additive to `errKind` rather than a replacement.

## 4. Bluff-catching checklist as an explicit breakdown

**The gap.** "Bluff-catch" exists as a single tag, but `explainFull()` only shows the
`"Price:"` math for call/fold — need vs. have, nothing about *why* the villain's range
holds enough bluffs to make that price good. The rubric's §6 checklist is a repeatable
tool rather than spot-specific trivia, which makes it a good fit for a reusable template:
beat value? → capable of bluffing? → size significant in absolute terms? → non-showdown
value present? → is raising better than calling?

**Concrete change.** For calls specifically (`a.kind === "call"` in `explainFull`), render
the checklist as short explicit lines using data the solver output already provides.
`topClasses(f, x.cont, ...)` already lists what villain continues with, which answers "do
we beat value" and "do they have non-showdown value" directly; `x.fold` and the sizing
answer "is the size significant". This turns an implicit judgment into a visible named
process the player can run themselves away from the app — which is the real goal:
teaching a transferable method, not grading one spot.

## 5. Keep blockers de-emphasized (a guardrail, not a change)

No gap today — `blocker` logic appears only in a code comment about made-hand detection,
never in explanation copy. Flagging it so it stays true: the rubric (§10) and Carrel
specifically warn that blocker reasoning is the most over-used crutch among amateurs
reaching for a sophisticated-sounding justification. As explanation copy grows via §1 and
§4, resist adding "...and you block their nut flush" as a default sentence. If blocker
logic is ever added, gate it behind "Full breakdown" / an advanced toggle, consistent with
the existing `state.beginner` philosophy — never in the one-sentence beginner explanation.

## 6. "Why this street", not just "why this hand" — small copy upgrade

**The gap.** `concept()` already *names* the street situation ("Second barrel", "River
probe") and that string already drives `logDecision` and `track` — but it's internal
bookkeeping, never surfaced in `explain()` / `whyPlay()` prose. The rubric's "carry the
thread" framing (§12) is explicitly about *showing* the street-to-street work, not just
computing it correctly underneath.

**Concrete change.** Prepend the `concept(f, s)` label — already computed, already a clean
short string — as a header at the top of `explain()`'s output on turn/river streets, so
the player sees "Second barrel" or "River probe" before the numbers. Nothing new to
compute; purely surfacing an existing label.

---

## What I'd skip, and why

- **Live / physical tells (§9)** — timing, breathing, speech patterns. No mapping to a
  solver-drill app; they need video/audio input Fold doesn't have. At most a short static
  "if you're playing live" page, separate from the drill engine.
- **Combinatorics as taught content (§10)** — Carrel's own point is that players already
  over-index on it. The app already avoids leading with combo-counting. No change needed;
  just don't regress it while doing the above.
- **A full 5-bucket hand-strength overlay (§3)** — `REASONS` plus `fineType` / `coarseType`
  already cover this ground at finer granularity than the rubric's five buckets.
  Re-skinning onto the rubric's exact names is a relabeling exercise, not a teaching
  improvement.

## Suggested order of work

1. **§1 — Showdown value as its own reason.** Smallest diff, biggest downstream leverage;
   unlocks §2 and sharpens existing copy for free.
2. **§3 — logic vs. calibration tagging.** Nearly free given existing signals, directly
   improves how actionable the Leaks sheet is.
3. **§6 — surface the `concept()` label.** Trivial, immediate legibility win.
4. **§4 — bluff-catching checklist.** More copywriting effort, but self-contained.
5. **§2 — dedicated showdown-value drill pack.** Depends on §1 landing first.

All six respect the two standing product constraints: range-first reasoning, and never
requiring keyboard input.
