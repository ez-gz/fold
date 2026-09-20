#!/usr/bin/env bash
# Mac: pull each measurement solve, extract per-class EVs (calib), drop the strategy file
set -u; exec </dev/null; cd "$(dirname "$0")"; mkdir -p out_pre ../solver/calib_pre
export FOLD_TREE=pre FOLD_EPS=0.02 FOLD_EPS_P=0
while read -r form flop <&3; do
  name="${form}_${flop}"; [ -s "../solver/calib_pre/$name.json" ] && continue
  until gpubox pull "out_pre/$name.f16" >/dev/null 2>&1 && [ -s "out_pre/$name.f16" ]; do sleep 45; done
  (cd ../solver && ./target-v1/release/fold-cli calib "$form" "$flop" "../gpu/out_pre/$name.f16" 2>/dev/null > "calib_pre/$name.json.tmp" && mv "calib_pre/$name.json.tmp" "calib_pre/$name.json" && echo "done $name")
  rm -f "out_pre/$name.f16"; gpubox run -- rm -f "out_pre/$name.f16" >/dev/null 2>&1
done 3< "$1"
echo COLLECT_DONE
