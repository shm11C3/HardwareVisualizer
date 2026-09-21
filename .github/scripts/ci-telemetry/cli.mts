// Side-effect-free helpers pulled out of aggregate.mts so the test suite can
// import them without an entry-point guard: aggregate.mts always runs
// main() (see its own comment on why), so anything the tests need to call
// in isolation has to live somewhere that merely importing it does not
// perform I/O. No fs/network access here, same rule as markers.mts,
// record.mts, and render.mts.

export type CliArgs = {
  dryRun: boolean;
  allLogs: boolean;
  runId: string | null;
};

export function parseArgs(argv: string[]): CliArgs {
  const args: CliArgs = { dryRun: false, allLogs: false, runId: null };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--dry-run") {
      args.dryRun = true;
    } else if (arg === "--all-logs") {
      args.allLogs = true;
    } else if (arg === "--run-id") {
      args.runId = argv[i + 1] ?? null;
      i += 1;
    }
  }
  return args;
}

export function resolveRunId(cliRunId: string | null | undefined): string {
  const value = cliRunId || process.env["RUN_ID"];
  if (!value || !/^\d{1,20}$/.test(value)) {
    throw new Error(
      `RUN_ID must be a 1-20 digit numeric string (env RUN_ID or --run-id), got: ${JSON.stringify(value)}`,
    );
  }
  return value;
}

// When multiple workflow runs share a head_sha (e.g. a re-run), only the
// most recently created run per workflow_id is kept — otherwise the comment
// would show a stale, superseded run alongside the current one. Generic so
// it works over both the real GitHubRun shape (aggregate.mts) and minimal
// test fixtures that only carry workflow_id/created_at.
export function keepLatestRunPerWorkflow<
  T extends { workflow_id: number; created_at: string | null },
>(runs: readonly T[]): T[] {
  const latestByWorkflow = new Map<number, T>();
  for (const run of runs) {
    const existing = latestByWorkflow.get(run.workflow_id);
    // String(...) mirrors Date.parse's own ToString coercion of a null
    // created_at (Date.parse(null) === Date.parse("null") === NaN), so a
    // missing timestamp compares as "not newer" exactly as the untyped
    // original did.
    if (
      !existing ||
      Date.parse(String(run.created_at)) >
        Date.parse(String(existing.created_at))
    ) {
      latestByWorkflow.set(run.workflow_id, run);
    }
  }
  return [...latestByWorkflow.values()];
}

// GET /actions/runs?head_sha=...&event=pull_request (used to find every
// workflow run for the PR's head commit) is eventually consistent: a page
// fetched right after the triggering run finishes can still omit it, or
// come back empty entirely (observed live: a --dry-run against a real run
// id printed "Found 0 workflow run(s)" on one call and 5 on an immediate
// retry against the identical head_sha). The triggering run itself was
// already fetched directly by id, so its existence is not in question —
// only whether the listing happened to include it yet. Appending it here
// guarantees the run that triggered the aggregation can never be missing
// from the rendered comment, without ever duplicating a run the listing did
// include. Generic so it works over both the real GitHubRun shape
// (aggregate.mts) and minimal test fixtures, matching
// keepLatestRunPerWorkflow's own style.
export function includeTriggerRun<T extends { id: number }>(
  runs: readonly T[],
  triggerRun: T,
): T[] {
  const alreadyListed = runs.some((run) => run.id === triggerRun.id);
  return alreadyListed ? [...runs] : [...runs, triggerRun];
}
