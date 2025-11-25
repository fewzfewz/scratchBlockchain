# Contributing to Modular Blockchain

Thank you for your interest in contributing! This document provides guidelines for contributing to the project.

---

## 🚀 Quick Start

1. **Fork the repository**
2. **Clone your fork**: `git clone https://github.com/YOUR_USERNAME/modular-blockchain.git`
3. **Create a branch**: `git checkout -b feature/your-feature-name`
4. **Make your changes**
5. **Run tests**: `cargo test`
6. **Submit a pull request**

---

## 📋 Development Setup

### Prerequisites
- Rust 1.75+ (`rustup update`)
- Docker & Docker Compose
- Node.js 18+ (for UI development)

### Build the Project
```bash
# Build all crates
cargo build --release

# Run tests
cargo test --all

# Run integration tests
cargo test --test '*'

# Run benchmarks
cargo bench
```

### Run a Local Node
```bash
# Start a single node
./target/release/node start

# Access the UIs
open explorer/index.html  # Block Explorer
open wallet/index.html    # Wallet
open docs/index.html      # Documentation
```

---

## 🎯 Areas Needing Contribution

See [ROADMAP.md](ROADMAP.md) for the full list. High-priority areas:

### 🔴 Critical (Mainnet Blockers)
- **Economic Engine**: Dynamic inflation, robust staking ([Phase 9](ROADMAP.md#phase-9-economic-engine-maturity))
- **Bridge Infrastructure**: Ethereum bridge, relayer network ([Phase 10](ROADMAP.md#phase-10-bridge-infrastructure))
- **Security Audit Prep**: Code cleanup, documentation ([Phase 12](ROADMAP.md#phase-12-security-audit))

### 🟡 High Priority
- **Developer SDKs**: JavaScript/TypeScript SDK ([Phase 13](ROADMAP.md#phase-13-developer-experience))
- **Governance**: Quorum, time-locks, runtime upgrades ([Phase 11](ROADMAP.md#phase-11-on-chain-governance-maturity))
- **Documentation**: Tutorials, examples, videos

### 🟢 Medium Priority
- **Performance Optimization**: Profiling, caching, parallelization
- **Testing**: More integration tests, chaos engineering
- **UI Improvements**: Better UX, mobile responsiveness

---

## 📝 Code Style & Standards

### Rust Code
- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Run `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Add documentation comments (`///`) for public APIs
- Write tests for new functionality

### Commit Messages
Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add dynamic inflation schedule
fix: resolve consensus timeout issue
docs: update staking documentation
test: add integration test for bridge
refactor: simplify block producer logic
```

### Pull Request Guidelines
1. **Title**: Clear, descriptive (e.g., "feat: implement delegation in staking contract")
2. **Description**: 
   - What problem does this solve?
   - How does it solve it?
   - Any breaking changes?
3. **Tests**: Include tests for new features
4. **Documentation**: Update relevant docs
5. **Changelog**: Add entry to `CHANGELOG.md`

---

## 🧪 Testing Guidelines

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_staking_delegation() {
        // Test implementation
    }
}
```

### Integration Tests
Place in `integration-tests/tests/`:
```rust
#[tokio::test]
async fn test_multi_node_consensus() {
    // Test implementation
}
```

### Running Specific Tests
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_staking_delegation

# Run tests for specific crate
cargo test -p consensus

# Run with output
cargo test -- --nocapture
```

---

## 🐛 Reporting Bugs

### Before Submitting
1. Check if the bug is already reported in [Issues](https://github.com/YOUR_USERNAME/modular-blockchain/issues)
2. Try to reproduce on the latest `main` branch
3. Collect relevant information (logs, system info, steps to reproduce)

### Bug Report Template
```markdown
**Description**
A clear description of the bug.

**Steps to Reproduce**
1. Start node with `./target/release/node start`
2. Submit transaction via wallet
3. Observe error in logs

**Expected Behavior**
Transaction should be included in next block.

**Actual Behavior**
Transaction rejected with error: "insufficient gas"

**Environment**
- OS: Ubuntu 22.04
- Rust version: 1.75.0
- Node version: 0.1.0

**Logs**
```
[paste relevant logs]
```
```

---

## 💡 Suggesting Features

### Feature Request Template
```markdown
**Problem**
Describe the problem this feature would solve.

**Proposed Solution**
Describe your proposed solution.

**Alternatives Considered**
Other approaches you've considered.

**Additional Context**
Any other relevant information.
```

---

## 🔒 Security Vulnerabilities

**DO NOT** open a public issue for security vulnerabilities.

Instead:
1. Email: security@yourproject.com
2. Include: Description, impact, reproduction steps
3. We'll respond within 48 hours
4. We'll credit you in our security advisories (if desired)

---

## 📚 Documentation Contributions

### Code Documentation
- Add `///` doc comments to public APIs
- Include examples in doc comments
- Run `cargo doc --open` to preview

### User Documentation
- Located in `docs/index.html`
- Update for new features or API changes
- Keep examples up-to-date

### Tutorials
- Place in `docs/tutorials/`
- Include working code examples
- Test all examples before submitting

---

## 🎨 UI/UX Contributions

### Block Explorer (`explorer/`)
- Vanilla JavaScript (no frameworks)
- Follow existing design system
- Test on Chrome, Firefox, Safari
- Ensure mobile responsiveness

### Wallet (`wallet/`)
- Security-first approach
- Clear error messages
- Accessibility (WCAG 2.1 AA)

---

## 🏗️ Architecture Guidelines

### Adding a New Crate
1. Create in appropriate directory (`consensus/`, `execution/`, etc.)
2. Add to workspace in root `Cargo.toml`
3. Document purpose in crate-level docs
4. Add to architecture diagram in `README.md`

### Modifying Consensus
- **CRITICAL**: Consensus changes require extensive testing
- Add integration tests in `integration-tests/tests/`
- Document protocol changes
- Consider backward compatibility

### Adding RPC Endpoints
1. Add handler in `node/src/rpc.rs`
2. Update OpenAPI spec (if we add one)
3. Add to documentation site
4. Add integration test

---

## 🤝 Code Review Process

### For Contributors
- Respond to feedback within 7 days
- Mark conversations as resolved when addressed
- Request re-review after making changes

### For Reviewers
- Be constructive and respectful
- Focus on code quality, not personal preferences
- Approve if changes are good enough (don't block on nitpicks)
- Suggest improvements for future PRs

---

## 📊 Performance Considerations

- Profile before optimizing (`cargo flamegraph`)
- Benchmark performance-critical code (`cargo bench`)
- Avoid premature optimization
- Document performance characteristics

---

## 🌍 Community

- **Discord**: [Join our server]
- **Twitter**: [@YourProject]
- **Weekly Calls**: Thursdays 3pm UTC
- **Forum**: [Discourse link]

---

## 📜 License

By contributing, you agree that your contributions will be licensed under the same license as the project (see [LICENSE](LICENSE)).

---

## 🙏 Recognition

Contributors will be:
- Listed in `CONTRIBUTORS.md`
- Mentioned in release notes
- Eligible for testnet incentives
- Considered for core team positions

---

**Thank you for contributing to the future of decentralized infrastructure!** 🚀
