#!/usr/bin/env bash
cd "$(dirname "$0")"; mkdir -p calib
for f in cache/*.f16; do n=$(basename "$f" .f16); [ -s "calib/$n.json" ] && continue
  ./target-v1/release/fold-cli calib "${n%_*}" "${n##*_}" "$f" 2>/dev/null > "calib/$n.json.tmp" && mv "calib/$n.json.tmp" "calib/$n.json"; done
echo CALIB_DONE
