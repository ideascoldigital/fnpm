# Command: release-smoke

Quick smoke test before publishing a release.

## Steps

1. Build release:
   ```bash
   make build-release
   ```
2. Verify CLI help:
   ```bash
   ./target/release/fnpm --help
   ```
3. Verify critical commands:
   ```bash
   ./target/release/fnpm version
   ./target/release/fnpm doctor
   ```
4. Validate baseline tests:
   ```bash
   make test
   ```

## Expected result

- Functional release binary.
- Core CLI commands respond correctly.
- No failing tests.
