#!/usr/bin/env python3
"""Alternate two implementations; report host timings without extrapolating to MCUs."""
import json
import pathlib
import statistics
import subprocess
import sys

roots = [pathlib.Path(p) for p in sys.argv[1:]]
cases = [
    ["insert", "1000", "1000", "ordered"],
    ["insert", "1000", "1000", "out_of_order"],
    ["insert", "60000", "1000", "ordered"],
    ["insert", "60000", "1000", "out_of_order"],
    ["time_lookup", "100000", "1", "forward", "exact"],
    ["time_lookup", "100000", "4", "forward", "interpolated"],
]
output = []
for case in cases:
    readings = {str(root): [] for root in roots}
    for repetition in range(7):
        for root in roots if repetition % 2 == 0 else roots[::-1]:
            elapsed = int(subprocess.check_output([str(root / "probe"), *case], text=True))
            operations = int(case[2] if case[0] == "insert" else case[1])
            readings[str(root)].append(elapsed / operations)
    output.append({"case": case, "nanoseconds_per_operation": {
        root: {"median": statistics.median(values), "min": min(values), "max": max(values)}
        for root, values in readings.items()
    }})
print(json.dumps(output, indent=2))
