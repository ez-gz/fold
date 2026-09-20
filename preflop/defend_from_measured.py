"""Big-blind calling ranges from measured flop EVs (solver/calib_pre/*.json, made by solver/measure_all.sh).
Each solve carried every hand at a tiny weight, so cells hold the real flop EV of every class against the opener's
range. Calling costs 1.5bb more than folding, so a class calls when its mean root EV exceeds 1.5bb; hands already in the
v1 3-bet range keep that 3-bet share. usage: python3 defend_from_measured.py -> preflop/defend_v2.json"""
import json, glob, os, math, collections
HERE = os.path.dirname(os.path.abspath(__file__))
RANKS = "AKQJT98765432"
def name(c):
    r, k = divmod(c, 13)
    return RANKS[r] * 2 if r == k else RANKS[r] + RANKS[k] + "s" if r < k else RANKS[k] + RANKS[r] + "o"
COMBOS = [6 if c // 13 == c % 13 else 4 if c // 13 < c % 13 else 12 for c in range(169)]
acc = collections.defaultdict(lambda: [[] for _ in range(169)])
for p in glob.glob(os.path.join(HERE, "../solver/calib_pre/*.json")):
    d = json.load(open(p))
    if d["formation"] not in ("btn_bb", "co_bb", "utg_bb"): continue
    for c, (n, ev, eq) in enumerate(d["cells"][0]):
        if n > 0: acc[d["formation"]][c].append(ev / n)
out = {}
for form, cells in acc.items():
    rows = []
    for c, v in enumerate(cells):
        if not v: continue
        m = sum(v) / len(v); se = (sum((x - m) ** 2 for x in v) / max(len(v) - 1, 1) / len(v)) ** 0.5
        rows.append((c, m - 1.5, se))
    w = {c: 1 / (1 + math.exp(-max(-40, min(40, gain / 0.05)))) for c, gain, se in rows}
    pct = 100 * sum(w[c] * COMBOS[c] for c in w) / 1326
    out[form] = {"flops": len(cells[0]), "continue_pct": round(pct, 1),
                 "continue": ",".join(f"{name(c)}:{w[c]:.2f}" for c, g, se in sorted(rows, key=lambda r: -r[1]) if w[c] >= 0.03),
                 "closest_calls": [f"{name(c)} {g:+.2f}bb (se {se:.2f})" for c, g, se in sorted(rows, key=lambda r: abs(r[1]))[:12]]}
    print(f"{form}: {len(cells[0])} flops, BB continues {pct:.1f}% | closest: " + ", ".join(out[form]["closest_calls"][:6]))
json.dump(out, open(os.path.join(HERE, "defend_v2.json"), "w"), indent=1)
