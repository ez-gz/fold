#!/usr/bin/env bash
# usage: collect_it.sh <k> <batch>  -- Mac: pull each GPU measurement of round k, extract per-class EVs, drop the strategy file.
# Resumable; skips flops the Mac CPU already measured. Ends with COLLECT_DONE.
set -u; exec </dev/null; cd "$(dirname "$0")"; k="$1"; out="../solver/calib_it_r$k"; o="out_it_r$k"; mkdir -p "$o" "$out"
while read -r form flop side <&3; do
  name="${form}_${flop}_$side"; [ -s "$out/$name.json" ] && continue
  until gpubox pull "$o/$name.f16" >/dev/null 2>&1 && [ -s "$o/$name.f16" ]; do
    [ -s "$out/$name.json" ] && continue 2
    sleep 60
    gpubox pull "$o/failed.txt" >/dev/null 2>&1; grep -qx "$name" "$o/failed.txt" 2>/dev/null && { echo "$(date +%H:%M) gpu FAILED $name"; continue 2; }
  done
  eval "$(python3 ../preflop/iterate.py env "$k" "$form" "$side")"
  (cd ../solver && ./target-v1/release/fold-cli calib "$form" "$flop" "../gpu/$o/$name.f16" 2>/dev/null > "$out/$name.gpu.tmp" && [ -s "$out/$name.gpu.tmp" ] && mv "$out/$name.gpu.tmp" "$out/$name.json" && echo "$(date +%H:%M) gpu done $name")
  rm -f "$o/$name.f16" "$out/$name.gpu.tmp"
done 3< "$2"
echo COLLECT_DONE
