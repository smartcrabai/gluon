# CSRF specifications verification evidence

Date: 2026-10-23 (local execution on macOS arm64).

## Status and provenance

The CSRF formal and production-connected scenario checks passed at the bounds
below. Full workspace verification is **not green**: three PostgreSQL integration
targets cannot connect through this machine's Docker/OrbStack published ports.
No remote GitHub Actions execution was invoked or observed. No commits or ref
movements were performed. HEAD remained
`c5f2040cc7cb453fea43326534933427dbd07f01`.

CSRF-01 through CSRF-07 are **provisional reconstructed IDs**. The original audit
and full design were not present in the supplied evidence, so this report does
not claim an independently established mapping to the original seven IDs.
The contracts below were checked against the repository's actual source and
existing tests, not inferred solely from the formal models.

## Contract and production mappings

All source paths below are relative to `crates/gluon/` unless otherwise stated.

| Provisional ID | Source contract and exercised behavior |
| --- | --- |
| CSRF-01 | `src/middleware/csrf.rs::ensure_session_token`: create a 32-byte lowercase-hex token when absent, reuse an existing token, and establish logical session state even when a first unsafe request subsequently receives 403. Gherkin verifies persistent cookie reuse and subsequent successful submission. |
| CSRF-02 | `is_safe_method` and `csrf_middleware`: only GET/HEAD/OPTIONS bypass token validation and unsafe body buffering, after session access succeeds. Tests exercise all three plus oversized safe GET. |
| CSRF-03 | `csrf_middleware`: POST/PUT/PATCH/DELETE/TRACE/CONNECT/CUSTOM reject missing/wrong tokens and admit a matching session token. Assertions check handler invocation counts, extension exposure, and original bytes. |
| CSRF-04 | `extract_header_token`, `is_form_content_type`, `extract_form_token`: trimmed nonempty UTF-8 header wins even if incorrect, empty/invalid headers permit form fallback, MIME matching ignores case/parameters, and the first decoded `_csrf` field wins without trimming its value. Tests cover duplicates, percent decoding, invalid/binary bytes, and non-form rejection. |
| CSRF-05 | `csrf_middleware` and `MAX_REQUEST_BODY_BYTES`: unsafe bodies at 4,194,304 bytes are within the middleware limit, at 4,194,305 return 413 before token validation. The safe path does not impose this buffer limit. |
| CSRF-06 | `Request::from_parts` and `CsrfToken`: admitted unsafe requests preserve bytes, safe requests preserve their payload, and rejected requests do not invoke the handler. CONNECT response-body stripping is an Axum response rule, so the handler's observed bytes are checked independently. |
| CSRF-07 | `ensure_session_token` and the real `tower_sessions::SessionManagerLayer`: independent sessions reject each other's tokens; backend load errors and malformed stored values fail before the handler. Deferred create/save failures produce 500 after handler execution, not transactional rollback. |

The scenario router imports the **public production `csrf_middleware`**, not a
copy. It uses the real SessionManagerLayer and MemoryStore. Only backend failure
injection wraps the store. Axum's separate default 2 MiB handler extractor limit
is disabled in this test router so the production middleware's own 4 MiB limit
can be tested directly. Normal production layering is also confirmed in
`src/boot.rs::serve_with_shutdown`: CSRF is inside the session layer.
The existing generated-application HTTP probe
`crates/gluon-cli/tests/http_csrf.rs::csrf_blocks_state_changing_without_token`
also passed in the completed workspace run, exercising Boot with a real server
and signed session cookies.

## Files and repairs

The implemented set comprises:

- `crates/gluon/tests/specs/csrf.qnt`, `csrf.als`, `package.json`,
  `package-lock.json`, and `.gitignore`.
