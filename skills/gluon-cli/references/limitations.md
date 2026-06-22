# 既知の制約・落とし穴

MVP として動くが手当てが必要な箇所。フレームワーク側の正式対応が入るまで、ユーザー側でワークアラウンドが要る。

## テンプレート / 生成

1. **`gluon new` 後の `Cargo.toml` が `path = "../gluon/crates/gluon{,-build}"` の暫定値**
   crates.io 公開前なので、生成直後に実体パスや絶対パスに書き換える必要がある。書き換え例は [`workflows.md`](workflows.md) の A 節。

2. **`gluon g resource` は GET ハンドラしか生成しない**
   POST / PUT / DELETE は手で関数を追加する。`api/<name>/route.rs` 側も `get` のみ。

3. **`gluon g domain` は migration を生成しない**
   集約境界とテーブル境界が独立なので意図的。migration が必要なら `gluon g migration` を別途呼ぶ。詳細は [`conventions.md`](conventions.md) の "Domain と Table" 節。

4. **テンプレ修正は CLI 再ビルドが必要**
   `crates/gluon-cli/templates/` の `.j2` ファイルは `rust-embed` で焼き込まれている。

## HTTP / レンダリング

5. **HTMX fragment mode は middleware で `HX-Request` を検出するが、View<P> 側で fragment 切替がまだ伝搬していない**
   `HtmxRequest` extractor で flag は取れるが、`jsxrs::RenderConfig.fragment` への接続は未配線。`page.tsx` 側で fragment 出力したい場合は、handler 内で自前に `RenderConfig.fragment = true` を制御するしかない。

6. **`SECRET_KEY_BASE` は framework 側で読まれていない**
   `.env.example` には書いてあるが、現状 session 署名鍵には接続されていない。signed cookie に依存する設計を書かない。

7. **`Redirect::to(url)` は任意の URL を受け付ける(Open Redirect)**
   ユーザー入力をそのまま渡すと別ドメインへのリダイレクトが書ける。allowlist は呼び出し側責任。

## CLI

8. **`gluon db seed` は未実装**
   `bail!` する。seed が必要なら `cargo run --bin seed` 等を別途用意。

9. **`gluon dev` のファイル監視フィルタは粗い**
   `.swp` / `.tmp` / `.tsx` / `.j2` / IDE 一時ファイルでも再起動が走ることがある。Windows パス対応も弱い (`/target/` のみで `\target\` を見ていない)。

10. **`gluon d ...` に `--yes` フラグはない**
    非対話モードは `yes | gluon d ...` で代用。CI で使うときは pipe 必須。

## DI コンテナ

11. **Container::resolve は未 bind 時に panic**
    `Inject<T>` extractor は `Infallible` Rejection なので、未 bind のまま route がヒットすると HTTP リクエスト中に panic する(axum がそれを 500 に変換するので致命ではないが、ログには panic が残る)。

12. **`Boot::run()` はカスタム middleware を受け付けない**
    `session_layer` / `csrf_middleware` / `htmx_middleware` / `ServeDir` は強制的に積まれる。CORS や独自 logging を外側に挟みたい場合は現状 fork が必要。

## Tower / 外部依存

13. **`ServeDir` の symlink follow は tower-http 0.6 では off にできない**
    `public/` 配下に untrusted symlink を置くと辿られる。運用で symlink を作らない前提。

14. **`MemoryStore` ベース session は永続化されない**
    プロセス再起動で全 session が消える。production で水平スケールするなら別 store(`tower-sessions-redis-store` など)に差し替える必要があるが、framework 側にスイッチはまだない。
