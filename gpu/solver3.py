"""GPU DCFR for one flop with three seats, batched over runouts. Mirrors solver/src/{tree3,cfr3,three}.rs.

Tensors carry leading batch dims by street: flop [], turn [ND], river [ND, ND] (both index the non-flop cards;
the diagonal and card-conflicting hands carry zero reach). Seats act in index order (0 first). A folded seat
drops out of the action but its range keeps removing cards, so every terminal value is a mass of card-disjoint
pairs of the other two seats' hands, split by strength band (weaker / equal / any) relative to the hero hand:
    pairs(P, Q) = A_P A_Q - sum_c C_P[c] C_Q[c] + sum_h w_P(h) w_Q(h)
restricted to hands that avoid the hero's two cards (three.rs has the derivation).
"""
import argparse, json, time
import numpy as np
import torch

p = argparse.ArgumentParser()
p.add_argument("spec")
p.add_argument("--iters", type=int, default=300)
p.add_argument("--target", type=float, default=0.5)
p.add_argument("--every", type=int, default=25)
p.add_argument("--device", default="cuda")
p.add_argument("--chunk", type=int, default=0, help="turn cards per showdown chunk; 0 = as many as ~1.5 GB of transients allow")
p.add_argument("--dump", default="")
p.add_argument("--dtype", default="float32", help="storage dtype for regrets and strategy sums")
p.add_argument("--compile", type=int, default=1, help="torch.compile the showdown and pair-mass kernels")
args = p.parse_args()
dev = torch.device(args.device)
STORE = getattr(torch, args.dtype)

S = json.load(open(args.spec + ".json"))
HANDS = [np.array(h, dtype=np.int64) for h in S["hands"]]
NH = [len(h) for h in HANDS]
DECK = S["deck"]
ND = len(DECK)
P0, STACK = S["start_pot"], S["eff_stack"]
CAPS = S["max_raises"]
W = [torch.tensor(w, dtype=torch.float32, device=dev) for w in S["weights"]]
raw = np.fromfile(args.spec + ".bin", dtype=np.uint32).reshape(ND, ND, sum(NH)).astype(np.int64)
off = np.cumsum([0] + NH)
STR = [torch.tensor(raw[:, :, off[i]:off[i + 1]], device=dev) for i in range(3)]
BIG = 1 << 40
STRP = [torch.nn.functional.pad(STR[i], (0, 1), value=BIG) for i in range(3)]   # pad slot: never in a band, zero reach
del raw


def others(t):
    return [(t + 1) % 3, (t + 2) % 3]


# ---------------------------------------------------------------- card bookkeeping
INC = []
for pl in range(3):
    m = torch.zeros(NH[pl], 52, device=dev)
    ar = torch.arange(NH[pl], device=dev)
    m[ar, torch.tensor(HANDS[pl][:, 0], device=dev)] = 1
    m[ar, torch.tensor(HANDS[pl][:, 1], device=dev)] = 1
    INC.append(m)
DECK_T = torch.tensor(DECK, device=dev)
VALID = [1.0 - INC[pl][:, DECK_T].T.contiguous() for pl in range(3)]           # [ND, H]: hand does not hold deck card
NOTMINE = [1.0 - INC[pl] for pl in range(3)]                                   # [H, 52]: card is not one of the hand's
IDX = []                                                                       # [52, 52] -> hand index in pl's list, NH[pl] = none
for pl in range(3):
    m = torch.full((52, 52), NH[pl], device=dev, dtype=torch.int64)
    a, b = torch.tensor(HANDS[pl][:, 0], device=dev), torch.tensor(HANDS[pl][:, 1], device=dev)
    ar = torch.arange(NH[pl], device=dev)
    m[a, b] = ar; m[b, a] = ar
    IDX.append(m)
CARD = [(torch.tensor(HANDS[pl][:, 0], device=dev), torch.tensor(HANDS[pl][:, 1], device=dev)) for pl in range(3)]
# per (hero t, other o): index in o's list of the combo (c, hero card) for every card c -> [H_t, 52], and the identical combo
PARTNER = {}
SAME = {}
for t in range(3):
    for o in range(3):
        if o == t: continue
        PARTNER[t, o] = (IDX[o][:, CARD[t][0]].T.contiguous(), IDX[o][:, CARD[t][1]].T.contiguous())   # [H_t, 52] each
        SAME[t, o] = IDX[o][CARD[t][0], CARD[t][1]]                                                    # [H_t]


