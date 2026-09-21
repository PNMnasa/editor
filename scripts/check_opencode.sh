#!/usr/bin/env bash
#
# Validates the allowlist invariants of the opencode bash permissions.
# Same checks as scripts/check_opencode.ps1, native bash + jq. Run on Linux/macOS.
#
# Checks opencode.json in the repo root:
#   1. JSON parses.
#   2. permission.bash exists and every value is allow/ask/deny.
#   3. The catch-all "*" is the FIRST key (last matching rule wins) and is NOT "allow".
#   4. At least one "allow" rule exists.
#
# Usage:
#   bash scripts/check_opencode.sh
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cfg="$script_dir/../opencode.json"

if [ ! -f "$cfg" ]; then
    echo "error: opencode.json not found at $cfg" >&2
    exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
    echo "error: jq is required (install jq, e.g. apt-get/brew install jq, or use scripts/check_opencode.ps1 on Windows)" >&2
    exit 1
fi

if ! jq -e . "$cfg" >/dev/null 2>&1; then
    echo "error: opencode.json is not valid JSON" >&2
    exit 1
fi

if [ "$(jq -r '.permission.bash | type' "$cfg")" != "object" ]; then
    echo "error: permission.bash missing in opencode.json" >&2
    exit 1
fi

first="$(jq -r '.permission.bash | to_entries[0].key' "$cfg")"
if [ "$first" != "*" ]; then
    echo "error: Rule order broken: catch-all '*' must be the FIRST key (last matching rule wins). First key is '$first'" >&2
    exit 1
fi

catchall="$(jq -r '.permission.bash["*"]' "$cfg")"
if [ "$catchall" = "allow" ]; then
    echo "error: Catch-all '*' must not be 'allow' (would auto-run every command)" >&2
    exit 1
fi

bad="$(jq -r '
    .permission.bash
    | to_entries[]
    | select(.value as $v | ["allow", "ask", "deny"] | index($v) | not)
    | .key
' "$cfg")"
if [ -n "$bad" ]; then
    echo "error: rule(s) with unknown action: $(echo "$bad" | tr '\n' ' ')" >&2
    exit 1
fi

allow_count="$(jq -r '[.permission.bash[] | select(. == "allow")] | length' "$cfg")"
if [ "$allow_count" -eq 0 ]; then
    echo "error: permission.bash has no 'allow' rules (allowlist is empty)" >&2
    exit 1
fi

echo "opencode.json OK: catch-all '*'=$catchall, $allow_count allow rules, $(jq -r '.permission.bash | length' "$cfg") rules total."