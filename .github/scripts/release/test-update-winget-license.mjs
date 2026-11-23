import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { updateLicense } from "./update-winget-license.mjs";

const directory = mkdtempSync(join(tmpdir(), "winget-license-"));
const defaultPath = join(
  directory,
  "shm11C3.HardwareVisualizer.locale.en-US.yaml",
);
const localePath = join(
  directory,
  "shm11C3.HardwareVisualizer.locale.ja-JP.yaml",
);
const partialLocalePath = join(
  directory,
  "shm11C3.HardwareVisualizer.locale.ru-RU.yaml",
);
const installerPath = join(
  directory,
  "shm11C3.HardwareVisualizer.installer.yaml",
);
const versionPath = join(directory, "shm11C3.HardwareVisualizer.yaml");
const licenseUrl =
  "https://github.com/shm11C3/HardwareVisualizer/blob/v1.11.1/LICENSE";
const original =
  "PackageVersion: 1.11.1\r\nLicense: MIT\r\nShortDescription: Hardware monitor.\r\nManifestType: defaultLocale\r\n";
writeFileSync(defaultPath, original);
writeFileSync(
  localePath,
  "License: MIT\nLicenseUrl: https://example.com/MIT\nManifestType: locale\n",
);
writeFileSync(
  partialLocalePath,
  "ShortDescription: Localized description.\nManifestType: locale\n",
);
writeFileSync(
  installerPath,
  "InstallerUrl: https://example.com/app.msi\nInstallerSha256: unchanged\n",
);
writeFileSync(versionPath, "PackageVersion: 1.11.1\nManifestType: version\n");
const untouched = [installerPath, versionPath, partialLocalePath].map((path) =>
  readFileSync(path),
);

assert.equal(
  updateLicense(directory, "GPL-3.0-or-later", licenseUrl),
  directory,
);
const expected = original.replace(
  "License: MIT",
  `License: GPL-3.0-or-later\r\nLicenseUrl: ${licenseUrl}`,
);
assert.equal(readFileSync(defaultPath, "utf8"), expected);
assert.equal(
  readFileSync(localePath, "utf8"),
  `License: GPL-3.0-or-later\nLicenseUrl: ${licenseUrl}\nManifestType: locale\n`,
);
updateLicense(directory, "GPL-3.0-or-later", licenseUrl);
assert.equal(
  readFileSync(defaultPath, "utf8"),
  expected,
  "Correction must be idempotent.",
);
for (const [index, path] of [
  installerPath,
  versionPath,
  partialLocalePath,
].entries()) {
  assert.deepEqual(readFileSync(path), untouched[index]);
}

// A historical tag still declares MIT even when the submitting workflow is GPL.
const historicalUrl = licenseUrl.replace("v1.11.1", "v1.10.1");
updateLicense(directory, "MIT", historicalUrl);
assert.match(readFileSync(defaultPath, "utf8"), /^License: MIT\r?$/m);
assert.ok(
  readFileSync(defaultPath, "utf8").includes(`LicenseUrl: ${historicalUrl}`),
);
assert.throws(
  () => updateLicense(directory, "MIT\nInjected: value", licenseUrl),
  /SPDX/,
);
assert.throws(
  () => updateLicense(directory, "MIT", "http://example.com/LICENSE"),
  /HTTPS/,
);
writeFileSync(defaultPath, "ManifestType: defaultLocale\n");
assert.throws(
  () => updateLicense(directory, "MIT", historicalUrl),
  /must declare License/,
);
writeFileSync(defaultPath, "ManifestType: locale\n");
assert.throws(
  () => updateLicense(directory, "MIT", historicalUrl),
  /exactly one default locale/,
);
writeFileSync(defaultPath, original);
writeFileSync(localePath, original);
assert.throws(
  () => updateLicense(directory, "MIT", historicalUrl),
  /exactly one default locale/,
);

console.log("WinGet license metadata tests passed.");
