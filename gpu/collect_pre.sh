#!/usr/bin/env bash
# usage: collect_pre.sh <k> <list>  -- Mac: pull each GPU measurement solve of round k, extract per-class EVs, drop the strategy file
set -u; exec </dev/null; cd "$(dirname "$0")"; k="$1"; out="../solver/calib_pre_r$k"; mkdir -p out_pre "$out"
export FOLD_TREE=pre FOLD_EPS=0.02 FOLD_EPS_P=0
while read -r form flop <&3; do
  name="${form}_${flop}"; [ -s "$out/$name.json" ] && continue
  until gpubox pull "out_pre/$name.f16" >/dev/null 2>&1 && [ -s "out_pre/$name.f16" ]; do [ -s "$out/$name.json" ] && continue 2; sleep 45; done
  export FOLD_RANGE0="$(cat ../preflop/rounds/r$k/$form.p0)"
  (cd ../solver && ./target-v1/release/fold-cli calib "$form" "$flop" "../gpu/out_pre/$name.f16" 2>/dev/null > "calib_pre_r$k/$name.gpu.tmp" && [ -s "calib_pre_r$k/$name.gpu.tmp" ] && mv "calib_pre_r$k/$name.gpu.tmp" "calib_pre_r$k/$name.json" && echo "$(date +%H:%M) gpu done $name")
  rm -f "out_pre/$name.f16"
done 3< "$2"
echo COLLECT_DONE
