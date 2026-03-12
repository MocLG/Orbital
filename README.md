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

- **🔮 Zero-Config Auto-Discovery** — Just run `orbital`. It detects git repos, Docker daemons, listening ports, AI CLI tools, and system stats automatically.
- **⚡ Interactive Widgets** — Not read-only. Kill processes, stage & commit git changes, restart Docker containers, explore disk usage, launch AI tools — all from the dashboard.
- **🧭 Built-in Disk Explorer** — Press `l` on Disks to drill down into any mount point. Navigate directories, see recursive sizes, delete files — a native `ncdu` replacement.
- **🤖 AI Intel** — Auto-detects installed AI CLI tools (Claude, Codex, Copilot, Gemini) and lets you launch them with `i`.
- **🎨 Cyberdeck Aesthetic** — Neon cyan/violet/green palette, braille-resolution graphs, scanline effects, and boot sequence animation.
- **🔒 Vault** — Quick access to project config files (`.env`, `Cargo.toml`, `package.json`, etc.) with editor launch.
- **🧩 Modular Trait System** — Every widget implements `WidgetModule`. Drop in new modules without touching the core.
- **🦀 Single Binary** — Compiled Rust. No runtime dependencies. No interpreters. Just one executable.

## Install

```bash
cargo install orbital-tui
```

Or build from source:

```bash
git clone https://github.com/MocLG/orbital.git
cd orbital
cargo build --release
./target/release/orbital
```

## Auto-Detected Modules

| Module | Detection | Interactive Actions |
|---|---|---|
| **◈ System** | Always | CPU/RAM braille graphs, alert on >85% |
| **◈ Processes** | Always | `↑↓` select, `k` kill process |
| **◈ Disks** | Always | `↑↓` scroll, `l` disk explorer |
| **◈ Network** | Always | RX/TX sparklines |
| **◈ Git** | `.git/` in cwd | `a` stage/unstage, `e` edit, `c` commit, `p` push, `l` toggle view |
| **◈ Docker** | Docker socket | `r` restart, `s` stop container |
| **◈ Ports** | Always | `↑↓` scroll |
| **◈ Spectre** | Always | Active TCP connections, external IPs highlighted |
| **◈ Vault** | Config files in cwd | `e` open file in editor |
| **◈ AI Intel** | AI CLI in $PATH | `i` launch AI tool |

## Keybindings

| Key | Action |
|---|---|
| `Tab` / `→` | Next widget |
| `Shift+Tab` / `←` | Previous widget |
| `↑` / `↓` | Scroll / select within widget |
| `?` | Toggle help overlay |
| `q` / `Ctrl+C` | Quit |

**Disk Explorer** (press `l` on Disks):

| Key | Action |
|---|---|
| `↑` / `↓` / `j` / `k` | Navigate entries |
| `Enter` | Drill into directory |
| `Backspace` | Go up one level |
| `d` | Delete (with confirmation) |
| `Esc` / `q` | Close explorer |

## Architecture

```
src/
├── main.rs          // Terminal setup & teardown
├── app.rs           // Core loop, layout grid, input routing, explorer overlay
├── event.rs         // Async event handler (tick + keypress)
├── theme.rs         // Cyberdeck color palette & styles
├── discovery.rs     // Auto-detection engine
├── ops/
│   ├── mod.rs       // Module declarations
│   └── scanner.rs   // Recursive directory size scanner (jwalk)
├── ui/
│   ├── mod.rs       // Module declarations
│   └── explorer.rs  // Interactive disk explorer overlay
└── widgets/
    ├── mod.rs       // WidgetModule trait + WidgetAction enum
    ├── system.rs    // CPU, RAM braille graphs
    ├── processes.rs // Top processes with kill support
    ├── disk.rs      // Disk usage gauges + explorer launch
    ├── network.rs   // Network interface sparklines
    ├── git.rs       // Staged/changed files, commit, push
    ├── docker.rs    // Container management
    ├── ports.rs     // Listening port scanner
    ├── spectre.rs   // Active TCP connection monitor
    ├── vault.rs     // Project config file browser
    └── ai_intel.rs  // AI CLI tool discovery & launch
```

## Requirements

- Rust 1.70+
- Linux (primary target — uses `sysinfo`, `ss` for ports)
- Optional: `git`, `docker` CLI for respective widgets

## License

MIT
