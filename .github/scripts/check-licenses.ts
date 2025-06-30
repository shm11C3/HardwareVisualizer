import fs from "node:fs";

const licensesFile = process.argv[2];
if (!licensesFile) {
  console.error("Usage: ts-node check-licenses.ts <licenses.json>");
  process.exit(1);
}

const allowedLicenses = new Set([
  "MIT",
  "Apache-2.0",
  "BSD-3-Clause",
  "BSD-2-Clause",
  "0BSD",
  "ISC",
  "BlueOak-1.0.0",
  "MPL-2.0",
]);

const licenses = JSON.parse(fs.readFileSync(licensesFile, "utf-8"));
let errorCount = 0;

const normalize = (
  raw: string,
): { type: "OR" | "AND" | "SINGLE"; values: string[] } => {
  if (raw.includes(" OR ")) {
    return { type: "OR", values: raw.split(/\s+OR\s+/).map((s) => s.trim()) };
  }
  if (raw.includes(" AND ")) {
    return { type: "AND", values: raw.split(/\s+AND\s+/).map((s) => s.trim()) };
  }
  return { type: "SINGLE", values: [raw.trim()] };
};

for (const [pkg, info] of Object.entries<any>(licenses)) {
  // 自身のチェックはスキップ
  if (pkg.split("@")[0] === "hardware-visualizer") {
    continue;
  }

  const license = info.licenses;
  const { type, values } = normalize(license);

  const valid =
    type === "OR"
      ? values.some((l) => allowedLicenses.has(l))
      : type === "AND"
        ? values.every((l) => allowedLicenses.has(l))
        : allowedLicenses.has(values[0]);

  if (!valid) {
    console.error(`❌ Unsupported license detected: ${license} (${pkg})`);
    errorCount++;
  }
}

if (errorCount > 0) {
  console.error(`\n🚫 License check failed with ${errorCount} issue(s).`);
  process.exit(1);
}

console.log("✅ License check passed.");
