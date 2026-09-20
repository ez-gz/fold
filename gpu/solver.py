"""GPU DCFR for one flop, batched over runouts. Mirrors solver/src/{tree,cfr}.rs (the oracle).

Tensors carry leading batch dims by street: flop [], turn [49], river [49, 49] (both index the 49 non-flop
cards; the diagonal and card-conflicting hands carry zero reach). The betting tree is built once per street
"shape" rather than once per card, so every node update is one batched tensor op.
"""
import argparse, json, time
import numpy as np
import torch

p = argparse.ArgumentParser()
p.add_argument("spec")
p.add_argument("--iters", type=int, default=300)
p.add_argument("--target", type=float, default=0.3)
p.add_argument("--every", type=int, default=25)
p.add_argument("--device", default="cuda")
p.add_argument("--compile", type=int, default=1)
p.add_argument("--dump", default="")
p.add_argument("--rake", type=float, nargs=2, default=[0.0, 0.0])
args = p.parse_args()
dev = torch.device(args.device)
torch.backends.cuda.matmul.allow_tf32 = False

S = json.load(open(args.spec + ".json"))
HANDS = [np.array(h, dtype=np.int64) for h in S["hands"]]
NH = [len(h) for h in HANDS]
DECK = S["deck"]
ND = len(DECK)
P0, STACK = S["start_pot"], S["eff_stack"]
W = [torch.tensor(w, dtype=torch.float32, device=dev) for w in S["weights"]]
raw = np.fromfile(args.spec + ".bin", dtype=np.uint32).reshape(ND, ND, NH[0] + NH[1]).astype(np.int64)
STR = [torch.tensor(raw[:, :, :NH[0]], device=dev), torch.tensor(raw[:, :, NH[0]:], device=dev)]


def rake(pot):
    return min(pot * args.rake[0], args.rake[1])


# ---------------------------------------------------------------- card bookkeeping
INC = []      # [H, 52] incidence
for pl in range(2):
    m = torch.zeros(NH[pl], 52, device=dev)
    m[torch.arange(NH[pl]), torch.tensor(HANDS[pl][:, 0])] = 1
    m[torch.arange(NH[pl]), torch.tensor(HANDS[pl][:, 1])] = 1
    INC.append(m)
DECK_T = torch.tensor(DECK, device=dev)
VALID = [1.0 - INC[pl][:, DECK_T].T.contiguous() for pl in range(2)]          # [49, H]: hand does not hold deck card d
SAME = []     # index of the identical combo in the other player's list (NH[o] = zero pad)
for pl in range(2):
    o = 1 - pl
    lut = {tuple(h): i for i, h in enumerate(HANDS[o].tolist())}
    SAME.append(torch.tensor([lut.get(tuple(h), NH[o]) for h in HANDS[pl].tolist()], device=dev))


SAME_MASK = [(SAME[pl] < NH[1 - pl]).float() for pl in range(2)]
SAME_IDX = [SAME[pl].clamp_max(NH[1 - pl] - 1) for pl in range(2)]
INC_PAIR = [INC[1 - t] @ INC[t].T for t in range(2)]                           # [H_o, H_t]: cards shared (0, 1 or 2)


def shared(t, reach):
    """Per hero hand: opponent reach holding one of its cards, the identical combo counted once."""
    raw = (reach @ INC[1 - t]) @ INC[t].T                                                  # identical combo counted twice
    return raw, raw - reach[..., SAME_IDX[t]] * SAME_MASK[t]


def valid_mass(t, reach):
    """Opponent reach not sharing a card with each of t's hands. reach [..., H_o] -> [..., H_t]."""
    return reach.sum(-1, keepdim=True) - shared(t, reach)[1]


