import fs from "node:fs";

const HIGH = 3;
const DEVELOPMENT_GROUPS = new Set(["dev", "development", "test", "tests"]);
const SEVERITY_RANK = new Map([
  ["NONE", 0],
  ["UNKNOWN", 0],
  ["LOW", 1],
  ["MEDIUM", 2],
  ["MODERATE", 2],
  ["HIGH", 3],
  ["IMPORTANT", 3],
  ["CRITICAL", 4],
]);
const DECISION_RANK = new Map([
  ["emergency_release_candidate", 4],
  ["emergency_mitigation_candidate", 4],
  ["triage_required", 3],
  ["maintenance", 1],
]);

function object(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? value
    : {};
}

function list(value) {
  return Array.isArray(value) ? value : [];
}

function strings(value) {
  return list(value).filter((item) => typeof item === "string");
}

function requiredString(value, field) {
  if (typeof value !== "string" || !value.trim()) {
    throw new Error(`${field} must be a non-empty string.`);
  }
  return value;
}

function date(value, field) {
  const result = requiredString(value, field);
  const parsed = new Date(`${result}T00:00:00Z`);
  if (
    !/^\d{4}-\d{2}-\d{2}$/.test(result) ||
    Number.isNaN(parsed.valueOf()) ||
    parsed.toISOString().slice(0, 10) !== result
  ) {
    throw new Error(`${field} must use a valid YYYY-MM-DD date.`);
  }
  return result;
}

function readJson(path) {
  const value = JSON.parse(fs.readFileSync(path, "utf8"));
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`${path} must contain a JSON object.`);
  }
  return value;
}

function parsePackage(value, field) {
  const pkg = object(value);
  return {
    ecosystem: requiredString(pkg.ecosystem, `${field}.ecosystem`),
    name: requiredString(pkg.name, `${field}.name`),
    version: requiredString(pkg.version, `${field}.version`),
  };
}

function samePackage(left, right, includeVersion = true) {
  return (
    left.ecosystem === right.ecosystem &&
    left.name === right.name &&
    (!includeVersion || left.version === right.version)
  );
}

function loadAssessments(path) {
  const policy = readJson(path);
  if (policy.schema_version !== 1 || !Array.isArray(policy.assessments)) {
    throw new Error(
      "Release assessments must use schema_version 1 and an assessments array.",
    );
  }

  const keys = new Set();
  return policy.assessments.map((value, index) => {
    const item = object(value);
    const field = `assessments[${index}]`;
    const assessment = {
      advisoryId: requiredString(item.advisory_id, `${field}.advisory_id`),
      releaseTag: requiredString(item.release_tag, `${field}.release_tag`),
      package: parsePackage(item.package, `${field}.package`),
      exposure: requiredString(item.exposure, `${field}.exposure`),
      reason: requiredString(item.reason, `${field}.reason`),
      reviewedOn: date(item.reviewed_on, `${field}.reviewed_on`),
      reviewAfter: date(item.review_after, `${field}.review_after`),
    };
    if (!["affected", "not_affected"].includes(assessment.exposure)) {
      throw new Error(`${field}.exposure must be affected or not_affected.`);
    }
    if (assessment.reviewAfter < assessment.reviewedOn) {
      throw new Error(`${field}.review_after must not precede reviewed_on.`);
    }

    const key = [
      assessment.releaseTag,
      assessment.advisoryId,
      assessment.package.ecosystem,
      assessment.package.name,
      assessment.package.version,
    ].join("\0");
    if (keys.has(key)) throw new Error(`Duplicate assessment: ${key}.`);
    keys.add(key);
    return assessment;
  });
}

function loadKev(path) {
  const catalog = readJson(path);
  if (!Array.isArray(catalog.vulnerabilities)) {
    throw new Error("CISA KEV catalog must contain a vulnerabilities array.");
  }
  return {
    version:
      typeof catalog.catalogVersion === "string" ? catalog.catalogVersion : "",
    released:
      typeof catalog.dateReleased === "string" ? catalog.dateReleased : "",
    cves: new Set(
      catalog.vulnerabilities.map((item, index) =>
        requiredString(
          object(item).cveID,
          `CISA KEV vulnerabilities[${index}].cveID`,
        ),
      ),
    ),
  };
}

function severity(value) {
  if (typeof value === "number" && Number.isFinite(value)) {
    return severityFromScore(value);
  }
  if (typeof value !== "string") return null;

  const normalized = value.trim().toUpperCase();
  if (!normalized) return null;
  if (SEVERITY_RANK.has(normalized)) {
    return {
      label: normalized,
      rank: SEVERITY_RANK.get(normalized),
      score: null,
    };
  }
  const score = Number(value);
  if (Number.isFinite(score)) return severityFromScore(score);

  return null;
}

