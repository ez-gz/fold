#!/bin/bash
# Re-export every cached v1 strategy with river sweeps (no re-solve, best response reused) into out_v1r, then pack to
# proto/pack-v1r.foldpack. Resumable. Swap into proto/pack-v1.foldpack after checking the trainer.
cd "$(dirname "$0")"; mkdir -p out_v1r
for f in cache/*.f16; do n=$(basename "$f" .f16); [ -s "out_v1r/$n.json" ] && continue
  form="${n%_*}"; flop="${n##*_}"
  e=$(python3 -c "import json;d=json.load(open('out_v1/$n.json'));print(d['exploitability_pct_pot']*d['start_pot']/100)") || continue
  target-v1/release/fold-cli import "$form" "$flop" "$f" --iters "$(cat cache/$n.iters)" --expl "$e" --out "out_v1r/$n.tmp" >/dev/null 2>&1 && mv "out_v1r/$n.tmp" "out_v1r/$n.json" && echo "$(date +%H:%M) $n"
done
target-v1/release/fold-cli pack --dir out_v1r --out ../proto/pack-v1r.foldpack 2>&1 | tail -1; echo REEXPORT_DONE
