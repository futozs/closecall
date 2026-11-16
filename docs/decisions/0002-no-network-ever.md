---
status: accepted
date: 2026-11-16
decision-makers: futozs
---

# No network access, ever

## Context and Problem Statement

The app sees which bank, which webmail and which PDFs a person has open, and its config names the
sites they consider sensitive. Users have to grant it Accessibility and Automation. A privacy tool
that phones home, even for update checks or crash reports, loses the reason to trust it. How strict
should the network policy be, and how do we make it checkable?

## Considered Options

- No sockets at all, loopback included, enforced in layers and tested at runtime
- No internet, but allow loopback integrations (obs-websocket, the Teams local API)
- Allow an opt-in update check

## Decision Outcome

Chosen option: "No sockets at all", because "no network" is only believable if it is absolute and
anyone can check it. The process and its helpers never open an `AF_INET` or `AF_INET6` socket. IPC
uses stdio, Unix domain sockets and Windows named pipes with per-user ACLs.

### Consequences

- Good, because the claim is simple and verifiable by users and in CI.
- Good, because no telemetry, update checks, crash reporter or accounts means less to secure.
- Bad, because there is no in-app updater: users update through Homebrew, winget, their package
  manager, or by downloading a release. The app may show a link that opens the releases page in the
  user's browser; our process does not fetch it.
- Bad, because lists only update with a release or a user-imported file ([0009](0009-bundled-plain-text-lists.md)).
- Bad, because loopback integrations such as obs-websocket are off the table.

### Confirmation

- `cargo-deny` bans HTTP, TLS and WebSocket crates (`reqwest`, `hyper`, `h2`, `ureq`, `isahc`, `curl`,
  `surf`, `attohttpc`, `tungstenite`, `quinn`, `rustls`, `native-tls`, `openssl`).
- Clippy `disallowed-types` forbids `std::net::{TcpStream, TcpListener, UdpSocket}` and the `tokio::net` equivalents.
- Biome bans `fetch`, `XMLHttpRequest`, `WebSocket` and `EventSource` in the UI; the CSP limits
  `connect-src` to Tauri IPC.
- No `updater`, `http`, `websocket` or `upload` Tauri plugin.
- Linux CI runs scripted `platform-fake` scenarios under `strace` and fails on any `AF_INET`/`AF_INET6`
  socket, then repeats them inside `unshare -rn`.

## More Information

[architecture-recommendation.md §12](../research/architecture-recommendation.md), [PRIVACY.md](../../PRIVACY.md),
the `verify-no-network` skill in `.claude/skills/`.
