# Command: security-check

Run a security-focused review for FNPM.

## Steps

1. Run security tests:
   ```bash
   cargo test --test security_tests --all-features
   ```
2. Run the quality baseline:
   ```bash
   make fmt
   make clippy
   make test
   ```
3. If audit rules changed, review `docs/SECURITY.md`.

## Expected result

- Critical detections covered by tests.
- No regressions in the full pipeline.
