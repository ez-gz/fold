"""Chart-anchored preflop iteration over every formation (SPEC 13.7, revised 2026-09-22).

Ranges live in preflop/it/r<k>/<role>.rng, one file per role (open_BTN, call_BB_vs_CO, ...). Each round measures the
flop EV of every hand class with one-sided FOLD_EPS solves on the coarse "pre" tree (a solve yields all 169 classes at
once, so pruning hands saves nothing; the cost is formations x sides x flops x rounds), then best-responds:

  caller S facing an open:  call when  mean flop EV(h) - cost_to_call > 0, capped by 1 - v1 3-bet weight
  opener R:  open when  EV_open(h) = P(everyone folds) * blinds + sum_j P(seat j is first to act) *
             [ P(j 3-bets) * (-open) + P(j calls) * (flop EV vs j - open) ]  > 0     (folding = 0)

Chart anchoring (the "start from AA and work down" idea in the form that is actually cheap): the round-1 ranges
(v1, chart-shaped; the converged r6 defence for BB) are the prior. A hand locked in by the prior (weight 1) only moves when its measured margin is worse than -LOCK bb; a hand
locked out (weight 0) only moves when its margin is better than +LOCK bb. Everything else is the free band and takes a
50/50 damped best response. Locks are checked against the measurements every round, so a wrong prior unlocks itself.

usage: iterate.py init                      -> it/r1 from v1 (+ the converged r6 BB defence)
       iterate.py env <k> <form> <side>     -> shell exports for the measurement solve (side 0 | 1 | b)
       iterate.py batch <k> [--stage 1|2|all] -> gpu/batch_it_r<k>.txt   ("<form> <flop> <side>", open roles only)
       iterate.py step <k>                  -> it/r<k+1>/*.rng, it/r<k>/summary.json, prints GAP
"""
import json, glob, os, sys, math, re, collections
HERE = os.path.dirname(os.path.abspath(__file__)); ROOT = os.path.dirname(HERE)
RANKS = "AKQJT98765432"
LOCK, DAMP, SCALE, GAP_OK = 0.3, 0.5, 0.1, 2.0

def name(c):
    r, q = divmod(c, 13)
    return RANKS[r] * 2 if r == q else RANKS[r] + RANKS[q] + "s" if r < q else RANKS[q] + RANKS[r] + "o"
