"""Independent TOML/JSON/YAML parsers for Rust-generated fixture data."""
import json
import sys
import tomllib

import yaml

sys.stdin.reconfigure(encoding="utf-8")
if len(sys.argv) > 1:
    with open(sys.argv[1], encoding="utf-8") as source:
        cases = json.load(source)
else:
    cases = json.load(sys.stdin)
for case in cases:
    for name, parser in (("toml", tomllib.loads), ("json", json.loads), ("yaml", yaml.safe_load)):
        if parser(case["formats"][name]) != case["expected"]:
            # Never include generated credentials in diagnostics.
            raise AssertionError(f"Round-trip mismatch: {case['name']} / {name}")
print(f"{len(cases)} objects round-tripped across TOML, JSON and YAML")
