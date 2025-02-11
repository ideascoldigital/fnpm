# FNPM (F*ck NPM)

A unified package manager interface that helps teams standardize their workflow while allowing developers to use their preferred tool (npm, yarn, or pnpm). FNPM ensures consistent lock files across the team regardless of individual package manager preferences, making it easier to maintain dependencies and avoid conflicts.

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