#!/usr/bin/env bash
# Overnight CPU run: per-hand flop EVs for the preflop solve. One JSON per flop, written atomically;
# rerun the script any time and it resumes where it stopped. Failures are logged and skipped.
cd "$(dirname "$0")"; mkdir -p calib_pre
export FOLD_TREE=pre FOLD_EPS=0.02
for list in ../gpu/batch_pre.txt ../gpu/batch_pre2.txt; do
  while read -r form flop <&3; do
    n="${form}_${flop}"; [ -s "calib_pre/$n.json" ] && continue
    if ./target-v1/release/fold-cli measure "$form" "$flop" --target 1.0 --iters 250 > "calib_pre/$n.json.tmp" 2> "calib_pre/$n.log" </dev/null && [ -s "calib_pre/$n.json.tmp" ]; then
      mv "calib_pre/$n.json.tmp" "calib_pre/$n.json"; rm -f "calib_pre/$n.log"; echo "$(date +%H:%M) done $n"
    else rm -f "calib_pre/$n.json.tmp"; echo "$(date +%H:%M) FAILED $n"; fi
  done 3< "$list"
done
echo MEASURE_DONE
