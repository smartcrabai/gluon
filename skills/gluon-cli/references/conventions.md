# gluon 規約

CLI が機械的に編集する場所と、framework が前提とする規約。手動編集する場合もこれらの規約を守らないと CLI が再度触れなくなる。

## wiring.rs (DI コンテナ)

`src/wiring.rs` は composition root。CLI は次のマーカー間を機械的に編集する:

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

- 外側マーカー: `// <gluon:binds>` / `// </gluon:binds>`
- 内側マーカー: `// <gluon:bind:<key>>` / `// </gluon:bind:<key>>`
- key は `usecase:<snake>` / `domain:<snake>` 形式
- ブロックは key で sort 済み挿入

手動編集する場合もマーカーを残すこと。CLI は regex で内側マーカーを抽出するため、行頭インデント + コメント文法が崩れると拾えなくなる。

## `mod.rs` マーカー

`src/{domain,usecases,dto,infrastructure/{persistence,mocks}}/mod.rs` には次のマーカーがある:

```rust
// <gluon:domain-mods>
pub mod user;
// </gluon:domain-mods>
```

| ファイル | マーカー名 |
|---|---|
| `src/domain/mod.rs` | `domain-mods` |
| `src/usecases/mod.rs` | `usecase-mods` |
| `src/dto/mod.rs` | `dto-mods` |
| `src/infrastructure/persistence/mod.rs` | `persistence-mods` |
| `src/infrastructure/mocks/mod.rs` | `mock-mods` |

`gluon g` がここに `pub mod <name>;` を sort 済み挿入、`gluon d` が同じくマーカー間から削除。

## Routing

`app/` 配下を `gluon-build` の build.rs が scan して `OUT_DIR/__gluon_app.rs` に Router を生成する。`src/main.rs` の `gluon::app!()` マクロが include して `__gluon_router()` 関数を expose する。

ファイル名規約:

- `page.rs` = View 付き controller(同階層 `page.tsx` を render)
- `route.rs` = View 無し controller(API エンドポイント)
- `layout.tsx` = jsxrs レイアウト(子ページをラップ)
- `_error/{404,401,403,422,500}.tsx` = エラー専用 View(予約名、routing 対象外)

HTTP メソッドは **関数名** で表現する:

```rust
// app/users/page.rs
pub async fn get(...) -> gluon::Result<View<...>> { ... }
pub async fn post(...) -> gluon::Result<Redirect> { ... }
```

サポートは `get / post / put / patch / delete` のみ。

動的セグメント:

| ディレクトリ名 | URL パス | mod 名(内部) |
|---|---|---|
| `users/` | `/users` | `users` |
| `[id]/` | `:id` | `_dyn_id` |
| `[...slug]/` | `*slug` | `_catch_slug` |
| `(marketing)/` | (URL から除外) | `_group_marketing` |

ハイフン入り segment (`[user-id]`) は axum のパラメータ名として無効なので避けること。

同じ URL に `page.rs` と `route.rs` が両方ある場合は **build.rs が build error で reject** する(axum の起動時 panic 回避)。

## View<P> と template 解決

```rust
use gluon::prelude::*;

#[derive(serde::Serialize)]
pub struct UsersIndex { users: Vec<UserDto> }

pub async fn get(Inject(list): Inject<dyn ListUsers>) -> gluon::Result<View<UsersIndex>> {
    let output = list.execute(...).await?;
    Ok(View::new(UsersIndex { users: output.users }))
}
```

- `View::new(props)` のみが公開 API。`props` は `Serialize`。
- **template path はユーザーが指定しない**。`gluon-build` が生成する handler wrapper が `CURRENT_TEMPLATE` task-local で同階層 `page.tsx` の絶対パスを注入し、`View::<P>::into_response` がそれを読む。
- 任意ファイル読み込みを防ぐため、`View::with_template_path` のような pub API は意図的に提供していない(security)。

## Domain と Table は 1:1 ではない

`gluon g domain` は **migration を生成しない**。

- DDD の集約境界は「整合性を守る単位」、テーブル境界は「正規化と JOIN 戦略の単位」。両者は独立した設計判断。
- `User` aggregate が `users` + `user_profiles` + `user_credentials` の 3 テーブルにまたがるケースは普通。
- CLI が `g domain user` で migration を自動生成すると「domain = table」という誤解を植え付ける。

migration が必要になったら明示的に `gluon g migration <name>` で作成し、Repository 実装で複数テーブルから 1 aggregate を組み立てる責務をユーザーに残す。

## Error 規約

- `gluon::Result<T> = std::result::Result<T, gluon::AppError>`
- variants: `NotFound` / `Unauthorized` / `Forbidden` / `Validation(Vec<FieldError>)` / `Conflict(String)` / `BadRequest(String)` / `Internal(Box<dyn Error>)`
- 各 status code は variant から自動マップ
- `Internal` の中身はレスポンス body に出さない(`"internal server error"` 固定 + `tracing::error!` でログ)
- エラー専用 view: `app/_error/{404,401,403,422,500}.tsx` を置くと該当 status で render される