- `crates/gluon/tests/csrf_spec.rs` and `tests/features/csrf.feature`.
- `crates/gluon/Cargo.toml` and root `Cargo.lock` for cucumber 0.21.1.
- `scripts/csrf-tools.env`, `setup-csrf-specs.sh`, `check-csrf-quint.sh`,
  `check-csrf-alloy.sh`, `check-csrf-alloy-results.cjs`, and
  `check-csrf-runners.test.cjs`.
- `.github/workflows/ci.yml` and this evidence file.

This verification pass repaired an actual coverage omission: the Quint runner
previously verified only the aggregate `contract`. It now explicitly verifies
each of the nine named invariants and the aggregate, retains separate logs in a
fresh per-run directory, continues after an individual verification failure,
and fails overall if any property or log write fails.

The Alloy checker now validates command identity, scope/source, solver,
execution metadata, complete command coverage, expected results, and SAT instance
structure. Seventeen regression tests exercise malformed/partial receipts,
unexpected SAT and UNSAT, expectation mismatches, and failure propagation. Their
Quint subprocess fixture is explicitly synthetic and is **not** evidence of
formal verification. The real Quint executions are recorded separately below.
Alloy 6.2.0 omits default zero expectation fields and UNSAT solution fields, so
receipt validation is paired with the real CLI's successful exit and observed
per-command CLI log, rather than treating metadata alone as proof of execution.

A new production-connected scenario injects a non-string CSRF value into an
actual persisted MemoryStore record and checks 500, no handler effects, and no
state rotation. No contract-confirmed production implementation defect was
found in the selected lifecycle. Production source was not changed. No model,
invariant, assertion, witness, meaningful bound, or strictness setting was
weakened or removed.

## Pinned tools, bounds, and assumptions

Observed pinned versions: Node 22.14.0, npm 10.9.2, Temurin 21.0.6+7,
Rust 1.99.1, Quint 0.29.1, Apalache 0.51.1 (build 48b9aca), Alloy 6.2.0,
and cucumber-rs 0.21.1. Exact Node/JDK versions were installed side by side using
mise without changing global defaults. Setup checked the pinned archive SHA-256
values and completed `npm ci --ignore-scripts` from the dependency lock.
Root Cargo.lock adds cucumber's dependency closure without removing any existing
package version.

The installed Quint 0.29.1 help was read for `typecheck`, `test`, `run`, and
`verify`. This version's verify command uses Apalache and does not expose a
`--backend` switch. The commands below use its actual file-first Quint option
ordering. No `quint lint` command was used. Installed Alloy 6.2.0 `help exec`
explicitly documents wildcard command selection, and its actual invocation
selected `--command '*'` with options before the model path.

Both models use two sequential sessions, two distinct issued tokens plus one
wrong token, absent/empty normalization, safe/unsafe method classes, three body
size categories (below/at/above 4 MiB), two opaque payload values, and a session
read-failure flag. Quint checks **six transitions**. Alloy uses **exactly seven
ordered State atoms**, initial state plus six requests, with scope 7. The models
assume initially empty logical sessions, collision-free generated tokens,
readable request bodies in the size abstraction, and no independent external
session mutation except modeled read failure. They do not prove entropy,
constant-time execution, concurrency, expiry, unbounded behavior, arbitrary
stream I/O errors, or durable database commits. Concrete normalization and byte
handling are checked by Gherkin. Deferred SessionManager create/save failure
semantics are tested separately rather than incorrectly modeled as rollback.

## Exact executed commands and observed results

Commands ran from the repository root. Formal commands were wrapped with:

```sh
mise exec node@22.14.0 java@temurin-21.0.6+7 -- bash scripts/setup-csrf-specs.sh
mise exec node@22.14.0 java@temurin-21.0.6+7 -- bash scripts/check-csrf-quint.sh
mise exec node@22.14.0 java@temurin-21.0.6+7 -- bash scripts/check-csrf-alloy.sh
mise exec node@22.14.0 -- node --test scripts/check-csrf-runners.test.cjs
```