# ---------------------------------------------------------------- showdown tables (per traverser)
class Showdown:
    def __init__(self, t):
        o = 1 - t
        st, so = STR[t], STR[o]                                                # [49,49,H]
        srt, self.order = torch.sort(so, dim=-1)
        self.pos_less = torch.searchsorted(srt, st, right=False)               # opp hands strictly weaker
        self.pos_le = torch.searchsorted(srt, st, right=True)
        # per-card groups of opponent hands, strength-sorted per runout, for card removal
        groups = [np.where((HANDS[o] == c).any(1))[0] for c in range(52)]
        G = max(len(g) for g in groups)
        gi = torch.full((52, G), NH[o], device=dev, dtype=torch.int64)
        for c, g in enumerate(groups):
            gi[c, :len(g)] = torch.tensor(g, device=dev)
        big = 1 << 40
        sop = torch.nn.functional.pad(so, (0, 1), value=big)                   # pad hand -> sorts last, zero reach
        gs, idx = torch.sort(sop[:, :, gi], dim=-1)                            # [49,49,52,G]
        self.grp = gi.expand(ND, ND, 52, G).gather(-1, idx).reshape(ND, ND, 52 * G)
        self.G = G
        # x @ tri gives [0, cumsum...] directly (no pad); pad slots sort last so lookups never reach them
        self.tri = torch.cat([torch.zeros(G, 1, device=dev), torch.triu(torch.ones(G, G, device=dev))], 1)
        c1 = torch.tensor(HANDS[t][:, 0], device=dev)
        c2 = torch.tensor(HANDS[t][:, 1], device=dev)
        row = (torch.arange(ND * ND, device=dev) * 52 * (G + 1)).view(ND, ND, 1)
        parts = []
        for right in (False, True):
            for c in (c1, c2):
                out = torch.empty(ND, ND, NH[t], device=dev, dtype=torch.int64)
                for a in range(ND):                                            # chunked: [49,H,G] at a time
                    out[a] = torch.searchsorted(gs[a][:, c, :], st[a].unsqueeze(-1), right=right).squeeze(-1)
                parts.append(out + (c * (G + 1)).view(1, 1, -1) + row)
        self.cidx = torch.stack(parts, 2).reshape(-1).to(torch.int32)          # [49,49,(less,le)x(c1,c2),H] flat
        prow = (torch.arange(ND * ND, device=dev) * (NH[o] + 1)).view(ND, ND, 1)
        self.pidx = torch.stack([self.pos_less + prow, self.pos_le + prow], 2).reshape(-1).to(torch.int32)
        orow = (torch.arange(ND * ND, device=dev) * NH[o]).view(ND, ND, 1)
        self.oidx = (self.order + orow).reshape(-1).to(torch.int32)
        self.gidx = (self.grp.clamp_max(NH[o] - 1) + orow).reshape(-1).to(torch.int32)
        del self.order, self.grp, self.pos_less, self.pos_le
        self.nt = NH[t]

    def masses(self, reach, cs_t):
        """reach [49,49,H_o], cs_t = per-hero-hand sum of opp reach sharing a card -> (win, lose) masses."""
        G, flat = self.G, reach.reshape(-1)
        pre = torch.nn.functional.pad(flat.index_select(0, self.oidx).view(ND, ND, -1).cumsum(-1), (1, 0))
        sp = pre.view(-1).index_select(0, self.pidx).view(ND, ND, 2, self.nt)
        C = flat.index_select(0, self.gidx).view(ND, ND, 52, G) @ self.tri
        sc = C.view(-1).index_select(0, self.cidx).view(ND, ND, 2, 2, self.nt).sum(3)
        win = sp[:, :, 0] - sc[:, :, 0]
        lose = (pre[..., -1:] - sp[:, :, 1]) - (cs_t - sc[:, :, 1])
        return win, lose


# ---------------------------------------------------------------- tree (shape per street, not per card)
class Node:
    pass


def batch_shape(street):
    return (ND,) * street


N_ACTION, N_FLOATS = 0, 0


