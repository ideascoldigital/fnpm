# Windows Compatibility Fixes

This document tracks the Windows-specific compatibility issues that have been resolved in FNPM.

## Issues Resolved

### 1. Symlink API Compatibility

**Issue**: Code was using Unix-specific symlink functions that don't exist on Windows.

**Error**:
```
error[E0433]: failed to resolve: could not find `unix` in `os`
 --> src\package_managers\npm.rs:3:14
  |
3 | use std::os::unix::fs::symlink;
  |              ^^^^ could not find `unix` in `os`
```

**Solution**: Implemented conditional compilation with platform-specific imports:

```rust
#[cfg(unix)]
use std::os::unix::fs::symlink;

#[cfg(windows)]
use std::os::windows::fs::symlink_file;

/// Create a symlink in a cross-platform way
fn create_symlink<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> Result<()> {
    #[cfg(unix)]
    { symlink(original, link)?; }
    #[cfg(windows)]
    { symlink_file(original, link)?; }
    Ok(())
}
```

**Files Modified**: `src/package_managers/npm.rs`

### 2. Error Message Differences

**Issue**: Integration tests were expecting Unix error messages but Windows has different messages.

**Error**:
```
Unexpected stderr, failed var.contains(No such file or directory)
├── var: Error: The system cannot find the file specified. (os error 2)
```

**Solution**: Updated test to accept both error message formats:

```rust
.stderr(
    predicate::str::contains("No such file or directory")
        .or(predicate::str::contains("The system cannot find the file specified"))
)
```

**Files Modified**: `tests/integration_tests.rs`

## Platform-Specific Considerations

### Error Messages

| Platform | Error Message |
|----------|---------------|
| Unix/Linux/macOS | "No such file or directory" |
| Windows | "The system cannot find the file specified" |

### File System APIs

| Operation | Unix | Windows |
|-----------|------|---------|
| Create symlink | `std::os::unix::fs::symlink` | `std::os::windows::fs::symlink_file` |
| File permissions | `std::os::unix::fs::PermissionsExt` | `std::os::windows::fs::OpenOptionsExt` |

### Path Handling

- **Windows**: Uses backslashes (`\`) as path separators
- **Unix**: Uses forward slashes (`/`)
- **Solution**: Always use `std::path::Path` and `PathBuf`

## Testing Strategy

### CI Matrix Testing

The CI pipeline tests on multiple platforms to catch these issues early:

```yaml
strategy:
  matrix:
    os: [ubuntu-latest, windows-latest, macos-latest]
    rust: [stable, beta]
```

### Local Testing

To test Windows compatibility locally on Unix systems:

```bash
# Install cross-compilation target
rustup target add x86_64-pc-windows-msvc

# Check compilation for Windows
cargo check --target x86_64-pc-windows-msvc

# Use cross for full testing (requires Docker)
cargo install cross
cross test --target x86_64-pc-windows-msvc
```

## Best Practices

### 1. Conditional Compilation

Always use `#[cfg()]` attributes for platform-specific code:

```rust
#[cfg(windows)]
fn windows_specific_function() {
    // Windows-only code
}

#[cfg(unix)]
fn unix_specific_function() {
    // Unix-only code
}
```

### 2. Error Handling

When testing error conditions, account for platform differences:

```rust
// Good: Accepts both error formats
.stderr(
    predicate::str::contains("No such file")
        .or(predicate::str::contains("cannot find the file"))
)

// Bad: Only works on Unix
.stderr(predicate::str::contains("No such file or directory"))
```

### 3. Path Handling

Always use Rust's path abstractions:

```rust
// Good: Cross-platform
let path = Path::new("src").join("main.rs");

// Bad: Unix-specific
let path = "src/main.rs";
```

### 4. Testing

Include platform-specific tests when necessary:

```rust
#[cfg(windows)]
#[test]
fn test_windows_specific_behavior() {
    // Test Windows-specific functionality
}

#[cfg(unix)]
#[test]
fn test_unix_specific_behavior() {
    // Test Unix-specific functionality
}
```

## Troubleshooting

### Common Windows CI Failures

1. **Import errors**: Use conditional compilation for OS-specific modules
2. **Path separator issues**: Use `std::path::Path` instead of string manipulation
3. **Permission errors**: Windows has different permission models than Unix
4. **Error message mismatches**: Use `.or()` predicates in tests

### Debugging Steps

1. **Check the error message**: Look for OS-specific API usage
2. **Review imports**: Ensure platform-specific imports use `#[cfg()]`
3. **Test locally**: Use cross-compilation or Windows VM
4. **Update tests**: Make assertions platform-agnostic

## Resources

- [Rust Conditional Compilation](https://doc.rust-lang.org/reference/conditional-compilation.html)
- [Platform-specific modules](https://doc.rust-lang.org/std/os/index.html)
- [Cross-compilation guide](https://rust-lang.github.io/rustup/cross-compilation.html)
- [Windows-specific APIs](https://doc.rust-lang.org/std/os/windows/index.html)

## Status

✅ **All Windows compatibility issues resolved**
- Symlink API compatibility implemented
- Error message differences handled
- Cross-platform testing in place
- Documentation updated

The project now builds and tests successfully on Windows, macOS, and Linux.
