"""Pick a texture-weighted flop set and deal it across the matchups. Deterministic (seeded).
usage: python3 pick_flops.py [n_per_formation=8] -> batch1.txt lines "<formation> <flop>"."""
import itertools, random, sys, collections
R = "23456789TJQKA"
FORMS = ["btn_bb", "co_bb", "utg_bb", "sb_bb", "co_btn", "utg_btn", "btn_bb_3b", "btn_sb_3b", "co_btn_3b"]
per = int(sys.argv[1]) if len(sys.argv) > 1 else 8
rng = random.Random(7)

def category(ranks, suits):
    hi = max(ranks)
    high = "A" if hi == 12 else "KQ" if hi >= 10 else "JT" if hi >= 8 else "low"
    ns = len(set(suits))
    suit = {1: "mono", 2: "two-tone", 3: "rainbow"}[ns]
    u = sorted(set(ranks))
    if len(u) < 3: struct = "paired"
    else:
        span = u[2] - u[0]
        wheel = 12 in u and sorted(x if x != 12 else -1 for x in u)[2] - -1 <= 4
        struct = "connected" if span <= 4 or wheel else "semi" if min(u[1] - u[0], u[2] - u[1]) <= 2 else "dry"
    return (high, suit, struct)

# every concrete flop once, grouped by category
cats = collections.defaultdict(list)
deck = [(r, s) for r in range(13) for s in range(4)]
for f in itertools.combinations(deck, 3):
    cats[category([c[0] for c in f], [c[1] for c in f])].append(f)
total = sum(len(v) for v in cats.values())
n = per * len(FORMS)
quota = {k: len(v) / total * n for k, v in cats.items()}
take = {k: max(1 if q >= 0.35 else 0, int(q)) for k, q in quota.items()}
for k in sorted(quota, key=lambda k: quota[k] - int(quota[k]), reverse=True):
    if sum(take.values()) >= n: break
    take[k] += 1
while sum(take.values()) > n:
    k = max(take, key=lambda k: take[k] - quota[k]); take[k] -= 1
picked = []
for k in sorted(take):
    if not take[k]: continue
    seen = set()
    pool = cats[k][:]; rng.shuffle(pool)
    for f in pool:
        key = tuple(sorted(c[0] for c in f))
        if key in seen: continue
        seen.add(key); picked.append((k, f))
        if len(seen) == take[k]: break
# deal: walk categories in order, rotate formations so each gets a spread of textures
out = []
for i, (k, f) in enumerate(picked):
    cards = sorted(f, reverse=True)
    out.append((FORMS[i % len(FORMS)], "".join(R[r] + "shdc"[s] for r, s in cards), k))
with open("batch1.txt", "w") as fh:
    for form, flop, k in out: fh.write(f"{form} {flop}\n")
for form in FORMS:
    print(form, " ".join(f"{fl}" for fo, fl, k in out if fo == form))
print(len(out), "flops;", len([k for k in take if take[k]]), "texture categories")
