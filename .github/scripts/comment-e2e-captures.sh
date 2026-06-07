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
ROOT_DIR="$PWD"

if ! ls "$CAPTURES_DIR"/*.png >/dev/null 2>&1; then
  echo "No captures found in $CAPTURES_DIR; skipping PR comment."
  exit 0
fi

# --- 1. Publish captures to the dedicated branch (single orphan commit) ---
WORKTREE="$(mktemp -d)"
REMOTE_URL="https://x-access-token:${GH_TOKEN}@github.com/${GITHUB_REPOSITORY}.git"

git init -q "$WORKTREE"
cd "$WORKTREE"
git config user.name "github-actions[bot]"
git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
git remote add origin "$REMOTE_URL"

# Start from the current branch tree when it exists so other PR directories
# survive, but always create a fresh parentless commit and force-push:
# the branch history stays at exactly one commit.
if git fetch -q --depth 1 origin "$BRANCH" 2>/dev/null; then
  git checkout -q FETCH_HEAD
fi
git checkout -q --orphan "$BRANCH"

PR_DIR="pr-${PR_NUMBER}"
rm -rf "$PR_DIR"
mkdir -p "$PR_DIR"
cp "$ROOT_DIR/$CAPTURES_DIR"/*.png "$PR_DIR/"

git add -A
git commit -q -m "e2e captures (auto-generated, do not edit)"
git push -q --force origin "$BRANCH"
cd "$ROOT_DIR"
rm -rf "$WORKTREE"
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
