#!/usr/bin/env bash
#
# Quick release pipeline: analyze changes, auto-commit (grouped), push main, tag.
# Native bash counterpart of scripts/release.ps1 — same steps. Run on Linux/macOS.
#
#   1. Runs the 4 CI steps (bash scripts/ci.sh) - can be skipped.
#   2. Analyzes pending changes and commits them with Conventional Commits messages,
#      splitting into multiple commits per area (ci / docs / src / tooling / other).
#   3. Pulls origin/main (fast-forward) and pushes main.
#   4. Creates a tag (auto patch bump from the latest tag when no version given) and pushes it.
#
# Usage:
#   bash scripts/release.sh "feat: ..." [--version v0.2.0] [--skip-checks] [--dry-run]
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

MESSAGE=""
VERSION=""
SKIP_CHECKS=0
DRYRUN=0

usage() {
    echo "usage: $0 [message] [--version vX.Y.Z] [--skip-checks] [--dry-run]" >&2
}

args=("$@")
i=0
while [ "$i" -lt "${#args[@]}" ]; do
    case "${args[$i]}" in
        --skip-checks) SKIP_CHECKS=1 ;;
        --dry-run) DRYRUN=1 ;;
        --version)
            i=$((i + 1))
            if [ "$i" -ge "${#args[@]}" ]; then
                echo "error: --version requires a value" >&2
                exit 1
            fi
            VERSION="${args[$i]}"
            ;;
        -h|--help) usage; exit 0 ;;
        *)
            if [ -z "$MESSAGE" ]; then
                MESSAGE="${args[$i]}"
            else
                echo "error: unexpected argument '${args[$i]}'" >&2
                usage
                exit 1
            fi
            ;;
    esac
    i=$((i + 1))
done

# ---- Auto commit message analysis -------------------------------------------

bucket_of() {
    case "$1" in
        .github/*) echo "ci" ;;
        *.md) echo "docs" ;;
        src/*.rs) echo "rust" ;;
        opencode.json|.opencode/*|.vscode/*) echo "tooling" ;;
        *) echo "other" ;;
    esac
}

declare -a b_ci=() bx_ci=()
declare -a b_docs=() bx_docs=()
declare -a b_rust=() bx_rust=()
declare -a b_tooling=() bx_tooling=()
declare -a b_other=() bx_other=()

add_to() {
    case "$1" in
        ci) b_ci+=("$2"); bx_ci+=("$3") ;;
        docs) b_docs+=("$2"); bx_docs+=("$3") ;;
        rust) b_rust+=("$2"); bx_rust+=("$3") ;;
        tooling) b_tooling+=("$2"); bx_tooling+=("$3") ;;
        *) b_other+=("$2"); bx_other+=("$3") ;;
    esac
}

get_path() { eval "printf '%s' \"\${b_${1}[$2]:-}\""; }
get_x() { eval "printf '%s' \"\${bx_${1}[$2]:-}\""; }
get_count() { eval "printf '%s' \"\${#b_${1}[@]}\""; }

join_names() {
    local out="" f bn
    for f in "$@"; do
        bn="$(basename "$f")"
        [ -n "$out" ] && out="$out, "
        out="$out$bn"
    done
    printf '%s' "$out"
}

added_fns() {
    git diff --cached -U0 -- "$@" 2>/dev/null |
        sed -n -E 's/^\++[[:space:]]*(pub[[:space:]]+)?fn[[:space:]]+([A-Za-z0-9_]+).*/\2/p'
}

