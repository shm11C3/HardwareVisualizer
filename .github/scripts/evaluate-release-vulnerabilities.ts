import fs from "node:fs";

type JsonObject = Record<string, unknown>;
type SeverityRank = 0 | 1 | 2 | 3 | 4;
type SeverityInfo = {
  label: string;
  rank: SeverityRank;
  score: number | null;
};
type PackageIdentity = {
  ecosystem: string;
  name: string;
  version: string;
};
type RuntimeExposure = "reachable" | "not_reachable" | "unknown";
type Ternary = "yes" | "no" | "unknown";
type TechnicalImpact = "total" | "partial" | "unknown";
type Assessment = {
  advisoryId: string;
  releaseTag: string;
  package: PackageIdentity;
  runtimeExposure: RuntimeExposure;
  automatable: Ternary;
  technicalImpact: TechnicalImpact;
  buildArtifactTainted: boolean;
  reason: string;
  reviewedOn: string;
  reviewAfter: string;
};
type AssessmentMatch = {
  assessment: Assessment | null;
  status: "active" | "expired" | "none";
};
type Decision =
  | "emergency_release_candidate"
  | "emergency_mitigation_candidate"
  | "triage_required"
  | "maintenance";
type Finding = {
  advisory_id: string;
  identifiers: string[];
  package: PackageIdentity;
  dependency_groups: string[];
  source: string;
  severity: string;
  score: number | null;
  fix_available: boolean;
  known_exploited: boolean;
  assessment_status: AssessmentMatch["status"];
  assessment_reason: string | null;
  decision: Decision;
  reason_code: string;
};
type Candidate = {
  advisoryId: string;
  identifiers: string[];
  package: PackageIdentity;
  dependencyGroups: string[];
  source: string;
  severity: SeverityInfo;
  fixAvailable: boolean;
};
type Evaluation = {
  schema_version: 1;
  release_tag: string;
  evaluated_on: string;
  generated_at: string;
  threat_intelligence: {
    cisa_kev_catalog_version: string;
    cisa_kev_date_released: string;
  };
  summary: Record<Decision | "total", number>;
  findings: Finding[];
};

const HIGH_SEVERITY_RANK = 3 satisfies SeverityRank;
const SUMMARY_FINDING_LIMIT = 50;
const NON_RUNTIME_DEPENDENCY_GROUPS = new Set([
  "dev",
  "development",
  "test",
  "tests",
]);
const severityRank = new Map<string, SeverityRank>([
  ["NONE", 0],
  ["UNKNOWN", 0],
  ["LOW", 1],
  ["MEDIUM", 2],
  ["MODERATE", 2],
  ["HIGH", 3],
  ["IMPORTANT", 3],
  ["CRITICAL", 4],
]);
const decisionRank = new Map<Decision, number>([
  ["emergency_release_candidate", 4],
  ["emergency_mitigation_candidate", 4],
  ["triage_required", 3],
  ["maintenance", 1],
]);

function isObject(value: unknown): value is JsonObject {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function asObject(value: unknown): JsonObject {
  return isObject(value) ? value : {};
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
}

function stringArray(value: unknown): string[] {
  return asArray(value).filter(
    (item): item is string => typeof item === "string",
  );
}

function requiredString(value: unknown, field: string): string {
  if (typeof value !== "string" || value.trim() === "") {
    throw new Error(`${field} must be a non-empty string.`);
  }
  return value;
}

function requiredBoolean(value: unknown, field: string): boolean {
  if (typeof value !== "boolean") {
    throw new Error(`${field} must be a boolean.`);
  }
  return value;
}

function enumValue<T extends string>(
  value: unknown,
  allowed: readonly T[],
  field: string,
): T {
  if (typeof value !== "string" || !allowed.includes(value as T)) {
    throw new Error(`${field} must be one of: ${allowed.join(", ")}.`);
  }
  return value as T;
}

function dateValue(value: unknown, field: string): string {
  const date = requiredString(value, field);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date) || Number.isNaN(Date.parse(date))) {
    throw new Error(`${field} must use YYYY-MM-DD.`);
  }
  return date;
}

function loadJson(path: string): JsonObject {
  return asObject(JSON.parse(fs.readFileSync(path, "utf8")));
}

