# Testing Guide

This document describes the testing strategy and structure for FNPM.

## Test Structure

The project uses a comprehensive testing approach with three types of tests:

### 1. Unit Tests (`src/`)

Unit tests are located alongside the source code using Rust's built-in testing framework.

- **Config Tests** (`src/config.rs`): Test configuration loading, saving, and validation
- **Package Manager Tests** (`src/package_manager.rs`): Test package manager factory functions

**Running unit tests:**
```bash
make test-unit
```

### 2. Integration Tests (`tests/`)

Integration tests verify the CLI behavior and end-to-end functionality.

- **`integration_tests.rs`**: Tests CLI commands and user workflows
- **`package_managers_tests.rs`**: Tests package manager implementations

**Running integration tests:**
```bash
make test-integration
make test-package-managers
```

### 3. Test Categories

- **Unit Tests**: Fast, isolated tests for individual functions
- **Integration Tests**: Test CLI behavior and command interactions
- **Ignored Tests**: Tests requiring external dependencies (npm, yarn, etc.)

## Test Commands

```bash
# Run all tests
make test

# Run specific test types
make test-unit              # Unit tests only
make test-integration       # Integration tests only
make test-package-managers  # Package manager tests only

# Run ignored tests (requires package managers installed)
make test-ignored

# Run tests with verbose output
make test-verbose

# Generate test coverage report
make test-coverage
```

## Test Dependencies

The project uses these testing dependencies:

- **`tempfile`**: Temporary directories for file system tests
- **`assert_cmd`**: CLI testing framework
- **`predicates`**: Assertion helpers for complex conditions
- **`serial_test`**: Sequential test execution to avoid conflicts

## Writing Tests

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_function_name() {
        // Arrange
        let input = "test input";
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected_value);
    }
}
```

### Integration Tests

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;

#[test]
#[serial]
fn test_cli_command() {
    let mut cmd = Command::cargo_bin("fnpm").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("expected output"));
}
```

## Test Guidelines

1. **Use descriptive test names** that explain what is being tested
2. **Follow the Arrange-Act-Assert pattern** for clarity
3. **Use `#[serial]` for tests** that modify global state or file system
4. **Create temporary directories** for file system tests
5. **Test both success and error cases**
6. **Keep tests focused and independent**

## Continuous Integration

Tests are automatically run on:
- Every push to main branch
- Pull requests
- Release builds

The CI pipeline runs:
- Code formatting checks (`cargo fmt`)
- Linting (`cargo clippy`)
- All test suites
- Coverage reporting (optional)

## Coverage

Generate a coverage report:

```bash
make test-coverage
```

This creates an HTML report in the `coverage/` directory showing which code paths are tested.

## Troubleshooting

### Common Issues

1. **Tests failing due to file permissions**: Ensure test directories are writable
2. **Concurrent test failures**: Use `#[serial]` for tests that modify shared state
3. **Integration tests timing out**: Check that the binary builds successfully first

### Debug Tests

Run tests with detailed output:

```bash
make test-verbose
RUST_BACKTRACE=1 cargo test test_name
```

### Test in Isolation

Run a specific test:

```bash
cargo test test_name
cargo test --test integration_tests test_name
```