function severityFromScore(score) {
  if (score >= 9) return { label: "CRITICAL", rank: 4, score };
  if (score >= 7) return { label: "HIGH", rank: 3, score };
  if (score >= 4) return { label: "MEDIUM", rank: 2, score };
  if (score > 0) return { label: "LOW", rank: 1, score };
  return { label: "NONE", rank: 0, score };
}

function highestSeverity(group, vulnerabilities) {
  const signals = [group.max_severity, group.maxSeverity];
  for (const vulnerability of vulnerabilities) {
    signals.push(
      object(vulnerability.database_specific).severity,
      object(vulnerability.ecosystem_specific).severity,
      ...list(vulnerability.severity).map((item) => object(item).score),
    );
  }
  const parsed = signals
    .map(severity)
    .filter(Boolean)
    .sort(
      (left, right) =>
        right.rank - left.rank || (right.score ?? -1) - (left.score ?? -1),
    )[0];
  if (parsed) return parsed;

  // Prefer OSV's normalized severity. Only fail closed when a CVSS vector is
  // present and no normalized label or numeric score exists.
  return signals.some(
    (value) => typeof value === "string" && value.startsWith("CVSS:"),
  )
    ? { label: "UNPARSED_CVSS", rank: HIGH, score: null }
    : { label: "UNKNOWN", rank: 0, score: null };
}

function identifiers(vulnerability) {
  return [
    ...new Set([
      ...(typeof vulnerability.id === "string" ? [vulnerability.id] : []),
      ...strings(vulnerability.aliases),
    ]),
  ];
}

function preferredIdentifier(values) {
  if (!values.length) throw new Error("OSV group has no identifier.");
  const priority = (id) =>
    id.startsWith("RUSTSEC-")
      ? 0
      : id.startsWith("GHSA-")
        ? 1
        : id.startsWith("CVE-")
          ? 2
          : 3;
  return [...values].sort(
    (left, right) =>
      priority(left) - priority(right) || left.localeCompare(right),
  )[0];
}

function hasFix(vulnerability, pkg) {
  return list(vulnerability.affected)
    .map(object)
    .filter((affected) => {
      const affectedPackage = parsePackage(
        { ...object(affected.package), version: pkg.version },
        "OSV affected package",
      );
      return samePackage(affectedPackage, pkg, false);
    })
    .some((affected) =>
      list(affected.ranges).some((range) =>
        list(object(range).events).some(
          (event) => typeof object(event).fixed === "string",
        ),
      ),
    );
}

function candidates(results) {
  if (!Array.isArray(results.results)) {
    throw new Error("OSV results must contain a results array.");
  }

  const output = [];
  for (const result of results.results) {
    const source = object(object(result).source).path ?? "";
    for (const packageValue of list(object(result).packages)) {
      const packageResult = object(packageValue);
      const pkg = parsePackage(packageResult.package, "OSV package");
      const vulnerabilities = list(packageResult.vulnerabilities).map(object);
      const groups = list(packageResult.groups).map(object);
      if (vulnerabilities.length && !groups.length) {
        throw new Error(
          `OSV package ${pkg.name}@${pkg.version} has vulnerabilities but no groups.`,
        );
      }

      for (const group of groups) {
        const groupIds = new Set([
          ...strings(group.ids),
          ...strings(group.aliases),
        ]);
        const matches = vulnerabilities.filter((vulnerability) =>
          identifiers(vulnerability).some((id) => groupIds.has(id)),
        );
        if (!matches.length) {
          throw new Error(
            `OSV group for ${pkg.name}@${pkg.version} has no matching vulnerability.`,
          );
        }
        for (const vulnerability of matches) {
          for (const id of identifiers(vulnerability)) groupIds.add(id);
        }

        const allIds = [...groupIds].sort();
        output.push({
          advisoryId: preferredIdentifier(allIds),
          identifiers: allIds,
          package: pkg,
          dependencyGroups: strings(packageResult.dependency_groups),
          source: typeof source === "string" ? source : "",
          severity: highestSeverity(group, matches),
          fixAvailable: matches.some((vulnerability) =>
            hasFix(vulnerability, pkg),
          ),
        });
      }
    }
  }
  return output;
}

function assessmentFor(candidate, assessments, releaseTag, evaluationDate) {
  const assessment =
    assessments.find(
      (item) =>
        item.releaseTag === releaseTag &&
        item.advisoryId === candidate.advisoryId &&
        samePackage(item.package, candidate.package),
    ) ?? null;
  if (!assessment) return { assessment: null, status: "none" };
  if (assessment.reviewedOn > evaluationDate) {
    return { assessment, status: "future" };
  }
  if (assessment.reviewAfter < evaluationDate) {
    return { assessment, status: "expired" };
  }
  return { assessment, status: "active" };
}

function emergency(fixAvailable) {
  return fixAvailable
    ? "emergency_release_candidate"
    : "emergency_mitigation_candidate";
}

