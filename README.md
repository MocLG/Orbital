<p align="center">
<pre>
     ██████╗ ██████╗ ██████╗ ██╗████████╗ █████╗ ██╗     
    ██╔═══██╗██╔══██╗██╔══██╗██║╚══██╔══╝██╔══██╗██║     
    ██║   ██║██████╔╝██████╔╝██║   ██║   ███████║██║     
    ██║   ██║██╔══██╗██╔══██╗██║   ██║   ██╔══██║██║     
    ╚██████╔╝██║  ██║██████╔╝██║   ██║   ██║  ██║███████╗
     ╚═════╝ ╚═╝  ╚═╝╚═════╝ ╚═╝   ╚═╝   ╚═╝  ╚═╝╚══════╝
</pre>
</p>

<h3 align="center">⚡ A zero-config cyberdeck terminal dashboard for developers ⚡</h3>

<p align="center">
  <a href="#features"><img src="https://img.shields.io/badge/zero--config-magic-00ffff?style=for-the-badge" alt="Zero Config"></a>
  <a href="#features"><img src="https://img.shields.io/badge/interactive-widgets-ff00c8?style=for-the-badge" alt="Interactive"></a>
  <a href="#install"><img src="https://img.shields.io/badge/rust-powered-39ff14?style=for-the-badge" alt="Rust"></a>
  <img src="https://img.shields.io/badge/license-MIT-ffbf00?style=for-the-badge" alt="MIT License">
</p>

<p align="center">
  <img src="demo.gif" alt="Orbital Demo" width="800">
</p>

---

**Orbital** drops you into a sci-fi command center the moment you run it. No config files. No setup. It scans your system, detects your tools, and lights up a neon dashboard — ready to use in under a second.

## Features

- **🔮 Zero-Config Auto-Discovery** — Just run `orbital`. It detects git repos, Docker daemons, listening ports, and system stats automatically.
- **⚡ Interactive Widgets** — Not read-only. Kill processes, commit & push git changes, restart Docker containers — all from the dashboard.
- **🎨 Cyberdeck Aesthetic** — Neon cyan/magenta/green palette, rounded borders, focus states, and clean layouts. Looks like it belongs in a spaceship.
- **🧩 Modular Trait System** — Every widget implements `WidgetModule`. Drop in new modules without touching the core.
- **🦀 Single Binary** — Compiled Rust. No runtime dependencies. No interpreters. Just one executable.

## Install

```bash
cargo install orbital-tui
```

Or build from source:

```bash
git clone https://github.com/youruser/orbital.git
cd orbital
cargo build --release
./target/release/orbital
```

## Auto-Detected Modules

| Module | Detection | Interactive Actions |
|---|---|---|
| **◈ System** | Always | `Enter` refresh |
| **◈ Processes** | Always | `↑↓` select, `k` kill process |
| **◈ Disks** | Always | `↑↓` scroll, `Enter` refresh |
| **◈ Network** | Always | `↑↓` scroll, `Enter` refresh |
| **◈ Git** | `.git/` in cwd | `c` commit all, `p` push, `l` toggle log/changes |
| **◈ Docker** | Docker socket | `r` restart, `s` stop, `u` start container |
| **◈ Ports** | Always | `↑↓` scroll, `Enter` refresh |

## Keybindings

| Key | Action |
|---|---|
| `Tab` / `→` | Next widget |
| `Shift+Tab` / `←` | Previous widget |
| `↑` / `↓` | Scroll / select within widget |
| `?` | Toggle help overlay |
| `q` / `Ctrl+C` | Quit |

## Architecture

```
src/
├── main.rs          // Terminal setup & teardown
├── app.rs           // Core loop, layout grid, input routing
├── event.rs         // Async event handler (tick + keypress)
├── theme.rs         // Cyberdeck color palette & styles
├── discovery.rs     // Auto-detection engine
└── widgets/
    ├── mod.rs       // WidgetModule trait definition
    ├── system.rs    // CPU, RAM, uptime gauges
    ├── processes.rs // Top processes with kill support
    ├── disk.rs      // Disk usage gauges
    ├── network.rs   // Network interface stats
    ├── git.rs       // Branch, changes, commits, push
    ├── docker.rs    // Container management
    └── ports.rs     // Listening port scanner
```

## Requirements

- Rust 1.70+
- Linux (primary target — uses `sysinfo`, `ss` for ports)
- Optional: `git`, `docker` CLI for respective widgets

## License

MIT
