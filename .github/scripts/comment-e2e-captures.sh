#!/usr/bin/env bash
# Publish E2E capture PNGs to the `e2e-captures` branch and upsert a sticky
# PR comment that embeds them inline.
#
# Required environment:
#   GH_TOKEN          - token with contents:write + pull-requests:write
#   GITHUB_REPOSITORY - owner/repo
#   PR_NUMBER         - pull request number
#   HEAD_SHA          - PR head commit SHA (used for cache busting)
# Optional:
#   CAPTURES_DIR      - capture source dir (default: test-results/captures)
#   CAPTURES_BRANCH   - publishing branch (default: e2e-captures)
set -euo pipefail

CAPTURES_DIR="${CAPTURES_DIR:-test-results/captures}"
BRANCH="${CAPTURES_BRANCH:-e2e-captures}"
PR_DIR="pr-${PR_NUMBER}"
ROOT_DIR="$PWD"
REMOTE_URL="https://x-access-token:${GH_TOKEN}@github.com/${GITHUB_REPOSITORY}.git"

if ! ls "$CAPTURES_DIR"/*.png >/dev/null 2>&1; then
  echo "No captures found in $CAPTURES_DIR; skipping PR comment."
  exit 0
fi

# --- 1. Publish captures to the dedicated branch ---
#
# The branch is a single parentless commit shared by every PR (each under its
# own pr-<number>/ directory). Concurrent runs therefore race on it, so build
# the new tree from a fresh fetch and push with --force-with-lease: if another
# run advanced the branch between our fetch and push, the lease fails and we
# retry on the new tip instead of clobbering its captures.
publish_captures() {
  local worktree attempt lease
  for attempt in 1 2 3 4 5; do
    worktree="$(mktemp -d)"
    git init -q "$worktree"
    git -C "$worktree" config user.name "github-actions[bot]"
    git -C "$worktree" config user.email "41898282+github-actions[bot]@users.noreply.github.com"
    git -C "$worktree" remote add origin "$REMOTE_URL"

    # Fetch into the remote-tracking ref so --force-with-lease has an anchor.
    if git -C "$worktree" fetch -q --depth 1 \
      origin "$BRANCH:refs/remotes/origin/$BRANCH" 2>/dev/null; then
      git -C "$worktree" checkout -q "refs/remotes/origin/$BRANCH"
      lease="--force-with-lease=$BRANCH:refs/remotes/origin/$BRANCH"
    else
      lease="--force" # branch does not exist yet
    fi

    # Replace only this PR's directory; other PRs' captures carry over.
    git -C "$worktree" checkout -q --orphan "$BRANCH"
    rm -rf "${worktree:?}/$PR_DIR"
    mkdir -p "$worktree/$PR_DIR"
    cp "$ROOT_DIR/$CAPTURES_DIR"/*.png "$worktree/$PR_DIR/"

    git -C "$worktree" add -A
    git -C "$worktree" commit -q -m "e2e captures (auto-generated, do not edit)"

    if git -C "$worktree" push -q "$lease" origin "$BRANCH"; then
      rm -rf "$worktree"
      return 0
    fi

    echo "Push rejected (attempt ${attempt}/5); refetching and retrying..."
    rm -rf "$worktree"
    sleep "$((attempt * 2))"
  done
  return 1
}

if ! publish_captures; then
  echo "Failed to publish captures after retries; skipping PR comment." >&2
  exit 0
fi
echo "Published captures to ${BRANCH}/${PR_DIR}."

# --- 2. Upsert the sticky PR comment ---
MARKER="<!-- e2e-captures-comment -->"
RAW_BASE="https://raw.githubusercontent.com/${GITHUB_REPOSITORY}/${BRANCH}/${PR_DIR}"

BODY="${MARKER}
## E2E captures

Evidence captures for \`${HEAD_SHA}\` (also available as the \`e2e-captures\` workflow artifact).
"
for file in "$CAPTURES_DIR"/*.png; do
  name="$(basename "$file" .png)"
  BODY+="
<details><summary><code>${name}</code></summary>

![${name}](${RAW_BASE}/${name}.png?rev=${HEAD_SHA})

</details>
"
done

COMMENT_ID="$(gh api "repos/${GITHUB_REPOSITORY}/issues/${PR_NUMBER}/comments" \
  --paginate --jq "map(select(.body | startswith(\"${MARKER}\"))) | .[0].id // empty")"

if [ -n "$COMMENT_ID" ]; then
  gh api -X PATCH "repos/${GITHUB_REPOSITORY}/issues/comments/${COMMENT_ID}" \
    -f body="$BODY" >/dev/null
  echo "Updated capture comment ${COMMENT_ID}."
else
  gh api -X POST "repos/${GITHUB_REPOSITORY}/issues/${PR_NUMBER}/comments" \
    -f body="$BODY" >/dev/null
  echo "Posted capture comment."
fi
