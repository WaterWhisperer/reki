# Reki

Reki is a fast, read-only Git log browser for the terminal, written in Rust.

> **Work In Progress** — not yet ready for daily use.

Inspired by [tig](https://github.com/jonas/tig), [yazi](https://github.com/sxyazi/yazi), and [lazygit](https://github.com/jesseduffield/lazygit).

## V1 Direction

- **Tig-like graph fidelity** for common branches, merges, tags, and refs.
- **Responsive browsing** with a dense single-pane commit log.
- **Read-only safety**: no staging, checkout, reset, or repository mutation in v1.
- **Commit inspect view** opened from the log, focused on metadata and diffstat.

## Usage

```sh
cd /path/to/your/git/repo
reki
```

### Keybindings

|      Key     |   Action  |
|--------------|-----------|
|  `j` / `↓`   | Move down |
|  `k` / `↑`   |  Move up  |
|  `q` / `Esc` |   Quit    |

## Building from source

```sh
git clone https://github.com/WaterWhisperer/reki.git
cd reki
cargo build --release
```

## Development

```sh
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

## Changelog

Reki uses [git-cliff](https://github.com/orhun/git-cliff) for release notes.

```sh
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