function parsePackage(value: unknown, field: string): PackageIdentity {
  const pkg = asObject(value);
  return {
    ecosystem: requiredString(pkg.ecosystem, `${field}.ecosystem`),
    name: requiredString(pkg.name, `${field}.name`),
    version: requiredString(pkg.version, `${field}.version`),
  };
}

function loadAssessments(path: string): Assessment[] {
  const policy = loadJson(path);
  if (policy.schema_version !== 1) {
    throw new Error("release assessment schema_version must be 1.");
  }

  return asArray(policy.assessments).map((value, index) => {
    const item = asObject(value);
    const prefix = `assessments[${index}]`;
    const reviewedOn = dateValue(item.reviewed_on, `${prefix}.reviewed_on`);
    const reviewAfter = dateValue(item.review_after, `${prefix}.review_after`);
    if (reviewAfter < reviewedOn) {
      throw new Error(`${prefix}.review_after must not precede reviewed_on.`);
    }

    return {
      advisoryId: requiredString(item.advisory_id, `${prefix}.advisory_id`),
      releaseTag: requiredString(item.release_tag, `${prefix}.release_tag`),
      package: parsePackage(item.package, `${prefix}.package`),
      runtimeExposure: enumValue(
        item.runtime_exposure,
        ["reachable", "not_reachable", "unknown"] as const,
        `${prefix}.runtime_exposure`,
      ),
      automatable: enumValue(
        item.automatable,
        ["yes", "no", "unknown"] as const,
        `${prefix}.automatable`,
      ),
      technicalImpact: enumValue(
        item.technical_impact,
        ["total", "partial", "unknown"] as const,
        `${prefix}.technical_impact`,
      ),
      buildArtifactTainted: requiredBoolean(
        item.build_artifact_tainted,
        `${prefix}.build_artifact_tainted`,
      ),
      reason: requiredString(item.reason, `${prefix}.reason`),
      reviewedOn,
      reviewAfter,
    };
  });
}

function loadKevCatalog(path: string) {
  const catalog = loadJson(path);
  const vulnerabilities = asArray(catalog.vulnerabilities);
  if (vulnerabilities.length === 0) {
    throw new Error("CISA KEV catalog must contain vulnerabilities.");
  }

  const cves = new Set(
    vulnerabilities.map((item, index) =>
      requiredString(
        asObject(item).cveID,
        `CISA KEV vulnerabilities[${index}].cveID`,
      ),
    ),
  );

  return {
    catalogVersion:
      typeof catalog.catalogVersion === "string" ? catalog.catalogVersion : "",
    dateReleased:
      typeof catalog.dateReleased === "string" ? catalog.dateReleased : "",
    cves,
  };
}

function severityFromScore(score: number): SeverityInfo {
  if (score >= 9) return { label: "CRITICAL", rank: 4, score };
  if (score >= 7) return { label: "HIGH", rank: 3, score };
  if (score >= 4) return { label: "MEDIUM", rank: 2, score };
  if (score > 0) return { label: "LOW", rank: 1, score };
  return { label: "NONE", rank: 0, score };
}

function parseVector(vector: string): Record<string, string> {
  return Object.fromEntries(
    vector
      .split("/")
      .map((part) => part.split(":"))
      .filter(([key, value]) => key && value),
  );
}

function cvssRoundUp(value: number): number {
  return Math.ceil((value - 1e-10) * 10) / 10;
}

function metric(key: string | undefined, values: Record<string, number>) {
  return key ? values[key] : undefined;
}

