#!/usr/bin/env bash
# Auto-resolve rebase conflicts by always preferring our canonical files
set -e

REPO_ROOT="$(git rev-parse --show-toplevel)"

# Canonical file contents are written fresh each iteration.
# We rely on the fact that our lib.rs/main.rs/Cargo.toml are already
# the desired final state — we just re-stage them after each conflict.

resolve_conflicts() {
    local conflicted
    conflicted=$(git diff --name-only --diff-filter=U 2>/dev/null)

    if [ -z "$conflicted" ]; then
        return 0
    fi

    echo ">>> Conflicted files: $conflicted"

    for f in $conflicted; do
        case "$f" in
            "tooling/sanctifier-core/src/lib.rs")
                echo "    Resolving $f with HEAD version (ours)"
                git checkout --ours "$f"
                # Remove any leftover conflict markers (ours should be clean)
                git add "$f"
                ;;
            "tooling/sanctifier-cli/src/main.rs")
                echo "    Resolving $f with HEAD version (ours)"
                git checkout --ours "$f"
                git add "$f"
                ;;
            "tooling/sanctifier-cli/Cargo.toml")
                echo "    Resolving $f with HEAD version (ours)"
                git checkout --ours "$f"
                git add "$f"
                ;;
            *)
                echo "    Resolving $f: attempting ours"
                git checkout --ours "$f" 2>/dev/null && git add "$f" || \
                    (git checkout --theirs "$f" && git add "$f")
                ;;
        esac
    done

    # Handle modify/delete conflicts (files deleted in one side)
    local deleted
    deleted=$(git status --short | grep "^DU\|^UD" | awk '{print $2}')
    for f in $deleted; do
        echo "    Removing deleted-in-one-side file: $f"
        git rm "$f" 2>/dev/null || true
    done
}

MAX_ITERATIONS=40
i=0

while [ $i -lt $MAX_ITERATIONS ]; do
    i=$((i+1))
    echo "=== Rebase iteration $i ==="

    status=$(git status --short 2>&1)
    echo "$status"

    if echo "$status" | grep -q "^UU\|^AA\|^DU\|^UD"; then
        resolve_conflicts
        GIT_EDITOR=true git rebase --continue 2>&1 || true
    else
        echo "No conflicts detected, checking rebase state..."
        if [ -d "$REPO_ROOT/.git/rebase-merge" ] || [ -d "$REPO_ROOT/.git/rebase-apply" ]; then
            GIT_EDITOR=true git rebase --continue 2>&1 || true
        else
            echo "Rebase complete!"
            break
        fi
    fi

    sleep 1
done

echo "=== Final status ==="
git log --oneline -5
