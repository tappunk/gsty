![gsty](https://raw.githubusercontent.com/tappunk/.github/refs/heads/main/assets/gsty.webp)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Crates.io Version](https://img.shields.io/crates/v/gsty?color=orange&cacheSeconds=3600)](https://crates.io/crates/gsty)
[![GitHub Release](https://img.shields.io/github/v/release/tappunk/gsty?color=blue)](https://github.com/tappunk/gsty/releases)
[![X Follow](https://img.shields.io/twitter/follow/tappunk?style=social)](https://x.com/tappunk)

# gsty

**Ghostty terminal theme browser with live preview TUI.** Browse, filter, and apply themes with real-time palette display.

[Installation](#installation) • [Usage](#usage) • [Theme Discovery](#theme-discovery)

## Features

- **Live preview** — applies themes in real-time, signals running Ghostty to reload via SIGUSR2
- **TUI interface** — ratatui-based terminal UI with syntax-highlighted Rust code preview
- **Dark/light filtering** — toggle between all, dark, and light themes
- **Name search** — filter themes by name with `/` search mode
- **Palette display** — 16-color palette grid with background, foreground, and sample code rendering
- **Plain text listing** — `--list` flag for non-TTY environments and scripting
- **Undo on cancel** — restores the previous theme if you cancel without confirming

## Installation

### Homebrew

```bash
brew install tappunk/gsty/gsty
```

### Cargo

```bash
cargo install gsty
```

### Build from source

```bash
git clone https://github.com/tappunk/gsty.git
cd gsty
cargo build --release
sudo cp target/release/gsty /usr/local/bin/gsty
```

## Usage

```bash
gsty                          # Interactive TUI theme picker
gsty --list                   # Plain text listing (non-TTY fallback)
```

### TUI keybindings

```
j / Down        Move selection down
k / Up          Move selection up
g / Home        Jump to first theme
G / End         Jump to last theme
f               Cycle filter (all → dark → light)
/               Toggle search mode
Backspace       Clear search
Enter / y       Confirm selection and apply
q / Esc         Cancel, restore previous theme
```

## Theme Discovery

gsty searches these paths for Ghostty theme files:

```
/Applications/Ghostty.app/Contents/Resources/ghostty/themes
/opt/homebrew/share/ghostty/themes
/usr/share/ghostty/themes
~/.config/ghostty/themes
```

Selected themes are applied to `~/.config/ghostty/auto/theme.ghostty`. Ghostty reloads its theme automatically via the SIGUSR2 signal.
