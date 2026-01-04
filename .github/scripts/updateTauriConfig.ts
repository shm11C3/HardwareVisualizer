import fs from "node:fs";

type Stage = "alpha" | "beta" | "rc";

function getArg(args: string[], name: string) {
  const i = args.indexOf(name);
  if (i === -1) return null;
  return args[i + 1] ?? null;
}

/**
 * Convert tag to MSI-safe version string.
 *
 * tag:
 *  - vMAJOR.MINOR.PATCH
 *  - vMAJOR.MINOR.PATCH-(alpha|beta|rc).N
 *
 * output:
 *  - MAJOR.MINOR.PATCH
 *  - MAJOR.MINOR.PATCH-<numeric>  (numeric-only prerelease, <= 65535)
 */
function tagToTauriVersion(tag: string): string {
  const m = tag.match(/^v(\d+)\.(\d+)\.(\d+)(?:-(alpha|beta|rc)\.(\d+))?$/);
  if (!m) {
    throw new Error(`Invalid tag format: ${tag}`);
  }

  const major = Number(m[1]);
  const minor = Number(m[2]);
  const patch = Number(m[3]);
  const stage = m[4] as Stage | undefined;
  const stageNumRaw = m[5];

  if (![major, minor, patch].every((n) => Number.isInteger(n) && n >= 0)) {
    throw new Error(`Invalid version numbers in tag: ${tag}`);
  }

  let version = `${major}.${minor}.${patch}`;

  if (!stage) return version;

  const stageNum = Number(stageNumRaw);
  if (!Number.isInteger(stageNum) || stageNum < 0 || stageNum > 9999) {
    throw new Error(`Prerelease number must be 0..9999 (tag: ${tag})`);
  }

  const stageBase: Record<Stage, number> = {
    alpha: 10000,
    beta: 20000,
    rc: 30000,
  };

  const encoded = stageBase[stage] + stageNum;
  if (encoded > 65535) {
    throw new Error(
      `Encoded prerelease must be <= 65535 (got ${encoded}, tag: ${tag})`,
    );
  }

  // MSI-safe: numeric-only prerelease
  version = `${version}-${encoded}`;
  return version;
}

/**
 * Update `tauri.conf.json` for release
 *
 * @param {string[]} args
 */
function main(args: string[]) {
  const configPath = "src-tauri/tauri.conf.json";

  const version = getArg(args, "--version");
  const signCommand = getArg(args, "--sign");

  if (!version) {
    console.error("--version is required");
    process.exit(1);
  }

  const tauriVersion = tagToTauriVersion(version);

  const config = JSON.parse(fs.readFileSync(configPath, "utf8"));
  config.version = tauriVersion;

  if (signCommand) {
    config.bundle ??= {};
    config.bundle.windows ??= {};
    config.bundle.windows.signCommand = signCommand;
  }

  config.bundle ??= {};
  config.bundle.createUpdaterArtifacts = true;

  fs.writeFileSync(configPath, JSON.stringify(config, null, 2));
  console.log("tauri.conf.json has been updated");
  console.log(`version: ${config.version}`);
}

try {
  main(process.argv.slice(2));
} catch (e) {
  console.error(e instanceof Error ? e.message : e);
  process.exit(1);
}
