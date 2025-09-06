# Contributing to cpdb-rs

Thank you for your interest in contributing to cpdb-rs! This document provides guidelines and information for contributors.

## Getting Started

### Prerequisites

- Rust 1.70+ (2021 edition)
- cpdb-libs C library installed on your system
- Git

### Development Setup

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/your-username/cpdb-rs.git
   cd cpdb-rs
   ```
3. Add the upstream repository:
   ```bash
   git remote add upstream https://github.com/original-username/cpdb-rs.git
   ```
4. Install dependencies:
   ```bash
   cargo build
   ```

### Building and Testing

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Check formatting
cargo fmt --all -- --check

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings

# Build documentation
cargo doc --no-deps --open
```

## Contribution Guidelines

### Code Style

- Follow Rust naming conventions
- Use `cargo fmt` to format code
- Use `cargo clippy` to catch common issues
- Write comprehensive documentation
- Add tests for new functionality

### Commit Messages

Use clear, descriptive commit messages:

```
feat: add printer options support
fix: handle null pointer in settings
docs: update README with new examples
test: add unit tests for media handling
```

### Pull Request Process

1. Create a feature branch from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Make your changes and commit them:
   ```bash
   git add .
   git commit -m "feat: add your feature"
   ```

3. Push your branch:
   ```bash
   git push origin feature/your-feature-name
   ```

4. Create a pull request on GitHub

5. Ensure all CI checks pass

6. Address any review feedback

### Testing Requirements

- Add unit tests for new functionality
- Add integration tests for complex features
- Ensure all existing tests pass
- Test on multiple platforms when possible

### Documentation

- Update README.md for user-facing changes
- Add doc comments for public APIs
- Include examples in documentation
- Update CHANGELOG.md for significant changes

## Areas for Contribution

### High Priority

- [ ] Additional printer backends support
- [ ] Enhanced error handling and recovery
- [ ] Performance optimizations
- [ ] Cross-platform testing
- [ ] Documentation improvements

### Medium Priority

- [ ] Async/await support
- [ ] More comprehensive examples
- [ ] Benchmarking suite
- [ ] Fuzz testing
- [ ] Windows support (if feasible)

### Low Priority

- [ ] Additional utility functions
- [ ] More detailed logging
- [ ] Configuration management
- [ ] Plugin system

## Reporting Issues

### Bug Reports

When reporting bugs, please include:

- Operating system and version
- Rust version (`rustc --version`)
- cpdb-libs version
- Steps to reproduce
- Expected vs actual behavior
- Error messages or logs

### Feature Requests

For feature requests, please include:

- Use case description
- Proposed API design
- Implementation considerations
- Backward compatibility impact

## Code Review Process

### For Contributors

- Respond to review feedback promptly
- Make requested changes in new commits
- Ask questions if feedback is unclear
- Test changes thoroughly

### For Reviewers

- Be constructive and helpful
- Focus on code quality and correctness
- Check for security issues
- Verify tests and documentation

## Release Process

Releases are managed by maintainers:

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create release tag
4. Publish to crates.io
5. Create GitHub release

## Community Guidelines

- Be respectful and inclusive
- Help others learn and grow
- Share knowledge and experience
- Follow the Rust Code of Conduct

## Getting Help

- GitHub Issues for bug reports and feature requests
- GitHub Discussions for questions and general discussion
- Rust Discord for real-time chat
- Documentation for API reference

## License

By contributing to cpdb-rs, you agree that your contributions will be licensed under the MIT License.

## Recognition

Contributors will be recognized in:
- CONTRIBUTORS.md file
- Release notes
- GitHub contributors page

Thank you for contributing to cpdb-rs!
