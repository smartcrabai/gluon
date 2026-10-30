# gluon Conventions

Places the CLI edits mechanically, and the conventions the framework assumes. Even when editing by hand, breaking these conventions means the CLI can no longer touch those areas again.

## wiring.rs (DI container)

`src/wiring.rs` is the composition root. The CLI mechanically edits between the following markers:

```rust
#[allow(unused_mut)]
pub fn build_container(builder: ContainerBuilder) -> ContainerBuilder {
    let mut builder = builder;
    // <gluon:binds>
    // <gluon:bind:usecase:list_users>
    builder = builder.bind::<dyn crate::usecases::list_users::ListUsers, _>(
        |_| std::sync::Arc::new(crate::usecases::list_users::ListUsersImpl::new())
    );
    // </gluon:bind:usecase:list_users>
    // <gluon:bind:domain:user>
    builder = builder.bind::<dyn crate::domain::user::UserRepository, _>(
        |c| std::sync::Arc::new(
            crate::infrastructure::persistence::user_repository::PostgresUserRepository::new(
                c.resolve::<sqlx::PgPool>()
            )
        )
    );
    // </gluon:bind:domain:user>
    // </gluon:binds>
    builder
}
```

- Outer marker: `// <gluon:binds>` / `// </gluon:binds>`
- Inner markers: `// <gluon:bind:<key>>` / `// </gluon:bind:<key>>`
- Keys use the `usecase:<snake>` / `domain:<snake>` form
- Blocks are inserted sorted by key

If you edit manually, keep the markers in place. The CLI extracts inner markers with a regex, so if leading indentation or comment syntax is broken it can no longer pick them up.

## `mod.rs` markers

`src/{domain,usecases,dto,infrastructure/{persistence,mocks}}/mod.rs` contain the following markers:

```rust
// <gluon:domain-mods>
pub mod user;
// </gluon:domain-mods>
```

| File | Marker name |
|---|---|
| `src/domain/mod.rs` | `domain-mods` |
| `src/usecases/mod.rs` | `usecase-mods` |
| `src/dto/mod.rs` | `dto-mods` |
| `src/infrastructure/persistence/mod.rs` | `persistence-mods` |
| `src/infrastructure/mocks/mod.rs` | `mock-mods` |

`gluon g` inserts `pub mod <name>;` here sorted; `gluon d` likewise removes entries from between the same markers.

## Routing

The `app/` directory is scanned by `gluon-build`'s build.rs, which generates a Router into `OUT_DIR/__gluon_app.rs`. The `gluon::app!()` macro in `src/main.rs` includes it and exposes the `__gluon_router()` function.

File name conventions:

- `page.rs` = controller with a View (renders the sibling-directory `page.tsx`)
- `route.rs` = controller endpoint; an optional sibling `route.tsx` supplies TSX fragments for SSE events
- `layout.tsx` = jsxrs layout (wraps child pages)
- `app/_error/<status>.tsx` = HTML error template (422 validation remains JSON)

HTTP methods are expressed as **function names**:

```rust
// app/users/page.rs
pub async fn get(...) -> gluon::Result<View<...>> { ... }
pub async fn post(...) -> gluon::Result<Redirect> { ... }
```

Only `get / post / put / patch / delete` are supported.

Dynamic segments:

| Directory name | URL path | mod name (internal) |
|---|---|---|
| `users/` | `/users` | `users` |
| `[id]/` | `{id}` | `_dyn_id` |
| `[...slug]/` | `{*slug}` | `_catch_slug` |
| `(marketing)/` | (excluded from URL) | `_group_marketing` |

Hyphenated segments (`[user-id]`) are invalid as axum parameter names, so avoid them.

If both `page.rs` and `route.rs` exist for the same URL, **build.rs rejects it with a build error** (to avoid an axum startup panic).

## View<P> and template resolution

```rust
use gluon::prelude::*;

#[derive(serde::Serialize)]
pub struct UsersIndex { users: Vec<UserDto> }

pub async fn get(Inject(list): Inject<dyn ListUsers>) -> gluon::Result<View<UsersIndex>> {
    let output = list.execute(...).await?;
    Ok(View::new(UsersIndex { users: output.users }))
}
```

- `View::new(props)` is the public constructor. `props` must be `Serialize`.
- The generated page wrapper resolves the sibling `page.tsx` and scopes `CURRENT_TEMPLATE` and `CURRENT_LAYOUTS`; route wrappers with `route.tsx` scope `CURRENT_TEMPLATE` for SSE events. Users do not provide template paths.
- Boot applies the `[view]` settings from `gluon.toml` through `CURRENT_RENDER_CONFIG`.
- Ancestor `layout.tsx` files wrap a page from `app/layout.tsx` inward, including layouts inside route-group directories such as `(marketing)`. Layouts receive the same serialized page props; non-object props are kept under `pageProps`. `children` contains the rendered inner HTML as raw HTML.
- The page and inherited layouts contribute to one merged `<Head>`. `HX-Request: true` skips all layouts and the document wrapper, returning only the page fragment, except when `HX-Boosted: true` or `HX-History-Restore-Request: true`; these requests retain all layouts and the document wrapper.
- `app/_error/<status>.tsx` handles `AppError` on HTML-accepting page requests, with `500.tsx` as fallback. Full-document errors use only `app/layout.tsx`; ordinary HTMX fragment errors are layout-free, while boosted and history-restore errors retain the root layout and full document. `route.rs` responses are not wrapped.

## Server-Sent Events

Return `gluon::prelude::sse::Sse` from a route `get` handler. `Sse::new(stream)` enables keep-alive comments by default; call `.keep_alive(KeepAlive::new().interval(...).text(...))` to customize them. Each stream item is `Result<Event, E>`, where events can be built with Axum's `Event` API.

For named HTMX SSE events, call `sse::view_event("query-progress", View::new(props))` from the event-producing stream. It renders the sibling `route.tsx` in fragment mode and places that HTML in the event's `data` field. Use `hx-ext="sse"`, `sse-connect="/route"`, and matching `sse-swap` event names in the page.

The response includes `X-Accel-Buffering: no`; configure any reverse proxy to honor it. The standard gluon CSRF, session, and HTMX middleware do not buffer SSE responses. GET streams are safe methods and do not require a CSRF token.

## Domain vs Table is not 1:1

`gluon g domain` does **not** generate migrations.

- A DDD aggregate boundary is "the unit of consistency", while a table boundary is "the unit of normalization and JOIN strategy". The two are independent design decisions.
- It is normal for one `User` aggregate to span 3 tables: `users` + `user_profiles` + `user_credentials`.
- Auto-generating a migration on `g domain user` would instill the misconception that "domain = table".

When a migration is needed, create it explicitly with `gluon g migration <name>`, leaving the responsibility of assembling 1 aggregate from multiple tables to the user in the Repository implementation.

## Error conventions

- `gluon::Result<T> = std::result::Result<T, gluon::AppError>`
- variants: `NotFound` / `Unauthorized` / `Forbidden` / `Validation(Vec<FieldError>)` / `Conflict(String)` / `BadRequest(String)` / `Internal(Box<dyn Error>)`
- Each status code is mapped automatically from its variant
- The contents of `Internal` are never emitted in the response body (fixed `"internal server error"` + logged via `tracing::error!`)
- Error templates may be added under `app/_error/<status>.tsx`; missing status-specific templates fall back to `500.tsx`. `Validation` remains JSON at 422 and is not rendered through `_error`.
