# Kanbanban 🦀

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Crates.io](https://img.shields.io/crates/v/kanbanban.svg)](https://crates.io/crates/kanbanban)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()

**Kanbanban** is a high-performance, terminal-based Kanban board written in Rust.

Version 1.0.0 represents a complete architectural shift, focusing on a **modal-driven interface**, **Markdown task descriptions**, and deep integration with your terminal environment (using your system's `$EDITOR` like Vim or Nano).

## Features

- **📝 Markdown Support:** Task descriptions now render with full Markdown support (bold, italics, lists, and headings).
- **📟 External Editor Bridge:** Press `o` to instantly jump into Vim, Nano, or VS Code to write long-form task notes, then jump back into the TUI.
- **🖼️ Modal-First UI:** A clean, focused editing experience that prevents accidental changes and maximizes vertical screen space.
- **🏷️ Smart Tagging:** A global tag registry that ensures consistent coloring across your entire board.
- **🏢 Category Management:** Organize cards into projects or categories with a dedicated selection modal.
- **⚡ Performance:** Rewritten from the ground up for zero-latency navigation, even with thousands of tasks.

## Installation

### From crates.io (Recommended)

```bash
cargo install kanbanban
```

### From Source

```bash
git clone https://github.com/luizvbo/kanbanban.git
cd kanbanban
cargo install --path .
```

## Usage

Simply run the binary. By default, it looks for `kbb.yaml` in your current directory, or you can specify a path:

```bash
kanbanban                     # Loads/Creates kbb.yaml
kanbanban my_work_board.yaml  # Loads specific file
```

### Keybindings

**Kanbanban** uses a "Vim-lite" modal system.

| Mode        | Key                 | Action                                      |
| :---------- | :------------------ | :------------------------------------------ |
| **Normal**  | `h` / `l`           | Switch Columns                              |
|             | `j` / `k`           | Select Card                                 |
|             | `a` / `n`           | **A**dd **N**ew Card                        |
|             | `Enter` / `e`       | **E**dit Card                               |
|             | `v`                 | **V**iew Details (Full Markdown)            |
|             | `d`                 | **D**elete Card                             |
|             | `H` / `L`           | Move Card to Left / Right Column            |
|             | `J` / `K`           | Move Card Up / Down inside Column           |
|             | `r` / `R`           | **R**ename Column / **R**ename Board        |
|             | `/`                 | **Filter** cards by title, tag, or category |
|             | `?`                 | Toggle Help Menu                            |
| **Editing** | `Tab` / `Shift+Tab` | Cycle through Title, Category, Tags, etc.   |
|             | `Enter`             | Edit selected field / Cycle to next         |
|             | `o`                 | Open Description in **External Editor**     |
|             | `Esc`               | Stop editing field / Exit Modal             |

## Data Storage

Kanbanban stores everything in a single, human-readable **YAML** file.

- The default filename is `kbb.yaml`.
- To see exactly where your data is saved, press `?` inside the app; the full file path is displayed at the bottom of the help menu.

## Contributing

We welcome contributions! The codebase is heavily documented to help you get started.

1. **Check the logic:** See `src/app/mod.rs` for the main application state.
2. **Change the look:** Check `src/ui/board.rs` for the Kanban rendering logic.
3. **Add a feature:** Add your new hotkey to `src/handler/modes.rs`.

Please see [DEVELOPMENT.md](DEVELOPMENT.md) for instructions on our workflow, testing, and release process.

## License

MIT License. See [LICENSE](LICENSE) for details.

---

### Architecture Note

_Version 1.0.0 introduces breaking changes to the YAML format compared to older 0.x versions to support the new Markdown and Category features._
