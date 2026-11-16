## What and why

<!-- What changes, and the problem it solves. Link the issue if there is one. -->

## Checks

- [ ] `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test`
- [ ] `bun run check` and `bun x biome check .` (if `app/` changed)
- [ ] Generated files regenerated and committed (TS bindings, config schema) if `crates/core` types changed

## Tested on

<!-- Which OS and version you ran it on, and what you could not test. Device results go in docs/test-matrix.md. -->

## Privacy impact

- [ ] None
- [ ] New or updated dependency (checked with the `verify-no-network` steps)
- [ ] Reads or stores new data (PRIVACY.md updated)
- [ ] New OS permission, trigger type or changed default (decision record in `docs/decisions/`)

## Docs and strings

- [ ] User-visible strings added to both `locales/en` and `locales/hu`
- [ ] Docs updated where behaviour changed (`docs/config.md`, `docs/platforms/`, `docs/status.md`)
