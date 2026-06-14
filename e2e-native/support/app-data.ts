import os from "node:os";
import path from "node:path";

export const E2E_APP_IDENTIFIER = "HardwareVisualizerE2E";

type Platform = NodeJS.Platform;
type Env = NodeJS.ProcessEnv;

const requireEnv = (env: Env, key: string) => {
  const value = env[key];

  if (!value) {
    throw new Error(`${key} is required to resolve the E2E app data directory`);
  }

  return value;
};

export const assertSafeE2eDataDir = (targetPath: string) => {
  const resolved = path.resolve(targetPath);
  const parsed = path.parse(resolved);
  const parent = path.dirname(resolved);

  if (resolved === parsed.root || parent === parsed.root) {
    throw new Error(`Refusing to delete root-like path: ${resolved}`);
  }

  if (path.basename(resolved) !== E2E_APP_IDENTIFIER) {
    throw new Error(
      `Refusing to delete non-E2E app data directory: ${resolved}`,
    );
  }

  return resolved;
};

export const resolveE2eAppDataDir = (
  platform: Platform = process.platform,
  env: Env = process.env,
) => {
  if (platform === "win32") {
    return assertSafeE2eDataDir(
      path.join(requireEnv(env, "APPDATA"), E2E_APP_IDENTIFIER),
    );
  }

  if (platform === "darwin") {
    return assertSafeE2eDataDir(
      path.join(
        requireEnv(env, "HOME"),
        "Library",
        "Application Support",
        E2E_APP_IDENTIFIER,
      ),
    );
  }

  return assertSafeE2eDataDir(
    path.join(requireEnv(env, "HOME"), ".config", E2E_APP_IDENTIFIER),
  );
};

export const defaultE2eAppDataDir = () =>
  resolveE2eAppDataDir(process.platform, {
    ...process.env,
    HOME: process.env.HOME ?? os.homedir(),
  });
