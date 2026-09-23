#!/usr/bin/env python3
"""Three turn cards for a flop: an overcard, a card pairing the top flop card, and a low blank; all off the flush suit."""
import sys
R, SU = "23456789TJQKA", "cdhs"
b = sys.argv[1]; cards = [b[i:i+2] for i in range(0, 6, 2)]
ranks = [c[0] for c in cards]; suits = [c[1] for c in cards]
flush = max(set(suits), key=suits.count) if len(set(suits)) < 3 else None
def pick(rank):
    for s in SU:
        if s != flush and rank + s not in cards: return rank + s
top = max(ranks, key=R.index)
over = next((r for r in "AKQJT" if R.index(r) > R.index(top)), None)
out = []
if over: out.append(pick(over))
else: out.append(pick("9" if "9" not in ranks else "8"))
out.append(pick(top))
out.append(pick(next(r for r in "23456" if r not in ranks)))
print(" ".join(out))
