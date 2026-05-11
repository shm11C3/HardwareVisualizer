import fs from "node:fs";

const HIGH_OR_CRITICAL_SCORE = 7.0;
const SUMMARY_FINDING_LIMIT = 50;
const severityRank = new Map([
  ["NONE", 0],
  ["UNKNOWN", 0],
  ["LOW", 1],
  ["MEDIUM", 2],
  ["MODERATE", 2],
  ["HIGH", 3],
  ["IMPORTANT", 3],
  ["CRITICAL", 4],
]);

function isObject(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function asObject(value) {
  return isObject(value) ? value : {};
}

function asArray(value) {
  return Array.isArray(value) ? value : [];
}

function cvssRoundUp(value) {
  return Math.ceil((value - 1e-10) * 10) / 10;
}

function parseVector(vector) {
  return Object.fromEntries(
    vector
      .split("/")
      .map((part) => part.split(":"))
      .filter(([key, value]) => key && value),
  );
}

function cvssV3Score(vector) {
  const metrics = parseVector(vector);
  const scope = metrics.S;

  const av = { N: 0.85, A: 0.62, L: 0.55, P: 0.2 }[metrics.AV];
  const ac = { L: 0.77, H: 0.44 }[metrics.AC];
  const pr =
    scope === "C"
      ? { N: 0.85, L: 0.68, H: 0.5 }[metrics.PR]
      : { N: 0.85, L: 0.62, H: 0.27 }[metrics.PR];
  const ui = { N: 0.85, R: 0.62 }[metrics.UI];
  const c = { H: 0.56, L: 0.22, N: 0 }[metrics.C];
  const i = { H: 0.56, L: 0.22, N: 0 }[metrics.I];
  const a = { H: 0.56, L: 0.22, N: 0 }[metrics.A];

  if (
    [av, ac, pr, ui, c, i, a].some((metric) => typeof metric !== "number") ||
    (scope !== "U" && scope !== "C")
  ) {
    return null;
  }

  const impactSubScore = 1 - (1 - c) * (1 - i) * (1 - a);
  const impact =
    scope === "U"
      ? 6.42 * impactSubScore
      : 7.52 * (impactSubScore - 0.029) - 3.25 * (impactSubScore - 0.02) ** 15;
  const exploitability = 8.22 * av * ac * pr * ui;

  if (impact <= 0) {
    return 0;
  }

  const rawScore =
    scope === "U"
      ? Math.min(impact + exploitability, 10)
      : Math.min(1.08 * (impact + exploitability), 10);

  return cvssRoundUp(rawScore);
}

function cvssV2Score(vector) {
  const metrics = parseVector(vector);

  const av = { L: 0.395, A: 0.646, N: 1 }[metrics.AV];
  const ac = { H: 0.35, M: 0.61, L: 0.71 }[metrics.AC];
  const au = { M: 0.45, S: 0.56, N: 0.704 }[metrics.Au];
  const c = { N: 0, P: 0.275, C: 0.66 }[metrics.C];
  const i = { N: 0, P: 0.275, C: 0.66 }[metrics.I];
  const a = { N: 0, P: 0.275, C: 0.66 }[metrics.A];

  if ([av, ac, au, c, i, a].some((metric) => typeof metric !== "number")) {
    return null;
  }

  const impact = 10.41 * (1 - (1 - c) * (1 - i) * (1 - a));
  const exploitability = 20 * av * ac * au;
  const impactFactor = impact === 0 ? 0 : 1.176;

  return (
    Math.round(
      (0.6 * impact + 0.4 * exploitability - 1.5) * impactFactor * 10,
    ) / 10
  );
}

function severityFromScore(score) {
  if (score >= 9.0) return { label: "CRITICAL", rank: 4, score };
  if (score >= HIGH_OR_CRITICAL_SCORE) return { label: "HIGH", rank: 3, score };
  if (score >= 4.0) return { label: "MEDIUM", rank: 2, score };
  if (score > 0) return { label: "LOW", rank: 1, score };
  return { label: "NONE", rank: 0, score };
}

function normalizeSeverityLabel(value) {
  if (typeof value !== "string") {
    return null;
  }

  const normalized = value
    .trim()
    .toUpperCase()
    .replace(/[-_\s]+/g, "_");
  const label = normalized === "MEDIUM" ? "MEDIUM" : normalized;
  const rank = severityRank.get(label);

  return rank === undefined ? null : { label, rank };
}

function severityFromSignal(value, type = "") {
  if (typeof value === "number" && Number.isFinite(value)) {
    return severityFromScore(value);
  }

  if (typeof value !== "string") {
    return null;
  }

  const label = normalizeSeverityLabel(value);
  if (label) {
    return label;
  }

  const numeric = Number(value);
  if (Number.isFinite(numeric)) {
    return severityFromScore(numeric);
  }

  if (value.startsWith("CVSS:3.")) {
    const score = cvssV3Score(value);
    return score === null ? null : severityFromScore(score);
  }

  if (type.includes("CVSS_V2")) {
    const score = cvssV2Score(value);
    return score === null ? null : severityFromScore(score);
  }

  if (type.includes("CVSS_V3")) {
    const score = cvssV3Score(value);
    return score === null ? null : severityFromScore(score);
  }

  return null;
}

function collectSeveritySignals(vulnerability) {
  const signals = [];
  const databaseSpecific = asObject(vulnerability.database_specific);
  const ecosystemSpecific = asObject(vulnerability.ecosystem_specific);
  const cvss = asObject(databaseSpecific.cvss);

  signals.push([databaseSpecific.severity]);
  signals.push([databaseSpecific.cvss_score]);
  signals.push([ecosystemSpecific.severity]);
  signals.push([cvss.score]);
  signals.push([cvss.base_score]);
  signals.push([cvss.vector]);
  signals.push([cvss.vector_string]);
  signals.push([cvss.vectorString]);

  if (!Array.isArray(vulnerability.severity)) {
    signals.push([vulnerability.severity]);
  }

  for (const item of asArray(vulnerability.severity)) {
    const severity = asObject(item);
    signals.push([severity.score, String(severity.type ?? "")]);
  }

  return signals;
}

function highestSeverity(vulnerability) {
  let highest = null;

  for (const [value, type = ""] of collectSeveritySignals(vulnerability)) {
    const severity = severityFromSignal(value, type);
    if (!severity) {
      continue;
    }

    if (!highest || severity.rank > highest.rank) {
      highest = severity;
    }
  }

  return highest;
}

function packageName(packageResult) {
  const metadata = asObject(packageResult.package ?? packageResult.Package);
  const name = metadata.name ?? metadata.Name ?? "(unknown package)";
  const version = metadata.version ?? metadata.Version;

  return version ? `${name}@${version}` : String(name);
}

function sourcePath(result) {
  const source = asObject(result.source ?? result.packageSource);
  return typeof source.path === "string" ? source.path : "";
}

function vulnerabilityId(vulnerability) {
  if (typeof vulnerability.id === "string") {
    return vulnerability.id;
  }

  const alias = asArray(vulnerability.aliases).find(
    (value) => typeof value === "string",
  );

  return alias ?? "(unknown advisory)";
}

function escapeMarkdownCell(value) {
  return String(value).replace(/\r?\n/g, " ").replace(/\|/g, "\\|");
}

function advisoryMarkdown(id) {
  const escaped = escapeMarkdownCell(id);

  if (id.startsWith("(")) {
    return escaped;
  }

  return `[${escaped}](https://osv.dev/vulnerability/${encodeURIComponent(id)})`;
}

function severityLabel(severity) {
  return severity?.label ?? "UNKNOWN";
}

function severityScore(severity) {
  return typeof severity?.score === "number" ? severity.score : null;
}

function compareFindings(a, b) {
  const severityDiff = b.rank - a.rank;
  if (severityDiff !== 0) {
    return severityDiff;
  }

  const scoreDiff = (b.score ?? -1) - (a.score ?? -1);
  if (scoreDiff !== 0) {
    return scoreDiff;
  }

  return a.id.localeCompare(b.id);
}

function countBySeverity(findings) {
  const counts = new Map();

  for (const finding of findings) {
    counts.set(finding.severity, (counts.get(finding.severity) ?? 0) + 1);
  }

  return [...counts.entries()].sort(
    ([left], [right]) =>
      (severityRank.get(right) ?? 0) - (severityRank.get(left) ?? 0),
  );
}

function summaryTable(findings) {
  const visibleFindings = findings.slice(0, SUMMARY_FINDING_LIMIT);
  const rows = visibleFindings.map((finding) => {
    const score =
      typeof finding.score === "number" ? finding.score.toFixed(1) : "";

    return [
      escapeMarkdownCell(finding.severity),
      escapeMarkdownCell(score),
      advisoryMarkdown(finding.id),
      escapeMarkdownCell(finding.package),
      escapeMarkdownCell(finding.source),
    ].join(" | ");
  });

  return [
    "| Severity | Score | Advisory | Package | Source |",
    "| --- | ---: | --- | --- | --- |",
    ...rows.map((row) => `| ${row} |`),
  ].join("\n");
}

function writeStepSummary({ allFindings, blockingFindings, releaseTag }) {
  const summaryPath = process.env.GITHUB_STEP_SUMMARY;
  if (!summaryPath) {
    return;
  }

  const sortedFindings = [...allFindings].sort(compareFindings);
  const lines = [
    "## OSV release tag scan",
    "",
    `Release tag: \`${releaseTag || "(not provided)"}\``,
    "Failure threshold: High or Critical",
    `Detected vulnerabilities: ${allFindings.length}`,
    `High or Critical vulnerabilities: ${blockingFindings.length}`,
    "",
  ];

  if (allFindings.length === 0) {
    lines.push("No OSV vulnerabilities were reported for this release tag.");
    fs.appendFileSync(summaryPath, `${lines.join("\n")}\n`);
    return;
  }

  lines.push("### Severity Counts", "");
  lines.push("| Severity | Count |", "| --- | ---: |");

  for (const [severity, count] of countBySeverity(sortedFindings)) {
    lines.push(`| ${escapeMarkdownCell(severity)} | ${count} |`);
  }

  lines.push("", "### Detected Vulnerabilities", "");
  lines.push(summaryTable(sortedFindings));

  if (sortedFindings.length > SUMMARY_FINDING_LIMIT) {
    lines.push(
      "",
      `Showing ${SUMMARY_FINDING_LIMIT} of ${sortedFindings.length} vulnerabilities. Download the SARIF artifact for the full report.`,
    );
  }

  fs.appendFileSync(summaryPath, `${lines.join("\n")}\n`);
}

function loadResults(path) {
  const raw = fs.readFileSync(path, "utf8");
  return JSON.parse(raw);
}

function main() {
  const path = process.argv[2] ?? "latest-release-results.json";
  const data = loadResults(path);
  const allFindings = [];
  const blockingFindings = [];
  const releaseTag = process.env.OSV_RELEASE_TAG ?? "";
  let vulnerabilityCount = 0;

  for (const result of asArray(data.results)) {
    const source = sourcePath(asObject(result));

    for (const packageResult of asArray(asObject(result).packages)) {
      const pkg = packageName(asObject(packageResult));

      for (const vulnerability of asArray(
        asObject(packageResult).vulnerabilities,
      )) {
        vulnerabilityCount++;
        const severity = highestSeverity(asObject(vulnerability));
        const finding = {
          id: vulnerabilityId(asObject(vulnerability)),
          package: pkg,
          rank: severity?.rank ?? 0,
          score: severityScore(severity),
          severity: severityLabel(severity),
          source,
        };

        allFindings.push(finding);

        if (finding.rank >= severityRank.get("HIGH")) {
          blockingFindings.push(finding);
        }
      }
    }
  }

  writeStepSummary({ allFindings, blockingFindings, releaseTag });

  console.log(`Checked ${vulnerabilityCount} OSV vulnerabilities.`);

  if (blockingFindings.length === 0) {
    console.log("No High or Critical OSV vulnerabilities found.");
    return;
  }

  console.error(
    `::error::Found ${blockingFindings.length} High or Critical OSV vulnerabilities.`,
  );

  for (const finding of blockingFindings.sort(compareFindings)) {
    const score =
      typeof finding.score === "number" ? ` (${finding.score.toFixed(1)})` : "";
    const source = finding.source ? ` from ${finding.source}` : "";
    console.error(
      `- [${finding.severity}${score}] ${finding.id} in ${finding.package}${source}`,
    );
  }

  process.exit(1);
}

try {
  main();
} catch (e) {
  console.error(e instanceof Error ? e.message : String(e));
  process.exit(1);
}
