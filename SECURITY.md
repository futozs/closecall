# Security policy

## Reporting a vulnerability

Report privately through GitHub: open the repository's **Security** tab and choose **Report a
vulnerability** ([direct link](https://github.com/futozs/closecall/security/advisories/new)). Please
do not open a public issue for a security problem.

Include what you found, how to reproduce it, the OS and app version, and the output of
`closecall diag` if it is relevant. Remove your own private URLs, titles and paths from anything you send.

Close Call is maintained by one person. Expect an acknowledgement within 7 days. We aim to fix and
disclose within 90 days, and will agree a date with you if it needs longer.

## In scope

- Anything that makes the app open a network connection, or bypasses the no-network checks.
- Titles, URLs, file paths or document names leaking into logs, notifications, crash output or files.
- The webview reaching more than it should: CSP bypass, Tauri capability or IPC command escalation.
- Data loss caused by an action on a document with unsaved changes.
- From v0.4: messages to the native-messaging host that are spoofed or come from another user.
- Supply chain: a dependency or build step that could change what we ship.

## Out of scope

- Frames that were captured before cleanup finished. That limit is documented in
  [docs/threat-model.md](docs/threat-model.md); a report showing it is much worse than documented is welcome.
- Attacks that need administrator rights or malware already on the machine.
- Missed detection of a share source we list as undetectable.

## Supported versions

Close Call is pre-alpha and has no releases yet. Once it does, only the latest release gets fixes.
Release artifacts will come with SHA-256 checksums and GitHub build provenance attestations that you
can check with `gh attestation verify <file> --repo futozs/closecall`.
