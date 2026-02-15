# SKILL: fnpm-dev

## When to use this skill

Use it for general FNPM development tasks: features, fixes, refactors, and maintenance.

## Minimum context to load

1. `README.md`
2. `llms.txt`
3. Files for the affected module in `src/`
4. Related tests in `tests/`

## Recommended sequence

1. Identify the exact scope of the change.
2. Make minimal changes consistent with the existing style.
3. Add or update tests if behavior changes.
4. Run:
   - `make fmt`
   - `make clippy`
   - `make test`
5. Report results and modified files.

## Guardrails

- Do not break CLI compatibility without documentation.
- Do not introduce broad, unrelated changes.
- If environment issues appear, report them clearly.

## Expected output

- Short list of changed files.
- Validation results.
- Risks or pending items (if any).
