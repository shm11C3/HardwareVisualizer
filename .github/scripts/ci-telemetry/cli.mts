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
