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
  "evaluate-release-vulnerabilities.mjs",
);
const testDir = mkdtempSync(
  join(tmpdir(), "release-vulnerability-evaluation-"),
);

function affectedEntry({
  ecosystem = "npm",
  name = "runtime-package",
  fixed = "2.0.0",
} = {}) {
  return {
    package: { ecosystem, name },
    ranges: [
      {
        type: "ECOSYSTEM",
        events: [{ introduced: "0" }, ...(fixed ? [{ fixed }] : [])],
      },
    ],
  };
}

function vulnerability({
  id,
  aliases = [],
  score = "7.5",
  affected = [affectedEntry()],
}) {
  return {
    id,
    aliases,
    severity: [{ type: "CVSS_V3", score }],
    affected,
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
    exposure: "not_affected",
    reason: "The affected API is not used by this release.",
    reviewed_on: "2026-08-14",
    review_after: "2026-11-14",
    ...overrides,
  };
}

function runEvaluation({
  packages = [],
  assessments = [],
  kev = ["CVE-2099-0001"],
  results,
}) {
  const resultsPath = join(testDir, "results.json");
  const policyPath = join(testDir, "policy.json");
  const kevPath = join(testDir, "kev.json");
  const outputPath = join(testDir, "evaluation.json");
  rmSync(outputPath, { force: true });

  writeFileSync(
    resultsPath,
    `${JSON.stringify(
      results ?? {
        results: [
          {
            source: { path: "/workspace/package-lock.json", type: "lockfile" },
            packages,
          },
        ],
      },
      null,
      2,
    )}\n`,
  );
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
    [scriptPath, resultsPath, policyPath, kevPath, outputPath],
    {
      encoding: "utf8",
      env: {
        ...process.env,
        GITHUB_STEP_SUMMARY: "",
        OSV_EVALUATION_DATE: "2026-08-14",
        OSV_RELEASE_TAG: "v1.9.2",
      },
    },
  );

  return {
    ...result,
    evaluation: existsSync(outputPath)
      ? JSON.parse(readFileSync(outputPath, "utf8"))
      : null,
  };
}

function assertDecision(result, status, decision, reasonCode) {
  assert.equal(result.status, status, result.stderr);
  assert.ok(result.evaluation, result.stderr);
  assert.equal(result.evaluation.findings.length, 1);
  assert.equal(result.evaluation.findings[0].decision, decision);
  assert.equal(result.evaluation.findings[0].reason_code, reasonCode);
}

try {
  assertDecision(
    runEvaluation({
      packages: [
        packageResult({
          name: "nanoid",
          version: "3.3.12",
          dependencyGroups: ["dev"],
          vulnerabilities: [
            vulnerability({
              id: "GHSA-dev-only",
              affected: [affectedEntry({ name: "nanoid" })],
            }),
          ],
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
    vulnerabilities: [
      vulnerability({
        id: "RUSTSEC-2026-0195",
        affected: [
          affectedEntry({ ecosystem: "crates.io", name: "quick-xml" }),
        ],
      }),
    ],
  });
  assertDecision(
    runEvaluation({ packages: [quickXmlPackage], assessments: [assessment()] }),
    0,
    "maintenance",
    "not_affected",
  );

  assertDecision(
    runEvaluation({
      packages: [quickXmlPackage],
      assessments: [assessment({ exposure: "affected" })],
    }),
    1,
    "emergency_release_candidate",
    "affected_release",
  );

  const quickXmlWithoutFix = packageResult({
    name: "quick-xml",
    version: "0.39.2",
    ecosystem: "crates.io",
    vulnerabilities: [
      vulnerability({
        id: "RUSTSEC-2026-0195",
        affected: [
          affectedEntry({
            ecosystem: "crates.io",
            name: "quick-xml",
            fixed: "",
          }),
        ],
      }),
    ],
  });
  assertDecision(
    runEvaluation({
      packages: [quickXmlWithoutFix],
      assessments: [assessment({ exposure: "affected" })],
    }),
    1,
    "emergency_mitigation_candidate",
    "affected_release",
  );

  for (const [dates, expectedStatus] of [
    [{ reviewed_on: "2026-08-01", review_after: "2026-08-13" }, "expired"],
    [{ reviewed_on: "2026-08-15", review_after: "2026-11-14" }, "future"],
  ]) {
    const result = runEvaluation({
      packages: [quickXmlPackage],
      assessments: [assessment(dates)],
    });
    assertDecision(
      result,
      1,
      "triage_required",
      "high_or_critical_requires_review",
    );
    assert.equal(
      result.evaluation.findings[0].assessment_status,
      expectedStatus,
    );
  }

  for (const [fixed, decision] of [
    ["2.0.0", "emergency_release_candidate"],
    ["", "emergency_mitigation_candidate"],
  ]) {
    assertDecision(
      runEvaluation({
        packages: [
          packageResult({
            vulnerabilities: [
              vulnerability({
                id: "GHSA-known-exploited",
                aliases: ["CVE-2026-12345"],
                affected: [affectedEntry({ fixed })],
              }),
            ],
          }),
        ],
        kev: ["CVE-2026-12345"],
      }),
      1,
      decision,
      "known_exploited",
    );
  }

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
            affected: [affectedEntry({ ecosystem: "crates.io", name: "rand" })],
          }),
          vulnerability({
            id: "GHSA-cq8v-f236-94qc",
            aliases: ["RUSTSEC-2026-0097"],
            score: "5.0",
            affected: [affectedEntry({ ecosystem: "crates.io", name: "rand" })],
          }),
        ],
        groups: [
          {
            ids: ["RUSTSEC-2026-0097", "GHSA-cq8v-f236-94qc"],
            aliases: ["RUSTSEC-2026-0097", "GHSA-cq8v-f236-94qc"],
            max_severity: "5.0",
          },
        ],
      }),
    ],
  });
  assert.equal(groupedAliases.status, 0, groupedAliases.stderr);
  assert.equal(groupedAliases.evaluation.findings.length, 1);

  assertDecision(
    runEvaluation({
      packages: [
        packageResult({
          vulnerabilities: [
            vulnerability({
              id: "GHSA-multi-package",
              aliases: ["CVE-2026-12345"],
              affected: [
                affectedEntry({ fixed: "" }),
                affectedEntry({ name: "other-package", fixed: "9.9.9" }),
              ],
            }),
          ],
        }),
      ],
      kev: ["CVE-2026-12345"],
    }),
    1,
    "emergency_mitigation_candidate",
    "known_exploited",
  );

  const cvssV4 = runEvaluation({
    packages: [
      packageResult({
        vulnerabilities: [
          vulnerability({
            id: "GHSA-cvss-v4",
            score:
              "CVSS:4.0/AV:N/AC:L/AT:N/PR:N/UI:N/VC:H/VI:H/VA:H/SC:H/SI:H/SA:H",
          }),
        ],
        groups: [
          {
            ids: ["GHSA-cvss-v4"],
            aliases: ["GHSA-cvss-v4"],
            max_severity: "",
          },
        ],
      }),
    ],
  });
  assertDecision(
    cvssV4,
    1,
    "triage_required",
    "high_or_critical_requires_review",
  );
  assert.equal(cvssV4.evaluation.findings[0].severity, "UNPARSED_CVSS");

  const malformed = runEvaluation({ results: {} });
  assert.equal(malformed.status, 1);
  assert.equal(malformed.evaluation, null);
  assert.match(malformed.stderr, /results array/);
} finally {
  rmSync(testDir, { recursive: true, force: true });
}

console.log("Release vulnerability evaluation tests passed.");
