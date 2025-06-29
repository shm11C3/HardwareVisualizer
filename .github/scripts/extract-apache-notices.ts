import fs from "node:fs";
import path from "node:path";

const licensesFile = process.argv[2];
const outputDir = process.argv[3];

if (!licensesFile || !outputDir) {
  console.error(
    "Usage: node extract-apache-notices <licenses.json> <outputDir>",
  );
  process.exit(1);
}

const licenses: Record<
  string,
  { licenses: string; path: string; licenseFile: string }
> = JSON.parse(fs.readFileSync(licensesFile, "utf-8"));

if (!fs.existsSync(outputDir)) {
  fs.mkdirSync(outputDir, { recursive: true });
}

for (const [pkgName, info] of Object.entries(licenses)) {
  if (info.licenses === "Apache-2.0") {
    const noticePath = path.join(info.path, "NOTICE");
    if (fs.existsSync(noticePath)) {
      const content = fs.readFileSync(noticePath, "utf-8");
      const sanitized = pkgName.replace(/[\\/]/g, "_");
      fs.writeFileSync(
        path.join(outputDir, `${sanitized}_NOTICE.txt`),
        content,
      );
    } else {
      console.warn(`⚠️ NOTICE not found for: ${pkgName}`);
    }
  }
}
