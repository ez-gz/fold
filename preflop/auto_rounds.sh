#!/usr/bin/env bash
# usage: auto_rounds.sh <first_running_round> [max_round=9]
# Waits for the running round, best-responds, launches the next; stops when the best response is within 2 points of the
# range it was solved against (fixed point) or at max_round. Everything underneath is resumable; log: preflop/auto.log
cd "$(dirname "$0")"; k="$1"; max="${2:-9}"
while :; do
  until grep -q "ROUND_DONE $k" "../solver/measure_r$k.log" 2>/dev/null; do sleep 120; pgrep -f "measure_round.sh $k" >/dev/null || { sleep 5; grep -q "ROUND_DONE $k" "../solver/measure_r$k.log" || (cd ../solver && nohup caffeinate -i bash ./measure_round.sh "$k" >> "measure_r$k.log" 2>&1 &); }; done
  pkill -f "collect_pre.sh $k" 2>/dev/null
  echo "== $(date '+%H:%M') round $k" >> auto.log; python3 round.py "$k" >> auto.log 2>&1
  gap=$(grep GAP auto.log | tail -1 | cut -d' ' -f2)
  if python3 -c "import sys; sys.exit(0 if float('$gap') < 2.0 else 1)"; then echo "CONVERGED after round $k (gap $gap)" >> auto.log; break; fi
  [ "$k" -ge "$max" ] && { echo "STOPPED at max round $k (gap $gap)" >> auto.log; break; }
  k=$((k+1)); bash ./launch_round.sh "$k"
done
