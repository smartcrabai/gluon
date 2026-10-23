#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/csrf-tools.env
root="$PWD"
export QUINT_HOME="$root/target/formal-tools/quint"
quint="$root/crates/gluon/tests/specs/node_modules/.bin/quint"
model="$root/crates/gluon/tests/specs/csrf.qnt"
printf '%s  %s\n' "$APALACHE_SHA256" target/formal-tools/apalache.tgz | shasum -a 256 --check
[[ -x "$QUINT_HOME/apalache-dist-$APALACHE_VERSION/apalache/bin/apalache-mc" ]]
[[ "$("$quint" --version)" == "$QUINT_VERSION" ]]
mkdir -p target/csrf-spec/quint
# Keep each execution separate so old logs/traces cannot masquerade as this run.
output="$(mktemp -d "$root/target/csrf-spec/quint/run-XXXXXXXX")"
printf 'Quint artifacts: %s\n' "$output"
cd "$output"
"$quint" typecheck "$model" 2>&1 | tee typecheck.log
# Select all twelve witness runs explicitly. Each emits an ITF execution trace.
"$quint" test "$model" --match '_test$' --max-samples=1 --seed=0x43535246 \
  --out-itf='witness_{test}_{seq}.itf.json' --verbosity=3 2>&1 | tee witnesses.log
# Reproducible bounded random exploration complements, not replaces, SMT checks.
"$quint" run "$model" --invariant=contract --max-steps=6 --max-samples=1000 \
  --seed=0x43535246 --out-itf='simulation_{seq}.itf.json' --verbosity=3 2>&1 | tee simulation.log
# A separate fixed local endpoint avoids accidentally using a default server.
# Quint launches/kills the checksum-verified 0.51.1 server as its child process.
properties=(
  issuance reuse isolation safe_admission unsafe_admission precedence
  body_limit payload_preserved store_failure contract
)
failed=0
for property in "${properties[@]}"; do
  printf 'Verifying %s through 6 steps\n' "$property"
  # A failed solver or log write must not skip the remaining properties.
  # pipefail preserves either failure while the conditional suppresses errexit.
  if "$quint" verify "$model" --invariant="$property" --max-steps=6 \
    --apalache-version="$APALACHE_VERSION" --server-endpoint=localhost:18822 \
    --out-itf="$property.counterexample.itf.json" --verbosity=3 2>&1 | tee "verification-$property.log"; then
    printf '%s PASS\n' "$property" | tee -a verification-status.log || failed=1
  else
    printf '%s FAIL\n' "$property" | tee -a verification-status.log || true
    failed=1
  fi
done
exit "$failed"