Setup completed with exit 0. All three check/test commands were rerun after the
runner changes and completed with exit 0. The actual Quint commands inside the
script were (`Q` denotes the installed pinned binary and
`M` the absolute `crates/gluon/tests/specs/csrf.qnt` path):

```sh
"$Q" typecheck "$M"
"$Q" test "$M" --match '_test$' --max-samples=1 --seed=0x43535246 \
  --out-itf='witness_{test}_{seq}.itf.json' --verbosity=3
"$Q" run "$M" --invariant=contract --max-steps=6 --max-samples=1000 \
  --seed=0x43535246 --out-itf='simulation_{seq}.itf.json' --verbosity=3
# Executed separately for each P listed below, not a simulation:
"$Q" verify "$M" --invariant="$P" --max-steps=6 \
  --apalache-version=0.51.1 --server-endpoint=localhost:18822 \
  --out-itf="$P.counterexample.itf.json" --verbosity=3
```

Observed: typecheck success, **12/12 witness tests**, seeded bounded run success
with 1,000 requested samples, and **NoError/no violation** for each of
`issuance`, `reuse`, `isolation`, `safe_admission`, `unsafe_admission`,
`precedence`, `body_limit`, `payload_preserved`, `store_failure`, and `contract`.
Apalache logs confirm checks through state 6 and the pinned backend version.
No counterexample was emitted, so this report does not claim a counterexample
file exists. Twelve witness ITF traces and one simulation ITF trace do exist in
`target/csrf-spec/quint/run-a8OE6kDR/`, alongside per-property logs and
`verification-status.log`.

The actual Alloy command was:

```sh
java -jar target/formal-tools/alloy.jar exec --command '*' --solver sat4j \
  --type json --output "$output/instances" crates/gluon/tests/specs/csrf.als
node scripts/check-csrf-alloy-results.cjs "$output/instances/receipt.json"
```

Observed in both CLI output and the checked receipt: **all 21 intended commands**.
Nine checks (`Issuance`, `Reuse`, `Isolation`, `SafeAdmission`, `UnsafeAdmission`,
`Precedence`, `BodyLimit`, `PayloadPreserved`, `StoreFailure`) were UNSAT with
`expect 0`. Twelve runs (`Issue`, `Reused`, `SafeLarge`, `Header`, `Form`,
`HeaderWins`, `Missing`, `Mismatch`, `Foreign`, `Boundary`, `Oversized`,
`FailedStore`) were SAT with `expect 1`. No unexpected result occurred.

Production and validation commands actually executed:

```sh
cargo test --locked -p gluon --test csrf_spec
cargo test --locked -p gluon --test csrf_middleware
cargo fmt --all -- --check
cargo check --locked --workspace --all-features
cargo build --locked --workspace --all-features
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
cargo test --locked --workspace --all-features --no-fail-fast
actionlint .github/workflows/ci.yml
shellcheck -x scripts/*csrf*.sh
taplo lint Cargo.toml crates/gluon/Cargo.toml
taplo fmt --check --option reorder_keys=true Cargo.toml crates/gluon/Cargo.toml
sakoku --strict crates
graphify update .
git diff --check
```

Observed after changes: strict Gherkin **35 scenarios / 134 steps passed**,
existing middleware **8/8 tests passed**, formatting, workspace check/build,
Clippy with warnings denied, actionlint, shellcheck with source following,
Taplo, sakoku, and graph refresh succeeded. The first shellcheck invocation
without `-x` reported SC1091 because the sourced env file was not supplied. The
correct source-following invocation above passed. Rustfmt found one wrapping
issue in the added regression; it was corrected and the full formatting check
passed on rerun.

The ordinary workspace test command stopped at the PostgreSQL session test.
The subsequent `--no-fail-fast` run actually completed every workspace target,
including generated-app HTTP CSRF, HTTP smoke, generated examples, macro UI
checks, and doc tests. It failed exactly the three database targets below, with
one pre-existing ignored macro documentation example. No skip flags were used.
The second full post-change `--no-fail-fast` run completed with exit 101 and the
same three database failures. Its inspected result was **306 libtest tests
passed, 3 failed, 1 pre-existing ignored documentation example, 0 filtered**,
plus **35 Gherkin scenarios / 134 steps passed**. Every target ran. Exact final
output is retained in `target/csrf-spec/workspace-tests-final.log`.

