#!/usr/bin/env bash
# usage: launch_it.sh <k> [batch=gpu/batch_it_r<k>.txt]  -- write the specs for round k (skips existing), start the GPU
# queue on the box (bottom-up, resumable), the Mac collector and the Mac CPU measurer (top-down). Everything resumes.
cd "$(dirname "$0")/../gpu"; k="$1"; batch="${2:-batch_it_r$k.txt}"
while read -r form flop side; do
  [ -s "spec_it/$side/${form}_$flop.json" ] && continue
  mkdir -p "spec_it/$side"
  ( eval "$(python3 ../preflop/iterate.py env "$k" "$form" "$side")"; ../solver/target-v1/release/fold-cli spec "$form" "$flop" --out "spec_it/$side" >/dev/null 2>&1 </dev/null )
done < "$batch"
echo "$(date +%H:%M) specs ready for $batch"
gpubox run -d -n "fold-it-r$k" -- bash -lc "./run_it.sh $batch" </dev/null > "gpubox_it_r$k.log" 2>&1 || echo "gpubox launch failed, see gpu/gpubox_it_r$k.log (auto_it.sh resumes it when the box is idle)"
nohup bash ./collect_it.sh "$k" "$batch" > "collect_it_r$k.log" 2>&1 &
# Mac CPU measurer only when FOLD_MAC=1 (the main session may be running a big CPU solve)
[ "${FOLD_MAC:-0}" = 1 ] && (cd ../solver && nohup caffeinate -i bash ./measure_it.sh "$k" "../gpu/$batch" > "measure_it_r$k.log" 2>&1 &)
echo "$(date +%H:%M) round $k launched"
