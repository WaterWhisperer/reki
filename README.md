# Reki

A blazingly fast git log viewer for the terminal, written in Rust.

> **Work In Progress** — not yet ready for daily use.

Inspired by [tig](https://github.com/jonas/tig), [yazi](https://github.com/sxyazi/yazi), and [lazygit](https://github.com/jesseduffield/lazygit).

## Features (planned)

- **Extreme performance** — incremental loading, large repos open instantly
- **Aesthetic commit graph** — beautiful branch visualization
- **Minimal & information-dense** — one commit per line, efficient use of screen space

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

## License

Licensed under either of

- [MIT license](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
