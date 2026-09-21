#!/usr/bin/env bash
#
# Keeps a snapshot ref under refs/backup/<name> before destructive git operations.
# Same behavior as scripts/snapshot-ref.ps1. Run on Linux/macOS.
#
# Usage:
#   bash scripts/snapshot-ref.sh <name> [commit]
set -euo pipefail

name="${1:-}"
commit="${2:-HEAD}"

if [ -z "$name" ]; then
    echo "usage: $0 <name> [commit]" >&2
    exit 1
fi

if ! [[ "$name" =~ ^[a-zA-Z0-9_./-]+$ ]]; then
    echo "error: Invalid snapshot name '$name' (allowed: letters, digits, _ . / -)" >&2
    exit 1
fi

full="refs/backup/$name"
echo "==> Snapshot $full -> $commit"
git update-ref "$full" "$commit" || exit 1

git rev-parse --short "$full" || exit 1