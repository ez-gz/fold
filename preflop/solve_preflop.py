"""Preflop model v2: 6-max 100bb ranges from two-player subgames.

For every (first-in raiser R, responder S) pair the rest of the table is assumed to fold and the
game  S: fold/call/3-bet -> R: fold/call/4-bet -> S: fold/call/jam -> R: fold/call  is solved by CFR
over the 169 hand classes with exact class-level card removal. All-in pots pay preflop equity;
pots that go to a flop pay  realization x equity x pot, with realization factors measured on our
own solved flops (solver/calib/*.json) per role (position, aggressor or caller, pot type) and hand
category. R's opening range is the fixed point of "open when EV(open) > EV(fold)", where EV(open)
chains the responders in seat order (independent responders, no multiway pots).

Approximations, stated plainly: no multiway pots or squeezes, one size per action, realization is a
fitted factor rather than a flop solve per hand, the game is slightly general-sum.

usage: python3 solve_preflop.py [--iters 3000] [--outer 12]  -> preflop/ranges_v2.json
"""
import json, glob, sys, os
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
arg = lambda k, d: type(d)(sys.argv[sys.argv.index(k) + 1]) if k in sys.argv else d
ITERS, OUTER = arg("--iters", 3000), arg("--outer", 12)
RANKS = "AKQJT98765432"
SEATS = ["UTG", "HJ", "CO", "BTN", "SB", "BB"]
STACK = 100.0

def name(c):
    r, k = divmod(c, 13)
    return RANKS[r] * 2 if r == k else RANKS[r] + RANKS[k] + "s" if r < k else RANKS[k] + RANKS[r] + "o"
