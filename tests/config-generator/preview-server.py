"""Local preview with the same /tuic/ prefix as GitHub Pages."""
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[2]


class Handler(SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path.startswith('/tuic/'):
            self.path = self.path[len('/tuic'):]
        else:
            self.send_error(404)
            return
        super().do_GET()

    def log_message(self, format, *args):
        pass


port = int(sys.argv[1]) if len(sys.argv) > 1 else 8765
server = ThreadingHTTPServer(('127.0.0.1', port), partial(Handler, directory=str(root / 'site')))
print(f'Preview: http://127.0.0.1:{port}/tuic/tools/config-generator/', flush=True)
server.serve_forever()