def padded(x, pl):
    """append a zero slot so pad indices read zero. x [..., H_pl] -> [..., H_pl + 1]"""
    return torch.nn.functional.pad(x, (0, 1))


# ---------------------------------------------------------------- strength tables per seat (river runouts)
class Tables:
    """Sorted strengths of one seat's hands, whole-list and per card, so band masses are cumsum lookups."""
    def __init__(self, o):
        so = STR[o]
        self.srt, self.order = torch.sort(so, dim=-1)                          # [ND,ND,H_o]
        groups = [np.where((HANDS[o] == c).any(1))[0] for c in range(52)]
        G = max(len(g) for g in groups)
        gi = torch.full((52, G), NH[o], device=dev, dtype=torch.int64)
        for c, g in enumerate(groups):
            gi[c, :len(g)] = torch.tensor(g, device=dev)
        gs, idx = torch.sort(STRP[o][:, :, gi], dim=-1)                        # [ND,ND,52,G]
        self.gs = gs
        self.grp = gi.expand(ND, ND, 52, G).gather(-1, idx)                   # hand index (pad = NH[o]) in sorted group order
        self.G = G
        self.tri = torch.cat([torch.zeros(G, 1, device=dev), torch.triu(torch.ones(G, G, device=dev))], 1)  # [G, G+1]

    def bands(self, reach, st, sl):
        """reach [b,ND,H_o] for turn slice sl, st = hero strengths [b,ND,H_t] -> dict of band tensors.
        A: [b,ND,H_t] (W, E) and [b,ND,1] (T); C: [b,ND,H_t,52] (W, E) and [b,ND,1,52] (T)."""
        srt, order = self.srt[sl], self.order[sl]
        pre = torch.nn.functional.pad(reach.gather(-1, order).cumsum(-1), (1, 0))       # [b,ND,H_o+1]
        lt = torch.searchsorted(srt, st, right=False)
        le = torch.searchsorted(srt, st, right=True)
        aw, ae = pre.gather(-1, lt), pre.gather(-1, le) - pre.gather(-1, lt)
        at = reach.sum(-1, keepdim=True)
        # per-card prefix masses at the hero strength
        rp = padded(reach, None)                                                        # [b,ND,H_o+1]
        gr = rp.unsqueeze(2).expand(-1, -1, 52, -1).gather(-1, self.grp[sl])            # [b,ND,52,G] reach in group order
        cum = gr @ self.tri                                                             # [b,ND,52,G+1]
        stc = st.unsqueeze(2).expand(-1, -1, 52, -1)                                    # [b,ND,52,H_t]
        clt = torch.searchsorted(self.gs[sl], stc, right=False)
        cle = torch.searchsorted(self.gs[sl], stc, right=True)
        cw = cum.gather(-1, clt).transpose(-1, -2)                                      # [b,ND,H_t,52]
        ce = (cum.gather(-1, cle) - cum.gather(-1, clt)).transpose(-1, -2)
        ct = (reach @ INC[self.o]).unsqueeze(-2)                                        # [b,ND,1,52]
        return {"W": (aw, cw), "E": (ae, ce), "T": (at, ct), "pre": pre, "lt": lt, "le": le, "order": order}


TAB = [None, None, None]


def band_extras(t, o, band, reach, st, tb):
    """Weights of o's combos that touch the hero's cards, restricted to `band`:
    w_ab [b,ND,H_t] (the hero's own combo in o's list), wca / wcb [b,ND,H_t,52] (combo (c, hero card))."""
    rp = padded(reach, None)                                                             # [b,ND,H_o+1]
    sp = STRP[o] if reach.dim() == 3 else None
    b_, n_ = reach.shape[0], reach.shape[1]
    same = SAME[t, o]
    w_ab = rp[..., same]                                                                 # [b,ND,H_t]
    outs = []
    for pa in PARTNER[t, o]:
        w = rp[..., pa]                                                                  # [b,ND,H_t,52]
        if band != "T":
            s = sp[tb["sl"]][..., pa]                                                    # [b,ND,H_t,52] strengths
            m = (s < st.unsqueeze(-1)) if band == "W" else (s == st.unsqueeze(-1))
            w = w * m
        outs.append(w)
    if band == "W":
        w_ab = torch.zeros_like(w_ab)
    return w_ab, outs[0], outs[1]


