# Reki

Reki is a fast, read-only Git log browser for the terminal, written in Rust.

> **Work In Progress** — not yet ready for daily use.

Inspired by [tig](https://github.com/jonas/tig), [yazi](https://github.com/sxyazi/yazi), and [lazygit](https://github.com/jesseduffield/lazygit).

## V1 Direction

- **Tig-like graph fidelity** for common branches, merges, tags, and refs.
- **Responsive browsing** with a dense single-pane commit log.
- **Read-only safety**: no staging, checkout, reset, or repository mutation in v1.
- **Commit inspect view** opened from the log, focused on metadata, diffstat, and bounded patch details.

## Usage

```shell
cd /path/to/your/git/repo
reki
```

or

```shell
reki /path/to/your/git/repo
```

### Keybindings

|          Key          |          Action          |
|-----------------------|--------------------------|
| `j` / `↓`             | Move down / scroll down  |
| `k` / `↑`             | Move up / scroll up      |
| `Space` / `PageDown`  | Page down                |
| `-` / `a` / `PageUp`  | Page up                  |
| `g` / `Home`          | Jump / scroll to top     |
| `G` / `End`           | Jump / scroll to bottom  |
| `h` / `←`             | Scroll left              |
| `l` / `→`             | Scroll right             |
| `/`                   | Search loaded commits    |
| `n` / `N`             | Next / previous match    |
| `Enter`               | Open commit inspect view |
| `q` / `Esc`           | Quit or close view       |

## Building from source

```shell
git clone https://github.com/WaterWhisperer/reki.git
cd reki
cargo build --release
```

## Development

Rust formatting uses nightly rustfmt because `rustfmt.toml` enables unstable formatting options. The default toolchain remains stable for build, clippy, and tests.

```shell
cargo +nightly fmt -- --check
taplo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo +1.88.0 test --locked
cargo deny check
```

## Changelog

Reki uses [git-cliff](https://github.com/orhun/git-cliff) for release notes.

```shell
cargo install git-cliff
git cliff --unreleased
git cliff --output CHANGELOG.md
```

See [ARCHITECTURE.md](ARCHITECTURE.md) and [ROADMAP.md](ROADMAP.md) for the current design and roadmap.

## License

Licensed under either of

- [MIT license](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
