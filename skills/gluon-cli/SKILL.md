---
name: gluon-cli
description: gluon (Rails-like Rust web framework on Axum + jsxrs) の CLI バイナリ `gluon` を使ってアプリケーションをスキャフォールド・更新・起動する手順集。`gluon new`、`gluon g/generate`、`gluon d/destroy`、`gluon db {create,drop,migrate,rollback,prepare,seed}`、`gluon dev`、`gluon build`、`gluon run`、`gluon routes` のいずれかを呼ぶ場面・gluon プロジェクトの構造を生成/変更する場面・gluon の DI コンテナ (`src/wiring.rs`) や `app/` ルーティングを触る場面で発動する。trigger 語: "gluon new", "gluon g controller / usecase / domain / dto / migration / resource", "gluon d ...", "gluon dev", "gluon routes", "wiring.rs", "page.rs / page.tsx", `__gluon_router`。
---

# gluon CLI

gluon は jsxrs + Axum をベースにした Rails ライク Rust web フレームワーク。CLI バイナリ `gluon` でアプリケーションをスキャフォールドし、コントローラ / UseCase / Domain / DTO / migration を生成/削除し、開発サーバを起動する。

このスキルは「gluon CLI を使ってプロジェクトを操作するときの正しい手順と落とし穴」を網羅する。具体的な手順は目的別の reference ファイルに分けてある — 必要なものだけ読めばよい。

## CLI バイナリ

このリポジトリでは `cargo build --bin gluon` でビルドした `target/debug/gluon` を使う。グローバル install は `cargo install --path crates/gluon-cli`。

## サブコマンド早見表

```text
gluon new <name> [--no-git] [--no-install]
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
gluon dev          # notify watch + cargo run の再起動
gluon build        # cargo build --release
gluon run          [--release]
gluon routes       # app/ をスキャンして登録ルートを表示
```

短縮形: `g` = `generate`, `d` = `destroy` (Rails 流)。

## 必ず知っておくこと

- **`gluon new` 直後の `Cargo.toml` は `path = "../gluon/crates/..."` の暫定値**。crates.io 公開前なので絶対パスや実体に合わせて書き換える必要がある。詳細は [`references/workflows.md`](references/workflows.md) の Hello world セクション。
- **生成された name / route / field type は CLI 側で厳格に validate される**。`../`、`;}{`、非 ASCII は弾かれる。仕様は [`references/validation.md`](references/validation.md)。
- **wiring.rs と `<layer>/mod.rs` はマーカーコメント間が機械編集される領域**。手動編集する場合もマーカーは残すこと。詳細は [`references/conventions.md`](references/conventions.md)。
- **`gluon g domain` は migration を生成しない**。集約境界とテーブル境界は独立した設計判断。理由は [`references/conventions.md`](references/conventions.md) の "Domain と Table" 節。
- **`zsh` で `[id]` は glob 展開される**。`gluon g controller 'users/[id]'` のようにシングルクォート必須。

## reference 一覧

- [`references/commands.md`](references/commands.md) — 各サブコマンド (`new` / `g` / `d` / `db` / `dev` / `build` / `run` / `routes`) の引数・生成物・挙動の詳細。
- [`references/conventions.md`](references/conventions.md) — wiring.rs / `mod.rs` のマーカー方式、`app/` のルーティング規約、`View<P>` の template 自動注入、Domain と Table の独立性。
- [`references/validation.md`](references/validation.md) — CLI 入力 (route / identifier / field type) の検証ルールと reject 例。
- [`references/environment.md`](references/environment.md) — `Boot::run()` が読む環境変数(`DATABASE_URL`、`GLUON_BIND`、`GLUON_TELEMETRY_DISABLED`、`GLUON_INSECURE_COOKIE`、`OTEL_*`、`SECRET_KEY_BASE`)。
- [`references/workflows.md`](references/workflows.md) — 典型シナリオ:Hello world、Users CRUD、destroy で巻き戻し、CLI 自体の修正 → 検証ループ。
- [`references/limitations.md`](references/limitations.md) — MVP の既知制約(path 依存、HTMX fragment 未伝搬、`g resource` の GET only、`db seed` 未実装 ほか)。
- [`references/testing.md`](references/testing.md) — `gluon::testing::TestClient`、`#[gluon::gluon_test]` 属性マクロ、テスト雛形の置き場。