def ident(t, o1, o2, band, r1, r2, tb1):
    """sum over identical combos h of r1(h) r2(h) within `band`, per hero hand: [b,ND,H_t]. Mixed bands are zero."""
    prod = r1 * padded(r2, None)[..., IDXSAME[o1, o2]]                                   # [b,ND,H_o1]
    if band == "T":
        return prod.sum(-1, keepdim=True)
    pre = torch.nn.functional.pad(prod.gather(-1, tb1["order"]).cumsum(-1), (1, 0))
    if band == "W":
        return pre.gather(-1, tb1["lt"])
    return pre.gather(-1, tb1["le"]) - pre.gather(-1, tb1["lt"])


IDXSAME = {}
for o1 in range(3):
    for o2 in range(3):
        if o1 != o2:
            IDXSAME[o1, o2] = IDX[o2][CARD[o1][0], CARD[o1][1]]                          # [H_o1] -> index in o2 (pad NH[o2])


def pair_mass(t, P, Q, o1, o2, b1, b2, r1, r2, st, tb1, tb2, ident_v):
    """Mass of card-disjoint pairs (h1 in band b1 of o1, h2 in band b2 of o2) avoiding the hero's cards. [b,ND,H_t]"""
    (A1, C1), (A2, C2) = P, Q
    wab1, wca1, wcb1 = band_extras(t, o1, b1, r1, st, tb1)
    wab2, wca2, wcb2 = band_extras(t, o2, b2, r2, st, tb2)
    ca, cb = CARD[t]
    ga = lambda C: C.expand(*C.shape[:-2], NH[t], 52).gather(-1, ca.view(1, 1, -1, 1).expand(C.shape[0], C.shape[1], -1, 1)).squeeze(-1)
    gb = lambda C: C.expand(*C.shape[:-2], NH[t], 52).gather(-1, cb.view(1, 1, -1, 1).expand(C.shape[0], C.shape[1], -1, 1)).squeeze(-1)
    pa = A1 - ga(C1) - gb(C1) + wab1
    qa = A2 - ga(C2) - gb(C2) + wab2
    nm = NOTMINE[t]                                                                      # [H_t,52]: drop columns a, b
    x = (((C1 - wca1 - wcb1) * (C2 - wca2 - wcb2)) * nm).sum(-1)
    i = ((wca1 * wca2 + wcb1 * wcb2) * nm).sum(-1)
    return pa * qa - x + (ident_v - i - wab1 * wab2)


def flat_mass(t, r1, r2, o1, o2):
    """pairs(T, T) without strength tables: any batch shape. reach [..., H_o] -> [..., H_t]"""
    rp1, rp2 = padded(r1, None), padded(r2, None)
    A1, A2 = r1.sum(-1, keepdim=True), r2.sum(-1, keepdim=True)
    C1, C2 = r1 @ INC[o1], r2 @ INC[o2]                                                  # [..., 52]
    ca, cb = CARD[t]
    wab1, wab2 = rp1[..., SAME[t, o1]], rp2[..., SAME[t, o2]]
    pa = A1 - C1[..., ca] - C1[..., cb] + wab1
    qa = A2 - C2[..., ca] - C2[..., cb] + wab2
    wca1, wcb1 = rp1[..., PARTNER[t, o1][0]], rp1[..., PARTNER[t, o1][1]]                # [..., H_t, 52]
    wca2, wcb2 = rp2[..., PARTNER[t, o2][0]], rp2[..., PARTNER[t, o2][1]]
    nm = NOTMINE[t]
    x = (((C1.unsqueeze(-2) - wca1 - wcb1) * (C2.unsqueeze(-2) - wca2 - wcb2)) * nm).sum(-1)
    i = ((wca1 * wca2 + wcb1 * wcb2) * nm).sum(-1)
    idv = (r1 * rp2[..., IDXSAME[o1, o2]]).sum(-1, keepdim=True)
    return pa * qa - x + (idv - i - wab1 * wab2)


if args.compile and dev.type == "cuda":
    import torch._dynamo
    torch._dynamo.config.recompile_limit = 256
    torch._dynamo.config.cache_size_limit = 256
    pair_mass = torch.compile(pair_mass, dynamic=False)
    flat_mass = torch.compile(flat_mass, dynamic=False)
    Tables.bands = torch.compile(Tables.bands, dynamic=False)


# ---------------------------------------------------------------- tree (shape per street, not per card)
class Node:
    pass


N_ACTION, N_FLOATS = 0, 0


def batch_shape(street):
    return (ND,) * street


