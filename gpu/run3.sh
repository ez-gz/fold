#!/bin/bash
# three-seat flop solves, one after another: run3.sh batch3.txt (one spec name per line); skips names already dumped
cd "$(dirname "$0")"
while read -r n; do
  [ -z "$n" ] && continue
  [ -s "out3/$n.flopev.f32" ] && { echo "skip $n"; continue; }
  echo "=== $n $(date +%H:%M)"
  uv run python solver3.py "spec3/$n" --iters 300 --every 25 --target 0.3 --dump "out3/$n" 2>&1 | grep -E 'shape|iter|Error|error|dumped|Traceback'
done < "$1"
echo BATCH3_DONE
