#!/usr/bin/env bash
cd "$(dirname "$0")/.."
for job in "sb_bb Kc5c5d" "btn_sb_3b Th9h8h"; do
  set -- $job; n="$1_$2"
  FOLD_TREE=ref ./target-v1/release/fold-cli refcheck $1 $2 > refcheck/ours_$n.json 2> refcheck/ours_$n.log
  ~/Desktop/projects/fold-ref/target/release/fold-ref refcheck/ours_$n.json > refcheck/ref_$n.json 2> refcheck/ref_$n.log
  python3 refcheck/compare.py $n >> refcheck/results.txt
done
echo REF_DONE >> refcheck/results.txt
