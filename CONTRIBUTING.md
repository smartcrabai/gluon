# Contributing

Thank you for your interest in contributing.

## How to Contribute

1. Open an issue or discussion for non-trivial changes before starting work.
2. Fork the repository and create a topic branch.
3. Keep changes focused and include tests or documentation updates when appropriate.
4. Run the repository's formatter, linter, and test commands before opening a pull request.
5. Open a pull request with a clear summary and any relevant context.

## Testing

Run `cargo test --workspace --all-features --locked` and `node --test scripts/check-csrf-runners.test.cjs` as the standard test commands. Postgres tests require Docker with reachable ports.

The generator checks all new output paths before writing and refuses existing outputs or parent symlinks without changing files, modules, or wiring; it does not guarantee rollback for later I/O failures or concurrent creators.

HTTP smoke tests verify route-specific fixtures and health JSON. Session tests cover concurrent requests using an existing token and body, database snapshot replacement and isolation, malformed JSON, backend errors, and last-save-wins behavior; they do not provide atomic read-modify-write. See [bounded concurrent Quint model](crates/gluon/tests/specs/VERIFICATION.md) for details.

## Pull Request Guidelines

- Keep pull requests small and reviewable.
- Explain why the change is needed, not only what changed.
- Link related issues when applicable.
- Be respectful and constructive in reviews and discussions.

## Security Issues

Please do not disclose security vulnerabilities in public issues. See `SECURITY.md` for reporting instructions.
