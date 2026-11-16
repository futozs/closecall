---
name: verify-no-network
description: Prove that a change opens no network sockets. Use before finishing any change that adds or updates a dependency (Cargo or bun), adds a Tauri plugin, touches the CSP or capabilities, adds IPC, or adds code that talks to another process, and whenever someone asks whether the app could reach the network.
---

# Verify no network

The rule ([ADR-0002](../../../docs/decisions/0002-no-network-ever.md)): no `AF_INET` or `AF_INET6`
socket, loopback included. Local IPC (stdio, Unix sockets, named pipes, D-Bus) is fine.

## Static checks

```sh
cargo deny check bans                      # banned HTTP/TLS/WebSocket crates, all desktop targets
for c in reqwest hyper rustls native-tls tungstenite; do cargo tree --workspace -i "$c"; done
                                           # host target only; each must report "did not match any packages"
cargo clippy --workspace --all-targets -- -D warnings              # disallowed std::net / tokio::net types
bun x biome check .                        # bans fetch, XMLHttpRequest, WebSocket, EventSource
grep -rnE 'std::net|tokio::net|TcpStream|UdpSocket|socket2' crates app/src-tauri/src
grep -rnE 'fetch\(|XMLHttpRequest|WebSocket|EventSource|sendBeacon|https?://' app/src
```

Any hit from the two `grep` lines needs a reason. URLs in `app/src` are only allowed as text that the
opener hands to the user's browser (the repo or releases page).

Then review by hand:

- `app/src-tauri/tauri.conf.json`: CSP `connect-src` is still `ipc: http://ipc.localhost`; no remote
  URLs, fonts or scripts in `app/index.html` or CSS.
- `app/src-tauri/capabilities/*.json`: no new permission that reaches the network; the opener scope
  covers only `x-apple.systempreferences:*`, `ms-settings:*` and the repo URL.
- New Tauri plugins: never `updater`, `http`, `websocket`, `upload`.
- A new crate's own dependency tree: `cargo tree -p <crate> -e normal` and read it.

## Runtime check (Linux)

CI runs this in `.github/workflows/ci.yml`; to reproduce locally:

```sh
strace -f -e trace=socket,connect -o /tmp/closecall.strace <binary-or-test> <args>
grep -E 'AF_INET6?' /tmp/closecall.strace && echo "network socket opened" || echo "clean"
unshare -rn <binary-or-test> <args>        # must behave the same with no network namespace
```

On macOS, with the app running: `lsof -nP -i -a -p <pid>` must print nothing; LuLu or Little Snitch
must show no attempts.

Report what you ran and the result. If you could not run the runtime check, say so.
