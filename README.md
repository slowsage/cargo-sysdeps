# cargo-sysdeps

[![Crates.io](https://img.shields.io/crates/v/cargo-sysdeps)](https://crates.io/crates/cargo-sysdeps)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Discover system package dependencies for Rust projects. Scans your crate's dependency tree for pkg-config requirements and maps them to distro packages.

## Installation

```bash
cargo (b)install cargo-sysdeps
```

## Usage

```bash
# Inside container: auto-detects distro from /etc/os-release
cargo sysdeps generate | cargo sysdeps install

# Outside container: --distro required
cargo sysdeps generate -d debian-12        # Debian 12 (resolves to bookworm)
cargo sysdeps generate -d debian-bookworm  # Debian (codename directly)
cargo sysdeps generate -d ubuntu-24.04     # Ubuntu 24.04 (resolves to noble)
cargo sysdeps generate -d ubuntu-noble     # Ubuntu (codename directly)
cargo sysdeps generate -d arch             # Arch Linux

# Stream mode (no raw .gz caching, for space-constrained CI)
cargo sysdeps generate --stream

# Save to file (for use in different container)
cargo sysdeps generate -d debian-12 > deps.txt

# Install from file
cargo sysdeps install -i deps.txt

# Cross-compilation setup (container only)
cargo sysdeps cross-setup --arch arm64
cargo sysdeps generate | cargo sysdeps install --arch arm64
```

## How it works

### generate

Runs locally on any OS. No Docker or package manager required.

1. Runs `cargo fetch` to download crate sources (from both `registry/src` and `git/checkouts`)
2. Parses `build.rs` and any `.rs` files within the `build/` subdirectory using syn AST to find `.probe()` calls
3. Parses `Cargo.toml` for `[package.metadata.system-deps]` sections
4. If version number given (e.g., `12`, `24.04`), fetches [distro-info-data](https://salsa.debian.org/debian/distro-info-data) CSV to resolve codename
5. Downloads distro package index (cached in `~/.cache/cargo-sysdeps/`):
   - Debian: `Contents-amd64.gz` from deb.debian.org
   - Ubuntu: `Contents-amd64.gz` from archive.ubuntu.com
   - Arch: `core.files.tar.gz`, `extra.files.tar.gz` from mirrors.kernel.org
6. Maps `.pc` names to packages via cached index
7. Outputs package names to stdout

### install

Runs inside a container or CI environment only.

1. Reads package names from stdin or file
2. Detects distro from `/etc/os-release`
3. Runs package manager:
   - Debian/Ubuntu: `apt-get install`
   - Arch: `pacman -S --needed`

## CI Examples

### GitHub Actions (Debian/Ubuntu)

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    container: rust:latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo install cargo-sysdeps
      - run: cargo sysdeps generate | cargo sysdeps install # auto-detects distro
      - run: cargo build --release
```

### GitHub Actions (Arch Linux)

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    container: archlinux:latest
    steps:
      - run: pacman -Sy --noconfirm rust
      - uses: actions/checkout@v4
      - run: cargo install cargo-sysdeps
      - run: cargo sysdeps generate | cargo sysdeps install # auto-detects distro
      - run: cargo build --release
```

## Supported Distros

| Distro     | Generate       | Install |
| ---------- | -------------- | ------- |
| Debian     | Yes            | Yes     |
| Ubuntu     | Yes            | Yes     |
| Arch Linux | Yes            | Yes     |
| Manjaro    | Yes (use arch) | Yes     |

Generation works on any OS (macOS, Windows, Linux). Install requires the target distro and runs only in containers.

## Cache

Package indexes are cached in a `cargo-sysdeps` subdirectory within your system's standard cache directory:

- Linux: `~/.cache/cargo-sysdeps/` (XDG Base Directory Specification)
- macOS: `~/Library/Caches/cargo-sysdeps/`
- Windows: `C:\Users\<user>\AppData\Local\cargo-sysdeps\`

Example cache contents:

```
debian-12-pc.index     # Debian 12 .pc -> package mappings
ubuntu-24.04-pc.index  # Ubuntu 24.04 .pc -> package mappings
arch-pc.index          # Arch Linux .pc -> package mappings
Contents-amd64.gz      # Raw Debian/Ubuntu package index (unless --stream)
core.files.tar.gz      # Raw Arch core repo index (unless --stream)
extra.files.tar.gz     # Raw Arch extra repo index (unless --stream)
```

Delete index files to force a refresh of package mappings.

## Detected Patterns

The tool finds pkg-config dependencies from:

- `[package.metadata.system-deps]` sections in Cargo.toml
- `pkg_config::Config::new().probe("name")` calls in build.rs
- Variable assignments like `let lib = "name"` followed by `.probe(lib)`

## Options

```
cargo sysdeps generate [OPTIONS]
  -d, --distro <DISTRO>  Target distro: debian-12, ubuntu-24.04, arch, etc.
                         Auto-detected in containers, required outside.
      --stream           Stream index without caching raw .gz files

cargo sysdeps install [OPTIONS]
  -i, --input <FILE>     Input file (reads stdin if not specified)
  -d, --distro <DISTRO>  Target distro (auto-detected if not specified)
      --arch <ARCH>      Architecture suffix for cross-compilation

cargo sysdeps cross-setup [OPTIONS]
      --arch <ARCH>      Target architecture (required, e.g., arm64)
  -d, --distro <DISTRO>  Target distro (auto-detected if not specified)
```

## License

MIT
