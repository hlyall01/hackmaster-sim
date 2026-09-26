#!/usr/bin/env python3
"""Package the already-built WASM site for the dedicated character-test Pages project."""
from pathlib import Path
import shutil

ROOT = Path(__file__).resolve().parents[1]


def main():
    source = ROOT / 'target/web'
    output = ROOT / 'target/character-test'
    if not (source / 'pkg/sim_gui_bg.wasm').is_file():
        raise SystemExit('Run scripts/build_web.py first.')
    if output.is_symlink() or output.resolve().parent != (ROOT / 'target').resolve():
        raise SystemExit('Refusing output outside target/.')
    if output.exists():
        shutil.rmtree(output)
    shutil.copytree(source, output)
    index = output / 'index.html'
    content = index.read_text()
    content = content.replace('<head>', '<head>\n<meta name="hackmaster-cloud" content="character-test">', 1)
    index.write_text(content)
    shutil.copyfile(ROOT / 'cloud/pages-worker.js', output / '_worker.js')
    (output / '_routes.json').write_text('{"version":1,"include":["/*"],"exclude":[]}\n')
    (output / '_headers').write_text('/*\n  Cache-Control: no-store\n  X-Robots-Tag: noindex, nofollow\n')
    print(f'Test-only site: {output}')


if __name__ == '__main__':
    main()