def build_action(board_n, commit, street_start, raises, pl):
    global N_ACTION, N_FLOATS
    street = board_n - 3
    o = 1 - pl
    to_call = commit[o] - commit[pl]
    remaining = STACK - commit[pl]
    pot = P0 + commit[0] + commit[1]
    sized = lambda want: remaining if want >= remaining * S["allin_threshold"] else want
    n = Node(); n.kind = "action"; n.player = pl; n.street = street; n.commit = tuple(commit)
    n.labels, n.children = [], []
    if to_call <= 0:
        n.labels.append("check")
        n.children.append(build_action(board_n, commit, street_start, 0, 1) if pl == 0 else next_street(board_n, commit))
        seen = []
        for f in S["bets"][pl][street]:
            amt = sized(f * pot)
            if amt <= 0 or any(abs(s - amt) < 1e-3 for s in seen): continue
            seen.append(amt)
            c = list(commit); c[pl] += amt
            n.labels.append(f"bet {amt:.2f}")
            n.children.append(build_action(board_n, c, street_start, 0, o))
    else:
        f = Node(); f.kind = "fold"; f.folder = pl; f.commit = tuple(commit)
        n.labels.append("fold"); n.children.append(f)
        c = list(commit); c[pl] += to_call
        n.labels.append("call"); n.children.append(next_street(board_n, c))
        if raises < S["max_raises"] and remaining > to_call + 1e-3:
            seen = []
            for fr in S["raises"][pl][street]:
                amt = min(sized(to_call + fr * (pot + to_call)), remaining)
                if any(abs(s - amt) < 1e-3 for s in seen): continue
                seen.append(amt)
                c = list(commit); c[pl] += amt
                n.labels.append(f"raise {amt:.2f}")
                n.children.append(build_action(board_n, c, street_start, raises + 1, o))
    na = len(n.labels)
    shape = batch_shape(street) + (na, NH[pl])
    n.regret = torch.zeros(shape, device=dev)
    n.ssum = torch.zeros(shape, device=dev)
    N_ACTION += 1; N_FLOATS += 2 * n.regret.numel()
    return n


def next_street(board_n, commit):
    n = Node()
    if board_n == 5:
        n.kind = "showdown"; n.commit = commit[0]
        return n
    n.kind = "chance"; n.street = board_n - 3
    allin = commit[0] >= STACK - 1e-3
    n.child = next_street(board_n + 1, commit) if allin else build_action(board_n + 1, list(commit), commit[0], 0, 0)
    return n


t0 = time.time()
ROOT = build_action(3, [0.0, 0.0], 0.0, 0, 0)
SD = [Showdown(0), Showdown(1)]
DIAG = (1.0 - torch.eye(ND, device=dev)).unsqueeze(-1)
torch.cuda.synchronize() if dev.type == "cuda" else None
print(f"[{S['formation']}] hands {NH}  shape nodes {N_ACTION}  storage {N_FLOATS*4/1e9:.2f} GB  built {time.time()-t0:.1f}s", flush=True)


# ---------------------------------------------------------------- traversals
def terminal(node, t, reach):
    if node.kind == "fold":
        c = node.commit
        pay = -c[t] if node.folder == t else P0 + c[node.folder] - rake(P0 + 2 * c[node.folder])
        return valid_mass(t, reach) * pay
    c = node.commit
    pot = P0 + 2 * c
    rk = rake(pot)
    raw, sh = shared(t, reach)
    win, lose = SD[t].masses(reach, raw)
    if not hasattr(node, "pay"):
        node.pay = tuple(torch.tensor(x, device=dev, dtype=torch.float32) for x in (pot - rk - c, -c, (pot - rk) / 2 - c))
    return _payoff(win, lose, reach.sum(-1, keepdim=True), sh, *node.pay)


def chance(node, t, reach, fn):
    o = 1 - t
    if node.street == 0:
        v = fn(node.child, t, reach.unsqueeze(0) * VALID[o])                   # [49,H]
        return (v * VALID[t]).sum(0) / (ND - 4)
    v = fn(node.child, t, reach.unsqueeze(1) * VALID[o].unsqueeze(0) * DIAG)   # [49,49,H]
    return (v * VALID[t].unsqueeze(0) * DIAG).sum(1) / (ND - 1 - 4)


def normalize(x):
    s = x.sum(-2, keepdim=True)
    return torch.where(s > 0, x / s.clamp_min(1e-30), torch.full_like(x, 1.0 / x.shape[-2]))


