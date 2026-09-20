"""One round of the big-blind defence iteration.  usage: python3 round.py <k>
Reads measured EVs from solver/calib_pre_r<k>, best-responds (call when root EV > 1.5bb), blends 50/50 with the range
that round was solved with (damping: a pure best response overshoots, because the opener's strategy in that solve was
tuned against the narrower range), caps by 1 - (v1 3-bet share), writes preflop/rounds/r<k+1>/<formation>.p0."""
import json, glob, os, sys, math, re, subprocess, collections
HERE = os.path.dirname(os.path.abspath(__file__)); k = int(sys.argv[1])
RANKS = "AKQJT98765432"
def name(c):
    r, q = divmod(c, 13)
    return RANKS[r] * 2 if r == q else RANKS[r] + RANKS[q] + "s" if r < q else RANKS[q] + RANKS[r] + "o"
IDX = {name(c): c for c in range(169)}
COMBOS = [6 if c // 13 == c % 13 else 4 if c // 13 < c % 13 else 12 for c in range(169)]
def parse(s):
    w = [0.0] * 169
    for tok in s.split(","):
        body, _, wt = tok.partition(":"); wt = float(wt) if wt else 1.0
        plus = body.endswith("+"); body = body.rstrip("+")
        hi, lo = RANKS.index(body[0]), RANKS.index(body[1])
        if hi == lo: cells = [name(r * 14) for r in range(0 if plus else hi, hi + 1)]
        else: cells = [body[0] + RANKS[l] + body[2] for l in range(hi + 1 if plus else lo, lo + 1)]
        for n in cells: w[IDX[n]] = wt
    return w
src = open(os.path.join(HERE, "../solver/src/preflop.rs")).read()
three = {f: parse(re.search(rf'BB_3BET_VS_{s}: &str = "([^"]+)"', src).group(1)) for f, s in (("btn_bb", "BTN"), ("co_bb", "CO"), ("utg_bb", "UTG"))}
cli = os.path.join(HERE, "../solver/target-v1/release/fold-cli")
os.makedirs(os.path.join(HERE, f"rounds/r{k + 1}"), exist_ok=True)
for form in ("btn_bb", "co_bb", "utg_bb"):
    prev_file = os.path.join(HERE, f"rounds/r{k}/{form}.p0")
    prev = parse(open(prev_file).read().strip()) if os.path.exists(prev_file) else parse(subprocess.run([cli, "classrange", form, "0"], capture_output=True, text=True).stdout.strip())
    evs = collections.defaultdict(list)
    for p in glob.glob(os.path.join(HERE, f"../solver/calib_pre_r{k}/{form}_*.json")):
        d = json.load(open(p))
        if d["formation"] != form: continue
        for c, (n, ev, eq) in enumerate(d["cells"][0]):
            if n > 0: evs[c].append(ev / n)
    br = [0.0] * 169
    for c, v in evs.items(): br[c] = 1 / (1 + math.exp(-max(-40, min(40, (sum(v) / len(v) - 1.5) / 0.05))))
    new = [min(0.5 * prev[c] + 0.5 * br[c], 1 - three[form][c]) for c in range(169)]
    pct = lambda w: 100 * sum(w[c] * COMBOS[c] for c in range(169)) / 1326
    open(os.path.join(HERE, f"rounds/r{k + 1}/{form}.p0"), "w").write(",".join(f"{name(c)}:{new[c]:.3f}" for c in range(169) if new[c] >= 0.02))
    print(f"{form}: solved with {pct(prev):.1f}% calls -> best response {pct(br):.1f}% (incl. 3-bet hands) -> next round {pct(new):.1f}% calls + {pct(three[form]):.1f}% 3-bets, {len(next(iter(evs.values())))} flops")
