# CI Integration Example

This example demonstrates how to integrate wasm-slim into your CI/CD pipeline with automated size budget enforcement.

## What This Example Shows

- GitHub Actions workflow configuration
- Size budget enforcement in CI
- Build history tracking
- JSON output for automation
- Regression detection

## Prerequisites

```bash
# Install wasm-slim
cd ../..
cargo install --path .

# Install WASM target
rustup target add wasm32-unknown-unknown
```

## Project Structure

```
ci-integration/
├── .github/
│   └── workflows/
│       └── wasm-slim.yml   # GitHub Actions workflow
├── .wasm-slim.toml         # Size budget configuration
├── Cargo.toml
├── src/
│   └── lib.rs
└── README.md               # This file
```

## GitHub Actions Workflow

The workflow (`.github/workflows/wasm-slim.yml`) does the following:

1. **Build WASM module** with wasm-slim
2. **Check size budget** - fails if exceeded
3. **Track build history** - detects regressions
4. **Generate JSON report** - for further automation
5. **Upload artifacts** - for debugging

### Key Features

```yaml
# Fail if size budget exceeded
- name: Check size budget
  run: wasm-slim build --check

# Track size over time
- name: Track build history
  run: wasm-slim history --json > history.json

# Prevent size regressions
- name: Check for regressions
  run: |
    if wasm-slim build --json | jq -r '.regression' | grep -q 'true'; then
      echo "::error::WASM bundle size regressed!"
      exit 1
    fi
```

## Size Budget Configuration

The `.wasm-slim.toml` enforces size limits:

```toml
[size-budget]
target-size-kb = 800       # Ideal goal
warn-threshold-kb = 1000   # Warning (yellow)
max-size-kb = 1200         # Hard limit (fails CI)
```

### Budget Status Levels

1. **Under Target** (✅ Green) - Size ≤ 800 KB
   - CI passes
   - Best case scenario

2. **Above Target** (⚠️  Yellow) - 800 KB < Size ≤ 1000 KB
   - CI passes with warning
   - Consider optimization

3. **Warning** (🟡 Orange) - 1000 KB < Size ≤ 1200 KB
   - CI passes but alerts
   - Optimization recommended

4. **Over Budget** (❌ Red) - Size > 1200 KB
   - **CI fails**
   - Must fix before merge

## Local Testing

Test the CI workflow locally:

```bash
cd examples/ci-integration

# Check if build passes budget
wasm-slim build --check
echo "Exit code: $?"  # 0 = pass, 1 = fail

# View JSON output (for automation)
wasm-slim build --json | jq

# Simulate CI environment
wasm-slim build --check --no-emoji --json > result.json
```

## Workflow Triggers

The workflow runs on:

```yaml
on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]
  workflow_dispatch:  # Manual trigger
```

## JSON Output Schema

The `--json` flag outputs structured data:

```json
{
  "size_bytes": 1048576,
  "size_kb": 1024.0,
  "size_mb": 1.0,
  "budget": {
    "status": "warning",
    "target_kb": 800,
    "warn_threshold_kb": 1000,
    "max_size_kb": 1200,
    "within_budget": true
  },
  "regression": false,
  "previous_size_kb": 950.0,
  "delta_kb": 74.0,
  "delta_percent": 7.8
}
```

## Advanced CI Integration

### 1. Post Size to PR Comments

```yaml
- name: Comment on PR
  uses: actions/github-script@v6
  with:
    script: |
      const result = JSON.parse(fs.readFileSync('result.json'));
      const comment = `## 📦 WASM Bundle Size
      
      **Size**: ${result.size_kb.toFixed(1)} KB
      **Status**: ${result.budget.status}
      **Target**: ${result.budget.target_kb} KB
      ${result.regression ? '⚠️ Size regression detected!' : ''}`;
      
      github.rest.issues.createComment({
        issue_number: context.issue.number,
        owner: context.repo.owner,
        repo: context.repo.repo,
        body: comment
      });
```

### 2. Track Size Over Time

```yaml
- name: Upload size history
  uses: actions/upload-artifact@v3
  with:
    name: size-history
    path: .wasm-slim/history.json
```

### 3. Compare with Base Branch

```yaml
- name: Compare with main
  run: |
    git checkout main
    MAIN_SIZE=$(wasm-slim build --json | jq -r '.size_kb')
    git checkout -
    CURRENT_SIZE=$(wasm-slim build --json | jq -r '.size_kb')
    DIFF=$((CURRENT_SIZE - MAIN_SIZE))
    echo "Size difference: ${DIFF} KB"
```

## Troubleshooting

### CI fails with "Budget exceeded"

1. Run locally: `wasm-slim build --check`
2. Analyze: `wasm-slim analyze --mode top`
3. Optimize:
   - Remove unused dependencies
   - Externalize large assets
   - Use aggressive template

### Build history not tracking

Ensure `.wasm-slim/` directory is created:

```bash
mkdir -p .wasm-slim
wasm-slim build  # Creates history.json
```

### JSON output parsing fails

Validate JSON schema:

```bash
wasm-slim build --json | jq empty
```

## Next Steps

- Integrate with other CI providers (GitLab CI, CircleCI, etc.)
- Set up size monitoring dashboards
- Configure Slack/email notifications for size regressions
- See main [README](../../README.md) for more optimization strategies
