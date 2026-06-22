# 典型ワークフロー

具体的なシナリオごとの手順。コマンドの個別仕様は [`commands.md`](commands.md) を参照。

## A. 新規アプリで Hello world まで

```bash
gluon new myapp --no-install
cd myapp

# MVP の暫定対応: 生成された Cargo.toml の path 依存を実体に合わせる
sed -i.bak \
  -e 's|path = "../gluon/crates/gluon-build"|path = "/Users/takumi/apps/gluon/crates/gluon-build"|' \
  -e 's|path = "../gluon/crates/gluon"|path = "/Users/takumi/apps/gluon/crates/gluon"|' \
  Cargo.toml && rm Cargo.toml.bak

GLUON_TELEMETRY_DISABLED=1 cargo run
# 別シェルで:
curl http://localhost:3000/    # 200 + <h1>Hello, gluon</h1>
```

## B. Users CRUD を組み立てる

```bash
# Domain と Repository
gluon g domain user --field name:UserName --field email:Email

# Migration は別コマンド(domain と table は 1:1 ではない)
gluon g migration create_users
# migrations/<ts>_create_users.up.sql に CREATE TABLE を書く

DATABASE_URL=postgres://localhost/myapp_dev gluon db create
DATABASE_URL=postgres://localhost/myapp_dev gluon db migrate

# UseCase
gluon g usecase list_users
# src/usecases/list_users.rs の execute の todo!() を実装する
# Repository を Inject<dyn UserRepository> で受け取り、結果を Output に詰める

# Controller (REST 一括)
gluon g resource users
# app/users/page.rs の `get` で list_users.execute() を呼び、Output を View::new で返す

# 確認
gluon routes
cargo run
```

## C. destroy で巻き戻し

```bash
gluon d resource users       # app/users と app/api/users を確認付きで削除
gluon d usecase list_users
gluon d domain user
gluon d migration create_users
```

非対話モード:

```bash
yes | gluon d domain user
```

## D. CLI 自体の修正 → 検証ループ

gluon-cli / gluon / gluon-build / gluon-macros を編集したら:

```bash
# 静的検証
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --check
cargo test --workspace --all-features

# CLI バイナリを再ビルド
cargo build --bin gluon
# (グローバル install を更新するなら: cargo install --path crates/gluon-cli --force)

# E2E
cd /tmp && rm -rf myapp
/Users/takumi/apps/gluon/target/debug/gluon new myapp --no-git --no-install
cd myapp
sed -i.bak -e 's|path = "../gluon/crates/gluon-build"|path = "/Users/takumi/apps/gluon/crates/gluon-build"|' \
           -e 's|path = "../gluon/crates/gluon"|path = "/Users/takumi/apps/gluon/crates/gluon"|' \
           Cargo.toml && rm Cargo.toml.bak

# 起動 + HTTP 確認
GLUON_TELEMETRY_DISABLED=1 GLUON_INSECURE_COOKIE=1 GLUON_BIND=127.0.0.1:13580 cargo run &
SRV=$!
sleep 4
curl -sS -w "STATUS=%{http_code}\n" http://127.0.0.1:13580/
curl -sS -w "STATUS=%{http_code}\n" -X POST http://127.0.0.1:13580/  # CSRF なし → 403
kill $SRV
```

## E. テンプレートを修正したい

生成ファイルの雛形は `crates/gluon-cli/templates/` 配下に minijinja テンプレ (`.j2`) で置かれており、`rust-embed` で CLI バイナリに焼き込まれる。テンプレを変更したら `cargo build --bin gluon` で焼き直しが必要。
