"""Use the sibling TUIC checkout's real parsers, without editing that checkout."""
import json
from pathlib import Path
import subprocess
import shutil
import sys
import tomllib

root = Path(__file__).resolve().parents[2]
tuic = root.parent / "tuic"
target = root / ".cache" / "config-generator-rust"
target.mkdir(parents=True, exist_ok=True)
manifest = {
    "tuic-client": tuic / "crates" / "tuic-client",
    "tuic-server": tuic / "crates" / "tuic-server",
}
lines = ['[package]', 'name = "tuic-docs-config-check"', 'version = "0.0.0"', 'edition = "2024"',
         '[workspace]', '[[bin]]', 'name = "config-check"', 'path = "main.rs"', '[dependencies]']
for name, path in manifest.items():
    lines.append(f'{name} = {{ path = {json.dumps(path.as_posix())} }}')
lines += ['eyre = "0.6"', 'tokio = { version = "1", features = ["full"] }',
          'rustls = { version = "0.23", features = ["aws_lc_rs"] }']
# Preserve the consuming workspace's pinned fork patches.
patches = tomllib.loads((tuic / "Cargo.toml").read_text(encoding="utf-8"))["patch"]["crates-io"]
lines += ['[patch.crates-io]']
for name, patch in patches.items():
    lines.append(f'{name} = {{ path = {json.dumps((tuic / patch["path"]).as_posix())} }}')
(target / "Cargo.toml").write_text('\n'.join(lines) + '\n', encoding="utf-8")
(target / "main.rs").write_text(Path(__file__).with_name("config-check.rs").read_text(encoding="utf-8"), encoding="utf-8")
# Seed with TUIC's lock so Git revisions and existing versions match the baseline.
shutil.copyfile(tuic / 'Cargo.lock', target / 'Cargo.lock')
subprocess.run(['node', 'tests/config-generator/fixtures.mjs'], cwd=root, check=True)
args = ['cargo', 'run', '--manifest-path', str(target / 'Cargo.toml')]
if '--offline' in sys.argv:
    args += ['--offline']
args += ['--', str(root / '.cache' / 'config-generator-fixtures')]
subprocess.run(args, cwd=tuic, check=True)
