"""Compare a job's Scala output with its Rust output, record by record.

Usage: python3 tools/compare.py <scala-dir> <rust-dir> <job> [job ...]
Exits 1 if any job's outputs differ.
"""

import json
import sys
from collections import Counter
from pathlib import Path

DECIMALS = 6


def normalize(value):
    """Ignore differences that are not real: null vs missing field, int vs float, float noise."""
    if isinstance(value, dict):
        return {k: normalize(v) for k, v in sorted(value.items()) if v is not None}
    if isinstance(value, list):
        return [normalize(v) for v in value]
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return value
    return round(float(value), DECIMALS)


def load(path):
    with path.open() as f:
        return Counter(json.dumps(normalize(json.loads(line)), sort_keys=True) for line in f if line.strip())


def compare(scala_dir, rust_dir, job):
    rust_path = rust_dir / f"{job}.jsonl"
    if not rust_path.exists():
        print(f"{job:<28} NOT PORTED")
        return False
    scala, rust = load(scala_dir / f"{job}.jsonl"), load(rust_path)
    missing, extra = scala - rust, rust - scala
    ok = not missing and not extra
    status = "MATCH" if ok else "DIFF"
    print(f"{job:<28} {status:<5} scala={sum(scala.values()):<6} rust={sum(rust.values()):<6} "
          f"missing={sum(missing.values()):<4} extra={sum(extra.values())}")
    for label, diff in (("missing in rust", missing), ("extra in rust", extra)):
        for record in list(diff)[:3]:
            print(f"    {label}: {record}")
    return ok


def main():
    scala_dir, rust_dir, jobs = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3:]
    results = [compare(scala_dir, rust_dir, job) for job in jobs]
    print(f"\n{sum(results)}/{len(results)} jobs match")
    sys.exit(0 if all(results) else 1)


if __name__ == "__main__":
    main()
