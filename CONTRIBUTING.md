# Contributing

Thank you for your interest in contributing.

## How to Contribute

1. Open an issue or discussion for non-trivial changes before starting work.
2. Fork the repository and create a topic branch.
3. Keep changes focused and include tests or documentation updates when appropriate.
4. Run the repository's formatter, linter, and test commands before opening a pull request.
5. Open a pull request with a clear summary and any relevant context.

## Testing

Run `cargo test --workspace --all-features --locked` and
`node --test scripts/check-csrf-runners.test.cjs`.
The PostgreSQL integration tests require Docker with reachable published ports.

CLI regression tests cover these safety guarantees:

- `destroy` collects approval for all existing target files before changing any
  file or registration. A refusal, blank response, or EOF cancels the whole
  operation, including when an earlier target was approved. This does not
  provide rollback for filesystem errors during an approved deletion.
- A failed database migration preserves existing data and migration history,
  rolls back its DDL/DML, and can be retried after correcting its SQL.

## Pull Request Guidelines

- Keep pull requests small and reviewable.
- Explain why the change is needed, not only what changed.
- Link related issues when applicable.
- Be respectful and constructive in reviews and discussions.

## Security Issues

Please do not disclose security vulnerabilities in public issues. See `SECURITY.md` for reporting instructions.