def _regret_match(regret):
    x = regret.clamp_min(0)
    s = x.sum(-2, keepdim=True)
    return torch.where(s > 0, x / s.clamp_min(1e-30), 1.0 / x.shape[-2])


def _own_update(regret, strat, vals, dpos, dneg):
    util = (strat * vals).sum(-2, keepdim=True)
    new = regret * torch.where(regret > 0, dpos, dneg) + (vals - util)
    return new, util.squeeze(-2)


def _opp_update(ssum, strat, reach, d):
    return ssum * d + strat * reach.unsqueeze(-2)


def _payoff(win, lose, total, sh, a, b, c):
    return win * a + lose * b + (total - sh - win - lose) * c


if args.compile:
    import torch._dynamo
    torch._dynamo.config.recompile_limit = 128
    _regret_match, _own_update, _opp_update, _payoff = (torch.compile(f, dynamic=False) for f in (_regret_match, _own_update, _opp_update, _payoff))


def cfr(node, t, reach):
    if node.kind == "chance":
        return chance(node, t, reach, cfr)
    if node.kind != "action":
        return terminal(node, t, reach)
    strat = _regret_match(node.regret)
    if node.player == t:
        vals = torch.stack([cfr(ch, t, reach) for ch in node.children], dim=-2)
        new, util = _own_update(node.regret, strat, vals, DISC[0], DISC[1])
        node.regret.copy_(new)
        return util
    node.ssum.copy_(_opp_update(node.ssum, strat, reach, DISC[2]))
    out = 0
    for a, ch in enumerate(node.children):
        out = out + cfr(ch, t, reach * strat[..., a, :])
    return out


BR = False


def walk(node, t, reach):
    if node.kind == "chance":
        return chance(node, t, reach, walk)
    if node.kind != "action":
        return terminal(node, t, reach)
    strat = normalize(node.ssum)
    if node.player == t:
        vals = torch.stack([walk(ch, t, reach) for ch in node.children], dim=-2)
        return vals.max(-2).values if BR else (strat * vals).sum(-2)
    out = 0
    for a, ch in enumerate(node.children):
        out = out + walk(ch, t, reach * strat[..., a, :])
    return out


def exploitability():
    global BR
    v = [[0, 0], [0, 0]]
    for t in range(2):
        z = (valid_mass(t, W[1 - t]) * W[t]).sum()
        for k, br in enumerate((True, False)):
            BR = br
            v[t][k] = ((walk(ROOT, t, W[1 - t]) * W[t]).sum() / z).item()
    return ((v[0][0] - v[0][1]) + (v[1][0] - v[1][1])) / 2, v[0][1], v[1][1]


t0 = time.time()
with torch.no_grad():
    for it in range(1, args.iters + 1):
        a = it ** 1.5
        DISC = tuple(torch.tensor(x, device=dev, dtype=torch.float32) for x in (a / (a + 1), 0.5, (it / (it + 1)) ** 2))
        for t in range(2):
            cfr(ROOT, t, W[1 - t])
        if it % args.every == 0 or it == args.iters:
            e, ev0, ev1 = exploitability()
            mem = torch.cuda.max_memory_allocated() / 1e9 if dev.type == "cuda" else 0
            print(f"  iter {it:4d}  expl {e:.4f} bb = {100*e/P0:.3f}% pot   EV OOP {ev0:.3f} IP {ev1:.3f}   {time.time()-t0:.0f}s  peak {mem:.2f} GB", flush=True)
            if 100 * e / P0 < args.target:
                break

if args.dump:
    # average strategy per shape node, preorder, float16 [49^street, A, H]; solver/ imports this for export
    def preorder(n, out):
        if n.kind == "action":
            out.append(n)
            for ch in n.children: preorder(ch, out)
        elif n.kind == "chance":
            preorder(n.child, out)
        return out
    with open(args.dump, "wb") as f:
        for n in preorder(ROOT, []):
            f.write(normalize(n.ssum).to(torch.float16).cpu().numpy().tobytes())
    print(f"dumped strategy -> {args.dump}", flush=True)
