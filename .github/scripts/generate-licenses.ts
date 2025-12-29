import { execSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";

type NpmLicenseInfo = {
  licenses: string;
  repository?: string;
  publisher?: string;
  email?: string;
  licenseFile?: string;
};

type CargoLicenseInfo = {
  name: string;
  version: string;
  license: string;
  repository?: string;
  description?: string;
};

type CargoMetadata = {
  packages: CargoPackage[];
};

type CargoPackage = {
  name: string;
  version: string;
  manifest_path: string;
  targets: CargoTarget[];
};

type CargoTarget = {
  kind: string[];
};

// ==========================
// Argument processing
// ==========================
const target = process.argv[2]; // "linux" or "windows"
if (!target || !["linux", "windows", "macos", "tmp"].includes(target)) {
  console.error(
    "Usage: node --experimental-strip-types script.ts <linux|windows|macos|tmp>",
  );
  process.exit(1);
}

// Switch output directory based on OS
const outputDir =
  target === "tmp"
    ? path.resolve("./tmp")
    : path.resolve(`./docs/THIRD_PARTY_NOTICES/${target}`);
const outputPath = path.join(outputDir, "THIRD_PARTY_NOTICES.md");

const generateLicenseTxt = (
  name: string,
  licenses: string,
  repository?: string,
  publisher?: string,
  email?: string,
) => {
  let output = `## ${name}\n\n`;
  output += `- License: ${licenses}\n`;
  if (repository) output += `- Repository: [${repository}](${repository})\n`;
  if (publisher) output += `- Publisher: ${publisher}\n`;
  if (email) output += `- Email: <${email}>\n`;
  output += "\n";

  return output;
};

let output = "# THIRD_PARTY_NOTICES\n\n";

output +=
  "This application includes third-party libraries licensed under their respective licenses.\n\n";

//
// ====================
// 1. Node.js dependencies (prod only)
// ====================
//
try {
  const npmRawJson = execSync("npx license-checker --production --json", {
    encoding: "utf8",
  });
  const npmData = JSON.parse(npmRawJson);

  for (const [name, info] of Object.entries(npmData) as [
    string,
    NpmLicenseInfo,
  ][]) {
    output += generateLicenseTxt(
      name,
      info.licenses,
      info.repository,
      info.publisher,
      info.email,
    );

    // Get the contents of the license file
    if (info.licenseFile) {
      const licenseContent = readFileSync(info.licenseFile, {
        encoding: "utf8",
      });
      output += "```LICENSE\n";
      output += `${licenseContent.trim().replace(/```/g, "`` ``` ``")}\n`;
      output += "```\n\n";
    }
  }
} catch (e) {
  console.error("❌ Failed to collect NPM licenses:", e);
}

//
// ====================
// 2. Rust dependencies (filtered by metadata)
// ====================
//
try {
  const cargoJson = execSync("cargo license --json", {
    cwd: "src-tauri",
    encoding: "utf8",
  });
  const cargoData: CargoLicenseInfo[] = JSON.parse(cargoJson);

  const metadataJson = execSync("cargo metadata --format-version 1", {
    cwd: "src-tauri",
    encoding: "utf8",
    maxBuffer: 100 * 1024 * 1024,
  });
  const metadata: CargoMetadata = JSON.parse(metadataJson);

  // Keep only crates required at runtime
  const runtimeCrates = new Set<string>();
  for (const pkg of metadata.packages) {
    const kinds = pkg.targets.flatMap((t) => t.kind);
    const isBuildOnly = kinds.includes("custom-build"); // build.rs only
    const isTestOnly = kinds.includes("test") || kinds.includes("example"); // dev only

    if (!isBuildOnly && !isTestOnly) {
      runtimeCrates.add(`${pkg.name}@${pkg.version}`);
    }
  }

  const packageMap: Record<string, string> = {};
  for (const pkg of metadata.packages) {
    packageMap[`${pkg.name}@${pkg.version}`] = pkg.manifest_path
      .replace(/\\/g, "/")
      .replace(/\/Cargo.toml$/, "");
  }

  for (const crate of cargoData) {
    const crateKey = `${crate.name}@${crate.version}`;
    if (!runtimeCrates.has(crateKey)) continue; // Exclude build/test only crates

    output += generateLicenseTxt(crate.name, crate.license, crate.repository);

    // Find LICENSE file and add its contents
    const cratePath = packageMap[crateKey];
    if (cratePath) {
      const licenseFiles = [
        "LICENSE",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        "COPYING",
      ];
      for (const file of licenseFiles) {
        const licensePath = path.join(cratePath, file);
        if (existsSync(licensePath)) {
          const licenseContent = readFileSync(licensePath, "utf8");
          output += "```LICENSE\n";
          output += `${licenseContent.trim().replace(/```/g, "`` ``` ``")}\n`;
          output += "```\n\n";
          break;
        }
      }
    }
  }
} catch (e) {
  console.error("❌ Failed to collect Rust licenses:", e);
}

// ==========================
// Output
// ==========================
if (!existsSync(outputDir)) {
  mkdirSync(outputDir, { recursive: true });
}
writeFileSync(outputPath, output, "utf8");
console.log(`✅ Combined license file written to ${outputPath}`);