def build(board_n, commit, alive, acted, street_start, raises, last):
    """continue the street after seat `last` acted"""
    for k in (1, 2, 3):
        pl = (last + k) % 3
        if alive[pl] and commit[pl] < STACK - 1e-3 and not acted[pl]:
            return action(board_n, commit, alive, acted, street_start, raises, pl)
    return next_street(board_n, commit, alive)


def action(board_n, commit, alive, acted, street_start, raises, pl):
    global N_ACTION, N_FLOATS
    street = board_n - 3
    mx = max(commit)
    to_call = mx - commit[pl]
    remaining = STACK - commit[pl]
    pot = P0 + sum(commit)
    sized = lambda want: remaining if want >= remaining * S["allin_threshold"] else want
    n = Node(); n.kind = "action"; n.player = pl; n.street = street; n.commit = tuple(commit)
    n.labels, n.children = [], []
    if to_call <= 1e-6:
        n.labels.append("check")
        ac = list(acted); ac[pl] = True
        n.children.append(build(board_n, commit, alive, ac, street_start, raises, pl))
        amt = sized(S["bet"] * pot)
        if amt > 0:
            c = list(commit); c[pl] += amt
            ac = [False, False, False]; ac[pl] = True
            n.labels.append(f"bet {amt:.2f}")
            n.children.append(build(board_n, c, alive, ac, street_start, 0, pl))
    else:
        al = list(alive); al[pl] = False
        ac = list(acted); ac[pl] = True
        live = [q for q in range(3) if al[q]]
        if len(live) == 1:
            f = Node(); f.kind = "foldwin"; f.winner = live[0]; f.commit = tuple(commit)
        else:
            f = build(board_n, commit, al, ac, street_start, raises, pl)
        n.labels.append("fold"); n.children.append(f)
        c = list(commit); c[pl] += min(to_call, remaining)
        n.labels.append("call"); n.children.append(build(board_n, c, alive, ac, street_start, raises, pl))
        if raises < CAPS[street] and remaining > to_call + 1e-3:
            amt = min(sized(to_call + S["raise"] * (pot + to_call)), remaining)
            c = list(commit); c[pl] += amt
            ac = [False, False, False]; ac[pl] = True
            n.labels.append(f"raise {amt:.2f}")
            n.children.append(build(board_n, c, alive, ac, street_start, raises + 1, pl))
    na = len(n.labels)
    shape = batch_shape(street) + (na, NH[pl])
    n.regret = torch.zeros(shape, device=dev, dtype=STORE)
    n.ssum = torch.zeros(shape, device=dev, dtype=STORE)
    N_ACTION += 1; N_FLOATS += 2 * n.regret.numel()
    return n


def next_street(board_n, commit, alive):
    n = Node()
    live = [q for q in range(3) if alive[q]]
    if board_n == 5:
        n.kind = "sd3" if len(live) == 3 else "sd2"
        n.alive = live; n.dead = [q for q in range(3) if not alive[q]]
        n.commit = tuple(commit)
        return n
    n.kind = "chance"; n.street = board_n - 3
    actors = sum(1 for q in range(3) if alive[q] and commit[q] < STACK - 1e-3)
    n.child = next_street(board_n + 1, commit, alive) if actors <= 1 else build(board_n + 1, list(commit), list(alive), [False] * 3, max(commit), 0, 2)
    return n


if args.chunk <= 0:
    # the showdown transients are ~12 tensors of [chunk, ND, 52, H] floats; launches, not flops, bound the GPU, so use few big chunks
    args.chunk = max(1, min(ND, int(1.5e9 / (ND * 52 * max(NH) * 4 * 12))))
print(f"showdown chunk {args.chunk} turn cards", flush=True)
t0 = time.time()
ROOT = build(3, [0.0, 0.0, 0.0], [True] * 3, [False] * 3, 0.0, 0, 2)
for o in range(3):
    TAB[o] = Tables(o); TAB[o].o = o
DIAG = (1.0 - torch.eye(ND, device=dev)).unsqueeze(-1)
if dev.type == "cuda": torch.cuda.synchronize()
print(f"[{S['formation']}] hands {NH}  shape nodes {N_ACTION}  storage {N_FLOATS*torch.tensor([], dtype=STORE).element_size()/1e9:.2f} GB  built {time.time()-t0:.1f}s", flush=True)


# ---------------------------------------------------------------- terminals
def pays(commit, t):
    pot = P0 + sum(commit)
    return pot - commit[t], pot / 2 - commit[t], pot / 3 - commit[t], -commit[t]


