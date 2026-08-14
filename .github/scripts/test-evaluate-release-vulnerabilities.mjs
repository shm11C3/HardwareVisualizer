import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = join(
  dirname(fileURLToPath(import.meta.url)),
  "evaluate-release-vulnerabilities.ts",
);
const testDir = mkdtempSync(
  join(tmpdir(), "release-vulnerability-evaluation-"),
);

function vulnerability({ id, aliases = [], score = "7.5", fixed = "2.0.0" }) {
  return {
    id,
    aliases,
    severity: [{ type: "CVSS_V3", score }],
    affected: [
      {
        ranges: [
          {
            type: "ECOSYSTEM",
            events: [{ introduced: "0" }, ...(fixed ? [{ fixed }] : [])],
          },
        ],
      },
    ],
  };
}

function packageResult({
  name = "runtime-package",
  version = "1.0.0",
  ecosystem = "npm",
  vulnerabilities,
  dependencyGroups = [],
  groups,
}) {
  return {
    package: { name, version, ecosystem },
    dependency_groups: dependencyGroups,
    vulnerabilities,
    groups:
      groups ??
      vulnerabilities.map((item) => ({
        ids: [item.id],
        aliases: [item.id, ...(item.aliases ?? [])],
        max_severity: item.severity?.[0]?.score ?? "",
      })),
  };
}

function assessment(overrides = {}) {
  return {
    advisory_id: "RUSTSEC-2026-0195",
    release_tag: "v1.9.2",
    package: {
      ecosystem: "crates.io",
      name: "quick-xml",
      version: "0.39.2",
    },
    runtime_exposure: "not_reachable",
    automatable: "no",
    technical_impact: "partial",
    build_artifact_tainted: false,
    reason: "The affected API is not used by the release.",
    reviewed_on: "2026-08-14",
    review_after: "2026-11-14",
    ...overrides,
  };
}

function runEvaluation({
  packages,
  assessments = [],
  kev = ["CVE-2099-0001"],
}) {
  const resultsPath = join(testDir, "results.json");
  const policyPath = join(testDir, "policy.json");
  const kevPath = join(testDir, "kev.json");
  const outputPath = join(testDir, "evaluation.json");
  rmSync(outputPath, { force: true });
  const results = {
    results: [
      {
        source: { path: "/workspace/package-lock.json", type: "lockfile" },
        packages,
      },
    ],
  };

  writeFileSync(resultsPath, `${JSON.stringify(results, null, 2)}\n`);
  writeFileSync(
    policyPath,
    `${JSON.stringify({ schema_version: 1, assessments }, null, 2)}\n`,
  );
  writeFileSync(
    kevPath,
    `${JSON.stringify({
      catalogVersion: "test",
      dateReleased: "2026-08-14T00:00:00Z",
      vulnerabilities: kev.map((cveID) => ({ cveID })),
    })}\n`,
  );

  const result = spawnSync(
    process.execPath,
    [
      "--experimental-strip-types",
      scriptPath,
      resultsPath,
      "--policy",
      policyPath,
      "--kev",
      kevPath,
      "--output",
      outputPath,
    ],
    {
      encoding: "utf8",
      env: {
        ...process.env,
        OSV_EVALUATION_DATE: "2026-08-14",
        OSV_RELEASE_TAG: "v1.9.2",
      },
    },
  );

  const evaluation = existsSync(outputPath)
    ? JSON.parse(readFileSync(outputPath, "utf8"))
    : null;
  return { ...result, evaluation };
}

function assertDecision(result, status, decision, reasonCode) {
  assert.equal(result.status, status, result.stderr);
  assert.ok(result.evaluation, result.stderr);
  assert.equal(result.evaluation.findings.length, 1);
  assert.equal(result.evaluation.findings[0].decision, decision);
  if (reasonCode) {
    assert.equal(result.evaluation.findings[0].reason_code, reasonCode);
  }
}

