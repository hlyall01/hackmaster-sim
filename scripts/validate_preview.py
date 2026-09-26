#!/usr/bin/env python3
"""Reject server code and symlinks before a static-only preview reaches credentials."""
from pathlib import Path

root = Path('site')
total = 0
count = 0
for path in root.rglob('*'):
    if path.is_symlink():
        raise SystemExit('Preview contains a symlink')
    if path.is_dir():
        continue
    relative = path.relative_to(root)
    if any(part.startswith('.') or part.startswith('_') for part in relative.parts):
        raise SystemExit('Preview contains deployment controls or hidden files')
    if path.suffix not in {'.html', '.js', '.css', '.wasm', '.png', '.ico', '.map', '.json', '.ts', '.txt'}:
        raise SystemExit('Preview contains an unexpected file type')
    total += path.stat().st_size
    count += 1
    if path.stat().st_size > 25 * 1024 * 1024 or total > 100 * 1024 * 1024 or count > 200:
        raise SystemExit('Preview is too large')
if not (root / 'index.html').is_file() or not (root / 'pkg/sim_gui_bg.wasm').is_file():
    raise SystemExit('Preview is missing its entry point or WASM')
# Force deployment as static assets with no Functions routes.
(root / '_routes.json').write_text('{"version":1,"include":["/*"],"exclude":["/*"]}')
(root / '_headers').write_text('/*\n  X-Robots-Tag: noindex\n  Referrer-Policy: no-referrer\n')
print('Validated static preview:', count, 'files')