function evaluate(candidate, assessments, kevCves, releaseTag, evaluationDate) {
  const match = assessmentFor(
    candidate,
    assessments,
    releaseTag,
    evaluationDate,
  );
  const active = match.status === "active" ? match.assessment : null;
  const developmentOnly =
    candidate.dependencyGroups.length > 0 &&
    candidate.dependencyGroups.every((group) =>
      DEVELOPMENT_GROUPS.has(group.toLowerCase()),
    );
  const knownExploited = candidate.identifiers.some((id) => kevCves.has(id));

  let decision = "maintenance";
  let reason = "below_triage_threshold";
  if (developmentOnly) {
    reason = "development_only";
  } else if (active?.exposure === "not_affected") {
    reason = "not_affected";
  } else if (active?.exposure === "affected") {
    decision = emergency(candidate.fixAvailable);
    reason = "affected_release";
  } else if (knownExploited) {
    decision = emergency(candidate.fixAvailable);
    reason = "known_exploited";
  } else if (candidate.severity.rank >= HIGH) {
    decision = "triage_required";
    reason = "high_or_critical_requires_review";
  }

  return {
    advisory_id: candidate.advisoryId,
    identifiers: candidate.identifiers,
    package: candidate.package,
    dependency_groups: candidate.dependencyGroups,
    source: candidate.source,
    severity: candidate.severity.label,
    score: candidate.severity.score,
    fix_available: candidate.fixAvailable,
    known_exploited: knownExploited,
    assessment_status: match.status,
    assessment_reason: match.assessment?.reason ?? null,
    decision,
    reason_code: reason,
  };
}

function summary(findings) {
  const result = {
    total: findings.length,
    emergency_release_candidate: 0,
    emergency_mitigation_candidate: 0,
    triage_required: 0,
    maintenance: 0,
  };
  for (const finding of findings) result[finding.decision]++;
  return result;
}

function writeSummary(evaluation) {
  if (!process.env.GITHUB_STEP_SUMMARY) return;
  const counts = evaluation.summary;
  const lines = [
    "## Release vulnerability exposure evaluation",
    "",
    `Release: \`${evaluation.release_tag}\``,
    "",
    "| Decision | Count |",
    "| --- | ---: |",
    `| Emergency release | ${counts.emergency_release_candidate} |`,
    `| Emergency mitigation | ${counts.emergency_mitigation_candidate} |`,
    `| Triage required | ${counts.triage_required} |`,
    `| Maintenance | ${counts.maintenance} |`,
  ];
  fs.appendFileSync(process.env.GITHUB_STEP_SUMMARY, `${lines.join("\n")}\n`);
}

function main() {
  const [resultsPath, assessmentsPath, kevPath, outputPath, ...extra] =
    process.argv.slice(2);
  if (
    !resultsPath ||
    !assessmentsPath ||
    !kevPath ||
    !outputPath ||
    extra.length
  ) {
    throw new Error(
      "Usage: evaluate-release-vulnerabilities.mjs <results> <assessments> <cisa-kev> <output>",
    );
  }

  const releaseTag = requiredString(
    process.env.OSV_RELEASE_TAG,
    "OSV_RELEASE_TAG",
  );
  const evaluationDate = date(
    process.env.OSV_EVALUATION_DATE ?? new Date().toISOString().slice(0, 10),
    "OSV_EVALUATION_DATE",
  );
  const assessments = loadAssessments(assessmentsPath);
  const kev = loadKev(kevPath);
  const findings = candidates(readJson(resultsPath))
    .map((candidate) =>
      evaluate(candidate, assessments, kev.cves, releaseTag, evaluationDate),
    )
    .sort(
      (left, right) =>
        DECISION_RANK.get(right.decision) - DECISION_RANK.get(left.decision) ||
        (right.score ?? -1) - (left.score ?? -1) ||
        left.advisory_id.localeCompare(right.advisory_id),
    );
  const evaluation = {
    schema_version: 1,
    release_tag: releaseTag,
    evaluated_on: evaluationDate,
    generated_at: new Date().toISOString(),
    threat_intelligence: {
      cisa_kev_catalog_version: kev.version,
      cisa_kev_date_released: kev.released,
    },
    summary: summary(findings),
    findings,
  };

  fs.writeFileSync(outputPath, `${JSON.stringify(evaluation, null, 2)}\n`);
  writeSummary(evaluation);
  console.log(`Evaluated ${findings.length} grouped OSV vulnerabilities.`);

  const failures = findings.filter((item) => item.decision !== "maintenance");
  for (const finding of failures) {
    console.error(
      `- [${finding.decision}] ${finding.advisory_id} in ${finding.package.name}@${finding.package.version}`,
    );
  }
  if (failures.length) {
    console.error(
      `::error::Release vulnerability evaluation requires action on ${failures.length} finding(s).`,
    );
    process.exitCode = 1;
  }
}

try {
  main();
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
