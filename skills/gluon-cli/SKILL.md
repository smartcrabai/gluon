---
name: gluon-cli
description: Procedures for scaffolding, updating, and running applications with `gluon`, the CLI binary of gluon (a Rails-like Rust web framework on Axum + jsxrs). Activates whenever you invoke `gluon new`, `gluon g/generate`, `gluon d/destroy`, `gluon db {create,drop,migrate,rollback,prepare,seed}`, `gluon dev`, `gluon build`, `gluon run`, or `gluon routes`; when generating or changing the structure of a gluon project; or when touching the gluon DI container (`src/wiring.rs`) or `app/` routing. Trigger words: "gluon new", "gluon g controller / usecase / domain / dto / migration / resource", "gluon d ...", "gluon dev", "gluon routes", "wiring.rs", "page.rs / page.tsx", `__gluon_router`.
---

# gluon CLI

gluon is a Rails-like Rust web framework built on jsxrs + Axum. The `gluon` CLI binary scaffolds applications, generates/deletes controllers / UseCases / Domains / DTOs / migrations, and starts the dev server.

This skill covers "the correct procedures and pitfalls when operating a project with the gluon CLI." Concrete procedures are split into per-purpose reference files - read only what you need.

## CLI Binary

In this repository, use `target/debug/gluon` built with `cargo build --bin gluon`. For a global install: `cargo install --path crates/gluon-cli`.

## Subcommand Quick Reference

```text
gluon new <name> [--no-git] [--no-install] [--claude] [--agents]
gluon generate (g) <kind>
  controller <route> [--api]
  resource   <name>
  usecase    <name>
  domain     <name> [--field NAME:TYPE]*
  dto        <name>
  migration  <name>
gluon destroy (d) <kind>
  controller <route>
  resource <name> | usecase <name> | domain <name> | dto <name> | migration <name>
gluon db <op>      # create / drop / migrate / rollback / prepare / seed
gluon dev          # notify watch + cargo run restarts
gluon build        # cargo build --release
gluon run          [--release]
gluon routes       # scans app/ and lists registered routes
```

Short forms: `g` = `generate`, `d` = `destroy` (Rails style).

## Must-Know Facts

- **The `Cargo.toml` right after `gluon new` uses GitHub dependencies pinned to the exact framework revision used to build the CLI, or to the matching `v<version>` tag when Git metadata is unavailable**. For local CLI/framework development, rewrite them to local paths as described in [`references/workflows.md`](references/workflows.md).
- **Generated names / routes / field types are strictly validated by the CLI**. `../`, `;}{`, and non-ASCII are rejected. Spec: [`references/validation.md`](references/validation.md).
- **wiring.rs and `<layer>/mod.rs` are machine-edited regions between marker comments**. Keep the markers intact even when editing manually. Details: [`references/conventions.md`](references/conventions.md).
- **`gluon g domain` does not generate a migration**. Aggregate boundaries and table boundaries are independent design decisions. Rationale: the "Domain and Table" section in [`references/conventions.md`](references/conventions.md).
- **In `zsh`, `[id]` is glob-expanded**. Single quotes are required, e.g. `gluon g controller 'users/[id]'`.

## Reference Index

- [`references/commands.md`](references/commands.md) - details on arguments, generated artifacts, and behavior of each subcommand (`new` / `g` / `d` / `db` / `dev` / `build` / `run` / `routes`).
- [`references/conventions.md`](references/conventions.md) - wiring.rs / `mod.rs` marker scheme, `app/` routing conventions, automatic template injection for `View<P>`, independence of Domain and Table.
- [`references/validation.md`](references/validation.md) - validation rules for CLI inputs (route / identifier / field type) with rejection examples.
- [`references/environment.md`](references/environment.md) - environment variables read by `Boot::run()` (`DATABASE_URL`, `GLUON_BIND`, `GLUON_TELEMETRY_DISABLED`, `GLUON_INSECURE_COOKIE`, `OTEL_*`, `SECRET_KEY_BASE`).
- [`references/workflows.md`](references/workflows.md) - typical scenarios: Hello world, Users CRUD, rollback via destroy, editing the CLI itself -> verification loop.
- [`references/limitations.md`](references/limitations.md) - known MVP constraints (GET-only `g resource`, interactive destroy confirmation, etc.).
- [`references/testing.md`](references/testing.md) - `gluon::testing::TestClient`, the `#[gluon::gluon_test]` attribute macro, where to place test templates.
