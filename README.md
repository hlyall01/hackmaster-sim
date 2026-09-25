# Hackmaster Simulator

This project ships one application: `sim_gui`, the combat simulator for desktop
and WebAssembly browsers. Both builds use the same GUI and simulation engine.

## Run

```bash
cargo run --release
# Equivalent: cargo run --release --bin sim_gui
# Development shortcut: cargo sim
```

The simulator includes live/step combat, bulk win-rate and detailed statistics,
DPS and damage-distribution plots, fighter and NPC presets, character and gear
editors, weapon styles and conditional tactics, spells, and wound-healing,
essence-wound and Ego calculators. `--console` enables diagnostics on Windows.

## Browser build

For automatic hosting, see [GitHub Actions and Cloudflare Pages setup](docs/web-deployment.md).

Install Rust's WASM target and the CLI version matching `Cargo.lock`:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.106 --locked
python3 scripts/build_web.py
python3 -m http.server 8080 --bind 127.0.0.1 --directory target/web
```

Open <http://localhost:8080>. Python 3.11+ is required for the build helper;
on Windows use `python` if `python3` is unavailable. `--debug` selects a faster
development build. `WASM_BINDGEN` can point to a matching prebuilt CLI.

The generated `target/web/` directory is a self-contained static site, suitable
for serving at a domain root or a subdirectory. Serve it over HTTP(S), with
`.wasm` files using `application/wasm`; opening `index.html` via `file://` does
not work. A modern browser with WebAssembly, WebGL and module workers is required.
No server API, SharedArrayBuffer or cross-origin isolation headers are needed.

Bulk simulations, DPS and damage distributions run in a dedicated Web Worker,
with progress and immediate cancellation. Live combat and all editors, plots,
spells, macros and calculators share the desktop implementation. Browser jobs
use one CPU worker, while desktop calculations retain their native parallelism.

Catalogs are bundled into WASM. Fighter and tactical presets save in localStorage
for the current browser and site origin; they are separate from desktop files.
Clearing site data removes browser saves. If storage is blocked or full, saving
reports an error. Desktop data-directory overrides do not apply in browsers.

## Code and data

- `src/bin/sim_gui.rs`, `src/bin/sim_gui/`: GUI and background-job scheduling.
- `src/game_logic.rs`, `src/game_logic/`: character setup, stat derivation and calculation jobs.
- `src/core/`: combat, magic, tactics, healing, dice and deterministic RNG.
- `src/character.rs`: ability and progression tables and derived character stats.
- `src/data/`, `data/sim/`: simulator catalog/preset adapters and bundled data.
- `src/sim.rs`, `src/ui_widgets.rs`, `src/assets.rs`, `src/console.rs`: display helpers and desktop support.
- `references/`: rules references. Older root-level application plans are historical.

Saved fighter/tactical presets are kept outside build output, under
`%LOCALAPPDATA%/HackmasterSim/data/sim` on Windows or
`$XDG_DATA_HOME/HackmasterSim/data/sim` (default `~/.local/share`) on Linux.
`HACKMASTER_SIM_DATA_DIR` selects an explicit data root. Existing saved presets
and legacy `data/*.json` simulator aliases remain supported. Missing catalog
files have bundled defaults; spell definitions are currently embedded at build time.

The separate campaign, autobattler, squad, browser-demo and weapon-plotter apps
have been retired. Their dependencies and runtime content are no longer built
or packaged. Simulator plots, combat rules and NPC/fighter data remain.

## Validate

```bash
cargo test --lib --bin sim_gui
cargo check --all-targets
cargo check --target wasm32-unknown-unknown --bin sim_gui
cargo run --example sim_capabilities
```

The capability-report example is a simulator diagnostic. GUI smoke tests cover
all main tabs, editor tabs and calculators without opening a native window.

## Windows builds

From WSL with the Windows GNU toolchain installed:

```bash
cargo build --release --bin sim_gui --target x86_64-pc-windows-gnu
```

The executable is `target/x86_64-pc-windows-gnu/release/sim_gui.exe`.
`scripts/build_sim_gui_to_desktop.bat` uses native Visual Studio tools and copies
the executable to the desktop. `installer/build_installer.ps1` packages only the
simulator and `data/sim` catalogs using Inno Setup.

For signing, create a local development certificate using
`scripts/create_cert.sh`, then set `WIN_TARGET` and `CODESIGN_PFX_PASSWORD` and run
`scripts/build_release_signed.sh`. It signs only the newly built simulator;
stale executables in `target/` are excluded. Windows SDK signing tools are required.
See `installer/README.md` for installer options.
