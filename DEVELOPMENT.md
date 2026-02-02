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

To release a new version of Kanbanban, follow these steps:

### 1. Bump the version

Update the version in `Cargo.toml`. We follow [SemVer](https://semver.org/).

```bash
# For a major breaking change (e.g., 0.3.0 -> 1.0.0)
cargo set-version 1.0.0

# For a new feature (e.g., 1.0.0 -> 1.1.0)
cargo set-version 1.1.0

# For a bug fix (e.g., 1.0.0 -> 1.0.1)
cargo set-version 1.0.1
```

### 2. Update the Changelog

Document the changes in `CHANGELOG.md`.

### 3. Synchronize cargo-dist

If you've added new dependencies or changed the version significantly, ensure the CI/CD configuration is up to date:

```bash
cargo dist init
```

### 4. Tag the release

The GitHub Actions (via `cargo-dist`) are triggered by Git tags. **The tag must start with `v`**.

```bash
git add .
git commit -m "chore: release v1.0.0"
git tag v1.0.0
```

### 5. Push to GitHub

Push the commit and the tag. This will trigger the `release.yml` workflow which creates the GitHub Release and uploads binaries for Windows, Linux, and macOS.

```bash
git push origin main --tags
```

### 6. Publish to Crates.io

Finally, publish the library to the community:

```bash
cargo publish
```

## Architecture Overview

- **`src/domain/`**: Data models and YAML serialization logic.
- **`src/app/`**: Application state management and modal logic.
- **`src/handler/`**: Input processing (keyboard event mapping).
- **`src/ui/`**: TUI rendering logic using Ratatui.
