#!/bin/sh
cd "$(dirname "$0")"; B=target/release/proto-numerics
for m in "basin 1" "overflow 1" "slerp 1e8" "inverse 1e8" "drift 1e8" "drift9 1e9" "fixed 1.6e8"; do set -- $m; echo "=== $m start $(date +%T)"; $B $1 $2 > out-$1.txt 2>&1; echo "=== $m done $(date +%T) rc=$?"; done
