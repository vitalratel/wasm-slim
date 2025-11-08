# Development Scripts

This directory contains helper scripts for development and CI validation.

## Scripts Overview

### 🚀 ci-check.sh
**Comprehensive local CI checks** - Runs the same checks as GitHub Actions

**When to use:**
- Before pushing to `main` branch
- Before creating a pull request
- After making significant changes

**Duration:** 2-5 minutes

**Usage:**
```bash
./scripts/ci-check.sh
```

**What it checks:**
1. Code formatting (`cargo fmt`)
2. Clippy lints (all targets)
3. Unit tests
4. Integration tests
5. Doc tests
6. Examples build
7. Benchmark compilation
8. Cargo.lock up-to-date

### 🔬 full-ci-with-act.sh
**Complete CI validation using act** - Runs actual GitHub Actions workflows locally

**When to use:**
- Before major releases
- Testing workflow changes
- Platform-specific validation (requires Docker)

**Duration:** 5-30 minutes (depending on selection)

**Prerequisites:**
```bash
# Install act
# Linux:
curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash

# macOS:
brew install act

# Ensure Docker or Podman is running
docker info  # Docker
# OR
podman info  # Podman

# For Podman users, enable socket (required for act):
systemctl --user start podman.socket
systemctl --user enable podman.socket
```

**Usage:**
```bash
./scripts/full-ci-with-act.sh
```

**Interactive menu:**
1. All workflows (most thorough, ~30 min)
2. Tests only (recommended, ~5-10 min)
3. Tests + Clippy + Coverage (~10-15 min)
4. Custom selection

### ⚡ Pre-push Hook
**Fast validation before pushing** - Automatically runs on `git push`

**Installed at:** `.git/hooks/pre-push`

**What it does:**
- ✅ Checks code formatting
- ✅ Runs Clippy lints
- ✅ Runs unit tests (only on `main` branch)

**Behavior:**
- Feature branches: Fast checks only (formatting + clippy)
- Main branch: Strict mode (formatting + clippy + tests)

**Disable temporarily:**
```bash
# Skip the hook for one push
git push --no-verify
```

**Uninstall:**
```bash
rm .git/hooks/pre-push
```

## Recommended Workflow

### Daily Development
```bash
# Pre-push hook runs automatically
git push origin feature-branch
```

### Before Merging to Main
```bash
# Run comprehensive checks
./scripts/ci-check.sh

# If all pass, push
git push origin main
```

### Before Releases
```bash
# Run complete validation
./scripts/full-ci-with-act.sh

# Select option 3 (Tests + Clippy + Coverage)
```

## Troubleshooting

### "Cargo.lock is out of date"
```bash
cargo update --workspace
```

### "Integration tests skipped"
Install wasm-bindgen-cli:
```bash
cargo install wasm-bindgen-cli
```

### "act: command not found"
Install act following instructions in `full-ci-with-act.sh` section above.

### "Docker is not running" / "Podman is not running"
Start your container runtime:

**Docker:**
```bash
sudo systemctl start docker  # Linux
open -a Docker              # macOS
```

**Podman:**
```bash
systemctl --user start podman.socket  # Linux
# Make it permanent:
systemctl --user enable podman.socket
```

## Additional Resources

- [CI Setup Guide](../.github/CI_SETUP.md) - Complete GitHub Actions setup
- [Contributing Guide](../CONTRIBUTING.md) - How to contribute to the project
