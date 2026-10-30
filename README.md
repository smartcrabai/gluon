# gluon

> [!WARNING]
> This project is under active development. It is not ready for production use, and APIs, behavior, and project structure may change without notice.

## Install

The prebuilt `gluon-cli` package installs the `gluon` executable. Install it
with either Homebrew or the generated shell installer:

```sh
brew install smartcrabai/tap/gluon-cli
```

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/smartcrabai/gluon/releases/latest/download/gluon-cli-installer.sh | sh
```

Run `gluon --help` to confirm the installation.

Configured targets are Apple Silicon macOS, Linux x86_64 and aarch64, and Windows
x86_64 and aarch64. Intel macOS is not currently supported. Installing a
prebuilt binary does not require Rust; developing or building generated
applications does require Rust and Cargo.

## Framework guides

- [Server-Sent Events](docs/sse.md)

## Application templates and assets

`gluon-build` embeds every `app/**/*.tsx` template (including layouts, components,
and error pages), supported `.jsx`/`.ts`/`.js` import sources, and every file
under `public/`. Relative component imports resolve from that embedded source
map, so deployed applications do not need their source `app/` or `public/`
directories.

`gluon dev` enables `GLUON_DEV_TEMPLATES=1` by default so TSX and public asset
edits are read from the application source tree. An explicit environment value
is preserved; set `GLUON_DEV_TEMPLATES=1` yourself when running the app directly.
The default runtime remains embedded-only. The current JSX source-map API is
provided by the pinned jsxrs Git revision; a jsxrs crates.io release is needed
before gluon can replace that dependency pin.
## Contributing

Install the CLI from this checkout with:

```sh
cargo install --path crates/gluon-cli
```

The release workflow uses `cargo-dist` 0.32.0. Install that version, inspect
the release plan, and build local artifacts with:

```sh
cargo install cargo-dist --version 0.32.0 --locked
dist plan
dist build
```

## Maintainer releases
Pull requests build and upload artifacts for all configured targets; publication
starts when a release tag is pushed.

Update the workspace version and local dependency pins, then regenerate and
commit `Cargo.lock` before creating a new, unused `v<version>` tag. The existing
`v0.1.1` tag must not be moved. After confirming the plan and build, push the
new tag to trigger the configured GitHub Release and Homebrew tap publication.
GitHub Actions uses the `HOMEBREW_TAP_TOKEN` secret to write to