function cvssV3Score(vector: string): number | null {
  const metrics = parseVector(vector);
  const scope = metrics.S;
  const av = metric(metrics.AV, { N: 0.85, A: 0.62, L: 0.55, P: 0.2 });
  const ac = metric(metrics.AC, { L: 0.77, H: 0.44 });
  const pr =
    scope === "C"
      ? metric(metrics.PR, { N: 0.85, L: 0.68, H: 0.5 })
      : metric(metrics.PR, { N: 0.85, L: 0.62, H: 0.27 });
  const ui = metric(metrics.UI, { N: 0.85, R: 0.62 });
  const c = metric(metrics.C, { H: 0.56, L: 0.22, N: 0 });
  const i = metric(metrics.I, { H: 0.56, L: 0.22, N: 0 });
  const a = metric(metrics.A, { H: 0.56, L: 0.22, N: 0 });

  if (
    [av, ac, pr, ui, c, i, a].some((value) => typeof value !== "number") ||
    (scope !== "U" && scope !== "C")
  ) {
    return null;
  }

  const [avScore, acScore, prScore, uiScore, cScore, iScore, aScore] = [
    av,
    ac,
    pr,
    ui,
    c,
    i,
    a,
  ] as number[];
  const impactSubScore = 1 - (1 - cScore) * (1 - iScore) * (1 - aScore);
  const impact =
    scope === "U"
      ? 6.42 * impactSubScore
      : 7.52 * (impactSubScore - 0.029) - 3.25 * (impactSubScore - 0.02) ** 15;
  if (impact <= 0) return 0;
  const exploitability = 8.22 * avScore * acScore * prScore * uiScore;
  const score =
    scope === "U"
      ? Math.min(impact + exploitability, 10)
      : Math.min(1.08 * (impact + exploitability), 10);
  return cvssRoundUp(score);
}

function severityFromSignal(value: unknown): SeverityInfo | null {
  if (typeof value === "number" && Number.isFinite(value)) {
    return severityFromScore(value);
  }
  if (typeof value !== "string") return null;

  const normalized = value
    .trim()
    .toUpperCase()
    .replace(/[-_\s]+/g, "_");
  const rank = severityRank.get(normalized);
  if (rank !== undefined) return { label: normalized, rank, score: null };

  const numeric = Number(value);
  if (Number.isFinite(numeric)) return severityFromScore(numeric);
  if (value.startsWith("CVSS:3.")) {
    const score = cvssV3Score(value);
    return score === null ? null : severityFromScore(score);
  }
  return null;
}

function highestSeverity(vulnerabilities: JsonObject[], groups: JsonObject[]) {
  let highest: SeverityInfo = { label: "UNKNOWN", rank: 0, score: null };
  const signals: unknown[] = groups.flatMap((group) => [
    group.max_severity,
    group.maxSeverity,
  ]);

  for (const vulnerability of vulnerabilities) {
    const databaseSpecific = asObject(vulnerability.database_specific);
    const ecosystemSpecific = asObject(vulnerability.ecosystem_specific);
    signals.push(databaseSpecific.severity, ecosystemSpecific.severity);
    for (const severity of asArray(vulnerability.severity)) {
      signals.push(asObject(severity).score);
    }
  }

  for (const signal of signals) {
    const severity = severityFromSignal(signal);
    if (
      severity &&
      (severity.rank > highest.rank ||
        (severity.rank === highest.rank &&
          (severity.score ?? -1) > (highest.score ?? -1)))
    ) {
      highest = severity;
    }
  }
  return highest;
}

function vulnerabilityIdentifiers(vulnerability: JsonObject): string[] {
  const id = typeof vulnerability.id === "string" ? [vulnerability.id] : [];
  return [...new Set([...id, ...stringArray(vulnerability.aliases)])];
}

function hasFixedVersion(vulnerability: JsonObject): boolean {
  return asArray(vulnerability.affected).some((affected) =>
    asArray(asObject(affected).ranges).some((range) =>
      asArray(asObject(range).events).some(
        (event) => typeof asObject(event).fixed === "string",
      ),
    ),
  );
}

function preferredIdentifier(identifiers: string[]): string {
  const priority = (id: string) => {
    if (id.startsWith("RUSTSEC-")) return 0;
    if (id.startsWith("GHSA-")) return 1;
    if (id.startsWith("CVE-")) return 2;
    return 3;
  };
  return [...identifiers].sort(
    (left, right) =>
      priority(left) - priority(right) || left.localeCompare(right),
  )[0];
}

