# 環境変数

`Boot::run()` が起動時に読む環境変数。`gluon.toml` の値は将来読み込まれる予定だが、現状は env が source of truth。

## アプリ全体

| 変数 | 既定値 | 用途 |
|---|---|---|
| `DATABASE_URL` | (未設定) | sqlx `PgPool` の接続先。設定時は PostgreSQL 永続 session store も有効化。未設定なら PgPool は Container に登録されず session は `MemoryStore` |
| `GLUON_BIND` | `0.0.0.0:3000` | bind アドレス。dev では `127.0.0.1:3000` を推奨 |
| `GLUON_TELEMETRY_DISABLED` | (未設定) | `1` / `true` で OpenTelemetry をスキップし fmt subscriber のみを初期化する。OTLP コレクタが動いていない dev / test 環境で必須 |
| `GLUON_INSECURE_COOKIE` | (未設定) | `1` / `true` でセッション cookie の `Secure` 属性を外す。HTTP な dev サーバで session を維持するときに使う(production では絶対に外さない) |

## OpenTelemetry

| 変数 | 既定値 | 用途 |
|---|---|---|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4317` | OTLP gRPC コレクタの URL |
| `OTEL_SERVICE_NAME` | `gluon` | `service.name` リソース属性 |
| `RUST_LOG` | `info` | `EnvFilter` 経由で tracing-subscriber に渡される |

## Session

| 変数 | 既定値 | 用途 |
|---|---|---|
| `SECRET_KEY_BASE` | (未設定) | session cookie 署名鍵。64 bytes 以上。secure cookie mode では必須。`GLUON_INSECURE_COOKIE=1` の開発時のみ、未設定ならプロセス固有の一時鍵を生成 |

## 典型コマンド

dev で OTel を切って `127.0.0.1` バインド + insecure cookie:

```bash
GLUON_TELEMETRY_DISABLED=1 GLUON_INSECURE_COOKIE=1 GLUON_BIND=127.0.0.1:3000 cargo run
```

production:

```bash
DATABASE_URL=postgres://... \
SECRET_KEY_BASE=<64-bytes-or-longer-random-secret> \
OTEL_EXPORTER_OTLP_ENDPOINT=https://otel.example.com \
OTEL_SERVICE_NAME=my-app \
RUST_LOG=info \
./my-app
```

CI(DB なしで起動だけ確認):

```bash
GLUON_TELEMETRY_DISABLED=1 GLUON_INSECURE_COOKIE=1 cargo run
# DATABASE_URL が未設定なら PgPool は登録されないので、
# repository を resolve しない route だけならクエリなしで起動できる
```
