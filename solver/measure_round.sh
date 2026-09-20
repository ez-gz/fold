#!/usr/bin/env bash
# usage: measure_round.sh <k>   -- measure per-hand flop EVs with the round-k big-blind ranges (preflop/rounds/r<k>/*.p0).
# Resumable: one JSON per flop, written atomically, finished flops are skipped.
cd "$(dirname "$0")"; k="$1"; out="calib_pre_r$k"; mkdir -p "$out"
export FOLD_TREE=pre FOLD_EPS=0.02
while read -r form flop <&3; do
  n="${form}_${flop}"; [ -s "$out/$n.json" ] && continue
  unset FOLD_RANGE0; [ -s "../preflop/rounds/r$k/$form.p0" ] && export FOLD_RANGE0="$(cat ../preflop/rounds/r$k/$form.p0)"
  if ./target-v1/release/fold-cli measure "$form" "$flop" --target 1.0 --iters 250 > "$out/$n.json.tmp" 2> "$out/$n.log" </dev/null && [ -s "$out/$n.json.tmp" ]; then
    mv "$out/$n.json.tmp" "$out/$n.json"; rm -f "$out/$n.log"; echo "$(date +%H:%M) r$k done $n"
  else rm -f "$out/$n.json.tmp"; echo "$(date +%H:%M) r$k FAILED $n"; fi
done 3< ../gpu/batch_pre.txt
echo "ROUND_DONE $k"
