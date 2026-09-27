# Beginner flow: audit and improvement plan (2026-09-27)

Audience: someone who knows the rules of poker but not why any decision is better than another.
Method: walked the live beginner flow with cleared storage at 390px, read our copy as that person, and compared
against poker teaching material and learning research (sources at the end). Nothing here is built yet.

## What a beginner has to do to make one good decision

1. Read the situation: who bet before the flop, who acts first, how big the pot is.
2. Name their own hand: top pair, a draw, nothing.
3. Estimate what the opponent holds, as groups of hands, not one hand.
4. Decide whether they are ahead or behind that group.
5. Pick a reason to put money in: value, bluff, semi-bluff, protection. No reason means check or fold.
6. Facing a bet: compare the price with how often they win.
7. See what happened and keep one rule for next time.

Today we help with steps 3 and 5 only after the answer, and we give no help with steps 1, 2, 4 and 7.

## Audit findings

Before the decision
- The screen shows cards, seat codes and two buttons. The lower half is empty. Nothing says what the situation is
  in words, what the hand is called, or what to think about.
- Jargon with no explanation: UTG, BB, bb, "97.5bb behind", "to act".
- The first spot is graded. A novice gets tested before being shown a single example.
- The welcome sheet talks about a solver and Leaks. It teaches nothing and the word solver means nothing yet.

After the decision
- About 110 words arrive at once in three sections. The one sentence that matters is in the middle.
- Percentages read as contradictions: "beats most of that", then "what continues beats you 44%", then "72% equity".
  Equity is never defined.
- The why is not tied to a named reason. "Worse hands pay you" is value betting, but the word never appears, so
  nothing accumulates across spots.
- No takeaway rule, no retry. Feedback labels the last attempt and does not change the next one.

End of a hand
- Advanced material leaks in: "Decision score 10.0 / 10", "0.07 of 0.07bb captured", "expected from the flop",
  "top 44%", "bottom 29%", "Bet 1.8bb (33%)", and the grade name "Best" next to our "Fine".
- Many beginner hands are one decision long, so Hands feels like Drills with a longer ending.

Curriculum
- Rungs are named after situations, not ideas. Ten right answers unlock the next, but there is no lesson for a rung.
- A mistake takes a point back. That punishes without teaching.
- Missed spots never come back.

## Plan: twelve improvements, in the order I would build them

| # | Idea | What changes | Why | Size |
|---|------|--------------|-----|------|
| 1 | Situation in words | One line above the buttons: "You raised before the flop. The big blind called and has checked to you." A chip names the hand: "You have top pair." | Steps 1 and 2 stop being guesswork. | S |
| 2 | Name the reason | Every answer carries a badge: Value, Bluff, Semi-bluff, Protect, Give up, Catch a bluff, Pay the price. One line defines it. Accuracy is tracked per reason. | "If you can't name your reason, check" is the core beginner rule in teaching material. Names let learning accumulate. | M |
| 3 | Layered answer | Show verdict, reason badge and one takeaway sentence. "Why?" opens Compare. "Show more" stays last. | 110 words becomes about 25, with depth on demand. | S |
| 4 | Plain numbers | Replace equity with "you win about 7 times in 10". At most two numbers per panel. Fix the ahead/behind wording so the three statements agree. | Removes the contradictions and the undefined term. | S |
| 5 | Watch first, then play | Each rung opens with a lesson card (the one idea, three lines) and two narrated examples the app plays itself, then graded spots. | Novices learn faster from worked examples; testing pays off once something has been taught. | M |
| 6 | Think-first question | On early rungs, one tap before the buttons: "Against what they can have, are you ahead, about even, or behind?" The bucket bar shows before acting on rung 1 and moves after the answer on later rungs. | Trains steps 3 and 4 directly, then fades the support. | M |
| 7 | Retry and review | After a mistake: "Try it again" at once, then the same spot returns three spots later and in the next session. Replaces "a mistake takes one back". | Feedback has to change the next attempt. Spaced return is what makes it stick. | M |
| 8 | Beginner hand ending | Strip score, EV captured and range percentiles. Show what happened, the opponent's hand named by bucket, and one line separating result from decision: "You lost the pot. The bet was still right." | Stops results-based thinking, the most common beginner error. | S |
| 9 | Rules I have learned | Each spot maps to one of about fifteen rules ("Ahead of most of their hands: bet for value"). A rule is collected the first time it is met and shown again when it recurs. | Gives step 7 a home and makes progress visible. | M |
| 10 | Contrast pairs | Same board twice in a row with a different hand, or same hand on a different board. "Last time you bet top pair here. Now you have nothing. What changes?" | Teaches that the answer depends on hand group and board, not on the cards alone. Solver teaching stresses hand classes and thresholds. | M |
| 11 | Rungs by idea | Reorder the ladder: bet for value, give up with nothing, bet a draw, pay the right price (with a price bar for pot odds), bluff when they fold a lot, the turn, facing a raise, everything. | The ladder then teaches one idea at a time in dependency order. | M |
| 12 | Words on tap | Seat codes and terms are tappable: UTG becomes "first seat to act", bb "big blinds". Beginner labels use the long form by default. | Removes the jargon wall without hiding real vocabulary. | S |

Also worth doing, smaller: a first-run sheet that shows one worked example instead of a feature list; a daily goal
of ten spots with a streak; mastery bars per reason in place of the score badge.

## Suggested first slice

Items 1, 3, 4, 8 and 12 are copy and layout only, about two days, and remove most of the confusion.
Items 2 and 9 share one mapping from spot to reason to rule and should be built together.
Items 5, 6, 7, 10 and 11 change the curriculum engine and are the larger second phase.

## Open questions

- Should the think-first question be graded, or only answered and then revealed?
- Are fifteen rules the right grain, or should each rung own three?
- Does Arena get the situation line and reason badge too, or stay untouched as decided earlier?

## Sources

- https://beatthefish.com/poker/strategy/postflop/ (four reasons to bet; if you cannot name the reason, check)
- https://www.pokerpower.com/blog/4-poker-betting-strategies-value-bluffs-protection-and-balance
- https://pokertrainer.se/postflop-fundamentals-for-beginners/
- https://blog.gtowizard.com/how-to-become-a-gto-wizard/ (study ranges and hand classes, villain's response, the why)
- https://blog.gtowizard.com/how-to-use-practice-mode-in-gto-wizard-to-improve-your-game/
- https://www.learningscientists.org/blog/2026/6/18-1 (worked examples for novices, retrieval after instruction)
- https://www.structural-learning.com/post/desirable-difficulties
- https://blog.duolingo.com/chess-course (bite-sized, guided first move, fading, spaced return)
