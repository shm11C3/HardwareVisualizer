# CI Telemetry

CI telemetry makes degradation of the development infrastructure visible:
runner queue time, job and step duration, runner CPU / memory / disk pressure,
and cache effectiveness. Each pull request gets one sticky comment that
summarizes every workflow that ran for its head commit, and every aggregated
workflow run produces a versioned JSON run record.

## Data flow

```text
CI job                                  CI telemetry workflow (workflow_run)
  Start CI telemetry (main)               GitHub REST API: runs, jobs, steps
    -> detached sampler                   job logs -> *_METRICS_JSON markers
  setup-rust / setup-node / sccache       -> run records (JSON, schema 1)
    -> cache markers                      -> sticky PR comment
  Start CI telemetry (post)               -> records artifact
    -> CI_JOB_METRICS_JSON marker
```

Owners:

- `.github/actions/ci-telemetry/` samples the runner and emits
  `CI_JOB_METRICS_JSON`.
- Each cache owner emits its own marker: `setup-rust` emits
  `RUST_CACHE_METRICS_JSON`, `setup-node` emits `NODE_CACHE_METRICS_JSON`, and
  `cache-sccache/report` emits `CACHE_METRICS_JSON`.
- `.github/scripts/ci-telemetry/` validates markers, builds run records, and
  renders the comment. `.github/workflows/ci-telemetry.yml` runs it.

Timing needs no instrumentation. Queue time (job `created_at` to
`started_at`), job duration, and step duration come from the jobs API for
every workflow, including workflows that do not run the sampler.

## Marker contract

A marker is an ordinary stdout line of the form `NAME_METRICS_JSON=<json>`.

- Workflow commands such as `::notice` are consumed by the runner and do not
  survive as data in raw job logs. A marker must be a plain line.
- The runner echoes every `run:` script into the log before executing it.
  Shell emitters therefore build the marker name with
  `printf '%s_METRICS_JSON=%s\n' RUST_CACHE "$json"` so the literal name never
  appears in the echoed script. The aggregator also scans every occurrence and
  uses the last one that parses and validates.
- The step name `Start CI telemetry` is part of the contract. The aggregator
  downloads logs only for jobs that contain a step with exactly this name,
  which keeps API usage proportional to instrumented jobs.
- Marker field names are a storage contract. Change
  `.github/actions/ci-telemetry/metrics.mts` and
  `.github/scripts/ci-telemetry/markers.mts` together (both conform to the
  shared types in `.github/scripts/ci-telemetry/schema.mts`, so a field
  rename on either side without updating the other is a typecheck failure),
  and bump `schema` for an incompatible change.

Unavailable data is `null`, never `0`. A job with fewer than two samples
reports `cpu_pct: null` so a missing measurement cannot look like an idle
runner.

## Trust boundary

The aggregator is triggered by `workflow_run`, so it runs the workflow file and
scripts from the default branch. Its write token, and any storage credential
added later, never reach code controlled by a pull request. This also makes
the comment work for fork and Dependabot pull requests.

That is safe only while these hold:

- The workflow checks out the default branch. It must never check out or
  execute `workflow_run.head_sha`.
- Everything a pull request can influence is data: job, step, and workflow
  names, branch names, and log contents. A pull request can print a forged
  marker.
- Markers pass a whitelist schema. Only numbers, booleans, and enums survive;
  free-form marker strings are neither rendered nor stored.
- Names from the API are rendered only inside a code span with backticks,
  pipes, and newlines removed, which neutralizes mentions, links, HTML, and
  table breakage.
- Inside raw HTML such as `<summary>`, entity-escaping is not enough. GitHub
  still turns `@user`, `#123`, and bare URLs there into live links and
  notifications, and skips only text inside `<code>`, `<pre>`, or `<a>`. The
  escaped name is therefore wrapped in `<code>`.

A forged marker can therefore distort the numbers shown for its own pull
request, but it cannot inject content or reach a secret. Run records keep
`is_fork`, `head_repository`, and `actor` so later analysis can choose which
runs to trust as a baseline.

