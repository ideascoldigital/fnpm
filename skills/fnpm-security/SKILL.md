# SKILL: fnpm-security

## When to use this skill

Use it when a task touches security: auditing, AST analysis, install scripts, transitive dependencies, or risky package blocking.

## Minimum context to load

1. `docs/SECURITY.md`
2. `docs/TRANSITIVE_SECURITY.md`
3. `src/security.rs`
4. `src/ast_security_analyzer.rs`
5. Tests in `tests/security_tests.rs`

## Recommended sequence

1. Identify the risk surface (scripts, dynamic code, network, filesystem).
2. Validate that detection does not create obvious false positives.
3. If modifying AST analysis, verify the **symbol table** (`tracked_vars` / `VarKind`) in
   `src/ast_security_analyzer.rs` correctly classifies variables as `Regex` or `ChildProcess`.
4. Add or update security tests.
5. Run AST-specific tests first:
   - `cargo test --lib ast_security_analyzer`
6. Run security integration tests:
   - `cargo test --test security_tests --all-features`
7. Run the full baseline:
   - `make fmt`
   - `make clippy`
   - `make test`

## Severity criteria

- Critical: arbitrary execution, credential exfiltration, strong evasion patterns.
- High: untrusted command/shell execution, remote download and execution.
- Medium: suspicious patterns without direct exploitation evidence.
- Low: potential risks with clear mitigations.

## Expected output

- Findings prioritized by severity.
- Technical evidence (file and line).
- Concrete mitigation recommendation.
