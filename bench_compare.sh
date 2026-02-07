#!/bin/bash
set -e

DURATION=${1:-15}
OPTIMIZED="./target/release/examples/perf_bench"
BASELINE="./target/release/examples/perf_bench_baseline"
BASELINE_LOG="/tmp/gpui_baseline_fps.log"
OPTIMIZED_LOG="/tmp/gpui_optimized_fps.log"

echo "=========================================="
echo "  GPUI Performance Benchmark Comparison"
echo "=========================================="
echo "Duration: ${DURATION}s per run"
echo "Elements: 50x20 = 1000 (each with shadow, border, text)"
echo ""

echo "--- BASELINE (gpui v0.2.1, pre-optimization) ---"
$BASELINE 2>"$BASELINE_LOG" &
BASELINE_PID=$!
sleep "$DURATION"
kill $BASELINE_PID 2>/dev/null || true
wait $BASELINE_PID 2>/dev/null || true
sleep 1

echo "Baseline FPS readings:"
cat "$BASELINE_LOG"
echo ""

echo "--- OPTIMIZED (adabraka-gpui, with perf improvements) ---"
$OPTIMIZED 2>"$OPTIMIZED_LOG" &
OPTIMIZED_PID=$!
sleep "$DURATION"
kill $OPTIMIZED_PID 2>/dev/null || true
wait $OPTIMIZED_PID 2>/dev/null || true
sleep 1

echo "Optimized FPS readings:"
cat "$OPTIMIZED_LOG"
echo ""

echo "=========================================="
echo "  RESULTS SUMMARY"
echo "=========================================="
BASELINE_AVG=$(tail -5 "$BASELINE_LOG" | sed -n 's/.*FPS: \([0-9.]*\).*/\1/p' | awk '{s+=$1; n++} END {if(n>0) printf "%.1f", s/n; else print "N/A"}')
OPTIMIZED_AVG=$(tail -5 "$OPTIMIZED_LOG" | sed -n 's/.*FPS: \([0-9.]*\).*/\1/p' | awk '{s+=$1; n++} END {if(n>0) printf "%.1f", s/n; else print "N/A"}')
echo "Baseline avg (last 5 readings):  $BASELINE_AVG FPS"
echo "Optimized avg (last 5 readings): $OPTIMIZED_AVG FPS"
echo "=========================================="
