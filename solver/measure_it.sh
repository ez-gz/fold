#!/usr/bin/env bash
# usage: measure_it.sh <k> <batch>  -- Mac CPU: measure round k top-down (the GPU goes bottom-up); resumable, atomic writes.
cd "$(dirname "$0")"; k="$1"; out="calib_it_r$k"; mkdir -p "$out"
tail -r "$2" | while read -r form flop side; do
  n="${form}_${flop}_$side"; [ -s "$out/$n.json" ] && continue
  eval "$(python3 ../preflop/iterate.py env "$k" "$form" "$side")"
  if ./target-v1/release/fold-cli measure "$form" "$flop" --target 1.0 --iters 250 > "$out/$n.json.tmp" 2> "$out/$n.log" </dev/null && [ -s "$out/$n.json.tmp" ]; then
    mv "$out/$n.json.tmp" "$out/$n.json"; rm -f "$out/$n.log"; echo "$(date +%H:%M) r$k done $n"
  else rm -f "$out/$n.json.tmp"; echo "$(date +%H:%M) r$k FAILED $n"; fi
done
echo "ROUND_DONE $k"