COMBOS = np.array([6 if c // 13 == c % 13 else 4 if c // 13 < c % 13 else 12 for c in range(169)], float)

pre = json.load(open(os.path.join(HERE, "preeq.json")))
EQ, PAIRS = np.array(pre["eq"]), np.array(pre["pairs"])
EQ = (EQ + (1 - EQ.T)) / 2                      # halve the Monte Carlo noise, force e[i][j] + e[j][i] = 1
MN = PAIRS / COMBOS[:, None]                    # opponent combos compatible with one of my combos
ME = MN * EQ

# ---------- realization factors from our solved flops ----------
def category(c):
    r, k = divmod(c, 13); hi, lo = min(r, k), max(r, k)       # index 0 = Ace
    if r == k: return "pair_hi" if r <= 4 else "pair_mid" if r <= 8 else "pair_lo"
    suited = r < k
    if suited:
        if hi == 0: return "s_ace"
        if lo <= 4: return "s_broadway"
        if lo - hi <= 2: return "s_conn"
        return "s_other"
    if hi == 0: return "o_ace_hi" if lo <= 4 else "o_ace_lo"
    if lo <= 4: return "o_broadway"
    return "o_other"
CATS = [category(c) for c in range(169)]

def load_realization():
    acc = {}
    for path in glob.glob(os.path.join(HERE, "../solver/calib/*.json")):
        d = json.load(open(path))
        pot_type = "3bp" if d["three_bet"] > 0 else "srp"
        for t in (0, 1):
            role = ("ip" if t == 1 else "oop", "agg" if t == d["opener"] else "call", pot_type)
            for c, (w, ev, eq) in enumerate(d["cells"][t]):
                for key in ((role, CATS[c]), (role, "*")):
                    a = acc.setdefault(key, [0.0, 0.0]); a[0] += ev; a[1] += eq * d["pot"]
    return {k: v[0] / v[1] for k, v in acc.items() if v[1] > 0}
REAL = load_realization()

def rf(role):
    """169-vector of realization factors for a role; falls back to the role average, then to 1."""
    base = REAL.get((role, "*"), 1.0)
    return np.array([REAL.get((role, CATS[c]), base) for c in range(169)])
def rf_4bp(role):   # no 4-bet pots solved yet: low SPR pulls realization towards 1
    return 1 + 0.5 * (rf((role[0], role[1], "3bp")) - 1)

# ---------- one raiser-vs-responder subgame ----------
def rm(regret):
    p = np.maximum(regret, 0); s = p.sum(1, keepdims=True)
    return np.where(s > 0, p / np.where(s > 0, s, 1), 1.0 / regret.shape[1])

def subgame(R, S, open_w):
    iR, iS = SEATS.index(R), SEATS.index(S)
    posted = {"SB": 0.5, "BB": 1.0}
    o = 3.0 if R == "SB" else 2.5
    s_ip = (R == "SB") if S in posted else True      # blinds are out of position, except BB against an SB open
    t3 = 3 * o if s_ip else (9.0 if R == "SB" else 11.0) if S in posted else 3 * o
    q4 = round(2.3 * t3, 1)
    dead = sum(v for k, v in posted.items() if k not in (R, S))
    pR, pS = posted.get(R, 0.0), posted.get(S, 0.0)
    posR, posS = ("oop", "ip") if s_ip else ("ip", "oop")
    # realization vectors [R, S] per flop-going terminal
    real = {"srp": (rf((posR, "agg", "srp")), rf((posS, "call", "srp"))),
            "3bp": (rf((posR, "call", "3bp")), rf((posS, "agg", "3bp"))),
            "4bp": (rf_4bp((posR, "agg")), rf_4bp((posS, "call")))}
    reg = {"S0": np.zeros((169, 3)), "R1": np.zeros((169, 3)), "S2": np.zeros((169, 3)), "R3": np.zeros((169, 2))}
    avg = {k: np.zeros_like(v) for k, v in reg.items()}

    def flop_val(kind, who, opp_reach, pot, inv):
        return real[kind][who] * (ME @ opp_reach) * pot - inv * (MN @ opp_reach)
    def fold_val(opp_reach, net): return net * (MN @ opp_reach)
    def show_val(opp_reach, pot, inv): return (ME @ opp_reach) * pot - inv * (MN @ opp_reach)

    out = {}
    for it in range(1, ITERS + 1):
        sg = {k: rm(v) for k, v in reg.items()}
        rR0 = open_w.copy(); rS0 = np.ones(169)
        rS = {a: rS0 * sg["S0"][:, a] for a in range(3)}             # S: fold, call, 3bet
        rR = {a: rR0 * sg["R1"][:, a] for a in range(3)}             # R vs 3bet: fold, call, 4bet
        rS2 = {a: rS[2] * sg["S2"][:, a] for a in range(3)}          # S vs 4bet: fold, call, jam
        rR3 = {a: rR[2] * sg["R3"][:, a] for a in range(2)}          # R vs jam: fold, call
        # --- R's values (opponent reach = S's)
        vR3 = np.stack([fold_val(rS2[2], -q4), show_val(rS2[2], 2 * STACK + dead, STACK)], 1)
        nR3 = (vR3 * sg["R3"]).sum(1)
        vR1 = np.stack([fold_val(rS[2], -o),
                        flop_val("3bp", 0, rS[2], 2 * t3 + dead, t3),
                        fold_val(rS2[0], t3 + dead) + flop_val("4bp", 0, rS2[1], 2 * q4 + dead, q4) + nR3], 1)
        nR1 = (vR1 * sg["R1"]).sum(1)
        # --- S's values (opponent reach = R's)
        vS2 = np.stack([fold_val(rR[2], -t3),
                        flop_val("4bp", 1, rR[2], 2 * q4 + dead, q4),
                        fold_val(rR3[0], q4 + dead) + show_val(rR3[1], 2 * STACK + dead, STACK)], 1)
        nS2 = (vS2 * sg["S2"]).sum(1)
        vS0 = np.stack([fold_val(rR0, -pS),
                        flop_val("srp", 1, rR0, 2 * o + dead, o),
                        fold_val(rR[0], o + dead) + flop_val("3bp", 1, rR[1], 2 * t3 + dead, t3) + nS2], 1)
        nS0 = (vS0 * sg["S0"]).sum(1)
        w = it                                                           # linear averaging, regret matching+
        for k, v, n, reach in (("R3", vR3, nR3, rR[2]), ("R1", vR1, nR1, rR0), ("S2", vS2, nS2, rS[2]), ("S0", vS0, nS0, rS0)):
            reg[k] = np.maximum(reg[k] + v - n[:, None], 0)
            avg[k] += w * reach[:, None] * sg[k]
    st = {k: v / np.maximum(v.sum(1, keepdims=True), 1e-12) for k, v in avg.items()}
    # R's view for the opening decision: fold chance of S per R class, and R's EV from the branches where S continues
    sS = st["S0"]; ones = np.ones(169); tot = MN @ ones
    f = (MN @ sS[:, 0]) / tot
    rS = {a: sS[:, a] for a in range(3)}; rS2 = {a: rS[2] * st["S2"][:, a] for a in range(3)}
    vR3 = np.stack([fold_val(rS2[2], -q4), show_val(rS2[2], 2 * STACK + dead, STACK)], 1)
    vR1 = np.stack([fold_val(rS[2], -o), flop_val("3bp", 0, rS[2], 2 * t3 + dead, t3),
                    fold_val(rS2[0], t3 + dead) + flop_val("4bp", 0, rS2[1], 2 * q4 + dead, q4) + vR3.max(1)], 1)
    cont = (flop_val("srp", 0, rS[1], 2 * o + dead, o) + vR1.max(1)) / tot
    out.update(R=R, S=S, open=o, three_bet=t3, four_bet=q4, S0=st["S0"], R1=st["R1"], S2=st["S2"], R3=st["R3"], fold=f, cont=cont)
    return out

# ---------- opening ranges: fixed point over the responders ----------
def solve_seat(R, log):
    iR = SEATS.index(R)
    responders = SEATS[iR + 1:]
    openp = np.ones(169) * 0.5
    fold_ev = -0.5 if R == "SB" else 0.0
    steal = 1.0 if R == "SB" else 1.5
    games = {}
    for k in range(OUTER):
        games = {S: subgame(R, S, np.maximum(openp, 1e-3)) for S in responders}
        ev = np.zeros(169); alive = np.ones(169)
        for S in responders:
            g = games[S]; ev += alive * g["cont"]; alive = alive * g["fold"]
        ev += alive * steal
        target = 1 / (1 + np.exp(-(ev - fold_ev) / 0.04))
        openp = 0.5 * openp + 0.5 * target if k < OUTER - 1 else openp
        log(f"  {R} round {k + 1}: opens {100 * (openp * COMBOS).sum() / 1326:.1f}%")
    return openp, games, ev

def rng_string(w):
    return ",".join(f"{name(c)}:{w[c]:.2f}" for c in np.argsort(-w) if w[c] >= 0.03)
pct = lambda w: round(100 * float((w * COMBOS).sum()) / 1326, 1)

def main():
    log = lambda s: print(s, flush=True)
    log("realization factors (role averages): " + ", ".join(f"{'/'.join(k[0])}={v:.2f}" for k, v in sorted(REAL.items()) if k[1] == "*"))
    result = {"version": "ranges-v2", "model": __doc__.split("\n\n")[0], "seats": {}}
    for R in SEATS[:-1]:
        openp, games, ev = solve_seat(R, log)
        seat = {"open": rng_string(openp), "open_pct": pct(openp), "vs": {}}
        for S, g in games.items():
            call, three = g["S0"][:, 1], g["S0"][:, 2]
            r_call, r_four = openp * g["R1"][:, 1], openp * g["R1"][:, 2]
            seat["vs"][S] = {"sizes": {"open": g["open"], "three_bet": g["three_bet"], "four_bet": g["four_bet"]},
                             "call": rng_string(call), "call_pct": pct(call), "three_bet": rng_string(three), "three_bet_pct": pct(three),
                             "raiser_calls_3bet": rng_string(r_call), "raiser_4bets": rng_string(r_four),
                             "raiser_calls_3bet_pct": pct(r_call), "raiser_4bets_pct": pct(r_four),
                             "responder_calls_4bet": rng_string(three * g["S2"][:, 1]), "responder_jams": rng_string(three * g["S2"][:, 2])}
            log(f"    vs {S}: {S} calls {pct(call)}% 3-bets {pct(three)}% | {R} vs 3-bet: calls {pct(r_call)}% 4-bets {pct(r_four)}% of all hands")
        result["seats"][R] = seat
    json.dump(result, open(os.path.join(HERE, "ranges_v2.json"), "w"), indent=1)
    log("wrote preflop/ranges_v2.json")

if __name__ == "__main__":
    main()