def showdown(node, t, reach):
    """reach: list of 3 [ND,ND,H] tensors (t's slot unused) -> [ND,ND,H_t]"""
    win, tie2, tie3, lose = pays(node.commit, t)
    o1, o2 = others(t)
    if node.kind == "sd2" and t in node.dead:
        return flat_mass(t, reach[o1], reach[o2], o1, o2) * lose
    if node.kind == "sd2":
        opp = node.alive[0] if node.alive[1] == t else node.alive[1]
        dead = node.dead[0]
        o1, o2 = opp, dead
    out = torch.empty(ND, ND, NH[t], device=dev)
    for s0 in range(0, ND, args.chunk):
        sl = slice(s0, min(ND, s0 + args.chunk))
        st = STR[t][sl]                                                                  # [b,ND,H_t]
        r1, r2 = reach[o1][sl], reach[o2][sl]
        tb1 = TAB[o1].bands(r1, st, sl); tb1["sl"] = sl
        tb2 = TAB[o2].bands(r2, st, sl); tb2["sl"] = sl
        pm = lambda b1, b2, band_id: pair_mass(t, tb1[b1], tb2[b2], o1, o2, b1, b2, r1, r2, st, tb1, tb2, ident(t, o1, o2, band_id, r1, r2, tb1) if band_id else 0.0)
        valid = pm("T", "T", "T")
        if node.kind == "sd3":
            w = pm("W", "W", "W")
            ee = pm("E", "E", "E")
            ew = pm("W", "E", None) + pm("E", "W", None)
            v = w * win + ew * tie2 + ee * tie3 + (valid - w - ee - ew) * lose
        else:
            w = pm("W", "T", "W")
            e = pm("E", "T", "E")
            v = w * win + e * tie2 + (valid - w - e) * lose
        out[sl] = v
    return out


def terminal(node, t, reach):
    if node.kind == "foldwin":
        win, _, _, lose = pays(node.commit, t)
        o1, o2 = others(t)
        return flat_mass(t, reach[o1], reach[o2], o1, o2) * (win if node.winner == t else lose)
    return showdown(node, t, reach)


def chance(node, t, reach, fn):
    o1, o2 = others(t)
    if node.street == 0:
        r = list(reach)
        for o in (o1, o2): r[o] = reach[o].unsqueeze(0) * VALID[o]                        # [ND,H]
        v = fn(node.child, t, r)
        return (v * VALID[t]).sum(0) / (ND - 6)
    r = list(reach)
    for o in (o1, o2): r[o] = reach[o].unsqueeze(1) * VALID[o].unsqueeze(0) * DIAG        # [ND,ND,H]
    v = fn(node.child, t, r)
    return (v * VALID[t].unsqueeze(0) * DIAG).sum(1) / (ND - 1 - 6)


def normalize(x):
    s = x.sum(-2, keepdim=True)
    return torch.where(s > 0, x / s.clamp_min(1e-30), torch.full_like(x, 1.0 / x.shape[-2]))


def regret_match(regret):
    x = regret.float().clamp_min(0)
    s = x.sum(-2, keepdim=True)
    return torch.where(s > 0, x / s.clamp_min(1e-30), torch.full_like(x, 1.0 / x.shape[-2]))


def cfr(node, t, reach):
    if node.kind == "chance":
        return chance(node, t, reach, cfr)
    if node.kind != "action":
        return terminal(node, t, reach)
    strat = regret_match(node.regret)
    pl = node.player
    if pl == t:
        vals = []
        for a, ch in enumerate(node.children):
            if node.labels[a] == "fold":
                o1, o2 = others(t)
                vals.append(flat_mass(t, reach[o1], reach[o2], o1, o2) * (-node.commit[t]))
            else:
                vals.append(cfr(ch, t, reach))
        vals = torch.stack(vals, dim=-2)
        util = (strat * vals).sum(-2, keepdim=True)
        reg = node.regret.float()
        node.regret.copy_(reg * torch.where(reg > 0, DISC[0], DISC[1]) + (vals - util))
        return util.squeeze(-2)
    node.ssum.copy_(node.ssum.float() * DISC[2] + strat * reach[pl].unsqueeze(-2))
    out = 0
    for a, ch in enumerate(node.children):
        r = list(reach); r[pl] = reach[pl] * strat[..., a, :]
        out = out + cfr(ch, t, r)
    return out


