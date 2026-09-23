#!/usr/bin/env bash
# usage: run_it.sh <batch> <k>  -- GPU box: preflop measurement solves for round k
# (spec_it_r<k>/<side>/<form>_<flop> -> out_it_r<k>/<form>_<flop>_<side>.f16). Ranges change every round, so dirs are per round.
# Re-reads the batch every pass, so a batch file that grows (stage 2 synced later) or a restart just continues.
set -u; k="$2"; sp="spec_it_r$k"; out="out_it_r$k"; mkdir -p "$out"
while :; do
  next=""
  while read -r form flop side; do
    name="${form}_${flop}"; [ -s "$out/${name}_$side.f16" ] && continue
    grep -qx "${name}_$side" $out/failed.txt 2>/dev/null && continue
    [ -s "$sp/$side/$name.json" ] || continue     # spec not synced yet
    next="$form $flop $side"; break
  done < "$1"
  [ -z "$next" ] && { echo BATCH_DONE; break; }
  set -- "$1" $next; form=$2; flop=$3; side=$4; name="${form}_${flop}"
  uv run python solver.py "$sp/$side/$name" --iters 200 --target 1.0 --every 20 --dump "$out/${name}_$side.f16.tmp" 2>&1 | grep -E "iter|storage|rror|emory" | tail -3 > "$out/${name}_$side.log"
  [ -s "$out/${name}_$side.f16.tmp" ] && mv "$out/${name}_$side.f16.tmp" "$out/${name}_$side.f16" || echo "${name}_$side" >> $out/failed.txt
  echo "$(date +%H:%M) done ${name}_$side"
  set -- "$1"
done
