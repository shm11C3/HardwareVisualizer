import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

function localeFiles(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) return localeFiles(path);
    return entry.isFile() && /\.locale\.[^.]+\.yaml$/.test(entry.name)
      ? [path]
      : [];
  });
}

export function updateLicense(directory, license, licenseUrl) {
  // Values come from the release tag, never from the workflow's current branch.
  if (!/^[A-Za-z0-9.+-]+$/.test(license)) {
    throw new Error("Expected a single SPDX license identifier.");
  }
  if (!/^https:\/\/[^\s]+$/.test(licenseUrl)) {
    throw new Error("Expected an HTTPS license URL.");
  }

  const locales = localeFiles(directory).map((path) => ({
    path,
    content: readFileSync(path, "utf8"),
  }));
  const defaults = locales.filter(({ content }) =>
    /^ManifestType: defaultLocale\r?$/m.test(content),
  );
  if (defaults.length !== 1) {
    throw new Error("Expected exactly one default locale manifest.");
  }
  if (!/^License: [^\r\n]+\r?$/m.test(defaults[0].content)) {
    throw new Error("Default locale manifest must declare License.");
  }

  // These are wingetcreate-generated scalar fields. Leave installers, hashes,
  // version manifests and unrelated locale metadata byte-for-byte intact.
  for (const { path, content } of locales) {
    const newline = content.includes("\r\n") ? "\r\n" : "\n";
    let updated = content.replace(
      /^License: [^\r\n]+/m,
      () => `License: ${license}`,
    );
    if (/^LicenseUrl:/m.test(updated)) {
      updated = updated.replace(
        /^LicenseUrl:[^\r\n]*/m,
        () => `LicenseUrl: ${licenseUrl}`,
      );
    } else if (/^License:/m.test(updated)) {
      updated = updated.replace(
        /^License:[^\r\n]*/m,
        (line) => `${line}${newline}LicenseUrl: ${licenseUrl}`,
      );
    }
    if (updated !== content) writeFileSync(path, updated, "utf8");
  }
  return dirname(defaults[0].path);
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  const [directory, license, licenseUrl] = process.argv.slice(2);
  if (!directory || !license || !licenseUrl) {
    throw new Error(
      "Usage: update-winget-license.mjs <directory> <license> <license-url>",
    );
  }
  console.log(updateLicense(directory, license, licenseUrl));
}
