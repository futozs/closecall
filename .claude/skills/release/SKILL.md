---
name: release
description: Checklist for cutting a Close Call release (version bump, changelog, test matrix, macOS signing and notarization, artifacts, Homebrew tap). Use only when the owner explicitly asks to prepare or publish a release.
disable-model-invocation: true
---

# Release checklist

Releases are the owner's call. Prepare, check, and stop before anything public (tag push, GitHub
release, tap update) until the owner says go.

## 1. Ready

- [ ] CI green on `main`, including cargo-deny and the no-network runtime test.
- [ ] [docs/test-matrix.md](../../../docs/test-matrix.md) filled for this version; every ✗ is fixed or
      documented as a limitation. No data loss in any unsaved-document row.
- [ ] Measured share-to-conceal latency recorded in the test matrix.
- [ ] On a new macOS major or beta: `SLSIsScreenWatcherPresent` still resolves and behaves
      ([ADR-0010](../../../docs/decisions/0010-skylight-share-detection.md)).
- [ ] Hungarian strings reviewed by a native speaker.
- [ ] Licences of bundled lists current in `lists/manifest.toml`; About screen shows them.

## 2. Version and changelog

- [ ] Bump the version in all three places: `[workspace.package] version` in `Cargo.toml`,
      `app/package.json`, `app/src-tauri/tauri.conf.json`. Then `cargo check` to update `Cargo.lock`.
- [ ] Update `CHANGELOG.md` (create it at the first release) from the conventional commits since the
      last tag: features, fixes, known limitations.
- [ ] Update [docs/status.md](../../../docs/status.md) and the README status line.
- [ ] Commit `chore: release vX.Y.Z`.

## 3. macOS build, signing and notarization

- [ ] Universal build (`aarch64` + `x86_64`), `minimumSystemVersion` 14.0.
- [ ] Signed with the Developer ID Application certificate, hardened runtime on, entitlement
      `com.apple.security.automation.apple-events`, `NSAppleEventsUsageDescription` in `en.lproj` and `hu.lproj`.
- [ ] Notarized with an App Store Connect API key (`APPLE_API_ISSUER`, `APPLE_API_KEY`,
      `APPLE_API_KEY_PATH`) and stapled: `xcrun stapler validate "Close Call.app"`, `spctl -a -vv "Close Call.app"`.
- [ ] Bundle id is still `hu.futozs.closecall` (TCC grants depend on it).
- [ ] Smoke test the DMG on a clean user account: onboarding, Accessibility and Automation prompts,
      panic hotkey, one share trigger, restore.

## 4. Publish (owner go-ahead required)

- [ ] Tag `vX.Y.Z`, GitHub release with the DMG, `SHA256SUMS` and build provenance attestations.
- [ ] Update the Homebrew tap cask (`futozs/tap`) with the new version and SHA-256.
- [ ] No updater artifacts (`latest.json`); there is no in-app updater.
