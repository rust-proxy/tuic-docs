"""Independent TOML/JSON/YAML parsers, invoked by generator.test.mjs over stdin."""
import json
import sys
import tomllib

import yaml

sys.stdin.reconfigure(encoding="utf-8")
cases = json.load(sys.stdin)
for case in cases:
    for name, parser in (("toml", tomllib.loads), ("json", json.loads), ("yaml", yaml.safe_load)):
        if parser(case["formats"][name]) != case["expected"]:
            # Never include generated credentials in diagnostics.
            raise AssertionError(f"Round-trip mismatch: {case['name']} / {name}")
print(f"{len(cases)} objects round-tripped across TOML, JSON and YAML")
