#!/usr/bin/env bash
# Copies the Apple signing and notarization variables into $GITHUB_ENV, but only groups that are
# complete. The Tauri bundler treats a set-but-empty APPLE_* variable as a request to sign or
# notarize, and missing repository secrets arrive as empty strings.
set -euo pipefail

signing=(APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_SIGNING_IDENTITY)
notarization=(APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID)

all_set() {
  local name
  for name; do
    [[ -n "${!name:-}" ]] || return 1
  done
}

export_to_github_env() {
  local name delimiter
  for name; do
    delimiter="EOF_$(uuidgen)"
    printf '%s<<%s\n%s\n%s\n' "$name" "$delimiter" "${!name}" "$delimiter" >>"$GITHUB_ENV"
  done
}

if ! all_set "${signing[@]}"; then
  echo "::notice::macOS signing skipped: set ${signing[*]} to sign and notarize the macOS bundle."
  exit 0
fi
export_to_github_env "${signing[@]}"
echo "macOS signing enabled."

if all_set "${notarization[@]}"; then
  export_to_github_env "${notarization[@]}"
  echo "macOS notarization enabled."
else
  echo "::notice::macOS notarization skipped: set ${notarization[*]} to notarize."
fi
