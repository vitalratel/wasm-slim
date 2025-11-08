# CI Setup Guide

Quick start guide for setting up GitHub Actions CI for wasm-slim.

## Initial Setup Checklist

### 1. Enable GitHub Actions

If this is the first time using GitHub Actions:

- [ ] Go to repository Settings → Actions → General
- [ ] Select "Allow all actions and reusable workflows"
- [ ] Click "Save"

### 2. Coverage Reports

Coverage reports are automatically generated and uploaded as GitHub Actions artifacts:

- **LCOV format**: Available in the "coverage-report" artifact
- **HTML format**: Available in the "coverage-html" artifact (main branch only)

To view coverage:
1. Go to Actions → Select a workflow run
2. Scroll to "Artifacts" section
3. Download and open the HTML report

### 3. First Push to Main

The benchmark workflow needs an initial baseline:

```bash
# Ensure you're on main branch
git checkout main

# Run benchmarks locally to create initial results
cargo bench

# Build and create baseline
cargo build --release --bin bench-tracker
cargo run --release --bin bench-tracker -- baseline --version "initial"

# Commit the workflow files
git add .github/
git commit -m "ci: add GitHub Actions workflows"

# Push to main (this will create the first baseline in CI)
git push origin main
```

### 4. Verify Workflows Are Running

1. Go to your repository on GitHub
2. Click the "Actions" tab
3. You should see workflows running:
   - Tests
   - Code Coverage
   - Performance Benchmarks
   - Code Quality

### 5. Fix Any Initial Issues

Common first-run issues:

**Formatting Issues**:
```bash
cargo fmt --all
git add .
git commit -m "style: apply rustfmt"
```

**Clippy Warnings**:
```bash
cargo clippy --all-features --all-targets -- -D warnings
# Fix any warnings, then commit
```

**Test Failures**:
```bash
# Run tests locally first
cargo test --all-features
```

## Testing Workflows Locally

### Using act (GitHub Actions Local Runner)

Install act:
```bash
# macOS
brew install act

# Linux
curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash
```

Run workflows locally:
```bash
# Run all workflows
act -l  # List workflows

# Run specific workflow
act -j test  # Run test job
act -j clippy  # Run clippy job

# Run with secrets
act -j coverage --secret CODECOV_TOKEN=your-token
```

### Manual Validation

Before pushing, validate your changes:

```bash
# Run the same checks as CI
./scripts/ci-check.sh  # Create this helper script
```

Create `scripts/ci-check.sh`:
```bash
#!/bin/bash
set -e

echo "Running CI checks locally..."

echo "1. Formatting..."
cargo fmt --all -- --check

echo "2. Clippy..."
cargo clippy --all-features --all-targets -- -D warnings

echo "3. Tests..."
cargo test --all-features

echo "4. Benchmarks..."
cargo bench --no-run

echo "All checks passed!"
```

Make it executable:
```bash
chmod +x scripts/ci-check.sh
```

## Monitoring CI

### Status Badges

Add these to your README.md:

```markdown
[![Tests](https://github.com/vitalratel/wasm-slim/workflows/Tests/badge.svg)](https://github.com/vitalratel/wasm-slim/actions/workflows/test.yml)
[![Benchmarks](https://github.com/vitalratel/wasm-slim/workflows/Performance%20Benchmarks/badge.svg)](https://github.com/vitalratel/wasm-slim/actions/workflows/benchmarks.yml)
```

### Email Notifications

GitHub automatically sends notifications for:
- Workflow failures on your branches
- Workflow failures on main branch
- First-time workflow runs

Configure in: Settings → Notifications → Actions

### PR Status Checks

Require CI to pass before merging:

1. Go to Settings → Branches
2. Add branch protection rule for `main`
3. Enable "Require status checks to pass before merging"
4. Select required checks:
   - Test Suite
   - Clippy Lints
   - Code Formatting
   - Run Benchmarks (optional)
   - Generate Coverage Report

## Updating Workflows

### Testing Workflow Changes

