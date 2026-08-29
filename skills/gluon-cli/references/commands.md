# gluon CLI Command Reference

Arguments, generated artifacts, and typical invocations for each subcommand. The short forms `g` / `d` follow Rails conventions.

## `gluon new <name>`

Generates a new gluon application in the `<name>/` directory.

```bash
gluon new myapp                         # runs git init + cargo fetch
gluon new myapp --no-git                # skips git init
gluon new myapp --no-install            # skips cargo fetch
gluon new myapp --claude                # adds Claude Code skill and CLAUDE.md
gluon new myapp --agents                # adds agent skill and AGENTS.md
gluon new myapp --claude --agents       # shares the agent skill with Claude Code
```

Generated tree:

```
myapp/
|-- Cargo.toml         # depends on gluon, gluon-build, axum, sqlx, tower-sessions, ts-rs
|-- gluon.toml         # app configuration
|-- build.rs           # calls gluon_build::run()
|-- .env.example       # DATABASE_URL / SECRET_KEY_BASE / OTEL_ENABLED
|-- app/               # Presentation: page.rs / page.tsx, route.rs, layout.tsx
|   |-- page.rs        # GET / handler (with View)
|   |-- page.tsx       # View at the same level
|   |-- _error/{404,500}.tsx
|   `-- components/csrf_token.tsx
|-- migrations/        # SQL files for sqlx
|-- public/            # static assets (mounted under /public)
`-- src/
    |-- main.rs        # Boot::new().with_container(...).with_router(__gluon_router()).run()
    |-- wiring.rs      # DI container composition root (marker comment style)
    |-- domain/        # 1 domain = 1 directory
    |-- usecases/
    |-- infrastructure/{persistence,mocks}/
    `-- dto/
```

The `gluon` / `gluon-build` dependencies in `Cargo.toml` use the GitHub repository and are pinned to the exact framework revision used to build the CLI, or to the matching `v<version>` tag when Git metadata is unavailable. For local CLI/framework development, see [`workflows.md`](workflows.md) for how to rewrite them to local paths.

## `gluon g controller <route> [--api]`

Generates `app/<route>/page.rs` and a `page.tsx` at the same level. With `--api`, only `route.rs` is generated (no View).

```bash
gluon g controller users                 # GET /users
gluon g controller 'users/[id]'          # GET /users/{id} (dynamic segment)
gluon g controller 'users/[id]/edit'
gluon g controller 'api/health' --api    # route.rs only
```

In zsh, `[id]` undergoes glob expansion, so **single quotes are mandatory**.

## `gluon g resource <name>`

Bulk REST generation. Whether `<name>` is singular or plural is up to the user (the CLI has no English inflection logic).

```bash
gluon g resource posts
# -> app/posts/{page,new/page,[id]/page,[id]/edit/page}.{rs,tsx}
# -> app/api/posts/{route.rs, [id]/route.rs}
```

Note: each generated `page.rs` contains only a `get` handler. If you need POST / PUT / DELETE, add the functions manually.

## `gluon g usecase <name>`

Generates trait + impl + `Input` / `Output` / `Error` in `src/usecases/<name>.rs`. Additionally:
- inserts `pub mod <name>;` inside the markers in `src/usecases/mod.rs`, sorted
- inserts a `builder = builder.bind::<dyn ..., _>(...);` line inside the markers in `src/wiring.rs`

```bash
gluon g usecase list_users
```

## `gluon g domain <name> [--field NAME:TYPE]*`

Bulk-generates 1 domain = 1 directory (entity / value_objects / repository / error), an sqlx-based `Postgres<Name>Repository`, a mock repository, and bind lines into `wiring.rs`. The PostgreSQL table name is `<domain name in snake_case>s`. For PostgreSQL-supported scalars, generated value objects, and their `Option<T>` wrappers, CRUD is also implemented. For other types, repository methods are generated as `todo!()`.

```bash
gluon g domain user --field name:UserName --field email:Email --field age:u32
```

Interpretation of the `Type` part:
- primitives (`u32`, `String`, `bool`) are used as-is
- `PascalCase` types not found in another domain are generated as value object newtypes
- `Option<T>`, `Vec<T>` can also be used as entity types (mind shell escaping). See [`limitations.md`](limitations.md) for types supported by auto CRUD

**No migration is generated at the same time**. See the "Domain and Table" section of [`conventions.md`](conventions.md) for the reason.

## `gluon g dto <name>`

Generates `src/dto/<name>.rs` + inserts markers into `src/dto/mod.rs`.

## `gluon g migration <name>`

Generates `migrations/<UTC YYYYMMDDHHMMSS>_<name>.{up,down}.sql`.

```bash
gluon g migration create_users
```

## `gluon d <kind> <name>` / `gluon destroy <kind> <name>`

The inverse of generate.
- deletes files at conventional paths after a `[y/N]` confirmation
- removes the bind block from `wiring.rs` reliably via marker comments
- removes the `pub mod <name>;` line from the relevant `mod.rs`
- migrations are deleted by **timestamp + exact match** (`users` will not sweep up `add_users`)

```bash
gluon d controller users
gluon d usecase list_users
gluon d domain user
gluon d resource posts        # removes the api/ side as well
gluon d migration create_users
```

Use `yes | gluon d ...` for non-interactive mode (no `--yes` flag for now).

## `gluon db <op>`

PostgreSQL operations against `DATABASE_URL`. create / drop / migrate / rollback / seed use the built-in SQLx implementation. Only prepare uses `cargo sqlx prepare`, which requires `sqlx-cli`.

```bash
gluon db create     # database create
gluon db drop       # database drop -y
gluon db migrate    # migrate run
gluon db rollback   # migrate revert
gluon db prepare    # sqlx prepare
gluon db seed       # runs db/seeds.sql
```

## `gluon dev`

Watches for file changes with `notify` and restarts `cargo run`. When `GLUON_INSECURE_COOKIE` is unset, it sets it to `1` for local HTTP development. Watch targets are `app/`, `src/`, `migrations/`. TypeScript files, target/, templates, and editor temp files are excluded from restarts.

## `gluon build` / `gluon run`

Thin wrappers around `cargo build --release` / `cargo run [--release]`.

## `gluon routes`

Scans `app/` and lists registered routes. A dry-run of the auto-router construction that `gluon-build` performs at build time.

```text
GET     /                              app/page.rs::get
GET     /api/health                    app/api/health/route.rs::get
GET     /users                         app/users/page.rs::get
GET     /users/{id}                     app/users/[id]/page.rs::get
```
