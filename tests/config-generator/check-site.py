"""Check built local links, fragments and generator privacy/template assets."""
from html.parser import HTMLParser
from pathlib import Path
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

generator = (root / 'tools/config-generator/index.html').read_text(encoding='utf-8')
for marker in ('googletagmanager', 'google-analytics', 'gtag('):
    assert marker not in generator, 'Generator must not include third-party analytics'
for asset in ('app.mjs', 'styles.css'):
    assert f'../../assets/config-generator/{asset}' in generator, 'Generator asset prefix changed'
assert not errors, '\n'.join(errors)
print(f'All local links and fragments passed across {len(parsed)} pages; generator analytics disabled')
