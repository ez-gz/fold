# Poker reasoning rubric — distilled from Charlie Carrel + Hungry Horse Poker

Source: 20 YouTube transcripts (10 per channel), auto-captions, ~196k words condensed
and read in full. This is original synthesis — my own structuring of the recurring
concepts and question-order these two teach with, not their text. Transcripts are
saved locally in `research/transcripts/` for reference; not for reuse verbatim in
app copy (see note at bottom).

Hungry Horse is the denser source for this purpose — almost every video is a
structured strategy breakdown. Carrel's catalog has drifted toward drama/scandal
content; his useful material is narrower (tells, combinatorics, the "How Good Is X
Really" technical reviews) but shares the same bones once you're in it.

---

## 1. The one move underneath everything: ask about their range before your hand

Both creators open every spot the same way, almost liturgically: **"What is their
range?"** — not "what do I have," not "what should I do." The hand only gets
evaluated *after* the opponent's range is pinned down. This is the single biggest
structural takeaway for Fold's explanation copy: every "why" should start by
describing the villain's range, then locate the hero's hand inside it, then
derive the action. Never lead with the hero's hand strength in isolation.

The repeatable order:
1. What does their range look like right now (strong/weak, polarized/linear, capped/uncapped)?
2. Where does my hand sit relative to that range (nuts / thick value / thin value / showdown value / air)?
3. Given both, what does this specific action (check-raise, call, donk, fold) accomplish?

This maps directly onto the existing `range-first-no-typing` approach — it's
exactly the same posture, just spelled out as explicit teaching method rather than
an implicit UI constraint.

## 2. The vocabulary that does the actual work

A frequency count across the Hungry Horse transcripts is telling: "range" and its
variants dwarf every other technical term, "because" appears constantly (every claim
gets a reason attached), and specific jargon like "blockers" is comparatively rare —
used only when it's actually decision-relevant, not as a reflex. Carrel's
combinatorics video makes the same point explicitly: blockers get **wildly
overused** by amateurs reaching for a sophisticated-sounding reason when a simpler
range read already answers the question. His line, paraphrased: if your decision
hinges on a blocker, you've probably already gone wrong somewhere upstream.

**Takeaway for Fold's copy:** don't reach for "blockers," "combinatorics," or
other solver vocabulary as a default explanation. Lead with range texture and
specific, falsifiable tells (sizing, timing, line). Reserve blocker-talk for the
rare spots where it's genuinely the deciding factor — and when it is, that's worth
flagging as a slightly more advanced reasoning step (good candidate for a
"go deeper" / tap-to-expand layer, matching the beginner-mode progressive
disclosure already in the app).

## 3. The hand categories (this is close to a ready-made taxonomy)

Hungry Horse uses a consistent five-bucket classification for any hand on any
street, which could map almost directly onto drill/explanation tagging:

- **Can-play-for-stacks** (effectively the nuts or near it)
- **Thick value** (strong, but not a stacks hand — the cusp where sizing/street matters most)
- **Thin value** (ahead of parts of the range, not all of it — marginal bet-for-value)
- **Showdown value** (wins some nonzero % if hands were turned up now, but has no business betting)
- **Air** (no showdown value, no equity to speak of — pure bluff candidate or fold)

Each bucket gets a *different* default action depending on whether the opponent's
range is read as strong/weak and polarized/linear — the same hand category plays
differently against different range reads. This two-axis matrix (my hand's
bucket × their range's shape) is essentially the engine behind every one of their
"why" explanations and would be a strong organizing structure for Fold's own
explanation templates.

## 4. Range shape: the two properties that drive almost every decision

**Strong vs. weak** — inferred mainly from bet sizing and board/stack configuration:
- Bigger c-bet sizing, tighter preflop configurations (e.g. early-position-vs-early-position
  three-bet pots), and multiway pots all skew a range stronger.
- Smaller sizing, wide preflop configurations (e.g. button-vs-big-blind single-raised
  pots), and heads-up pots skew weaker — *except* on bone-dry static boards, where
  even strong hands often use a small size because there's little to protect against.

**Capped vs. uncapped** — whether the range plausibly contains the true nuts:
- A range is more capped after a passive line (check-back, small bet, call) on a
  card/street where a stronger hand would usually have raised or bet bigger.
- A range stays uncapped on "nut-changing" turns/rivers (a card that completes a
  flush/straight/full house) because the opponent could have just made the nuts.
- Capped ranges get attacked with bigger check-raise/donk sizes and wider bluffing
  frequency; uncapped ranges get played more cautiously, often with a probing small
  bet to let the opponent's genuinely strong hands raise themselves.

**Polarized vs. linear** on the river specifically changes sizing logic for both
value and bluffs:
- Against a **polarized** range (nuts-or-air), value should be sized to get looked
  up by the top of that range (often large/all-in), and bluffs can be sized *small*
  — efficient, not maximal, since the range either folds or doesn't regardless of size.
- Against a **linear/thin-value** range (lots of one-pair type hands, no explicit
  bluffs), the logic flips: bluffs need to be sized *large* to fold out the thin
  value, and value can be sized down since thin value is what's left to get paid by.

This inversion — "bluff big / value small vs. a linear range; bluff small / value
big vs. a polarized range" — is one of the more teachable, counter-intuitive rules
in the material and is a strong candidate for a standalone drill or explanation card.

## 5. Showdown value gets its own whole framework (not just "medium hand")

Hungry Horse treats showdown value as a distinct third category, not just a
weaker version of value or a reluctant fold:
- Definition: a hand that would win *some* nonzero percentage of the time if
  turned face-up right now — nothing more.
- Its default job is simply **to get to showdown**, not protect itself, not deny
  equity, not "find out where it's at." The explicit named mistake: betting a
  showdown-value hand "for protection" or "to find out" is almost always wrong —
  if it's already winning sometimes, forcing an action gives up free value it
  already had.
- The threshold for what counts as "showdown value" is range-relative, not
  hand-relative: the same holding (e.g. ace-high) might be real showdown value in
  a wide-vs-wide single-raised pot and have none at all in a tight three-bet pot,
  because the bar is set by what the rest of the range looks like.
- A concrete river checklist for *calling* with showdown value: lean toward
  calling when facing ≤60% pot, when an opponent snap-bets right as the nuts
  change, or when the bet is exactly double the previous street's size (read as an
  "I don't want to look suspicious" tell). Lean toward folding against >60% pot,
  against a big bet relative to the stakes in play (absolute size trumping relative
  size), and in tight preflop configurations where the opponent's range just
  doesn't have many natural bluffs to be polarized with.
- **Turning showdown value into a bluff** is its own decision with a *higher* bar
  than turning pure air into a bluff — because calling already has value (not
  zero), the raise-as-bluff has to beat that baseline, not just beat zero. The
  trigger is usually "the opponent's range looks linear/capped, not polarized" —
  against a polarized range, don't try it, because you're now bluffing into a
  range that still contains the stuff that beats you and folds out hands you
  were already beating anyway.

This whole module is genuinely novel-feeling compared to standard "value / bluff /
fold" teaching and would make a strong, fairly self-contained drill pack: "what do
you do with showdown value" across different range reads.

## 6. The bluff-catching checklist

A simple ordered checklist, explicitly taught as a repeatable tool rather than
intuition:

1. **Do we beat value?** (if the villain's betting range is never worse than us, stop here — fold)
2. **Is this opponent even capable of bluffing here?** (most players are, in most spots — treat "they never bluff" as the exception, not the default)
3. **Is the bet size significant for the stakes?** — absolute size, not just pot-relative percentage
4. **Does the opponent have non-showdown value in this specific spot** (i.e. natural bluff candidates given their range and the line)?
5. **Would raising as a bluff be more profitable than just calling?**

The ordering matters: questions 1–2 are cheap early exits (if either answer is
clearly "no," stop — don't bother with the rest). This is a strong candidate for a
literal interactive checklist UI component in a drill.

## 7. A taxonomy of *why mistakes happen*, not just what the mistake is

Hungry Horse splits every error into exactly two root causes, which is a genuinely
useful framing for Fold's feedback/leak-tracking:

- **Logic-based mistakes** — the reasoning chain itself breaks: the player doesn't
  follow through on what their own read implies. Classic example: correctly
  identifying an opponent's range as polarized, then taking an action that only
  makes sense against a linear range (or vice versa) — the read was fine, the
  action didn't follow from it. Another recurring pattern: sizing a bluff *down*
  specifically to avoid a rare trap, when the math says the fold-equity gained
  from going big outweighs the occasional snap-off.
- **Calibration mistakes** — the logic chain is internally consistent, but it's
  built on a wrong read of the *population* or *this specific opponent*. Example:
  correctly reasoning "if he's bluffing, I should call," while misjudging how
  often this player pool actually bluffs in that spot (usually: much less than
  assumed — recreational players chronically underbluff turns and rivers).

This is a clean, two-bucket way to tag drill misses: "your process was fine, your
read was off" vs. "your read was fine, you didn't act on it correctly." Worth
building into Fold's leak-tracking taxonomy directly — it's more actionable than a
generic "mistake" tag because it tells the player what kind of practice fixes it
(read calibration vs. decision discipline).

## 8. "Delayed gratification" — a named sizing pattern worth its own card

A specific, well-named pattern: choosing a smaller bet now specifically to keep a
wide, weak range in the pot longer, so that the *following* street's bet (once
their range has had another chance to fold its worst hands, or must now call a bet
after already committing chips) is more profitable than just blasting a big bet
immediately. Named mechanism: most players find it hard to fold after already
calling once ("sunk cost" momentum), and a small bet that looks unthreatening
tends to keep worse hands in the pot one street longer than a bet that triggers an
immediate fold decision. The flip side gets named too — "too fast to protect":
betting big too early (especially on static/dry boards or against trappy
opponents) spooks away exactly the hands you wanted to keep around, and also
telegraphs strength that makes the opponent's own continuing range stronger than
it needed to be.

## 9. Live/online tells, treated as a *range adjustment*, not an oracle

Both creators are explicit that a tell should shift a range estimate, not replace
one — "read the tell, then still do the range math" is stated outright. Concrete,
reusable tells across the material:
- **Snap-call** (especially on a scary/drawing turn) → usually marginal showdown
  value, not a monster — a monster more often takes at least a beat to decide
  how to proceed (raise? call? what size?).
- **Doubling the exact size of the previous street's bet** → read as a mild "I
  don't want to look suspicious by thinking too long about sizing" tell, skewing
  the range weaker/more bluff-heavy than the raw size alone would suggest.
- **A fast, confident snap-bet right as the nuts change** → read as more often
  a bluff — a genuine new-nuts hand usually takes a moment to pick a sizing.
- **Speech that's halting, "syncopated," or over-rehearsed while trying to sound
  confident** → skews toward a bluff; fluent, relaxed table talk (including
  banter/jokes) skews toward value, because sustaining normal conversational
  flow while bluffing is hard.
- **Breathing and physical stillness** are named as among the hardest tells to
  consciously fake — the advice given for *not* giving tells away is to
  consciously normalize breathing and body position before acting, especially on
  a big bluff.
- **Chip/cards glance order** (classic tell, both creators note it's well-known
  enough that good players will deliberately invert it) — look at a scary card or
  stack first with a strong hand, chips first with a weak one, is the "default"
  tell; sharp players sometimes fake it, so it's weighted lower for recognized
  regulars.

Framing device worth stealing directly: think of every street as the opponent
holding up a sign that says "I have a strong hand" or "I have a weak hand" —
most players broadcast this constantly through sizing and timing, just not
literally; the skill is learning to read the sign.

## 10. Combinatorics: important, but far less often than people think

Carrel's position (strongly stated, worth preserving as a design principle): combo
counting matters architecturally — understanding that a suited combo (e.g. a
specific suited connector) is only one combination while the offsuit version of
the same two ranks is many more combinations is foundational and genuinely changes
range math. But in practice, most players *overweight* blockers specifically — reaching
for "I block the nut flush" as a justification when the far bigger factor is simply
what their opponent's range looks like given the sizing and line. His heuristic:
if blockers are the deciding factor in your decision, that's a flag to double check
the read, not a green light.

## 11. "What actually matters" — Hungry Horse's own ranked priorities

One video runs through what their coaching program found did and didn't move the
needle across 800+ students, which doubles as a prioritization signal for what
Fold should spend explanation/drill depth on:

**Matters less than players think:**
- Worrying about *being* exploited (most opponents can't act on it even if they
  correctly identified a leak)
- Fine-grained balance/GTO-style frequency matching against recreational players
  specifically — if you have any population-level read that they're unbalanced,
  deviating from a theoretically "balanced" frequency is strictly better, not
  risky, against someone who can't punish it
- Over-indexing on your own hand/range in isolation, instead of starting from the
  opponent's

**Matters more than players think:**
- The *opponent's* range and tendencies, specifically: what mistake type do they
  make (under-bluffing is far more common in live low/mid stakes than
  over-bluffing — this shows up repeatedly as a background assumption across
  nearly every Hungry Horse hand)
- Understanding *why* a line is correct well enough to know when to deviate from
  it, versus memorizing the line itself — stated as the actual difference between
  a 117-page static strategy guide that didn't work for their students and what
  did
- The unglamorous fundamentals, since most edge against the actual population
  (not other regs) comes from these, not from advanced reg-vs-reg exploits

## 12. Reusable framing devices (good "voice" models for Fold's copy)

- **"Carry the thread"** — the repeated metaphor for not re-starting your read
  from scratch on every new street; each street's new information updates the
  *same* ongoing range estimate rather than resetting it. A logic-based mistake
  is very often literally "dropped the thread" — correctly read the range on the
  flop, then acted on the turn as if starting over.
- **"The question I ask is simply: X?"** — nearly every explanation is scaffolded
  as one or two short yes/no or either/or questions ("are they capped?" "will
  they stab?" "do we beat value?") rather than a paragraph of hedged analysis.
  This short-question scaffolding is very matchable to Fold's UI constraint of
  one-tap decisions with a one-line reason.
- **Teaching by question, not by answer** — Hungry Horse states directly that
  what actually improved their students wasn't being told what to think, it was
  being walked through pointed questions that let them arrive at the why
  themselves. This is argument *for* a Socratic drill format (ask the player
  "what's their range here?" before revealing the solver answer) over a flat
  explanation card — worth considering as a drill variant, not just an
  explanation-text style.

---

## What I'd actually do with this

Strongest, most concrete applications for Fold:

1. **Explanation copy template**: every "why" card restructured as (a) read their
   range — one line, naming the specific tell (sizing/line/configuration) that
   produced it, (b) locate the hero's hand in the 5-bucket taxonomy, (c) state the
   action as the thing that follows from (a)+(b), not as a free-standing rule.
2. **Leak tagging**: adopt the logic-vs-calibration split for categorizing missed
   drills — it's more actionable than a flat "wrong" and could drive different
   follow-up drill packs.
3. **A "read the range first" drill mode**: before showing the solver's answer,
   ask the player to classify the opponent's range (strong/weak, capped/uncapped,
   polarized/linear) as its own graded step, separate from the action choice —
   mirrors the Socratic teaching method directly and would catch logic-based
   mistakes specifically (right read, wrong conclusion) as distinct from
   calibration ones.
4. **A standalone showdown-value drill pack** — it's the most fleshed-out,
   least-covered-elsewhere framework in the material (the "90% of hands aren't
   nuts or air" angle), and underserved relative to how often it actually comes up.
5. **De-emphasize blocker/combinatorics language** in default explanations;
   reserve for an opt-in "advanced" layer, consistent with both the existing
   beginner-mode design and Carrel's own warning about overuse.

## Copyright note

These are notes on teaching *method and structure* — the recurring question-order,
concept taxonomy, and framing devices — written in my own words from reading the
material, not excerpts. The raw transcripts in `research/transcripts/` are kept
as private reference only; treat them as source material to learn *how* these two
reason, not as text to copy into shipped app copy, since that would be
reproducing someone else's copyrighted teaching content.
