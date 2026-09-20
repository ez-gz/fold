"""Compare our solver with the reference solver on the identical game. usage: compare.py <formation>_<flop>"""
import json, re, sys
n = sys.argv[1]
raw = open(f'refcheck/ref_{n}.json').read(); r = json.loads(raw[raw.rindex('{"actions"'):]); o = json.load(open(f'refcheck/ours_{n}.json'))
norm = lambda k: ''.join(sorted([k[:2], k[2:]]))
R = {norm(k): v for k, v in r['root_hands'].items()}; O = {norm(k): v for k, v in o['root_hands'].items()}
d = [max(abs(a - b) for a, b in zip(O[k], R[k])) for k in O if k in R]
print(f"{n}: expl% ours {o['exploitability_pct_pot']:.3f} ref {r['exploitability_pct_pot']:.3f} | EV OOP ours {o['ev_oop']:.4f} ref {r['ev_oop']:.4f} | EV IP ours {o['ev_ip']:.4f} ref {r['ev_ip']:.4f}")
print(f"   root freq ours {[round(x, 3) for x in o['root_freq']]} ref {[round(x, 3) for x in r['root_freq']]} {r['actions']} | per-hand strategy diff mean {sum(d)/len(d):.4f} max {max(d):.3f} over {len(d)} hands")
