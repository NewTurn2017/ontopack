# Wiki-style docs upgrade smoke · 2026-05-25

## Claim

`docs/index.html` now behaves as a wiki-style OSS documentation entrypoint instead of a one-screen marketing landing page.

## Covered UX

- fixed top bar with project identity and search input
- left grouped navigation for concept, install, operations, workflows, AI/media, and reference sections
- right in-page table of contents generated from content sections
- Korean-first explanatory content with concrete commands
- command tables, workflow cards, release/CI status badges, and document map links
- client-side search that hides non-matching sections

## Validation

```bash
ruby -e 'require "nokogiri"; Nokogiri::HTML5(File.read("docs/index.html")); puts "html parsed"'
python3 -m http.server 4173 --directory docs
python3 - <<'PY'
from html.parser import HTMLParser
from urllib.request import urlopen

class Audit(HTMLParser):
    def __init__(self):
        super().__init__()
        self.ids=[]; self.links=[]; self.h1=0; self.h2=0; self.search=False
    def handle_starttag(self, tag, attrs):
        d=dict(attrs)
        if 'id' in d: self.ids.append(d['id'])
        if tag=='a' and 'href' in d: self.links.append(d['href'])
        if tag=='h1': self.h1+=1
        if tag=='h2': self.h2+=1
        if tag=='input' and d.get('id')=='doc-search': self.search=True

html=open('docs/index.html',encoding='utf-8').read()
a=Audit(); a.feed(html)
missing=[href for href in a.links if href.startswith('#') and href[1:] not in a.ids]
print({'bytes':len(html),'ids':len(a.ids),'links':len(a.links),'h1':a.h1,'h2':a.h2,'search':a.search,'missing_anchors':missing[:10]})
resp=urlopen('http://127.0.0.1:4173/',timeout=5)
print({'http_status':resp.status})
PY
git diff --check
```

Observed:

- HTML parsed successfully.
- HTTP static serve returned 200.
- Anchor audit found no missing in-page anchors.
- `docs/index.html` contains one `h1`, 19 `h2`, 23 ids, 28 links, and the `#doc-search` input.
- `git diff --check` passed.

## Known gaps

- No browser screenshot was attached to this repo note.
- The page is still a single static HTML file; a future pass should split major sections into dedicated pages while preserving the same wiki navigation model.
