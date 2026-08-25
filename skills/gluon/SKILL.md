---
name: gluon
description: Procedures for scaffolding and developing applications with the gluon CLI.
---

# gluon application

This project is a gluon application. Use the `gluon` CLI for scaffolding and code generation; keep generated marker regions intact.

## CLI

```text
gluon new <name> [--no-git] [--no-install] [--claude] [--agents]
gluon generate (g) controller <route> [--api]
gluon generate (g) resource <name>
gluon generate (g) usecase <name>
gluon generate (g) domain <name> [--field NAME:TYPE]*
gluon generate (g) dto <name>
gluon generate (g) migration <name>
gluon destroy (d) <kind> <name>
gluon db <create|drop|migrate|rollback|prepare|seed>
gluon dev | gluon build | gluon run [--release] | gluon routes
```

## Conventions

- `src/wiring.rs` and each generated layer `mod.rs` contain machine-edited marker regions. Preserve the markers.
- `gluon g domain` does not create a migration; create one separately with `gluon g migration`.
- Generated `Cargo.toml` path dependencies point at the local gluon workspace and may need adjustment outside this repository.
- In zsh, quote routes containing bracket segments, for example `gluon g controller 'users/[id]'`.

## Verification

After generation, run `gluon routes` and `cargo check`. For runtime checks, use `GLUON_INSECURE_COOKIE=1 cargo run` locally.
