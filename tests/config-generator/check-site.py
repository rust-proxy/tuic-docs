"""Check combined static links and the standalone generator's privacy/assets."""
from html.parser import HTMLParser
from pathlib import Path
import re
from urllib.parse import unquote, urlsplit

root = Path(__file__).resolve().parents[2] / 'site'


class Links(HTMLParser):
    def __init__(self):
        super().__init__()
        self.links = []
        self.ids = set()

    def handle_starttag(self, tag, attrs):
        values = dict(attrs)
        if 'id' in values:
            self.ids.add(values['id'])
        if tag == 'a' and 'href' in values:
            self.links.append(values['href'])


parsed = {}
for path in root.rglob('*.html'):
    parser = Links()
    parser.feed(path.read_text(encoding='utf-8'))
    parsed[path.resolve()] = parser

errors = []
for path, page in parsed.items():
    for href in page.links:
        url = urlsplit(href)
        if url.scheme or url.netloc:
            continue
        local = unquote(url.path)
        if local.startswith('/tuic/'):
            target = root / local[len('/tuic/'):]
        elif local.startswith('/'):
            continue  # Outside this site's deployment prefix.
        else:
            target = path.parent / local if local else path
        target = target.resolve()
        if target.is_dir():
            target /= 'index.html'
        if not target.exists():
            errors.append(f'{path.name}: {href}')
        elif url.fragment and target in parsed and unquote(url.fragment) not in parsed[target].ids:
            errors.append(f'{path.name}: missing fragment {href}')

generator_path = root / 'config-generator/index.html'
generator = generator_path.read_text(encoding='utf-8')
for marker in ('googletagmanager', 'google-analytics', 'gtag('):
    assert marker not in generator, 'Generator must not include third-party analytics'
for marker in ('md-header', 'md-main', 'iframe', 'app.mjs'):
    assert marker not in generator, 'Generator must be a standalone Svelte/WASM application'
assets = re.findall(r'''(?:src|href)=["']([^"']+\.(?:js|wasm|css))["']''', generator)
assert any(asset.endswith('.js') for asset in assets), 'Application entry missing'
wasm_assets = list((root / 'config-generator/assets').glob('*.wasm'))
assert wasm_assets, 'Rust WASM engine missing'
bundles = list((root / 'config-generator/assets').glob('*.js'))
assert any(wasm.name in bundle.read_text(encoding='utf-8') for wasm in wasm_assets for bundle in bundles), 'WASM engine is not referenced by the application'
for asset in assets:
    url = urlsplit(asset)
    assert not url.scheme and not url.netloc, 'Generator must use local assets'
    target = root / url.path[len('/tuic/'):] if url.path.startswith('/tuic/') else generator_path.parent / url.path
    assert target.is_file(), f'Missing generator asset: {asset}'
assert not errors, '\n'.join(errors)
print(f'All local links and fragments passed across {len(parsed)} pages; standalone WASM assets present and analytics absent')
