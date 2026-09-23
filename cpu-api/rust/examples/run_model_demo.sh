#!/usr/bin/env bash
# Runs the model_writer + a few independent model_worker processes
# concurrently, all pointed at the same mmap-sync shared-memory path.
# None of these processes are related by fork()/parent-child -- they
# just agree on a filesystem path, same as Cloudflare's real setup
# (one retraining job, many unrelated request-handling workers).
set -euo pipefail
cd "$(dirname "$0")/.."

rm -f /tmp/ostep_cpu_api_model*

cargo run --quiet --example model_writer &
writer_pid=$!

sleep 0.3
cargo run --quiet --example model_worker -- 1 &
w1=$!
cargo run --quiet --example model_worker -- 2 &
w2=$!
cargo run --quiet --example model_worker -- 3 &
w3=$!

wait "$writer_pid"
sleep 0.5
kill "$w1" "$w2" "$w3" 2>/dev/null || true
wait "$w1" "$w2" "$w3" 2>/dev/null || true
