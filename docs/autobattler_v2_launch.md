# Autobattler v2 Launch and Packaging Plan

## Local Demo
Run the current v2 demo from the repository root:

```bash
cargo run --bin autobattler_v2_demo -- --port 8787
```

Then open `http://127.0.0.1:8787`. Omit `--port 8787` to use the same default port. The demo should stay a local preview surface for run-loop, event, reward, and UI flow validation; it is not the recommended release shell.

## App Technology Choice
Use a native Rust `eframe`/`egui` desktop app for the packaged v2 release.

Reasons:
- It matches the existing Rust simulation/data code and avoids a web runtime in the installer.
- `eframe`, `egui_plot`, and the Windows app packaging path already exist in the project.
- It can share save/data loading, deterministic run state, debug drawers, and harness exports with the current binaries.
- It is lower risk than moving the launch UI to Bevy while v2 is still mostly panel, map, reward, and log UX.

Keep the HTTP demo as a development-only binary until the egui v2 app reaches feature parity.

## Build Commands
Development checks:

```bash
cargo check --bin autobattler_v2_demo
cargo run --bin autobattler_v2_demo -- --port 8787
cargo run --bin autobattler_regression
```

Native release build for the v2 demo while it remains standalone:

```bash
cargo build --release --bin autobattler_v2_demo
```

Windows build from WSL:

```bash
WIN_TARGET=x86_64-pc-windows-gnu cargo build --release --target x86_64-pc-windows-gnu --bin autobattler_v2_demo
```

Full signed Windows build path for the existing release set:

```bash
export CODESIGN_PFX_PASSWORD="your-password"
WIN_TARGET=x86_64-pc-windows-gnu ./scripts/build_release_signed.sh
```

Installer build:

```bash
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$(wslpath -w /home/hazzal/projects/HackmasterSim/installer/build_installer.ps1)"
```

When the v2 egui shell is added, give it a dedicated binary name such as `autobattler_v2`, then replace the demo-specific build commands above with `--bin autobattler_v2`.

## Release Packaging
Package v2 as a new desktop executable in the Inno installer, alongside the existing `data/` directory.

Release package contents:
- `autobattler_v2.exe`: player-facing v2 run app.
- `data/`: catalogs, presets, events, quick starts, and any save-schema migration metadata needed by v2.
- `sim_gui.exe`, `sim_cli.exe`, and other existing tools only when they remain intentionally supported.
- Optional development certificate trust task for local/dev builds, following the current installer behavior.

Do not ship the HTTP demo as the primary launcher. If it is included at all, mark it as internal/dev and keep it out of Start Menu shortcuts.

Recommended installer behavior:
- Start Menu shortcut points to `autobattler_v2.exe` for the v2 launch.
- Existing `Hackmaster Sim` shortcut can remain on `sim_gui.exe` until the product name and launcher strategy are updated.
- Installer version increments with every packaged v2 build.
- Signed release artifacts include the v2 executable and final setup executable.

## Existing Binary Boundaries
Keep existing binaries focused on their current roles:

- `sim_gui`: legacy simulator, balancing tools, catalog inspection, and non-v2 analysis UI.
- `sim_cli`: scriptable simulator runs and command-line checks.
- `autobattler_cli`: text/CLI run smoke testing and deterministic debugging.
- `autobattler_regression`: seeded KPI harness and launch gating.
- `autobattler` / `autobattler_bevy`: legacy or experimental Bevy autobattler surface while it is still useful.
- `hackmaster_sim`: existing desktop simulator entry point.
- `autobattler_v2_demo`: local browser demo only, not the production packaged app.

Do not fold v2 launch behavior into the existing simulator binaries. Shared combat, run, event, save, and data code should live in the library; player-facing v2 UI and packaging should live behind the dedicated v2 binary.
