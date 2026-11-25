#!/usr/bin/env python3
"""Build a self-contained static WASM site into target/web (no Node toolchain)."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--debug', action='store_true', help='Build faster with debug symbols')
    args = parser.parse_args()
    bindgen = os.environ.get('WASM_BINDGEN', 'wasm-bindgen')
    lock = tomllib.loads((ROOT / 'Cargo.lock').read_text())
    version = next(p['version'] for p in lock['package'] if p['name'] == 'wasm-bindgen')
    try:
        installed = subprocess.check_output([bindgen, '--version'], text=True).strip().split()[-1]
    except FileNotFoundError:
        parser.error(f'Install the matching CLI: cargo install wasm-bindgen-cli --version {version} --locked')
    if installed != version:
        parser.error(f'wasm-bindgen {version} required; found {installed}')
    target = ROOT / 'target'
    command = ['cargo', 'build', '--locked', '--target', 'wasm32-unknown-unknown', '--bin', 'sim_gui', '--target-dir', str(target)]
    if not args.debug:
        command.append('--release')
    output = target / 'web'
    # Avoid shipping stale assets (especially an old production _worker.js) in a preview.
    if output.is_symlink() or output.resolve().parent != target.resolve():
        raise SystemExit('Refusing to clean an output directory outside target/')
    target.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(dir=target) as staging:
        site = Path(staging) / 'site'
        site.mkdir()
        wasm = target / 'wasm32-unknown-unknown' / ('debug' if args.debug else 'release') / 'sim_gui.wasm'
        for package, flags in [('pkg', []), ('pkg-simd', ['--features', 'wasm-simd-rng'])]:
            subprocess.run(command + flags, cwd=ROOT, check=True)
            subprocess.run([bindgen, str(wasm), '--target', 'web', '--out-dir', str(site / package),
                            '--out-name', 'sim_gui'], check=True)
        for source in (ROOT / 'web').iterdir():
            if source.is_file():
                shutil.copy2(source, site / source.name)
        shutil.copy2(ROOT / 'assets/icon_sim_gui.png', site / 'icon.png')
        # Only replace the previous site after both modules and all static assets are ready.
        previous = Path(staging) / 'previous'
        if output.exists():
            output.rename(previous)
        try:
            site.rename(output)
        except OSError:
            if previous.exists():
                previous.rename(output)
            raise
    print(f'Built {output}\nPreview: python3 -m http.server 8080 --bind 127.0.0.1 --directory "{output}"')

if __name__ == '__main__':
    main()