function componentsForPackage(
  packageResult: JsonObject,
  source: string,
): Candidate[] {
  const vulnerabilities = asArray(packageResult.vulnerabilities).map(asObject);
  const groups = asArray(packageResult.groups).map(asObject);
  const parent = vulnerabilities.map((_, index) => index);
  const find = (index: number): number => {
    if (parent[index] !== index) parent[index] = find(parent[index]);
    return parent[index];
  };
  const union = (left: number, right: number) => {
    const leftRoot = find(left);
    const rightRoot = find(right);
    if (leftRoot !== rightRoot) parent[rightRoot] = leftRoot;
  };
  const identifierOwner = new Map<string, number>();

  vulnerabilities.forEach((vulnerability, index) => {
    for (const identifier of vulnerabilityIdentifiers(vulnerability)) {
      const owner = identifierOwner.get(identifier);
      if (owner === undefined) identifierOwner.set(identifier, index);
      else union(owner, index);
    }
  });

  for (const group of groups) {
    const owners = stringArray(group.ids)
      .map((id) => identifierOwner.get(id))
      .filter((value): value is number => value !== undefined);
    for (const owner of owners.slice(1)) union(owners[0], owner);
  }

  const components = new Map<number, JsonObject[]>();
  vulnerabilities.forEach((vulnerability, index) => {
    const root = find(index);
    components.set(root, [...(components.get(root) ?? []), vulnerability]);
  });

  const metadata = asObject(packageResult.package ?? packageResult.Package);
  const packageIdentity = {
    ecosystem: String(metadata.ecosystem ?? metadata.Ecosystem ?? ""),
    name: String(metadata.name ?? metadata.Name ?? "(unknown package)"),
    version: String(metadata.version ?? metadata.Version ?? ""),
  };
  const dependencyGroups = stringArray(
    packageResult.dependency_groups ?? packageResult.dependencyGroups,
  );

  return [...components.values()].map((componentVulnerabilities) => {
    const identifiers = new Set(
      componentVulnerabilities.flatMap(vulnerabilityIdentifiers),
    );
    const matchingGroups = groups.filter((group) =>
      stringArray(group.ids).some((id) => identifiers.has(id)),
    );
    for (const group of matchingGroups) {
      for (const id of [
        ...stringArray(group.ids),
        ...stringArray(group.aliases),
      ]) {
        identifiers.add(id);
      }
    }
    const sortedIdentifiers = [...identifiers].sort();

    return {
      advisoryId: preferredIdentifier(sortedIdentifiers),
      identifiers: sortedIdentifiers,
      package: packageIdentity,
      dependencyGroups,
      source,
      severity: highestSeverity(componentVulnerabilities, matchingGroups),
      fixAvailable: componentVulnerabilities.some(hasFixedVersion),
    };
  });
}

function collectCandidates(results: JsonObject): Candidate[] {
  const candidates: Candidate[] = [];
  for (const result of asArray(results.results)) {
    const resultObject = asObject(result);
    const sourceObject = asObject(
      resultObject.source ?? resultObject.packageSource,
    );
    const source =
      typeof sourceObject.path === "string" ? sourceObject.path : "";
    for (const packageResult of asArray(resultObject.packages)) {
      candidates.push(...componentsForPackage(asObject(packageResult), source));
    }
  }
  return candidates;
}

function packageMatches(left: PackageIdentity, right: PackageIdentity) {
  return (
    left.ecosystem === right.ecosystem &&
    left.name === right.name &&
    left.version === right.version
  );
}

function matchAssessment(
  candidate: Candidate,
  assessments: Assessment[],
  releaseTag: string,
  evaluationDate: string,
): AssessmentMatch {
  const matches = assessments.filter(
    (assessment) =>
      assessment.releaseTag === releaseTag &&
      candidate.identifiers.includes(assessment.advisoryId) &&
      packageMatches(candidate.package, assessment.package),
  );
  if (matches.length > 1) {
    throw new Error(
      `Multiple assessments match ${candidate.advisoryId} in ${candidate.package.name}@${candidate.package.version}.`,
    );
  }
  if (matches.length === 0) return { assessment: null, status: "none" };

  const assessment = matches[0];
  return {
    assessment,
    status: assessment.reviewAfter < evaluationDate ? "expired" : "active",
  };
}

function isDevelopmentOnly(groups: string[]): boolean {
  return (
    groups.length > 0 &&
    groups.every((group) =>
      NON_RUNTIME_DEPENDENCY_GROUPS.has(group.trim().toLowerCase()),
    )
  );
}