IDX = {name(c): c for c in range(169)}
COMBOS = [6 if c // 13 == c % 13 else 4 if c // 13 < c % 13 else 12 for c in range(169)]
pct = lambda w: 100 * sum(w[c] * COMBOS[c] for c in range(169)) / 1326
fmt = lambda w: ",".join(f"{name(c)}:{w[c]:.3f}" for c in range(169) if w[c] >= 0.02)

def parse(s):
    w = [0.0] * 169
    for tok in s.split(","):
        tok = tok.strip()
        if not tok: continue
        body, _, wt = tok.partition(":"); wt = float(wt) if wt else 1.0
        plus = body.endswith("+"); body = body.rstrip("+")
        if "-" in body:                      # "JJ-22", "AJs-A2s"
            a, b = body.split("-"); hi, lo = RANKS.index(a[1]), RANKS.index(b[1])
            cells = [name(r * 14) for r in range(hi, lo + 1)] if a[0] == a[1] else [a[0] + RANKS[l] + a[2] for l in range(hi, lo + 1)]
        else:
            hi, lo = RANKS.index(body[0]), RANKS.index(body[1])
            if hi == lo: cells = [name(r * 14) for r in range(0 if plus else hi, hi + 1)]
            else: cells = [body[0] + RANKS[l] + body[2] for l in range(hi + 1 if plus else lo, lo + 1)]
        for n in cells: w[IDX[n]] = wt
    return w

SRC = open(os.path.join(ROOT, "solver/src/preflop.rs")).read()
def v1(const): return parse(re.search(rf'{const}: &str = "([^"]+)"', SRC).group(1))
PAIRS = json.load(open(os.path.join(HERE, "preeq.json")))["pairs"]      # compatible combo pairs per class pair
def opp_dist(h):
    """P(opponent holds class c | I hold class h), exact card removal."""
    row = [PAIRS[h][c] / COMBOS[h] for c in range(169)]; t = sum(row)
    return [x / t for x in row]

# formation -> (roles per player, opener player, open size, blind already posted by each player)
FORMS = {
    "btn_bb": (["call_BB_vs_BTN", "open_BTN"], 1, 2.5, [1.0, 0.0]),
    "co_bb":  (["call_BB_vs_CO", "open_CO"], 1, 2.5, [1.0, 0.0]),
    "utg_bb": (["call_BB_vs_UTG", "open_UTG"], 1, 2.5, [1.0, 0.0]),
    "sb_bb":  (["open_SB", "call_BB_vs_SB"], 0, 3.0, [0.5, 1.0]),
    "co_btn": (["open_CO", "call_BTN_vs_CO"], 0, 2.5, [0.0, 0.0]),
    "utg_btn": (["open_UTG", "call_BTN_vs_UTG"], 0, 2.5, [0.0, 0.0]),
}
V1 = {"open_UTG": "UTG_OPEN", "open_CO": "CO_OPEN", "open_BTN": "BTN_OPEN", "open_SB": "SB_OPEN",
      "call_BB_vs_UTG": "BB_CALL_VS_UTG", "call_BB_vs_CO": "BB_CALL_VS_CO", "call_BB_vs_BTN": "BB_CALL_VS_BTN",
      "call_BB_vs_SB": "BB_CALL_VS_SB", "call_BTN_vs_CO": "BTN_CALL_VS_CO", "call_BTN_vs_UTG": "BTN_CALL_VS_UTG"}
# 3-bet ranges stay v1 this pass (3-bet pots are stage 3, not scheduled); proxies where v1 has no constant
THREEBET = {"call_BB_vs_UTG": "BB_3BET_VS_UTG", "call_BB_vs_CO": "BB_3BET_VS_CO", "call_BB_vs_BTN": "BB_3BET_VS_BTN",
            "call_BB_vs_SB": "BB_3BET_VS_BTN", "call_BTN_vs_CO": "BTN_3BET_VS_CO", "call_BTN_vs_UTG": "BB_3BET_VS_UTG"}
# responder chain per opener, in seat order: (call range role or None, call scale, 3-bet v1 const, flop-EV measurement (form, side))
CHAIN = {
    "UTG": [("HJ", "call_BTN_vs_UTG", 0.5, "BB_3BET_VS_UTG", ("utg_btn", 0)), ("CO", "call_BTN_vs_UTG", 0.6, "BB_3BET_VS_UTG", ("utg_btn", 0)),
            ("BTN", "call_BTN_vs_UTG", 1.0, "BB_3BET_VS_UTG", ("utg_btn", 0)), ("SB", None, 0, "BB_3BET_VS_UTG", None),
            ("BB", "call_BB_vs_UTG", 1.0, "BB_3BET_VS_UTG", ("utg_bb", 1))],
    "CO": [("BTN", "call_BTN_vs_CO", 1.0, "BTN_3BET_VS_CO", ("co_btn", 0)), ("SB", None, 0, "BTN_3BET_VS_CO", None),
           ("BB", "call_BB_vs_CO", 1.0, "BB_3BET_VS_CO", ("co_bb", 1))],
    "BTN": [("SB", None, 0, "SB_3BET_VS_BTN", None), ("BB", "call_BB_vs_BTN", 1.0, "BB_3BET_VS_BTN", ("btn_bb", 1))],
    "SB": [("BB", "call_BB_vs_SB", 1.0, "BB_3BET_VS_BTN", ("sb_bb", 1))],
}
BLINDS = 1.5
# measurement jobs: stage 1 = opening ranges vs BB and the SB/BB pair, stage 2 = BTN's cold calls (and CO/UTG opens vs them)
JOBS = {1: [("btn_bb", 1), ("co_bb", 1), ("utg_bb", 1), ("sb_bb", 0), ("sb_bb", 1)],
        2: [("co_btn", 0), ("co_btn", 1), ("utg_btn", 0), ("utg_btn", 1)]}
FLOPS = {}
for path in ("gpu/batch_pre.txt", "gpu/batch_it_flops.txt"):
    for line in open(os.path.join(ROOT, path)) if os.path.exists(os.path.join(ROOT, path)) else []:
        f, fl = line.split()[:2]; FLOPS.setdefault(f, []).append(fl)

rdir = lambda k: os.path.join(HERE, f"it/r{k}")
def load(k, role): return parse(open(os.path.join(rdir(k), role + ".rng")).read().strip())
def save(k, role, w):
    os.makedirs(rdir(k), exist_ok=True); open(os.path.join(rdir(k), role + ".rng"), "w").write(fmt(w) + "\n")

def cmd_init():
    for role, const in V1.items():
        w = v1(const)
        if role in ("call_BB_vs_BTN", "call_BB_vs_CO", "call_BB_vs_UTG"):      # converged measured-EV defence (auto.log round 5)
            w = parse(open(os.path.join(HERE, f"rounds/r6/{role.split('_vs_')[1].lower()}_bb.p0")).read().strip())
        save(1, role, w)
    print("it/r1:", ", ".join(f"{r} {pct(load(1, r)):.1f}%" for r in V1))

def cmd_env(k, form, side):
    roles = FORMS[form][0]
    print(f"export FOLD_TREE=pre FOLD_EPS=0.02")
    print("unset FOLD_EPS_P" if side == "b" else f"export FOLD_EPS_P={side}")
    for p in (0, 1): print(f"export FOLD_RANGE{p}='{fmt(load(k, roles[p]))}'")

def cmd_batch(k, stage):
    stages = [1, 2] if stage == "all" else [int(stage)]
    done = json.load(open(os.path.join(rdir(k - 1), "summary.json")))["open_roles"] if k > 1 else None
    lines = []
    for s in stages:
        for form, side in JOBS[s]:
            if done is not None and FORMS[form][0][side] not in done and FORMS[form][0][1 - side] not in done: continue
            lines += [f"{form} {fl} {side}" for fl in FLOPS[form]]
    out = os.path.join(ROOT, f"gpu/batch_it_r{k}.txt"); open(out, "w").write("\n".join(lines) + "\n")
    print(f"{out}: {len(lines)} runs")

def measured(k, form, side):
    """mean root EV per class (chips from the pot) for player `side`, over the flops measured this round; None if none."""
    evs = collections.defaultdict(list)
    for p in glob.glob(os.path.join(ROOT, f"solver/calib_it_r{k}/{form}_*_{side}.json")) + glob.glob(os.path.join(ROOT, f"solver/calib_it_r{k}/{form}_*_b.json")):
        d = json.load(open(p))
        for c, cell in enumerate(d["cells"][side]):
            if cell[0] > 0: evs[c].append(cell[1] / cell[0])
    if len(evs) < 160: return None, 0
    return [sum(evs[c]) / len(evs[c]) if c in evs else -99.0 for c in range(169)], max(len(v) for v in evs.values())

sig = lambda x: 1 / (1 + math.exp(-max(-40, min(40, x / SCALE))))
def anchored(prior, prev, margin):
    """best response with chart locks and damping; returns (new weights, best-response weights)"""
    br = [sig(m) for m in margin]; new = [0.0] * 169
    for c in range(169):
        if prior[c] >= 0.999 and margin[c] > -LOCK: new[c] = 1.0
        elif prior[c] <= 0.001 and margin[c] < LOCK: new[c] = 0.0
        else: new[c] = (1 - DAMP) * prev[c] + DAMP * br[c]
    return new, br

def dominance_violations(w):
    """count of (better hand, worse hand) pairs where the worse one is played more (>0.3): a sanity gate, not enforced"""
    bad = 0
    for c in range(169):
        r, q = divmod(c, 13)
        if r == q and r > 0: bad += w[c] > w[(r - 1) * 14] + 0.3
        elif r != q:
            hi, lo = min(r, q), max(r, q)
            better = (hi, lo - 1) if lo - 1 > hi else None
            if better:
                b = better[0] * 13 + better[1] if r < q else better[1] * 13 + better[0]
                bad += w[c] > w[b] + 0.3
    return bad

def cmd_step(k):
    summary, gaps, open_roles = {}, {}, []
    # callers
    for form, (roles, opener, osize, blinds) in FORMS.items():
        p = 1 - opener; role = roles[p]
        ev, n = measured(k, form, p)
        if ev is None: continue
        prev, prior, three = load(k, role), load(1, role), v1(THREEBET[role])
        margin = [ev[c] - (osize - blinds[p]) for c in range(169)]
        new, br = anchored(prior, prev, margin)
        new = [min(new[c], 1 - three[c]) for c in range(169)]; br = [min(br[c], 1 - three[c]) for c in range(169)]
        save(k + 1, role, new); gaps[role] = abs(pct(br) - pct(prev))
        summary[role] = dict(flops=n, prev=pct(prev), br=pct(br), next=pct(new), v1=pct(v1(V1[role])), gap=gaps[role], dominance=dominance_violations([new[c] + three[c] for c in range(169)]))
    # openers (need every measured chain leg; unmeasured legs fall back to the previous round's estimate = no update)
    for seat in ("UTG", "CO", "BTN", "SB"):
        role = f"open_{seat}"; prev, prior = load(k, role), load(1, role)
        legs = []
        for (s, crole, cscale, tconst, meas) in CHAIN[seat]:
            call = [cscale * x for x in load(k, crole)] if crole else [0.0] * 169
            three = v1(tconst); ev = None
            if meas:
                ev, n = measured(k, meas[0], meas[1])
                if ev is None: break
            legs.append((call, three, ev))
        else:
            osize = 3.0 if seat == "SB" else 2.5
            margin = []
            for h in range(169):
                d = opp_dist(h); alive, total = 1.0, 0.0
                for call, three, ev in legs:
                    pc = sum(d[c] * call[c] * (1 - three[c]) for c in range(169)); p3 = sum(d[c] * three[c] for c in range(169))
                    if ev is not None: total += alive * pc * (ev[h] - osize)
                    total += alive * p3 * (-osize); alive *= 1 - pc - p3
                total += alive * BLINDS
                margin.append(total)
            new, br = anchored(prior, prev, margin)
            save(k + 1, role, new); gaps[role] = abs(pct(br) - pct(prev))
            summary[role] = dict(prev=pct(prev), br=pct(br), next=pct(new), v1=pct(v1(V1[role])), gap=gaps[role], dominance=dominance_violations(new))
            continue
        save(k + 1, role, prev)
    for role in V1:                                   # roles without a measurement carry over unchanged
        if not os.path.exists(os.path.join(rdir(k + 1), role + ".rng")): save(k + 1, role, load(k, role))
    open_roles = [r for r, g in gaps.items() if g >= GAP_OK]
    json.dump(dict(round=k, roles=summary, open_roles=open_roles), open(os.path.join(rdir(k), "summary.json"), "w"), indent=1)
    for r, s in summary.items():
        print(f"{r}: solved with {s['prev']:.1f}% -> best response {s['br']:.1f}% -> next {s['next']:.1f}% (v1 {s['v1']:.1f}%, gap {s['gap']:.1f}, dominance violations {s['dominance']})")
    print(f"GAP {max(gaps.values()) if gaps else 0:.1f}  open: {' '.join(open_roles) or 'none'}")

if __name__ == "__main__":
    a = sys.argv[1:]
    if a[0] == "init": cmd_init()
    elif a[0] == "env": cmd_env(int(a[1]), a[2], a[3])
    elif a[0] == "batch": cmd_batch(int(a[1]), a[a.index("--stage") + 1] if "--stage" in a else "all")
    elif a[0] == "step": cmd_step(int(a[1]))
    else: print(__doc__)
