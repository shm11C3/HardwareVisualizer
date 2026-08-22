# Review Data Retrieval

Prefer thread-aware data over flat comments: resolution and outdated state
matter. Fetch inline review threads, top-level PR comments, and review
submissions (bot summaries, approvals, nitpick-only reviews live there).

Tool order:

1. GitHub connector tools: `_list_pull_request_review_threads` for inline
   threads, `_fetch_pr_comments` for the merged PR timeline.
2. GitHub MCP server: `get_pull_request_comments` and `get_pull_request_reviews`;
   flat comments may not preserve thread resolution state.
3. `gh` CLI: `gh pr view` to resolve the PR, `gh api graphql` for
   `reviewThreads`, REST for `/pulls/{n}/comments`, `/pulls/{n}/reviews`,
   `/issues/{n}/comments`.

If auth or network fails, report the blocker and ask; do not guess from
incomplete data.
