# CLI Input Validation Rules

`gluon g` / `gluon d` strictly validate user input (`route` / `name` / `--field NAME:TYPE`). This is to prevent (1) arbitrary file writes (`../etc/passwd`), (2) arbitrary code injection into generated Rust sources, and (3) injection into generated SQL.

## Route validation

For `route` accepted by `gluon g controller <route>` / `gluon d controller <route>`, each slash-separated segment must be one of the following:

- Normal segment: `[A-Za-z0-9_-]+`
- Dynamic segment: `[<inner>]` where `inner` matches `[A-Za-z0-9_-]+`
- Catch-all: `[...<inner>]` where `inner` matches `[A-Za-z0-9_-]+`
- Route group: `(<inner>)` where `inner` matches `[A-Za-z0-9_-]+`

In addition, `inner == "." || inner == ".."` is rejected.

Rejection examples:

```text
$ gluon g controller '../../etc/passwd'
error: invalid route segment: ..

$ gluon g controller 'users/[id'
error: invalid route segment: [id
```

## Identifier (name) validation

For `name` in `gluon g {usecase,domain,dto,migration,resource} <name>`:

- Empty string is not allowed
- First character: ASCII letter (`A-Za-z`) or `_`
- Remaining characters: ASCII alphanumeric (`A-Za-z0-9`) or `_`

Rejection examples:

```text
$ gluon g usecase 'foo;}; fn bad()'
error: invalid usecase name: foo;}; fn bad() (only letters, digits and underscore are allowed)

$ gluon g domain 1user
error: invalid domain name: 1user (must start with a letter or underscore)
```

## Field type validation

For `NAME` and `TYPE` in `gluon g domain <name> --field NAME:TYPE`:

- `NAME` follows the identifier rules (above)
- `TYPE` allows only `[A-Za-z0-9_<>,: ']`

Rejection examples:

```text
$ gluon g domain user --field 'id:String;} fn evil'
error: invalid field type: String;} fn evil (contains disallowed character)
```

Allowing `<>,:` lets you write types such as `Option<UserId>`, `Vec<Tag>`, `std::sync::Arc<T>`, and `&'static str`, while characters that carry Rust syntax elements (`;` / `{` / `}` / `(` / `)` / `=` and so on) are rejected.

## Destroy migration name matching

`gluon d migration <name>` requires an exact filename match using a rule equivalent to the following regex:

```
^[0-9]{14}_<snake>\.(up|down)\.sql$
```

Rejection example:

```text
$ gluon d migration users
error: no migration matched name: users
# 20260620120000_add_users.up.sql and 20260620120000_create_users.up.sql are
# NOT swept up by the `users` argument
```

Since exact matching is assumed, invoke the command with **the same name shape used at generation time**, e.g. `gluon d migration create_users`.

## Behavior on failure

On validation failure, the CLI prints `error: ...` to stderr and exits with code 1. **It never generates files partially and then stops midway** (validation runs first).
