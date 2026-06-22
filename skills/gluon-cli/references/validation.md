# CLI 入力検証ルール

`gluon g` / `gluon d` はユーザー入力 (`route` / `name` / `--field NAME:TYPE`) を厳格に検証する。これは(1)任意ファイル書き込み(`../etc/passwd`)、(2)生成された Rust ソースへの任意コード注入、(3)生成された SQL への注入、を防ぐため。

## route の検証

`gluon g controller <route>` / `gluon d controller <route>` で受け取る `route` は、スラッシュ区切りで分割した各 segment が次のいずれか:

- 通常 segment: `[A-Za-z0-9_-]+`
- 動的 segment: `[<inner>]` で `inner` が `[A-Za-z0-9_-]+`
- catch-all: `[...<inner>]` で `inner` が `[A-Za-z0-9_-]+`
- ルートグループ: `(<inner>)` で `inner` が `[A-Za-z0-9_-]+`

加えて `inner == "." || inner == ".."` は弾く。

reject 例:

```text
$ gluon g controller '../../etc/passwd'
error: invalid route segment: ..

$ gluon g controller 'users/[id'
error: invalid route segment: [id
```

## identifier (name) の検証

`gluon g {usecase,domain,dto,migration,resource} <name>` の `name`:

- 空文字禁止
- 先頭文字: ASCII 英字 (`A-Za-z`) または `_`
- 以降: ASCII 英数字 (`A-Za-z0-9`) または `_`

reject 例:

```text
$ gluon g usecase 'foo;}; fn bad()'
error: invalid usecase name: foo;}; fn bad() (only letters, digits and underscore are allowed)

$ gluon g domain 1user
error: invalid domain name: 1user (must start with a letter or underscore)
```

## field type の検証

`gluon g domain <name> --field NAME:TYPE` の `NAME` と `TYPE`:

- `NAME` は identifier ルールに従う(上記)
- `TYPE` は `[A-Za-z0-9_<>,: ']` のみ許容

reject 例:

```text
$ gluon g domain user --field 'id:String;} fn evil'
error: invalid field type: String;} fn evil (contains disallowed character)
```

`<>,:` を許すことで `Option<UserId>`、`Vec<Tag>`、`std::sync::Arc<T>`、`&'static str` のような型を書ける一方、`;` / `{` / `}` / `(` / `)` / `=` などの「Rust の構文要素を持つ文字」は弾かれる。

## destroy migration の名前マッチ

`gluon d migration <name>` は次の正規表現と等価なルールでファイル名を完全一致させる:

```
^[0-9]{14}_<snake>\.(up|down)\.sql$
```

reject 例:

```text
$ gluon d migration users
error: no migration matched name: users
# 20260620120000_add_users.up.sql や 20260620120000_create_users.up.sql は
# `users` 指定では巻き込まれない
```

完全一致が前提なので、`gluon d migration create_users` のように **コマンドを生成時の name と同じ形で** 呼ぶ。

## 失敗時の動作

検証 NG の場合は CLI が `error: ...` を stderr に出して exit code 1。**部分的にファイルを生成して途中で止まることはない**(検証は最初に走る)。
