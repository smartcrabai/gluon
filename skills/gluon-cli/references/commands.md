# gluon CLI コマンド詳細

各サブコマンドの引数・生成物・典型呼び出しをまとめる。短縮形 `g` / `d` は Rails 流。

## `gluon new <name>`

新規 gluon アプリを `<name>/` ディレクトリに生成する。

```bash
gluon new myapp                # git init + cargo fetch を実行
gluon new myapp --no-git       # git init を抑制
gluon new myapp --no-install   # cargo fetch を抑制
```

生成されるツリー:

```
myapp/
├── Cargo.toml         # gluon, gluon-build, axum, sqlx, tower-sessions, ts-rs を依存
├── gluon.toml         # アプリ設定
├── build.rs           # gluon_build::run() を呼ぶ
├── .env.example       # DATABASE_URL / SECRET_KEY_BASE / OTEL_ENABLED
├── app/               # Presentation: page.rs / page.tsx, route.rs, layout.tsx
│   ├── page.rs        # GET / handler (View 付き)
│   ├── page.tsx       # 同階層 View
│   ├── _error/{404,500}.tsx
│   └── components/csrf_token.tsx
├── migrations/        # sqlx 用 SQL ファイル
├── public/            # 静的アセット (/public 配下にマウント)
└── src/
    ├── main.rs        # Boot::new().with_container(...).with_router(__gluon_router()).run()
    ├── wiring.rs      # DI コンテナの composition root (マーカーコメント方式)
    ├── domain/        # 1 domain = 1 ディレクトリ
    ├── usecases/
    ├── infrastructure/{persistence,mocks}/
    └── dto/
```

`Cargo.toml` の `gluon` / `gluon-build` 依存は `path = "../gluon/crates/gluon{,-build}"` という暫定値。詳細と書き換え例は [`workflows.md`](workflows.md) を参照。

## `gluon g controller <route> [--api]`

`app/<route>/page.rs` と同階層 `page.tsx` を生成する。`--api` は `route.rs` のみ(View 無し)。

```bash
gluon g controller users                 # GET /users
gluon g controller 'users/[id]'          # GET /users/:id (dynamic segment)
gluon g controller 'users/[id]/edit'
gluon g controller 'api/health' --api    # route.rs のみ
```

zsh では `[id]` が glob 展開されるので **シングルクォート必須**。

## `gluon g resource <name>`

REST 一括生成。`<name>` は単数形・複数形どちらでもユーザー責任(英語 inflection は CLI に無い)。

```bash
gluon g resource posts
# → app/posts/{page,new/page,[id]/page,[id]/edit/page}.{rs,tsx}
# → app/api/posts/{route.rs, [id]/route.rs}
```

注: 各 `page.rs` には `get` ハンドラのみ生成される。POST / PUT / DELETE が必要なら手で関数を追加する。

## `gluon g usecase <name>`

`src/usecases/<name>.rs` に trait + impl + `Input` / `Output` / `Error` を生成。さらに:
- `src/usecases/mod.rs` のマーカー内に `pub mod <name>;` を sort 済み挿入
- `src/wiring.rs` のマーカー内に `builder = builder.bind::<dyn ..., _>(...);` 行を挿入

```bash
gluon g usecase list_users
```

## `gluon g domain <name> [--field NAME:TYPE]*`

1 domain = 1 ディレクトリ (entity / value_objects / repository / error)、`Postgres<Name>Repository` (sqlx 前提、中身 `todo!()`)、mockall ベース mock、`wiring.rs` への bind 行を一括生成。

```bash
gluon g domain user --field name:UserName --field email:Email --field age:u32
```

`Type` 部分の解釈:
- プリミティブ (`u32`, `String`, `bool`) はそのまま
- `PascalCase` で他の domain にない型は value object として newtype 生成
- `Option<T>`, `Vec<T>` も OK(shell escape 注意)

**migration は同時生成しない**。理由は [`conventions.md`](conventions.md) の "Domain と Table" 節。

## `gluon g dto <name>`

`src/dto/<name>.rs` を生成 + `src/dto/mod.rs` にマーカー挿入。

## `gluon g migration <name>`

`migrations/<UTC YYYYMMDDHHMMSS>_<name>.{up,down}.sql` を生成。

```bash
gluon g migration create_users
```

## `gluon d <kind> <name>` / `gluon destroy <kind> <name>`

generate の対称操作。
- 規約パスのファイルを `[y/N]` 確認付きで削除
- `wiring.rs` の bind block をマーカーコメント方式で確実に削除
- 該当 `mod.rs` の `pub mod <name>;` 行を削除
- migration は **timestamp + 完全一致**で削除する(`users` 指定で `add_users` を巻き込まない)

```bash
gluon d controller users
gluon d usecase list_users
gluon d domain user
gluon d resource posts        # api/ 側も含めて消える
gluon d migration create_users
```

`yes | gluon d ...` で非対話モードに(`--yes` フラグは現状なし)。

## `gluon db <op>`

`sqlx-cli` ラッパー。`sqlx-cli` を `cargo install sqlx-cli --no-default-features --features rustls,postgres` でインストール済みである必要。

```bash
gluon db create     # database create
gluon db drop       # database drop -y
gluon db migrate    # migrate run
gluon db rollback   # migrate revert
gluon db prepare    # sqlx prepare
gluon db seed       # 未実装 (bail)
```

## `gluon dev`

`notify` でファイル変更を監視し `cargo run` を再起動する。watch 対象は `app/`, `src/`, `migrations/`。フィルタは粗く、エディタ一時ファイルでも再起動が走ることがある(詳細は [`limitations.md`](limitations.md))。

## `gluon build` / `gluon run`

`cargo build --release` / `cargo run [--release]` の薄いラッパー。

## `gluon routes`

`app/` をスキャンして登録ルートを一覧表示する。`gluon-build` が build 時に行う auto-router 構築の dry-run。

```text
GET     /                              app/page.rs::get
GET     /api/health                    app/api/health/route.rs::get
GET     /users                         app/users/page.rs::get
GET     /users/:id                     app/users/[id]/page.rs::get
```
