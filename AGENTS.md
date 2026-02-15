# AGENTS.md

Operational instructions for agents and LLMs in this repository.

## Objective

Keep FNPM stable, secure, and reproducible while accelerating development work.

## Mandatory Workflow

1. Read `README.md` and `llms.txt` before large changes.
2. If behavior changes, add or update tests.
3. Run local validations before finishing tasks:
   - `make fmt`
   - `make clippy`
   - `make test`
4. Document user-facing changes in `README.md` or `docs/` when applicable.

## Technical Conventions

- Primary language: Rust.
- Reference commands: defined in `Makefile`.
- Avoid adding new dependencies without a clear impact justification.
- Maintain compatibility with macOS, Linux, and Windows.

## Local Skills

- `skills/fnpm-dev/SKILL.md`: standard development and validation workflow.
- `skills/fnpm-security/SKILL.md`: security audit workflow and blocking criteria.

## Local Commands

- `commands/dev-check.md`: checklist for standard changes.
- `commands/security-check.md`: security review checklist.
- `commands/release-smoke.md`: pre-release smoke tests.

## Definition of done

- Builds without errors.
- Relevant tests pass.
- Existing CLI commands remain functional.
- Changes are documented if they affect public usage.
