"""Assemble docs and the independent SPA as sibling static sites. No publishing."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('--trunk', default='trunk', help='Path to Trunk 0.21.14')
args = parser.parse_args()
env = os.environ.copy()
# Trunk expects a boolean, while some terminals export NO_COLOR=1.
if 'NO_COLOR' in env:
    env['NO_COLOR'] = 'true'
subprocess.run([sys.executable, '-m', 'zensical', 'build', '--clean'], cwd=root, check=True)
subprocess.run([args.trunk, '--config', str(root / 'config-generator/Trunk.toml'),
                'build', '--release', '--public-url', '/tuic/config-generator/'], cwd=root, env=env, check=True)
shutil.copytree(root / 'config-generator/dist', root / 'site/config-generator', dirs_exist_ok=True)
print('Built documentation and standalone SPA under site/; nothing published')