try {
  assertDecision(
    runEvaluation({
      packages: [
        packageResult({
          name: "nanoid",
          version: "3.3.12",
          dependencyGroups: ["dev"],
          vulnerabilities: [vulnerability({ id: "GHSA-dev-only" })],
        }),
      ],
    }),
    0,
    "maintenance",
    "development_only",
  );

  assertDecision(
    runEvaluation({
      packages: [
        packageResult({
          vulnerabilities: [vulnerability({ id: "GHSA-needs-triage" })],
        }),
      ],
    }),
    1,
    "triage_required",
    "high_or_critical_requires_review",
  );

  const quickXmlPackage = packageResult({
    name: "quick-xml",
    version: "0.39.2",
    ecosystem: "crates.io",
    vulnerabilities: [vulnerability({ id: "RUSTSEC-2026-0195" })],
  });
  assertDecision(
    runEvaluation({ packages: [quickXmlPackage], assessments: [assessment()] }),
    0,
    "maintenance",
    "not_reachable",
  );

  assertDecision(
    runEvaluation({
      packages: [quickXmlPackage],
      assessments: [
        assessment({
          runtime_exposure: "reachable",
          automatable: "yes",
          technical_impact: "total",
        }),
      ],
    }),
    1,
    "emergency_release_candidate",
    "reachable_automatable_total_impact",
  );

  assertDecision(
    runEvaluation({
      packages: [quickXmlPackage],
      assessments: [assessment({ build_artifact_tainted: true })],
    }),
    1,
    "emergency_release_candidate",
    "build_artifact_tainted",
  );

  const expired = runEvaluation({
    packages: [quickXmlPackage],
    assessments: [
      assessment({ reviewed_on: "2026-08-01", review_after: "2026-08-13" }),
    ],
  });
  assertDecision(
    expired,
    1,
    "triage_required",
    "high_or_critical_requires_review",
  );
  assert.equal(expired.evaluation.findings[0].assessment_status, "expired");

  assertDecision(
    runEvaluation({
      packages: [
        packageResult({
          vulnerabilities: [
            vulnerability({
              id: "GHSA-known-exploited",
              aliases: ["CVE-2026-12345"],
            }),
          ],
        }),
      ],
      kev: ["CVE-2026-12345"],
    }),
    1,
    "emergency_release_candidate",
    "known_exploited",
  );

  assertDecision(
    runEvaluation({
      packages: [
        packageResult({
          vulnerabilities: [
            vulnerability({
              id: "GHSA-known-exploited-no-fix",
              aliases: ["CVE-2026-12346"],
              fixed: "",
            }),
          ],
        }),
      ],
      kev: ["CVE-2026-12346"],
    }),
    1,
    "emergency_mitigation_candidate",
    "known_exploited",
  );

  const groupedAliases = runEvaluation({
    packages: [
      packageResult({
        name: "rand",
        version: "0.7.3",
        ecosystem: "crates.io",
        vulnerabilities: [
          vulnerability({
            id: "RUSTSEC-2026-0097",
            aliases: ["GHSA-cq8v-f236-94qc"],
            score: "5.0",
          }),
          vulnerability({
            id: "GHSA-cq8v-f236-94qc",
            aliases: ["RUSTSEC-2026-0097"],
            score: "5.0",
          }),
        ],
        groups: [
          {
            ids: ["GHSA-cq8v-f236-94qc", "RUSTSEC-2026-0097"],
            aliases: ["GHSA-cq8v-f236-94qc", "RUSTSEC-2026-0097"],
            max_severity: "5.0",
          },
        ],
      }),
    ],
  });
  assert.equal(groupedAliases.status, 0, groupedAliases.stderr);
  assert.equal(groupedAliases.evaluation.findings.length, 1);
  assert.equal(groupedAliases.evaluation.summary.total, 1);

  console.log("Release vulnerability evaluation tests passed.");
} finally {
  rmSync(testDir, { recursive: true, force: true });
}
