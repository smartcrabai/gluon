# Environment Variables

Environment variables read by `Boot::run()` at startup. `gluon.toml` values are planned to be read in the future, but currently env is the source of truth.

## Application-wide

| Variable | Default | Purpose |
|---|---|---|
| `DATABASE_URL` | (unset) | Connection target for the sqlx `PgPool`. When set, the PostgreSQL persistent session store is also enabled. If unset, the PgPool is not registered with the Container and sessions use `MemoryStore` |
| `GLUON_BIND` | `0.0.0.0:3000` | Bind address. `127.0.0.1:3000` is recommended for dev |
| `GLUON_TELEMETRY_DISABLED` | (unset) | When set to `1` / `true`, skips OpenTelemetry and initializes only the fmt subscriber. Required in dev / test environments where no OTLP collector is running |
| `GLUON_INSECURE_COOKIE` | (unset) | When set to `1` / `true`, removes the `Secure` attribute from session cookies. Use when maintaining sessions on an HTTP dev server (never disable it in production) |

## OpenTelemetry

| Variable | Default | Purpose |
|---|---|---|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4317` | URL of the OTLP gRPC collector |
| `OTEL_SERVICE_NAME` | `gluon` | The `service.name` resource attribute |
| `RUST_LOG` | `info` | Passed to tracing-subscriber via `EnvFilter` |

## Session

| Variable | Default | Purpose |
|---|---|---|
| `SECRET_KEY_BASE` | (unset) | Session cookie signing key. At least 64 bytes. Required in secure cookie mode. Only when developing with `GLUON_INSECURE_COOKIE=1`: if unset, a process-specific ephemeral key is generated |

## Typical Commands

Disable OTel, bind to `127.0.0.1`, and use an insecure cookie for dev:

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

CI (startup check only, without a DB):

```bash
GLUON_TELEMETRY_DISABLED=1 GLUON_INSECURE_COOKIE=1 cargo run
# If DATABASE_URL is unset, no PgPool is registered, so
# routes that do not resolve a repository can start without any queries
```
