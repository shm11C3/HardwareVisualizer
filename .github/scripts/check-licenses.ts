import fs from "node:fs";

const licensesFile = process.argv[2];
if (!licensesFile) {
  console.error("Usage: ts-node check-licenses.ts <licenses.json>");
  process.exit(1);
}

const allowedLicenses = [
  "MIT",
  "BSD",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "Apache-2.0",
  "ISC",
  "Zlib",
  "CC0",
  "Unlicense",
];

const licenses: Record<
  string,
  { licenses: string; path: string; licenseFile: string }
> = JSON.parse(fs.readFileSync(licensesFile, "utf-8"));
let errorCount = 0;

for (const [pkgName, info] of Object.entries(licenses)) {
  const license = info.licenses;
  if (!allowedLicenses.includes(license)) {
    console.error(`❌ Unsupported license detected: ${license} (${pkgName})`);
    errorCount++;
  }
}

if (errorCount > 0) {
  console.error(`\n🚫 License check failed with ${errorCount} issue(s).`);
  process.exit(1);
}

console.log("✅ License check passed.");