function emergencyDecision(fixAvailable: boolean): Decision {
  return fixAvailable
    ? "emergency_release_candidate"
    : "emergency_mitigation_candidate";
}

function evaluateCandidate(
  candidate: Candidate,
  assessments: Assessment[],
  kevCves: Set<string>,
  releaseTag: string,
  evaluationDate: string,
): Finding {
  const assessmentMatch = matchAssessment(
    candidate,
    assessments,
    releaseTag,
    evaluationDate,
  );
  const assessment =
    assessmentMatch.status === "active" ? assessmentMatch.assessment : null;
  const knownExploited = candidate.identifiers.some((id) => kevCves.has(id));
  let decision: Decision;
  let reasonCode: string;

  if (assessment?.buildArtifactTainted) {
    decision = emergencyDecision(candidate.fixAvailable);
    reasonCode = "build_artifact_tainted";
  } else if (isDevelopmentOnly(candidate.dependencyGroups)) {
    decision = "maintenance";
    reasonCode = "development_only";
  } else if (assessment?.runtimeExposure === "not_reachable") {
    decision = "maintenance";
    reasonCode = "not_reachable";
  } else if (knownExploited) {
    decision = emergencyDecision(candidate.fixAvailable);
    reasonCode = "known_exploited";
  } else if (
    assessment?.runtimeExposure === "reachable" &&
    assessment.automatable === "yes" &&
    assessment.technicalImpact === "total"
  ) {
    decision = emergencyDecision(candidate.fixAvailable);
    reasonCode = "reachable_automatable_total_impact";
  } else if (candidate.severity.rank >= HIGH_SEVERITY_RANK) {
    decision = "triage_required";
    reasonCode = "high_or_critical_requires_review";
  } else {
    decision = "maintenance";
    reasonCode = "below_triage_threshold";
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
    assessment_status: assessmentMatch.status,
    assessment_reason: assessmentMatch.assessment?.reason ?? null,
    decision,
    reason_code: reasonCode,
  };
}

function compareFindings(left: Finding, right: Finding): number {
  return (
    (decisionRank.get(right.decision) ?? 0) -
      (decisionRank.get(left.decision) ?? 0) ||
    (right.score ?? -1) - (left.score ?? -1) ||
    left.advisory_id.localeCompare(right.advisory_id)
  );
}

function buildSummary(findings: Finding[]): Evaluation["summary"] {
  const summary: Evaluation["summary"] = {
    total: findings.length,
    emergency_release_candidate: 0,
    emergency_mitigation_candidate: 0,
    triage_required: 0,
    maintenance: 0,
  };
  for (const finding of findings) summary[finding.decision]++;
  return summary;
}

function escapeMarkdown(value: unknown): string {
  return String(value)
    .replace(/\\/g, "\\\\")
    .replace(/\r?\n/g, " ")
    .replace(/\|/g, "\\|");
}

function annotationValue(value: unknown): string {
  return String(value)
    .replace(/%/g, "%25")
    .replace(/\r/g, "%0D")
    .replace(/\n/g, "%0A");
}

function writeStepSummary(evaluation: Evaluation) {
  const path = process.env.GITHUB_STEP_SUMMARY;
  if (!path) return;

  const lines = [
    "## Release vulnerability response evaluation",
    "",
    `Release tag: \`${escapeMarkdown(evaluation.release_tag)}\``,
    `CISA KEV released: \`${escapeMarkdown(evaluation.threat_intelligence.cisa_kev_date_released || "unknown")}\``,
    "",
    "| Decision | Count |",
    "| --- | ---: |",
    `| Emergency release candidate | ${evaluation.summary.emergency_release_candidate} |`,
    `| Emergency mitigation candidate | ${evaluation.summary.emergency_mitigation_candidate} |`,
    `| Security triage required | ${evaluation.summary.triage_required} |`,
    `| Maintenance | ${evaluation.summary.maintenance} |`,
    "",
  ];

  if (evaluation.findings.length === 0) {
    lines.push("No OSV vulnerabilities were reported for this release tag.");
  } else {
    lines.push(
      "### Findings",
      "",
      "| Decision | Severity | Advisory | Package | Evidence |",
      "| --- | --- | --- | --- | --- |",
    );
    for (const finding of evaluation.findings.slice(0, SUMMARY_FINDING_LIMIT)) {
      const advisory = `[${escapeMarkdown(finding.advisory_id)}](https://osv.dev/vulnerability/${encodeURIComponent(finding.advisory_id)})`;
      const pkg = `${finding.package.name}@${finding.package.version}`;
      lines.push(
        `| ${escapeMarkdown(finding.decision)} | ${escapeMarkdown(finding.severity)} | ${advisory} | ${escapeMarkdown(pkg)} | ${escapeMarkdown(finding.reason_code)} |`,
      );
    }
    if (evaluation.findings.length > SUMMARY_FINDING_LIMIT) {
      lines.push(
        "",
        `Showing ${SUMMARY_FINDING_LIMIT} of ${evaluation.findings.length} findings. Download the evaluation artifact for the complete result.`,
      );
    }
  }
  fs.appendFileSync(path, `${lines.join("\n")}\n`);
}

