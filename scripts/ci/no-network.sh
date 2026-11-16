#!/usr/bin/env bash
# Runtime half of the no-network guarantee (architecture §12). Every test binary, and every
# command in SCENARIO_COMMANDS, runs twice:
#   1. under strace: any AF_INET/AF_INET6 socket, connect, sendto or sendmsg fails the check;
#   2. inside `unshare -rn` (new user and network namespace, not even loopback): it must still pass.
#
# Usage: scripts/ci/no-network.sh <file>
#   <file> is the output of `cargo test --workspace --no-run --message-format=json`.
#   strace logs go to $NO_NETWORK_LOG_DIR (default: target/no-network).
set -euo pipefail

# Extra command lines to check, run with `bash -c` from the repo root.
# The engine's scripted scenario binary goes here once it exists.
SCENARIO_COMMANDS=()

if [[ "$(uname -s)" != Linux ]]; then
  echo "no-network.sh: Linux only (strace and unshare -rn do not exist on $(uname -s)). CI runs it on ubuntu-latest." >&2
  exit 2
fi

if [[ $# -ne 1 || ! -f "$1" ]]; then
  echo "usage: $0 <output of: cargo test --workspace --no-run --message-format=json>" >&2
  exit 2
fi

for tool in strace unshare jq; do
  command -v "$tool" >/dev/null || { echo "no-network.sh: '$tool' is not installed" >&2; exit 2; }
done

if ! unshare -rn true 2>/dev/null; then
  echo "no-network.sh: 'unshare -rn' is not permitted here. On Ubuntu 24.04 and later run:" >&2
  echo "  sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=0" >&2
  exit 2
fi

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
log_dir=${NO_NETWORK_LOG_DIR:-$repo_root/target/no-network}
mkdir -p "$log_dir"

failures=()

# check <label> <working dir> <command...>
check() {
  local label=$1 dir=$2
  shift 2
  local log="$log_dir/$label.strace"

  echo "::group::$label"
  if ! (cd "$dir" && strace -f -qq -o "$log" -e trace=socket,connect,sendto,sendmsg -- "$@"); then
    failures+=("$label failed under strace")
  fi
  if grep -Ew 'AF_INET6?' "$log"; then
    failures+=("$label used an AF_INET/AF_INET6 socket, see $log")
  fi
  if ! (cd "$dir" && unshare -rn -- "$@"); then
    failures+=("$label failed without network access")
  fi
  echo "::endgroup::"
}

binaries=0
while IFS=$'\t' read -r executable manifest; do
  binaries=$((binaries + 1))
  dir=$(dirname "$manifest")
  # cargo runs test binaries from the package root with CARGO_MANIFEST_DIR set; do the same.
  CARGO_MANIFEST_DIR=$dir check "$(basename "$executable")" "$dir" "$executable"
done < <(jq -r 'select(.reason == "compiler-artifact" and .profile.test and .executable != null)
                | [.executable, .manifest_path] | @tsv' "$1")

if ((binaries == 0)); then
  echo "no-network.sh: no test binaries in $1" >&2
  exit 1
fi

for i in "${!SCENARIO_COMMANDS[@]}"; do
  check "scenario-$i" "$repo_root" bash -c "${SCENARIO_COMMANDS[$i]}"
done

if ((${#failures[@]})); then
  printf '::error::%s\n' "${failures[@]}"
  exit 1
fi
echo "no-network: $binaries test binaries and ${#SCENARIO_COMMANDS[@]} scenario commands passed without AF_INET/AF_INET6."
