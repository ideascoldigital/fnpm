# Cross-Platform Compatibility

This document describes the cross-platform considerations and solutions implemented in FNPM.

## Platform-Specific Issues

### Symlink Support

**Problem**: Different operating systems have different APIs for creating symbolic links:
- **Unix/Linux/macOS**: `std::os::unix::fs::symlink`
- **Windows**: `std::os::windows::fs::symlink_file`

**Solution**: Implemented a cross-platform helper function `create_symlink` that uses conditional compilation:

```rust
#[cfg(unix)]
use std::os::unix::fs::symlink;

#[cfg(windows)]
use std::os::windows::fs::symlink_file;

/// Create a symlink in a cross-platform way
fn create_symlink<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> Result<()> {
    #[cfg(unix)]
    {
        symlink(original, link)?;
    }
    #[cfg(windows)]
    {
        symlink_file(original, link)?;
    }
    Ok(())
}
```

### File Paths

**Considerations**:
- Windows uses backslashes (`\`) as path separators
- Unix systems use forward slashes (`/`)
- Rust's `std::path::Path` handles this automatically

**Best Practices**:
- Always use `std::path::Path` and `PathBuf` for file paths
- Use `Path::join()` instead of string concatenation
- Let Rust handle path separator conversion

### Error Messages

**Problem**: Different operating systems have different error messages:
- **Unix/Linux/macOS**: "No such file or directory"
- **Windows**: "The system cannot find the file specified"

**Solution**: Use predicate combinators in tests:

```rust
.stderr(
    predicate::str::contains("No such file or directory")
        .or(predicate::str::contains("The system cannot find the file specified"))
)
```

### Line Endings

**Considerations**:
- Windows uses CRLF (`\r\n`)
- Unix systems use LF (`\n`)

**Solution**: Git handles line ending conversion automatically with `.gitattributes`.

## CI/CD Cross-Platform Testing

### GitHub Actions Matrix

The CI pipeline tests on multiple platforms:

```yaml
strategy:
  matrix:
    os: [ubuntu-latest, windows-latest, macos-latest]
    rust: [stable, beta]
```

### Platform-Specific Exclusions

Some combinations are excluded for efficiency:

```yaml
exclude:
  - os: windows-latest
    rust: beta
  - os: macos-latest
    rust: beta
```

## Testing Cross-Platform Code

### Conditional Compilation in Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_cross_platform_functionality() {
        // Test that works on all platforms
        let result = create_symlink(&original, &link);
        
        // Handle platform-specific limitations gracefully
        match result {
            Ok(_) => assert!(link.exists()),
            Err(_) => {
                // Some systems might not support symlinks
                println!("Symlink creation failed (this is okay on some systems)");
            }
        }
    }
}
```

### Platform-Specific Tests

```rust
#[cfg(unix)]
#[test]
fn test_unix_specific_feature() {
    // Unix-only test
}

#[cfg(windows)]
#[test]
fn test_windows_specific_feature() {
    // Windows-only test
}
```

## Common Cross-Platform Patterns

### 1. Conditional Imports

```rust
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;
```

### 2. Platform-Specific Configuration

```rust
fn get_default_cache_dir() -> PathBuf {
    #[cfg(windows)]
    {
        dirs::cache_dir().unwrap_or_else(|| PathBuf::from("C:\\temp"))
    }
    
    #[cfg(unix)]
    {
        dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"))
    }
}
```

### 3. Executable Extensions

```rust
fn get_executable_name(name: &str) -> String {
    #[cfg(windows)]
    {
        format!("{}.exe", name)
    }
    
    #[cfg(not(windows))]
    {
        name.to_string()
    }
}
```

## Troubleshooting

### Windows CI Failures

**Common Issues**:
1. **Unix-specific imports**: Use conditional compilation
2. **Path separators**: Use `std::path::Path`
3. **Permissions**: Windows has different permission models
4. **Symlinks**: Require administrator privileges on older Windows

**Solutions**:
1. Use `#[cfg(unix)]` and `#[cfg(windows)]` attributes
2. Test locally with `cargo check --target x86_64-pc-windows-msvc`
3. Use cross-compilation for testing: `cargo install cross`

### macOS CI Failures

**Common Issues**:
1. **Case-sensitive filesystems**: Some macOS systems are case-insensitive
2. **Xcode command line tools**: Required for compilation

### Linux CI Failures

**Common Issues**:
1. **Missing system dependencies**: Install via apt/yum in CI
2. **Different distributions**: Test on multiple Linux variants

## Best Practices

1. **Always test on multiple platforms** before releasing
2. **Use conditional compilation** for platform-specific code
3. **Handle errors gracefully** when platform features aren't available
4. **Document platform requirements** in README
5. **Use Rust's standard library** abstractions when possible
6. **Test with different Rust versions** (stable, beta)

## Resources

- [Rust Conditional Compilation](https://doc.rust-lang.org/reference/conditional-compilation.html)
- [Platform-specific modules](https://doc.rust-lang.org/std/os/index.html)
- [Cross-compilation with Rust](https://rust-lang.github.io/rustup/cross-compilation.html)
