"""Assemble docs and the independent SPA as sibling static sites. No publishing."""
from pathlib import Path
import shutil
import subprocess
import sys

root = Path(__file__).resolve().parents[1]
npm = shutil.which('npm.cmd') or shutil.which('npm')
if not npm:
    raise SystemExit('Node.js/npm is required; run npm ci --prefix config-generator first')
subprocess.run([sys.executable, '-m', 'zensical', 'build', '--clean'], cwd=root, check=True)
subprocess.run([npm, 'run', 'build', '--prefix', 'config-generator', '--', '--base', '/tuic/config-generator/'],
               cwd=root, check=True)
shutil.copytree(root / 'config-generator/dist', root / 'site/config-generator', dirs_exist_ok=True)
print('Built documentation and standalone SPA under site/; nothing published')
