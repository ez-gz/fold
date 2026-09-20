#!/bin/bash
# Re-export every cached v1 strategy (no re-solve) into out_v1b, then repack. Resumable.
cd "$(dirname "$0")"; mkdir -p out_v1b
for f in cache/*.f16; do n=$(basename "$f" .f16); [ -s "out_v1b/$n.json" ] && continue
  form="${n%_*}"; flop="${n##*_}"
  target-v1/release/fold-cli import "$form" "$flop" "$f" --iters "$(cat cache/$n.iters)" --out "out_v1b/$n.tmp" >/dev/null 2>&1 && mv "out_v1b/$n.tmp" "out_v1b/$n.json" && echo "$(date +%H:%M) $n"
done
target-v1/release/fold-cli pack --dir out_v1b --out ../proto/pack-v1b.foldpack 2>&1 | tail -1; echo REBRIEF_DONE
