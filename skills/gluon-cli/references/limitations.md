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

5. **domain repository の自動 CRUD は PostgreSQL 対応型のみ**
   対応型は `bool`、`String`、`i8/i16/i32/i64`、`u8/u16/u32/u64/usize`、`f32/f64`、`Vec<u8>`、`uuid::Uuid`、生成 value object、およびそれらの `Option<T>`。`Vec<u32>` などその他の型は entity に使えるが、repository method は `todo!()` になる。

## CLI

6. **`gluon d ...` に `--yes` フラグはない**
    非対話モードは `yes | gluon d ...` で代用。CI で使うときは pipe 必須。

7. **migration 名の timestamp は秒単位**
   同名 migration を同じ秒内に連続生成すると overwrite 防止エラーになる。1秒待つか別名を使う。

## DI / Session

8. **`Container::resolve` の直接呼び出しは未 bind 時に panic**
   HTTP extractor の `Inject<T>` は安全に 500 を返す。composition root 内の直接 `resolve` は必須 binding の fail-fast 用途。

9. **`DATABASE_URL` 未設定時の session は `MemoryStore`**
   開発用 fallback。プロセス再起動で消える。本番・水平スケールでは `DATABASE_URL` を設定し PostgreSQL session store を使う。
