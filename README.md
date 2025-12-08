# Kanbanban

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()

**Kanbanban** is a fast, terminal-based Kanban board written in Rust.

It is a high-performance port of the Python [kanban-tui](https://github.com/Zaloog/kanban-tui), rewritten to utilize the efficiency of Rust and the [Ratatui](https://ratatui.rs/) ecosystem. It features a modal interface, mouse support, and structured logging.

## 🚀 Features

- **⚡ Blazing Fast:** Native binary with minimal resource usage.
- **📋 Full Kanban Workflow:** Create boards, columns, and tasks. Move tasks between columns with Vim-like keys or Drag & Drop.
- **🖱️ Mouse Support:** Click to select tasks/boards, drag to move tasks between columns.
- **📊 Visualization:** Built-in charts showing task distribution by category.
- **📝 JSONL Audit Logs:** An append-only, streaming audit log system that keeps history without bloating memory.
- **📅 Date Picker:** A visual calendar widget for selecting due dates.
- **🏷️ Categories:** Color-coded categories for better task organization.
- **⚙️ Configurable:** Supports XDG configuration standards.

## 📦 Installation

### From Source

```bash
git clone https://github.com/yourusername/kanbanban.git
cd kanbanban
cargo install --path .
```

### Binary

_(If you release binaries, add instructions here)_

## 🎮 Usage

Run the application:

```bash
kanbanban
```

### Keybindings

**Kanbanban** uses Vim-like navigation.

| Context    | Key                   | Action                                 |
| :--------- | :-------------------- | :------------------------------------- |
| **Global** | `q`                   | Quit application                       |
|            | `b`                   | Switch to **Board** View               |
|            | `v`                   | Switch to **Overview** (Charts/Logs)   |
|            | `s`                   | Switch to **Settings** (Column Config) |
|            | `C`                   | Switch to **Category Manager**         |
|            | `B`                   | Switch to **Board Selector**           |
| **Board**  | `h` / `j` / `k` / `l` | Navigate Columns/Tasks                 |
|            | `H` / `L`             | Move selected task Left / Right        |
|            | `n`                   | **N**ew Task                           |
|            | `e`                   | **E**dit Task                          |
|            | `d`                   | **D**elete Task                        |
|            | `c`                   | New **C**olumn                         |
|            | `D`                   | **D**elete Column                      |
| **Modals** | `Tab`                 | Cycle focus between inputs             |
|            | `Ctrl` + `s`          | Save and close                         |
|            | `Esc`                 | Cancel and close                       |
|            | `Ctrl` + `d`          | Open **Date Picker** (in Date field)   |

### Mouse Support

- **Click** on a task or column to select it.
- **Drag and Drop** a task from one column to another to move it.

## 📂 Data & Configuration

Kanbanban follows XDG Base Directory specifications.

- **Database:** `~/.local/share/kanbanban/kanbanban.yaml` (YAML format for Board/Task state).
- **Audit Logs:** `~/.local/share/kanbanban/kanbanban_audit.jsonl`.
- **Config:** `~/.config/kanbanban/config.toml`.

### Audit Logs (JSONL)

Unlike the original Python implementation which stored logs in the main database, **Kanbanban** uses **JSON Lines (.jsonl)**.

- **Append-Only:** extremely fast writing.
- **Streaming Read:** The UI only loads the last 100 entries, keeping memory usage low even with thousands of history events.
- **Parsable:** You can use tools like `jq` to analyze your own productivity history.

## 🏗️ Architecture

- **Backend:** Rust (File-based persistence).
- **TUI Framework:** [Ratatui](https://github.com/ratatui-org/ratatui).
- **Input Handling:** Crossterm.
- **Serialization:** Serde (YAML for state, JSON for logs).

## 🤝 Attribution

This project is a port of [kanban-tui](https://github.com/Zaloog/kanban-tui) by Zaloog.
While the codebase has been rewritten in Rust with architectural changes (JSONL logs, custom widgets), the design philosophy and feature set are heavily inspired by the original work.

## 📄 License

MIT License. See [LICENSE](LICENSE) for details.
