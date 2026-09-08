"""Serve one static build only for the lifetime of its browser regression test."""
import argparse
import functools
import http.server
import os
from pathlib import Path
import subprocess
import threading

parser = argparse.ArgumentParser()
parser.add_argument('--directory', default='config-generator/dist')
parser.add_argument('--script', default='tests/config-generator/browser.mjs')
parser.add_argument('--prefix', default='/', help='Public URL prefix used when building the static application')
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
prefix = '/' + args.prefix.strip('/') + '/' if args.prefix.strip('/') else '/'


class Handler(http.server.SimpleHTTPRequestHandler):
    def translate_path(self, path):
        if path.startswith(prefix):
            path = path[len(prefix) - 1:]
        return super().translate_path(path)

    def log_message(self, *_):
        pass


handler = functools.partial(Handler, directory=str(root / args.directory))
server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), handler)
thread = threading.Thread(target=server.serve_forever, daemon=True)
thread.start()
try:
    env = dict(os.environ, PREVIEW_URL=f'http://127.0.0.1:{server.server_port}{prefix}')
    subprocess.run(['node', str(root / args.script)], cwd=root, env=env, check=True, timeout=180)
finally:
    server.shutdown()
    server.server_close()
    thread.join()
