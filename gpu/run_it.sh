#!/usr/bin/env bash
# GPU box: chart-anchored preflop measurement solves (spec_it/<side>/<form>_<flop> -> out_it/<form>_<flop>_<side>.f16).
# Re-reads the batch every pass, so a batch file that grows (stage 2 synced later) or a restart just continues.
set -u; mkdir -p out_it
while :; do
  next=""
  while read -r form flop side; do
    name="${form}_${flop}"; [ -s "out_it/${name}_$side.f16" ] && continue
    grep -qx "${name}_$side" out_it/failed.txt 2>/dev/null && continue
    [ -s "spec_it/$side/$name.json" ] || continue     # spec not synced yet
    next="$form $flop $side"; break
  done < "$1"
  [ -z "$next" ] && { echo BATCH_DONE; break; }
  set -- "$1" $next; form=$2; flop=$3; side=$4; name="${form}_${flop}"
  uv run python solver.py "spec_it/$side/$name" --iters 200 --target 1.0 --every 20 --dump "out_it/${name}_$side.f16.tmp" 2>&1 | grep -E "iter|storage|rror|emory" | tail -3 > "out_it/${name}_$side.log"
  [ -s "out_it/${name}_$side.f16.tmp" ] && mv "out_it/${name}_$side.f16.tmp" "out_it/${name}_$side.f16" || echo "${name}_$side" >> out_it/failed.txt
  echo "$(date +%H:%M) done ${name}_$side"
  set -- "$1"
done
