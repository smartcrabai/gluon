# Typical Workflows

Step-by-step procedures for each scenario. For per-command specifications, see [`commands.md`](commands.md).

## A. New app up to Hello world

```bash
gluon new myapp --no-install
cd myapp

GLUON_TELEMETRY_DISABLED=1 GLUON_INSECURE_COOKIE=1 cargo run
# In another shell:
curl http://localhost:3000/    # 200 + <h1>Hello, gluon</h1>
```

## B. Assembling Users CRUD

```bash
# Domain and Repository
gluon g domain user --field name:UserName --field email:Email

# Migration is a separate command (domain and table are not 1:1)
gluon g migration create_users
# Write the CREATE TABLE in migrations/<ts>_create_users.up.sql

DATABASE_URL=postgres://localhost/myapp_dev gluon db create
DATABASE_URL=postgres://localhost/myapp_dev gluon db migrate

# UseCase
gluon g usecase list_users
# Implement the todo!() in execute of src/usecases/list_users.rs
# Receive the repository via Inject<dyn UserRepository> and fill the result into Output

# Controller (REST all-in-one)
gluon g resource users
# In app/users/page.rs, call list_users.execute() from `get` and return Output via View::new

# Verify
gluon routes
GLUON_INSECURE_COOKIE=1 cargo run
```

## C. Rolling back with destroy

```bash
gluon d resource users       # deletes app/users and app/api/users with confirmation
gluon d usecase list_users
gluon d domain user
gluon d migration create_users
```

Non-interactive mode:

```bash
yes | gluon d domain user
```

## D. Modifying the CLI itself -> verification loop

After editing gluon-cli / gluon / gluon-build / gluon-macros:

```bash
# Static checks
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --check
cargo test --workspace --all-features

# Rebuild the CLI binary
cargo build --bin gluon
# (To update a global install: cargo install --path crates/gluon-cli --force)

# E2E
cd /tmp && rm -rf myapp
/Users/takumi/apps/gluon/target/debug/gluon new myapp --no-git --no-install
cd myapp
# Use local framework crates while testing unpublished CLI/framework changes.
sed -i.bak -E \
  -e 's#gluon-build = \{ git = "https://github.com/smartcrabai/gluon", (tag|rev) = "[^"]+" \}#gluon-build = { path = "/Users/takumi/apps/gluon/crates/gluon-build" }#' \
  -e 's#gluon = \{ git = "https://github.com/smartcrabai/gluon", (tag|rev) = "[^"]+" \}#gluon = { path = "/Users/takumi/apps/gluon/crates/gluon" }#' \
  Cargo.toml && rm Cargo.toml.bak

# Startup + HTTP check
GLUON_TELEMETRY_DISABLED=1 GLUON_INSECURE_COOKIE=1 GLUON_BIND=127.0.0.1:13580 cargo run &
SRV=$!
sleep 4
curl -sS -w "STATUS=%{http_code}\n" http://127.0.0.1:13580/
curl -sS -w "STATUS=%{http_code}\n" -X POST http://127.0.0.1:13580/  # no CSRF -> 403
kill $SRV
```

## E. Modifying templates

The generated-file scaffolds live under `crates/gluon-cli/templates/` as minijinja templates (`.j2`) and are baked into the CLI binary via `rust-embed`. After changing a template, re-bake it with `cargo build --bin gluon`.
