#!/usr/bin/env python3
"""Measure requested heap bytes and allocation deltas with Valgrind, not unsafe Rust."""
import json
import pathlib
import re
import subprocess
import sys

root = pathlib.Path(sys.argv[1]).resolve()
probe = root / "probe"
results = {"memory": [], "lookups": []}
for count in [1000, 10000]:
    for length in [8, 128]:
        output = root / f"massif-{count}-{length}.out"
        command = ["valgrind", "--tool=massif", "--stacks=no", "--time-unit=B", "--detailed-freq=1",
                   "--max-snapshots=100", f"--massif-out-file={output}", str(probe), "memory", str(count), str(length)]
        subprocess.run(command, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
        snapshots = output.read_text().split("snapshot=")[1:]
        peak = next(s for s in snapshots if "heap_tree=peak" in s)
        requested = int(re.search(r"mem_heap_B=(\d+)", peak)[1])
        overhead = int(re.search(r"mem_heap_extra_B=(\d+)", peak)[1])
        results["memory"].append(dict(samples=count, name_length=length, requested=requested, estimated_allocator_overhead=overhead))
for depth, direction, stamp in [(1, "forward", "exact"), (1, "reverse", "exact"), (1, "forward", "interpolated"), (4, "forward", "interpolated"), (64, "forward", "exact")]:
    counts = []
    for iterations in [0, 1000]:
        log = root / f"alloc-{depth}-{direction}-{stamp}-{iterations}.log"
        command = ["valgrind", "--tool=memcheck", "--leak-check=no", "--error-exitcode=97", f"--log-file={log}", str(probe), "lookup", str(iterations), str(depth), direction, stamp]
        subprocess.run(command, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
        match = re.search(r"total heap usage: ([\d,]+) allocs, [\d,]+ frees, ([\d,]+) bytes allocated", log.read_text())
        counts.append(tuple(int(n.replace(",", "")) for n in match.groups()))
    # The process copies argv: "1000" occupies three more bytes than "0".
    # Remove that setup difference from the lookup byte delta.
    results["lookups"].append(dict(depth=depth, direction=direction, stamp=stamp,
                                  allocations=(counts[1][0]-counts[0][0])/1000,
                                  allocated_bytes=(counts[1][1]-counts[0][1]-3)/1000))
(root / "measurements.json").write_text(json.dumps(results, indent=2) + "\n")
print(json.dumps(results, indent=2))
