"""Go/no-go spike: can an RTX 2080 evaluate one river-level CFR traversal faster than the Rust CPU solver?

Workload mirrors the real tree: 49x49 runouts, ~25 betting lines reaching the river, ~550 hands per player,
~4 showdown terminals + ~8 action nodes per line. CPU reference: ~0.5 s per full traversal on the Mac (10 cores).
"""
import time, torch

dev = "cuda"
R, LINES, H, K = 49 * 49, 25, 550, 45            # runouts, river lines, hands, card-conflicting opp hands per hand
g = torch.Generator(device=dev).manual_seed(0)
order = torch.argsort(torch.rand(R, H, device=dev, generator=g), dim=1)          # opp hands sorted by strength, per runout
pos = torch.randint(0, H, (R, H), device=dev, generator=g)                       # hero hand -> rank position
conf = torch.randint(0, H, (H, K), device=dev, generator=g)                      # conflicting opp hands
weaker = torch.rand(R, H, K, device=dev, generator=g) < 0.5                       # is that conflicting hand weaker

def showdown(reach):                       # reach [R, H] for one line
    s = torch.gather(reach, 1, order).cumsum(1)
    less = torch.gather(s, 1, pos)
    corr = (reach[:, conf] * weaker).sum(2)  # [R, H, K] gather: the card-removal correction
    return less - corr

def action_node(regret, reach):            # regret [R, 3, H]
    p = regret.clamp_min(0); p = p / p.sum(1, keepdim=True).clamp_min(1e-9)
    return p * reach[:, None, :]

for dtype in (torch.float32, torch.float16):
    reach = torch.rand(R, H, device=dev, dtype=dtype)
    regret = torch.randn(R, 3, H, device=dev, dtype=dtype)
    weaker_t = weaker.to(dtype)
    def sd(reach): 
        s = torch.gather(reach, 1, order).float().cumsum(1).to(dtype)
        return torch.gather(s, 1, pos) - (reach[:, conf] * weaker_t).sum(2)
    for _ in range(3): sd(reach); action_node(regret, reach)
    torch.cuda.synchronize(); t = time.time()
    for _ in range(LINES):
        for _ in range(4): sd(reach)
        for _ in range(8): action_node(regret, reach)
    torch.cuda.synchronize(); dt = time.time() - t
    print(f"{dtype}: one river-level traversal {dt*1000:.0f} ms  (showdowns {LINES*4}, action nodes {LINES*8})  peak mem {torch.cuda.max_memory_allocated()/1e9:.2f} GB")
    for name, fn, n in (("showdown", lambda: sd(reach), 100), ("action node", lambda: action_node(regret, reach), 200), ("showdown w/o card-removal gather", lambda: torch.gather(torch.gather(reach, 1, order).float().cumsum(1).to(dtype), 1, pos), 100)):
        torch.cuda.synchronize(); t = time.time()
        for _ in range(n): fn()
        torch.cuda.synchronize(); print(f"   {name}: {(time.time()-t)*1000:.0f} ms for {n}")
    # card removal via per-card strength-ordered cumsums: 52 groups of ~G hands instead of an [R,H,K] gather
    G = 24
    grp = torch.randint(0, H, (R, 52 * G), device=dev)                 # opp hands holding card c, strength-sorted, per runout
    p1 = torch.randint(0, 52 * (G + 1), (R, H), device=dev); p2 = torch.randint(0, 52 * (G + 1), (R, H), device=dev)
    def sd2(reach):
        s = torch.gather(torch.gather(reach, 1, order).float().cumsum(1).to(dtype), 1, pos)
        c = torch.nn.functional.pad(torch.gather(reach, 1, grp).view(R, 52, G).float().cumsum(2).to(dtype), (1, 0)).view(R, -1)
        return s - torch.gather(c, 1, p1) - torch.gather(c, 1, p2)
    for _ in range(3): sd2(reach)
    torch.cuda.synchronize(); t = time.time()
    for _ in range(100): sd2(reach)
    torch.cuda.synchronize(); a = (time.time() - t) * 1000
    t = time.time()
    for _ in range(200): action_node(regret, reach)
    torch.cuda.synchronize(); b = (time.time() - t) * 1000
    print(f"   grouped card-removal showdown: {a:.0f} ms for 100  ->  traversal ~{a + b:.0f} ms")
print("CPU reference (Rust, M-series 10 cores): ~500 ms per traversal")