function parseArguments(argv: string[]) {
  const resultsPath = argv[0] ?? "release-results.json";
  const values = new Map<string, string>();
  for (let index = 1; index < argv.length; index += 2) {
    const flag = argv[index];
    const value = argv[index + 1];
    if (!flag?.startsWith("--") || !value) {
      throw new Error(`Invalid argument near ${flag ?? "(end)"}.`);
    }
    values.set(flag, value);
  }
  return {
    resultsPath,
    policyPath:
      values.get("--policy") ??
      ".github/osv/release-vulnerability-assessments.json",
    kevPath: values.get("--kev") ?? "cisa-kev.json",
    outputPath:
      values.get("--output") ?? "release-vulnerability-evaluation.json",
  };
}

function evaluationDate(): string {
  const value =
    process.env.OSV_EVALUATION_DATE ?? new Date().toISOString().slice(0, 10);
  return dateValue(value, "OSV_EVALUATION_DATE");
}

function main() {
  const paths = parseArguments(process.argv.slice(2));
  const releaseTag = requiredString(
    process.env.OSV_RELEASE_TAG,
    "OSV_RELEASE_TAG",
  );
  const evaluatedOn = evaluationDate();
  const assessments = loadAssessments(paths.policyPath);
  const kev = loadKevCatalog(paths.kevPath);
  const candidates = collectCandidates(loadJson(paths.resultsPath));
  const findings = candidates
    .map((candidate) =>
      evaluateCandidate(
        candidate,
        assessments,
        kev.cves,
        releaseTag,
        evaluatedOn,
      ),
    )
    .sort(compareFindings);
  const evaluation: Evaluation = {
    schema_version: 1,
    release_tag: releaseTag,
    evaluated_on: evaluatedOn,
    generated_at: new Date().toISOString(),
    threat_intelligence: {
      cisa_kev_catalog_version: kev.catalogVersion,
      cisa_kev_date_released: kev.dateReleased,
    },
    summary: buildSummary(findings),
    findings,
  };

  fs.writeFileSync(
    paths.outputPath,
    `${JSON.stringify(evaluation, null, 2)}\n`,
  );
  writeStepSummary(evaluation);
  console.log(`Evaluated ${findings.length} grouped OSV vulnerabilities.`);

  const emergencyCount =
    evaluation.summary.emergency_release_candidate +
    evaluation.summary.emergency_mitigation_candidate;
  if (emergencyCount > 0) {
    console.error(
      `::error::${annotationValue(`Found ${emergencyCount} emergency security response candidates.`)}`,
    );
  }
  if (evaluation.summary.triage_required > 0) {
    console.error(
      `::error::${annotationValue(`Found ${evaluation.summary.triage_required} vulnerabilities requiring security triage.`)}`,
    );
  }

  for (const finding of findings.filter(
    (item) => item.decision !== "maintenance",
  )) {
    console.error(
      `- [${finding.decision}] ${finding.advisory_id} in ${finding.package.name}@${finding.package.version}`,
    );
  }

  if (emergencyCount > 0 || evaluation.summary.triage_required > 0) {
    process.exit(1);
  }
}

try {
  main();
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}
