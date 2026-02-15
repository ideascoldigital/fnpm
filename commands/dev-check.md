# Command: dev-check

Validate a standard development change in FNPM.

## Steps

1. Review changed files and scope.
2. Run:
   ```bash
   make fmt
   make clippy
   make test
   ```
3. If anything fails, fix it and rerun.

## Expected result

- Correct formatting.
- No clippy warnings treated as errors.
- Test suite passing.