String assertions cannot prove how GitHub renders a comment. When changing
`render.mts`, render a hostile fixture through GitHub's side-effect-free
Markdown API and inspect the HTML:

```bash
jq -n --rawfile t comment.md \
  '{text:$t, mode:"gfm", context:"shm11C3/HardwareVisualizer"}' \
  | gh api markdown --input -
```

## Reading the comment

- **Longest queue / Queue**: time from job creation to start. This is runner
  availability, not repository code. macOS runners usually dominate it.
- **CPU avg / p95**: utilization of the whole runner VM. The average is
  weighted by elapsed time across the job. A low average on a long job points
  at serialized or I/O-bound work rather than missing capacity.
- **Mem peak**: peak used memory against total. On Linux this is
  `MemTotal - MemAvailable`; on macOS it is active + wired + compressor pages
  from `vm_stat`, because `os.freemem()` counts only the free list and makes
  macOS look permanently full; on Windows it is total minus available
  physical memory.
- **Disk peak**: peak used space on the workspace volume. Watch this before
  `No space left on device` becomes a job failure.
- **Caches**: `target exact` is an exact rust-cache key hit. A non-exact
  restore is labeled `target partial/miss` because rust-cache does not report
  whether a restore-key prefix matched. sccache shows hit rate and hit/request
  counts per language, plus a warning when the backend reported errors.

The aggregator is stateless and idempotent per head commit. Every invocation
re-renders the complete comment from the API, because GitHub cancels older
pending runs in a concurrency group and any single invocation may be the only
one that runs.

## Add telemetry to a job

Add the step immediately after checkout. The action is local, so checkout must
come first, and registering it early makes its post step run after the other
post steps (sccache report, rust-cache save) so the sampling window covers
them.

```yaml
- name: Start CI telemetry
  uses: ./.github/actions/ci-telemetry
```

Do not add it to `ubuntu-slim` jobs. Those are single-vCPU containers where
the numbers carry no signal. To have a new workflow appear with job details,
also add its `name` to the `workflow_run.workflows` list in
`ci-telemetry.yml`; workflows that ran for the same commit appear in the top
table without that.

Telemetry must never fail or hang a job. Every entry point downgrades its own
failure to a warning.

## Verify locally

The aggregator uses the same code path locally and in Actions. `--dry-run`
rejects every non-GET request.

```bash
GITHUB_TOKEN="$(gh auth token)" GITHUB_REPOSITORY=shm11C3/HardwareVisualizer \
  node .github/scripts/ci-telemetry/aggregate.mts --run-id <run id> --dry-run
```

`--all-logs` downloads logs for every executed job instead of only
instrumented jobs. Use it for runs that predate the sampler but already
contain cache markers.

```bash
node .github/scripts/test-ci-telemetry-action.mts
node .github/scripts/test-ci-telemetry-aggregate.mts
npm run typecheck:ci-telemetry
```

The `workflow_run` trigger only becomes active once `ci-telemetry.yml` is on
the default branch. After that, `workflow_dispatch` with a `run-id` re-renders
the comment for that run's head commit.

## Why not

- **A third-party telemetry action**: the common option sends samples to an
  external chart-rendering service and produces images instead of
  machine-readable data. That conflicts with this repository's pinned,
  dependency-minimal CI and gives nothing to accumulate.
- **Artifacts instead of log markers**: sccache statistics exist only in a
  post step, regular steps such as `upload-artifact` run before post steps,
  and uploading from a JavaScript post step requires bundling
  `@actions/artifact`. Log markers keep the local actions dependency-free.
- **Commenting from the CI job itself**: fork and Dependabot pull requests
  receive a read-only token there, and storage credentials would have to be
  exposed to pull-request-controlled code.
- **One workflow-run logs archive instead of per-job logs**: the archive names
  files by job name rather than job id, so joining it to the jobs API would
  depend on names a pull request controls.
