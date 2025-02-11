# FNPM

A unified package manager interface for npm, yarn, and pnpm.

## Installation

To install fnpm, run:

```bash
make install
```

## Development

### Testing

To test the CLI functionality, you can use the `make test` command with the `cmd` parameter. For example, to see all available commands:

```bash
make test cmd="--help"
```

This will run fnpm with the specified command in the test environment, which contains a sample Node.js project for testing purposes.