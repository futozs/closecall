# Contributing

Thanks for helping. Close Call is a small project with one maintainer, so focused pull requests with
a clear reason get merged fastest. For anything bigger than a fix, open an issue first.

## Setup

You need:

- Rust through [rustup](https://rustup.rs). `rust-toolchain.toml` pins the version (1.99.1) and
  installs `rustfmt` and `clippy` on first use.
- [bun](https://bun.sh) 1.3 or newer.
- macOS 14+ with the Xcode Command Line Tools for the macOS app. Core, lists and engine code builds
  and tests on any OS.

```sh
git clone https://github.com/futozs/closecall
cd closecall
bun install
bun run dev
```

`bun run dev` starts the app with the UI served from `localhost:1420`. It runs an unbundled binary,
so macOS attributes permissions to your terminal. To test Accessibility or Automation, read the
development notes in [docs/platforms/macos.md](docs/platforms/macos.md#development).

## Before you open a pull request

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test
cargo deny check
bun run check
bun x biome check .
```

If you changed types in `crates/core`, regenerate the TypeScript bindings with
`cargo test -p closecall-core export_bindings` and commit them. CI fails if generated files drift.

In the pull request, say which operating systems you tested on and which you could not. Platform
behaviour needs a real device; results go in [docs/test-matrix.md](docs/test-matrix.md).

## Rules that are not negotiable

- **No network code.** No HTTP clients, sockets, telemetry, update checks, remote fonts or CDNs. The
  CI checks are described in [ADR-0002](docs/decisions/0002-no-network-ever.md).
- **No titles, URLs or file paths in logs or notifications.**
- **No lost work.** Unknown unsaved state counts as unsaved.
- A new OS permission, a new kind of dependency or a changed default needs a decision record in
  [docs/decisions/](docs/decisions/README.md).

## Style

- Match the code around you. Comments explain why something non-obvious is done, never what the code
  says. No commented-out code, no TODOs you could just do.
- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`,
  `refactor:`, `docs:`, `test:`, `chore:`, `perf:`, `ci:`.
- Rust code does not write user-facing text. It sends a message key and parameters; the text lives in
  `locales/`.

## Adding or improving a translation

All strings live in `locales/<language>/ui.json` (the settings window) and
`locales/<language>/native.json` (tray menu and notifications). English (`en`) is the reference.

1. Copy `locales/en/` to `locales/<code>/` and translate every value. Keep the keys and the
   `{{placeholders}}` exactly as they are.
2. Use the plural forms your language needs (`_one`, `_other`, and so on, as i18next expects).
3. Add the language to the `Language` enum in `crates/core/src/config.rs` and to the language picker.
4. Translate the macOS permission text in `app/src-tauri/<code>.lproj/InfoPlist.strings`.
5. Run the tests: CI fails when a key is missing or unused in any language.
6. Look at every screen in the new language at the window's minimum size; long words break layouts.

Write the way a native speaker would say it, not word for word from English. For Hungarian: use „…”
quotes, correct ő and ű, and keep the noun singular after a number („3 lap”, not „3 lapok”).

## Using AI tools

They are welcome. You are responsible for every line you submit, and you should understand it.
[AGENTS.md](AGENTS.md) holds the instructions coding agents read; it is reviewed like code.

## Security problems

Do not open a public issue. See [SECURITY.md](SECURITY.md).
