#!/bin/bash
# Full re-export of every cached v1 strategy (no re-solve, best response reused from the old export) into out_v1c, then pack. Resumable.
cd "$(dirname "$0")"; mkdir -p out_v1c
for f in cache/*.f16; do n=$(basename "$f" .f16); [ -s "out_v1c/$n.json" ] && continue
  form="${n%_*}"; flop="${n##*_}"
  e=$(python3 -c "import json;d=json.load(open('out_v1/$n.json'));print(d['exploitability_pct_pot']*d['start_pot']/100)") || continue
  target-v1/release/fold-cli import "$form" "$flop" "$f" --iters "$(cat cache/$n.iters)" --expl "$e" --out "out_v1c/$n.tmp" >/dev/null 2>&1 && mv "out_v1c/$n.tmp" "out_v1c/$n.json" && echo "$(date +%H:%M) $n"
done
target-v1/release/fold-cli pack --dir out_v1c --out ../proto/pack-v1c.foldpack 2>&1 | tail -1; echo REEXPORT_DONE
