#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/csrf-tools.env
mkdir -p target/csrf-spec
printf '%s  %s\n' "$ALLOY_SHA256" target/formal-tools/alloy.jar | shasum -a 256 --check
# Alloy 6.2.0 `help exec` confirms wildcard --command selection. Default runs
# only one command. SAT4J is bundled Java, avoiding unpinned native solvers.
# mktemp avoids deleting or mixing reports from any earlier execution.
output="$(mktemp -d "$PWD/target/csrf-spec/alloy-XXXXXXXX")"
java -jar target/formal-tools/alloy.jar exec --command '*' --solver sat4j \
  --type json --output "$output/instances" crates/gluon/tests/specs/csrf.als 2>&1 | tee "$output/cli.log"
# Do not trust a zero process exit alone: enforce every SAT/UNSAT expectation.
node scripts/check-csrf-alloy-results.cjs "$output/instances/receipt.json" | tee "$output/results.log"
