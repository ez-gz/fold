#!/usr/bin/env python3
"""Merge export3 spot files into one gzip'd JSON for the trainer's 3-way mode.
usage: pack3.py OUT.json.gz spots_*.json
Keeps every flop and turn spot; river spots only when the line is common (line_p >= 0.03), at most 30 per file."""
import gzip, json, sys
out, files = sys.argv[1], sys.argv[2:]
spots, seen = [], set()
for fn in files:
    j = json.load(open(fn))
    riv = [s for s in j["spots"] if s["street"] == 2 and s["line_p"] >= 0.03]
    riv.sort(key=lambda s: -s["line_p"])
    for s in [x for x in j["spots"] if x["street"] < 2] + riv[:30]:
        key = (tuple(s["board"]), tuple((h["pos"], h["label"]) for h in s["history"]), s["hero"])
        if key in seen or not s["drills"]: continue
        seen.add(key)
        r2 = lambda x: round(x, 2)
        s["ranges"] = [[r2(v) for v in g] for g in s["ranges"]]
        s["drills"] = [{"hand": d["hand"][0] + d["hand"][1], "cls": d["cls"], "w": r2(d["w"]), "strat": [r2(v) for v in d["strat"]], "ev": [r2(v) for v in d["ev"]]} for d in s["drills"]]
        spots.append(s)
    print(f"{fn}: {len(j['spots'])} spots -> kept {len(spots)} total")
pack = {"version": 1, "seats": 3, "formation": "CO opens 2.5bb, BTN calls, BB calls", "spots": spots}
raw = json.dumps(pack, separators=(",", ":")).encode()
with gzip.open(out, "wb") as f: f.write(raw)
print(f"{len(spots)} spots, {len(raw)/1e6:.1f} MB raw -> {out}")
