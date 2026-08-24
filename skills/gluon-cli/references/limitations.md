# Known Limitations and Pitfalls

Works as an MVP, but these spots need attention. Until the framework provides official support, users need workarounds.

## Templates / Generation

1. **`Cargo.toml` after `gluon new` has the provisional value `path = "../gluon/crates/gluon{,-build}"`**
   Before publishing to crates.io, you must rewrite it to real paths or absolute paths right after generation. See section A of [`workflows.md`](workflows.md) for a rewrite example.

2. **`gluon g resource` only generates a GET handler**
   Add POST / PUT / DELETE functions by hand. The `api/<name>/route.rs` side is also `get`-only.

3. **`gluon g domain` does not generate a migration**
   Intentional: aggregate boundaries and table boundaries are independent. If a migration is needed, call `gluon g migration` separately. Details in the "Domain and Table" section of [`conventions.md`](conventions.md).

4. **Template fixes require rebuilding the CLI**
   The `.j2` files under `crates/gluon-cli/templates/` are baked in via `rust-embed`.

5. **Automatic CRUD for domain repositories supports only PostgreSQL-compatible types**
   Supported types are `bool`, `String`, `i8/i16/i32/i64`, `u8/u16/u32/u64/usize`, `f32/f64`, `Vec<u8>`, `uuid::Uuid`, generated value objects, and their `Option<T>` wrappers. Other types such as `Vec<u32>` can be used for entities, but the repository method becomes `todo!()`.

## CLI

6. **`gluon d ...` has no `--yes` flag**
    Use `yes | gluon d ...` as a substitute for non-interactive mode. A pipe is required when using it in CI.

7. **Migration name timestamps have second-level granularity**
   Generating same-named migrations consecutively within the same second triggers an overwrite-prevention error. Wait one second or use a different name.

## DI / Session

8. **Direct calls to `Container::resolve` panic on unbound dependencies**
   The HTTP extractor's `Inject<T>` safely returns a 500 instead. Direct `resolve` inside the composition root is intended for fail-fast of required bindings.

9. **Sessions fall back to `MemoryStore` when `DATABASE_URL` is unset**
   A development fallback; it is lost on process restart. In production and horizontal scaling, set `DATABASE_URL` and use the PostgreSQL session store.
