# Environment Variables

`Boot::run()` reads `gluon.toml` from the current working directory first. If absent, it uses the copy embedded by `gluon-build` at compile time; defaults are used only if neither exists. A runtime file is an explicit override, so it can be used for local development or deployment overrides without rebuilding. Unknown or unsupported keys cause a boot error.

`gluon dev` watches an existing `gluon.toml` and restarts the app when it changes.

## Configuration precedence

- `[app].name` is available as `Config.app.name` through `Inject<Config>`.
- `[server].host` / `[server].port` default to `0.0.0.0:3000`; `GLUON_BIND` overrides both.
- `[database].driver` is fixed to `postgres` (also accepted from older scaffolds); `[database].url_env` selects the environment variable containing the database URL (default `DATABASE_URL`); `[database].pool_size` sets the maximum number of pool connections.
- `[view].pretty` and `[view].tailwind` default to `false`; the scaffold enables Tailwind. Tailwind generates one cache-busted stylesheet from statically written classes in application sources.
- `[view].tailwind_content` optionally lists glob patterns relative to the application root for other files with classes, for example `["web/src/**/*.ts"]`. Each pattern needs an existing directory prefix below the project root; root-wide recursive patterns are rejected. Globs are scanned at build time and re-scanned from disk in dev mode. Dynamic or interpolated class names are not discovered; write complete class names as string literals in scanned sources.
- `[telemetry].enabled_env` defaults to `OTEL_ENABLED`. When set, `1` / `true` (case-insensitive) enables telemetry and any other value disables it; when unset, telemetry remains enabled by default. `GLUON_TELEMETRY_DISABLED=1` / `true` is a higher-precedence opt-out.
- `OTEL_SERVICE_NAME` overrides `[telemetry].service_name`. `OTEL_EXPORTER_OTLP_ENDPOINT` remains controlled by its environment variable (default `http://localhost:4317`).

## Application-wide

| Variable | Default | Purpose |
|---|---|---|
| `DATABASE_URL` | (unset) | Connection target for the sqlx `PgPool`, read from the variable named by `[database].url_env` (default `DATABASE_URL`). When set, the PostgreSQL persistent session store is also enabled. If unset, the PgPool is not registered with the Container and sessions use `MemoryStore` |
| `GLUON_BIND` | `0.0.0.0:3000` | Bind address; overrides `[server].host` and `[server].port`. `127.0.0.1:3000` is recommended for dev |
| `GLUON_TELEMETRY_DISABLED` | (unset) | Highest-precedence telemetry opt-out when set to `1` / `true`. Otherwise, a false value for the variable selected by `[telemetry].enabled_env` disables telemetry; if that variable is unset, telemetry is enabled by default |
| `GLUON_INSECURE_COOKIE` | (unset) | When set to `1` / `true`, removes the `Secure` attribute from session cookies. Use when maintaining sessions on an HTTP dev server (never disable it in production) |

## OpenTelemetry

| Variable | Default | Purpose |
|---|---|---|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4317` | URL of the OTLP gRPC collector |
| `OTEL_ENABLED` (scaffold default; configurable via `[telemetry].enabled_env`) | (unset) | `1` / `true` enables telemetry; any other set value disables it. If unset, telemetry is enabled by default |
| `OTEL_SERVICE_NAME` | configured value (default `gluon`) | Overrides `[telemetry].service_name`; otherwise identifies the `service.name` resource attribute |
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
