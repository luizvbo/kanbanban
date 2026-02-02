# Development & Release Workflow

This document outlines how to contribute to **Kanbanban** and the process for releasing new versions.

## Development Setup

1. **Clone the repo:**

   ```bash
   git clone https://github.com/luizvbo/kanbanban.git
   cd kanbanban
   ```

2. **Install required tools:**
   We use `cargo-edit` for versioning and `cargo-dist` for GitHub releases.

   ```bash
   cargo install cargo-edit cargo-dist
   ```

3. **Run tests:**
   Always ensure tests pass before making a PR or release.
   ```bash
   cargo test
   ```

## Release Process (Maintainers Only)

We use `cargo-release` to automate the release process and `cargo-dist` to build binaries.

### 1. Run the release command

This command will bump the version, update the lockfile, commit the changes, create a git tag, push to GitHub, and publish the crate to crates.io.

```bash
# Replace 'patch', 'minor', or 'major' as needed
cargo release patch --execute
```

### 2. Verify CI/CD

Pushing the tag automatically triggers the GitHub Action.

- Go to **GitHub > Actions** to monitor the "Release" workflow.
- Once finished, a new **GitHub Release** will be created with binaries and installers attached.

## Architecture Overview

- **`src/domain/`**: Data models and YAML serialization logic.
- **`src/app/`**: Application state management and modal logic.
- **`src/handler/`**: Input processing (keyboard event mapping).
- **`src/ui/`**: TUI rendering logic using Ratatui.