BR = False


def walk(node, t, reach):
    if node.kind == "chance":
        return chance(node, t, reach, walk)
    if node.kind != "action":
        return terminal(node, t, reach)
    strat = normalize(node.ssum.float())
    pl = node.player
    if pl == t:
        vals = []
        for a, ch in enumerate(node.children):
            if node.labels[a] == "fold":
                o1, o2 = others(t)
                vals.append(flat_mass(t, reach[o1], reach[o2], o1, o2) * (-node.commit[t]))
            else:
                vals.append(walk(ch, t, reach))
        vals = torch.stack(vals, dim=-2)
        return vals.max(-2).values if BR else (strat * vals).sum(-2)
    out = 0
    for a, ch in enumerate(node.children):
        r = list(reach); r[pl] = reach[pl] * strat[..., a, :]
        out = out + walk(ch, t, r)
    return out


def exploitability():
    global BR
    gain, ev = 0.0, [0.0] * 3
    for t in range(3):
        o1, o2 = others(t)
        z = (flat_mass(t, W[o1], W[o2], o1, o2) * W[t]).sum()
        v = []
        for br in (True, False):
            BR = br
            v.append(((walk(ROOT, t, list(W)) * W[t]).sum() / z).item())
        gain += v[0] - v[1]; ev[t] = v[1]
    return gain / 3, ev


t0 = time.time()
with torch.no_grad():
    for it in range(1, args.iters + 1):
        a = it ** 1.5
        DISC = tuple(torch.tensor(x, device=dev, dtype=torch.float32) for x in (a / (a + 1), 0.5, (it / (it + 1)) ** 2))
        for t in range(3):
            cfr(ROOT, t, list(W))
        if it % args.every == 0 or it == args.iters:
            e, ev = exploitability()
            mem = torch.cuda.max_memory_allocated() / 1e9 if dev.type == "cuda" else 0
            print(f"  iter {it:4d}  expl {e:.4f} bb = {100*e/P0:.3f}% pot   EV {ev[0]:.3f} {ev[1]:.3f} {ev[2]:.3f}   {time.time()-t0:.0f}s  peak {mem:.2f} GB", flush=True)
            if 100 * e / P0 < args.target:
                break

if args.dump:
    # flop + turn average strategies, preorder, float32 [ND^street, A, H]; rivers are re-solved on demand
    def preorder(n, out):
        if n.kind == "action":
            if n.street < 2: out.append(n)
            for ch in n.children: preorder(ch, out)
        elif n.kind == "chance":
            preorder(n.child, out)
        return out
    with open(args.dump + ".f32", "wb") as f:
        for n in preorder(ROOT, []):
            f.write(normalize(n.ssum.float()).cpu().numpy().tobytes())
    # header the Rust side (resolve3 / export3) reads next to the .f32
    R, SU = "23456789TJQKA", "cdhs"
    with open(args.dump + ".json", "w") as f:
        json.dump({"board": "".join(R[c // 4] + SU[c % 4] for c in S["flop"]), "pot": S["start_pot"], "stack": S["eff_stack"],
                   "raises": S["max_raises"], "hands": S["hands"], "weights": S["weights"]}, f)
    print(f"dumped flop+turn strategy -> {args.dump}.f32", flush=True)
    # flop action nodes: counterfactual value of every action for the acting seat under the average strategy
    # (others' reach at the node, hero reach unweighted), preorder, float32 [A, H]; rivers are in memory here only
    def flop_evs(node, reach, out):
        if node.kind != "action": return
        strat = normalize(node.ssum.float()); t = node.player
        vals = []
        for a, ch in enumerate(node.children):
            if node.labels[a] == "fold":
                o1, o2 = others(t)
                vals.append(flat_mass(t, reach[o1], reach[o2], o1, o2) * (-node.commit[t]))
            else:
                vals.append(walk(ch, t, reach))
        out.append(torch.stack(vals, dim=-2))
        for a, ch in enumerate(node.children):
            r = list(reach); r[t] = reach[t] * strat[..., a, :]
            flop_evs(ch, r, out)
    with torch.no_grad():
        BR = False
        out = []; flop_evs(ROOT, list(W), out)
    with open(args.dump + ".flopev.f32", "wb") as f:
        for v in out: f.write(v.float().cpu().numpy().tobytes())
    print(f"dumped {len(out)} flop-node action values -> {args.dump}.flopev.f32  {time.time()-t0:.0f}s", flush=True)
