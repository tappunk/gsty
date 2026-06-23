![gsty](https://raw.githubusercontent.com/tappunk/.github/refs/heads/main/assets/gsty.webp)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Crates.io Version](https://img.shields.io/crates/v/gsty?color=orange&cacheSeconds=3600)](https://crates.io/crates/gsty)
[![GitHub Release](https://img.shields.io/github/v/release/tappunk/gsty?color=blue)](https://github.com/tappunk/gsty/releases)
[![X Follow](https://img.shields.io/twitter/follow/tappunk?style=social)](https://x.com/tappunk)

# gsty

Ghostty live preview theme browser and installer TUI.

Browse, preview, and apply Ghostty terminal themes from a live TUI. Supports filtering by dark/light mode, searching by name, and real-time palette preview with a Rust code snippet.

## Usage

```bash
gsty                          # interactive TUI theme picker
gsty --list                   # plain text listing (non-TTY fallback)
```

### Theme Discovery

gsty searches these paths for Ghostty theme files:

```bash
/Applications/Ghostty.app/Contents/Resources/ghostty/themes
/opt/homebrew/share/ghostty/themes
/usr/share/ghostty/themes
~/.config/ghostty/themes
```

Selected themes are applied to `~/.config/ghostty/auto/theme.ghostty`. Hit return to apply theme.

## Installation

gsty is available on [crates.io](https://crates.io/crates/gsty) and [Homebrew](https://brew.sh/).

### Cargo

```bash
cargo install gsty
```

### Homebrew

```bash
brew install tappunk/gsty/gsty
```

Homebrew package support is Apple Silicon only.

### Build from Source

```bash
git clone https://github.com/tappunk/gsty.git
cd gsty
cargo build --release
sudo cp target/release/gsty /usr/local/bin/gsty
```

## Local Verification

Run the local verification gate before releases:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

## Releasing

Use the release pipeline to cut a GitHub release, publish to crates.io, and update the Homebrew tap:

```bash
./release.sh --dry-run
./release.sh patch
```