The existing examples test refreshed `examples/Cargo.lock` as a side effect.
That unrelated file was initially clean and was restored to its HEAD contents
after testing, without moving HEAD or any branch ref. Its incidental diff is
archived under `target/csrf-spec/`. The mandated AST-only `graphify update .`
refreshed tracked graph outputs; its automatic dated backup was moved into
`target/csrf-spec/graphify-backup-2026-10-23/` rather than left as unrelated
untracked graph data.

## Exact remaining blocker

The unavailable prerequisite is working **host-to-container PostgreSQL TCP port
forwarding** on this machine's OrbStack Docker endpoint
`unix:///Users/admin/.orbstack/run/docker.sock`.

Observed failures:

- `-p gluon --test postgres_session`:
  `postgres_sessions_persist_across_instances`, connection `PoolTimedOut`.
- `-p gluon-cli --test db_postgres`:
  `database_lifecycle_and_seed_work_against_postgres`, `gluon db create` gets
  `Connection refused (os error 61)`.
- `-p gluon-cli --test repository_postgres`:
  `generated_postgres_repository_supports_crud`, generated probe reaches
  `PgPool::connect` and gets `PoolTimedOut`.

A separate owned disposable PostgreSQL container reproduced the environment
failure: container-internal `SELECT 1` succeeded, PostgreSQL logged readiness on
5432, and a host `psql` connection to its explicitly published
`127.0.0.1:32768` failed immediately with connection refused (exit 2). Its exact
output is in `target/csrf-spec/docker-diagnostic.log`. Only this owned diagnostic
container was removed afterward. Existing containers and global Docker settings
were left unchanged. This does not establish a production SQL/session defect;
the database-backed checks remain **failed/blocked, not passed**.

## CI and local artifacts

The native GitHub Actions `csrf-spec` job is wired to normal PR and push events
for main. It installs exact runtimes, performs checksum-verified tool setup,
runs runner safety tests, real Quint and Alloy checks, strict Gherkin, and the
existing middleware tests. Shell pipelines use `pipefail`, scripts propagate
failures, and subsequent check steps use `!cancelled()` so one failure does not
silently skip the other suites. Artifact upload uses `always()`, retains
`target/csrf-spec/` for seven days, and errors if no files exist. Thus logs,
receipts, JUnit, witness traces, and any emitted counterexamples are included
even if a check step fails. The separate existing workspace job still runs
normal full production checks; its requirements were not disabled.

This is verified wiring and local equivalent execution, **not a claimed remote
CI run or remote artifact upload**. Local evidence includes `setup.log`, exact
version/help logs, `runner-tests.log`, `quint-final.log`, `alloy-final.log`,
per-execution Alloy receipts/instances, `gherkin.log`, `junit.xml`,
`production.log`, build/check/Clippy/format/lint logs, full workspace failure
logs, and the Docker diagnostic. The final Alloy run's checked receipt and
instances are in `target/csrf-spec/alloy-GGzjKBW8/instances/`. Older pre-existing
artifacts are not used as proof of this verification pass.

## Remote CI follow-up

PR #54's first run failed during JDK installation: `actions/setup-java` exposes
Temurin 21.0.6 build 7 as `21.0.6+7.0.LTS`, not `21.0.6+7`. The workflow now
uses that full release selector; the runtime checks retain `21.0.6+7`.
After this correction, [CI run 39936355289](https://github.com/smartcrabai/gluon/actions/runs/39936355289)
passed all four jobs, including the formal checks and both platform suites,
at commit `068c4a690e7d0d09dd43ec607fa4360507bb68ff`.
