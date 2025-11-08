# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take the security of wasm-slim seriously. If you believe you have found a security vulnerability, please report it to us as described below.

### How to Report

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report them via:
- **GitHub Security Advisories** (recommended): Navigate to the Security tab and click "Report a vulnerability"
- **Email**: security@vitalratel.com (if GitHub Security Advisories are unavailable)

Please include the following information:
- Type of issue (e.g., command injection, path traversal, etc.)
- Full paths of source file(s) related to the manifestation of the issue
- The location of the affected source code (tag/branch/commit or direct URL)
- Any special configuration required to reproduce the issue
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if possible)
- Impact of the issue, including how an attacker might exploit it

### Response Timeline

- **Initial Response**: Within 48 hours
- **Status Update**: Within 7 days
- **Fix Timeline**: Varies based on severity and complexity

### What to Expect

1. We will acknowledge receipt of your vulnerability report
2. We will investigate and validate the issue
3. We will work on a fix and coordinate disclosure timeline with you
4. We will release a security update and publicly disclose the issue (with credit to you, if desired)

## Security Measures

wasm-slim implements several security practices:

### Code Security
- **No shell expansion**: All external commands use `Command::arg()` to prevent command injection
- **Path validation**: File operations restricted to project directory and `.wasm-slim/` folder
- **Safe parsing**: Uses battle-tested libraries (toml_edit, syn) for parsing untrusted input
- **Dependency scanning**: Automated security audits via `cargo-audit` in CI

### Build Security
- **Minimal dependencies**: Only necessary, well-maintained crates
- **No network requests**: Tool operates entirely offline (except tool installation)
- **Backup safety**: UUID + timestamp prevents backup file collisions

### Operational Security
- **Read-only operations**: Analysis commands don't modify files
- **Explicit backups**: Modifications create timestamped backups before changes
- **Dry-run mode**: Preview changes before applying

## Known Security Considerations

### External Tool Execution
wasm-slim executes external tools (cargo, wasm-opt, etc.). Users should:
- Only install tools from trusted sources
- Verify tool integrity before use
- Be aware that wasm-slim inherits security properties of these tools

### Configuration Files
`.wasm-slim.toml` and `Cargo.toml` are trusted input. Users should:
- Review configuration files from untrusted sources
- Use version control to track configuration changes

### WASM Binary Analysis
When analyzing WASM binaries:
- Only analyze binaries from trusted sources
- wasm-slim uses `twiggy` for binary analysis, which may have its own security considerations

## Security Best Practices for Users

1. **Keep wasm-slim updated**: Run `cargo install wasm-slim` regularly for security fixes
2. **Review changes**: Use `--dry-run` before applying optimizations
3. **Check backups**: Backups are stored in `.wasm-slim/backups/` - verify important changes
4. **Audit dependencies**: Run `cargo audit` on your project regularly
5. **Use CI/CD**: Automate security checks in your build pipeline

## Acknowledgments

We appreciate the security research community's efforts in responsibly disclosing vulnerabilities. Security researchers who report valid vulnerabilities will be acknowledged in our release notes (unless they prefer to remain anonymous).

## Contact

For general security questions or concerns (non-vulnerability), please open a GitHub issue with the `security` label.

---

**Last Updated**: 2025-11-07