summarize_bucket() {
    local kind="$1" count i x deleted=0 added=0
    count="$(get_count "$kind")"
    [ "$count" -eq 0 ] && return 1
    local files=() names="" msg="" fns fns_list
    for ((i = 0; i < count; i++)); do
        files+=("$(get_path "$kind" "$i")")
    done
    names="$(join_names "${files[@]}")"
    for ((i = 0; i < count; i++)); do
        x="$(get_x "$kind" "$i")"
        [ "$x" = "D" ] && deleted=1
        [ "$x" = "A" ] && added=1
    done

    case "$kind" in
        ci) msg="ci: update workflow files" ;;
        docs)
            if [ "$deleted" = "1" ]; then msg="docs: remove $names"; else msg="docs: update documentation ($names)"; fi
            ;;
        rust)
            if [ "$deleted" = "1" ]; then
                msg="chore: remove source module ($names)"
            else
                fns="$(added_fns "${files[@]}")"
                if [ -n "$fns" ]; then
                    fns_list="$(printf '%s' "$fns" | tr '\n' ',' | sed 's/,$//')"
                    msg="feat: add $fns_list ($names)"
                elif [ "$added" = "1" ]; then
                    msg="feat: add module ($names)"
                else
                    msg="feat: update ($names)"
                fi
            fi
            ;;
        tooling) msg="chore: update tooling files ($names)" ;;
        *)
            if [ "$deleted" = "1" ]; then msg="chore: remove ($names)"; else msg="chore: update ($names)"; fi
            ;;
    esac
    printf '%s' "$msg"
}

commit_groups() {
    local type n i files msg
    for type in ci docs other rust tooling; do
        n="$(get_count "$type")"
        if [ "$n" -eq 0 ]; then continue; fi
        files=()
        for ((i = 0; i < n; i++)); do
            files+=("$(get_path "$type" "$i")")
        done
        echo "==> git add ${files[*]}"
        git add -- "${files[@]}" || exit 1
        msg="$(summarize_bucket "$type")"
        if [ "$DRYRUN" = "1" ]; then
            git reset -q -- "${files[@]}" || exit 1
            echo "==> [dryrun] commit ($type): $msg"
        else
            echo "==> commit ($type): $msg"
            git commit -q -m "$msg" || exit 1
        fi
    done
}

# ---- Main -------------------------------------------------------------------

# 1. Branch guard
current="$(git branch --show-current 2>/dev/null)" 
if [ "$current" != "main" ]; then
    echo "error: Must be on branch 'main' (currently: '$current')" >&2
    exit 1
fi

# 2. CI checks
if [ "$DRYRUN" = "1" ]; then
    echo "==> [dryrun] would run the 4 CI steps"
elif [ "$SKIP_CHECKS" = "0" ]; then
    echo "==> Running CI checks..."
    bash "$script_dir/ci.sh" || exit 1
else
    echo "==> Skipping CI checks."
fi

# 3. Commit pending changes
status_out="$(git status --porcelain || true)"
if [ -n "$status_out" ]; then
    while IFS= read -r line; do
        [ -z "$line" ] && continue
        x="${line:0:1}"
        y="${line:1:1}"
        [ "$x" = " " ] && x="$y"
        path="${line:3}"
        path="${path//\"/}"
        add_to "$(bucket_of "$path")" "$path" "$x"
    done <<< "$status_out"

    if [ -n "$MESSAGE" ]; then
        echo "==> Committing (single): $MESSAGE"
        if [ "$DRYRUN" = "1" ]; then
            echo "==> [dryrun] git add -A; git commit -m '$MESSAGE'"
        else
            git add -A || exit 1
            git commit -m "$MESSAGE" || exit 1
        fi
    else
        commit_groups
    fi
else
    echo "==> No changes to commit."
fi

# 4. Pull origin/main (fast-forward) and push
echo "==> git pull --ff-only origin main; git push origin main"
if [ "$DRYRUN" = "0" ]; then
    git pull --ff-only origin main || exit 1
    git push origin main || exit 1
fi

# 5. Determine version and tag
if [ -z "$VERSION" ]; then
    last="$(git describe --tags --abbrev=0 2>/dev/null || echo "v0.0.0")"
    v="${last#v}"
    IFS=. read -r maj min pat <<< "$v"
    maj="${maj:-0}"
    min="${min:-0}"
    pat="${pat:-0}"
    VERSION="v${maj}.${min}.$((pat + 1))"
fi
echo "==> Tagging: $VERSION"
if [ "$DRYRUN" = "0" ]; then
    git tag "$VERSION" || exit 1
    git push origin "$VERSION" || exit 1
fi

echo ""
echo "Done. Released $VERSION on main."
if [ "$DRYRUN" = "1" ]; then echo "(dry run - nothing was changed)"; fi