When modifying workflows:

1. Create a feature branch:
   ```bash
   git checkout -b ci/update-workflows
   ```

2. Make changes to `.github/workflows/*.yml`

3. Push and create PR:
   ```bash
   git add .github/
   git commit -m "ci: update workflow configuration"
   git push origin ci/update-workflows
   ```

4. Workflows will run on the PR
5. Check "Actions" tab to see results
6. Merge when satisfied

### Syntax Validation

Use yamllint to check syntax:

```bash
# Install yamllint
pip install yamllint

# Check workflows
yamllint .github/workflows/*.yml
```

Or use actionlint (specific to GitHub Actions):

```bash
# Install actionlint
brew install actionlint  # macOS
# or download from: https://github.com/rhysd/actionlint

# Check workflows
actionlint .github/workflows/*.yml
```

## Common Customizations

### Adjust Benchmark Threshold

Edit `benchmarks.yml`:
```yaml
--max-regression 10.0  # Change to 5.0 for stricter, 15.0 for looser
```

### Add More Test Platforms

Edit `test.yml`:
```yaml
matrix:
  os: [ubuntu-latest, macos-latest, windows-latest, ubuntu-20.04]
  rust: [stable, "1.86", nightly]  # Add nightly if needed
```

### Skip CI on Specific Commits

Add to commit message:
```bash
git commit -m "docs: update README [skip ci]"
```

### Run Only Specific Workflows

Add path filters to workflows:
```yaml
on:
  push:
    branches: [main]
    paths:
      - 'src/**'
      - 'Cargo.toml'
      - 'Cargo.lock'
  pull_request:
    branches: [main]
```

## Troubleshooting

### Workflow Not Running

Check:
- [ ] Actions are enabled in repository settings
- [ ] Workflow file syntax is valid (use actionlint)
- [ ] Trigger conditions match your event (push/PR/etc.)
- [ ] Branch protection rules aren't blocking it

### Cache Not Working

Symptoms: Workflows take longer than expected

Solutions:
- Check cache hit rate in workflow logs
- Verify cache keys match across workflow runs
- Clear cache: Settings → Actions → Caches → Delete all

### Benchmark Baseline Missing

If benchmarks fail with "No baseline found":

```bash
# Locally create and commit baseline
cargo bench
cargo run --release --bin bench-tracker -- baseline --version "v0.1.0"

# Push to main
git add .wasm-slim/benchmarks/baseline.json
git commit -m "ci: add benchmark baseline"
git push origin main
```

Or disable baseline check temporarily in `benchmarks.yml`:
```yaml
- name: Check for regressions (PR only)
  if: false  # Temporarily disable
```

### Coverage Report Not Generated

Check workflow logs for specific error.

Common issues:
- Missing llvm-tools-preview component
- Test failures preventing coverage generation
- Insufficient disk space

Coverage reports are uploaded as artifacts - check the Artifacts section in the workflow run.

### Tests Pass Locally But Fail in CI

Common causes:
- Platform differences (use PathBuf, not hardcoded paths)
- Missing dependencies in CI
- Timezone/locale differences
- File permissions

Debug:
1. Check exact error in Actions tab
2. Add debug logging:
   ```yaml
   - name: Debug info
     run: |
       rustc --version
       cargo --version
       env
   ```

## Getting Help

- GitHub Actions docs: https://docs.github.com/en/actions
- Rust CI examples: https://github.com/actions-rs
- wasm-slim specific: Open an issue in the repository

## Security Considerations

- [ ] Never commit secrets to workflow files
- [ ] Use repository secrets for sensitive data
- [ ] Review third-party actions before using
- [ ] Keep actions pinned to specific versions (v4, not @latest)
- [ ] Enable "Require approval for all outside collaborators" in Settings → Actions

## Next Steps

After CI is set up:

1. [ ] Add status badges to README
2. [ ] Configure branch protection rules
3. [ ] Set up dependabot for workflow updates
4. [ ] Consider adding release workflow
5. [ ] Document CI in contributor guide
