#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/csrf-tools.env
[[ "$(node --version)" == "v$NODE_VERSION" ]]
[[ "$(npm --version)" == "$NPM_VERSION" ]]
java -version 2>&1 | grep -F "\"${JAVA_VERSION%+*}\""
java -XshowSettings:properties -version 2>&1 | grep -F "java.runtime.version = $JAVA_VERSION-"
[[ "$(rustc --version)" == "rustc $RUSTUP_TOOLCHAIN "* ]]
mkdir -p target/formal-tools/quint "target/csrf-spec"
# Versioned URLs and verified bytes, never an unchecked latest-release download.
curl --fail --location --retry 3 "https://github.com/AlloyTools/org.alloytools.alloy/releases/download/v$ALLOY_VERSION/org.alloytools.alloy.dist.jar" -o target/formal-tools/alloy.jar
printf '%s  %s\n' "$ALLOY_SHA256" target/formal-tools/alloy.jar | shasum -a 256 --check
curl --fail --location --retry 3 "https://github.com/apalache-mc/apalache/releases/download/v$APALACHE_VERSION/apalache.tgz" -o target/formal-tools/apalache.tgz
printf '%s  %s\n' "$APALACHE_SHA256" target/formal-tools/apalache.tgz | shasum -a 256 --check
mkdir -p "target/formal-tools/quint/apalache-dist-$APALACHE_VERSION"
tar -xzf target/formal-tools/apalache.tgz -C "target/formal-tools/quint/apalache-dist-$APALACHE_VERSION"
# package.json overrides pin patched archive readers while retaining Quint 0.29.1.
npm --prefix crates/gluon/tests/specs ci --ignore-scripts
[[ "$(crates/gluon/tests/specs/node_modules/.bin/quint --version)" == "$QUINT_VERSION" ]]
# Keep the actual pinned CLI help with the reports, including wildcard selection.
java -jar target/formal-tools/alloy.jar version
java -jar target/formal-tools/alloy.jar help exec > target/csrf-spec/alloy-help.log 2>&1
crates/gluon/tests/specs/node_modules/.bin/quint verify --help > target/csrf-spec/quint-help.log
