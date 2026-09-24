#!/usr/bin/env bash
# usage: auto_it.sh <running_round> [max_round=3]  -- wait for round k, best-respond (iterate.py step), launch k+1 for
# the roles still moving; stops at GAP < 2 or max_round. Log: preflop/auto_it.log. Restart-safe: just rerun with the
# round that is currently measuring.
cd "$(dirname "$0")"; k="$1"; max="${2:-3}"; idle=0; down=0
while :; do
  batch="../gpu/batch_it_r$k.txt"; need=$(grep -c . "$batch")
  while :; do
    have=$(ls ../solver/calib_it_r$k/*.json 2>/dev/null | wc -l | tr -d ' ')
    failed=$(grep -c "FAILED" ../gpu/collect_it_r$k.log ../solver/measure_it_r$k.log 2>/dev/null | awk -F: '{s+=$2} END{print s+0}')
    [ "$((have + failed))" -ge "$need" ] && break
    pgrep -f "collect_it.sh $k" >/dev/null || { sleep 5; nohup bash ../gpu/collect_it.sh "$k" "$batch" >> "../gpu/collect_it_r$k.log" 2>&1 & }
    [ "${FOLD_MAC:-0}" = 1 ] && ! pgrep -f "measure_it.sh $k" >/dev/null && ! grep -q "ROUND_DONE $k" "../solver/measure_it_r$k.log" 2>/dev/null && (cd ../solver && nohup caffeinate -i bash ./measure_it.sh "$k" "$batch" >> "measure_it_r$k.log" 2>&1 &)
    # resume the GPU queue only when the box has been completely idle (no jobs from anyone) for two checks in a row;
    # the main session's jobs always take precedence
    ps="$(cd ../gpu && gpubox ps 2>&1 | sed 's/\x1b\[[0-9;]*m//g')"
    if echo "$ps" | grep -qiE "reset by peer|timed out|refused|non-zero exit|No route|unreachable"; then
      down=$((down+1)); idle=0; [ "$down" -eq 1 ] && echo "$(date '+%H:%M') box unreachable (WSL down? restart Ubuntu on the PC); will resume when it answers" >> auto_it.log
    elif [ ! -e it/HOLD ] && echo "$ps" | grep -q "(no jobs)"; then [ "$down" -gt 0 ] && echo "$(date '+%H:%M') box back after $down checks" >> auto_it.log; down=0; idle=$((idle+1)); else down=0; idle=0; fi
    if [ "$idle" -ge 5 ]; then idle=0; echo "$(date '+%H:%M') box idle, resuming fold-it-r$k" >> auto_it.log; (cd ../gpu && gpubox run -d -n "fold-it-r$k" -- bash -lc "./run_it.sh batch_it_r$k.txt $k" </dev/null >> "gpubox_it_r$k.log" 2>&1); fi
    sleep 120
  done
  pkill -f "collect_it.sh $k" 2>/dev/null; pkill -f "measure_it.sh $k" 2>/dev/null
  echo "== $(date '+%H:%M') round $k: $have measured, $failed failed" >> auto_it.log; python3 iterate.py step "$k" >> auto_it.log 2>&1
  gap=$(grep '^GAP' auto_it.log | tail -1 | awk '{print $2}')
  if python3 -c "import sys; sys.exit(0 if float('$gap') < 2.0 else 1)"; then echo "CONVERGED after round $k (gap $gap): ranges in it/r$((k+1))" >> auto_it.log; break; fi
  [ "$k" -ge "$max" ] && { echo "STOPPED at max round $k (gap $gap): ranges in it/r$((k+1))" >> auto_it.log; break; }
  k=$((k+1)); python3 iterate.py batch "$k" >> auto_it.log; bash ./launch_it.sh "$k" >> auto_it.log 2>&1
done
