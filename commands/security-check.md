# Command: security-check

Run a security-focused review for FNPM.

## Steps

1. Run AST analyzer unit tests (symbol table, false-positive regression):
   ```bash
   cargo test --lib ast_security_analyzer
   ```
2. Run security integration tests:
   ```bash
   cargo test --test security_tests --all-features
   ```
3. Run the quality baseline:
   ```bash
   make fmt
   make clippy
   make test
   ```
4. If audit rules changed, review `docs/SECURITY.md`.

## Expected result

- AST symbol table tests pass (regex vs child_process differentiation).
- No false-positive regressions for `RegExp.exec()` patterns.
- Critical detections covered by tests.
- No regressions in the full pipeline.
