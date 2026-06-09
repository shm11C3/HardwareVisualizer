import { type ChildProcess, spawn, spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, rm, writeFile } from "node:fs/promises";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  Builder,
  By,
  Capabilities,
  type Locator,
  until,
  type WebDriver,
} from "selenium-webdriver";
import {
  assertSafeE2eDataDir,
  defaultE2eAppDataDir,
} from "../support/app-data.ts";

const DRIVER_HOST = "127.0.0.1";
const DRIVER_PORT = 4444;
const DRIVER_URL = `http://${DRIVER_HOST}:${DRIVER_PORT}/`;
const DEFAULT_TIMEOUT_MS = 30_000;
const BOOT_TIMEOUT_MS = 60_000;
const REPO_ROOT = path.resolve(
  fileURLToPath(new URL("../..", import.meta.url)),
);
const CAPTURES_DIR = path.join(REPO_ROOT, "test-results", "captures");
const SCREEN_NAME = "settings";
const SCENARIO_NAME = "native-first-run-smoke";
const SUCCESS_CAPTURE = path.join(
  CAPTURES_DIR,
  SCREEN_NAME,
  `${SCENARIO_NAME}.png`,
);
const FAILURE_CAPTURE = path.join(
  CAPTURES_DIR,
  SCREEN_NAME,
  `${SCENARIO_NAME}-failure.png`,
);

const sanitizedProcessEnv = { ...process.env };
for (const key of ["GTK_PATH", "LD_LIBRARY_PATH", "GIO_MODULE_DIR"] as const) {
  delete sanitizedProcessEnv[key];
}

const e2eEnv = {
  ...sanitizedProcessEnv,
  HARDVIZ_E2E: "1",
  HARDVIZ_E2E_DEFAULT_LANGUAGE: "en",
  HARDVIZ_E2E_FORCE_CLOSE_TO_TRAY_PROMPT: "1",
  LANG: "C.UTF-8",
  LC_ALL: "C.UTF-8",
  LANGUAGE: "en",
};

const npmExecutable = process.platform === "win32" ? "npm.cmd" : "npm";
const tauriDriverExecutable =
  process.platform === "win32" ? "tauri-driver.exe" : "tauri-driver";

const log = (message: string) => {
  console.log(`[native-e2e] ${message}`);
};

const isPathCandidate = (candidate: string) =>
  path.isAbsolute(candidate) ||
  candidate.includes(path.sep) ||
  candidate.includes(path.posix.sep);

const commandExists = (command: string) => {
  if (isPathCandidate(command) && !existsSync(command)) {
    return false;
  }

  const result = spawnSync(command, ["--version"], {
    cwd: REPO_ROOT,
    env: e2eEnv,
    stdio: "ignore",
    timeout: 5_000,
    windowsHide: true,
  });

  return result.error == null && result.status != null;
};

const resolveTauriDriver = () => {
  const configured = process.env.TAURI_DRIVER_BIN;
  const candidates = [
    configured,
    tauriDriverExecutable,
    path.join(os.homedir(), ".cargo", "bin", tauriDriverExecutable),
  ].filter((candidate): candidate is string => Boolean(candidate));

  for (const candidate of candidates) {
    if (commandExists(candidate)) {
      return candidate;
    }
  }

  throw new Error(
    [
      "tauri-driver was not found.",
      "Install it with `cargo install tauri-driver --locked` or set TAURI_DRIVER_BIN.",
    ].join(" "),
  );
};

const tauriDriverArgs = () => {
  const nativeDriver = process.env.TAURI_NATIVE_DRIVER_BIN;

  return nativeDriver ? ["--native-driver", nativeDriver] : [];
};

const canConnectToDriver = () =>
  new Promise<boolean>((resolve) => {
    const socket = net.createConnection({
      host: DRIVER_HOST,
      port: DRIVER_PORT,
    });
    let settled = false;

    const settle = (value: boolean) => {
      if (settled) {
        return;
      }
      settled = true;
      socket.destroy();
      resolve(value);
    };

    socket.once("connect", () => settle(true));
    socket.once("error", () => settle(false));
    socket.setTimeout(1_000, () => settle(false));
  });

const ensureDriverPortFree = async () => {
  if (await canConnectToDriver()) {
    throw new Error(
      `${DRIVER_URL} is already accepting connections. Stop the existing tauri-driver before running native E2E.`,
    );
  }
};

const waitForDriver = async (getStartupError: () => Error | null) => {
  const startedAt = Date.now();

  while (Date.now() - startedAt < DEFAULT_TIMEOUT_MS) {
    const startupError = getStartupError();
    if (startupError) {
      throw startupError;
    }

    if (await canConnectToDriver()) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }

  throw new Error(`Timed out waiting for tauri-driver at ${DRIVER_URL}`);
};

const cleanE2eAppData = async () => {
  const e2eAppDataDir = assertSafeE2eDataDir(defaultE2eAppDataDir());

  log(`removing E2E app data: ${e2eAppDataDir}`);
  await rm(e2eAppDataDir, { force: true, recursive: true });
};

const buildE2eApp = () => {
  log("building native E2E debug app");
  const result = spawnSync(
    npmExecutable,
    [
      "run",
      "tauri",
      "build",
      "--",
      "--debug",
      "--no-bundle",
      "--config",
      "src-tauri/tauri.e2e.conf.json",
    ],
    {
      cwd: REPO_ROOT,
      env: e2eEnv,
      shell: process.platform === "win32",
      stdio: "inherit",
    },
  );

  if (result.error) {
    throw result.error;
  }

  if (result.status !== 0) {
    throw new Error(`Tauri E2E build failed with exit code ${result.status}`);
  }
};

const resolveAppBinary = () => {
  const binaryNames =
    process.platform === "win32"
      ? ["hardware-visualizer.exe", "hardware_visualizer.exe"]
      : ["hardware-visualizer", "hardware_visualizer"];
  const candidates = binaryNames.flatMap((binaryName) => [
    path.join(REPO_ROOT, "target", "debug", binaryName),
    path.join(REPO_ROOT, "src-tauri", "target", "debug", binaryName),
  ]);

  for (const candidate of candidates) {
    if (existsSync(candidate)) {
      return candidate;
    }
  }

  throw new Error(
    `Could not find native E2E app binary. Checked: ${candidates.join(", ")}`,
  );
};

const saveCapture = async (driver: WebDriver | null, capturePath: string) => {
  if (!driver) {
    return false;
  }

  await mkdir(path.dirname(capturePath), { recursive: true });
  const screenshot = await driver.takeScreenshot();
  await writeFile(capturePath, screenshot, "base64");
  return true;
};

const waitForVisible = async (
  driver: WebDriver,
  locator: Locator,
  timeoutMs = DEFAULT_TIMEOUT_MS,
) => {
  const element = await driver.wait(until.elementLocated(locator), timeoutMs);
  await driver.wait(until.elementIsVisible(element), timeoutMs);
  return element;
};

const hasExited = (child: ChildProcess) =>
  child.exitCode != null || child.signalCode != null;

const waitForChildExit = (child: ChildProcess, timeoutMs: number) =>
  new Promise<boolean>((resolve) => {
    if (hasExited(child)) {
      resolve(true);
      return;
    }

    const timer = setTimeout(() => {
      cleanup();
      resolve(false);
    }, timeoutMs);

    const onExit = () => {
      cleanup();
      resolve(true);
    };

    const cleanup = () => {
      clearTimeout(timer);
      child.off("exit", onExit);
    };

    child.once("exit", onExit);
  });

const terminateProcessTree = (child: ChildProcess, signal: NodeJS.Signals) => {
  if (!child.pid || hasExited(child)) {
    return;
  }

  if (process.platform === "win32") {
    const result = spawnSync(
      "taskkill",
      ["/pid", child.pid.toString(), "/T", "/F"],
      {
        stdio: "ignore",
        windowsHide: true,
      },
    );

    if (result.error == null && result.status === 0) {
      return;
    }
  }

  try {
    if (process.platform === "win32") {
      child.kill(signal);
    } else {
      process.kill(-child.pid, signal);
    }
  } catch {
    child.kill(signal);
  }
};

const clickVisible = async (
  driver: WebDriver,
  locator: Locator,
  timeoutMs = DEFAULT_TIMEOUT_MS,
) => {
  const element = await waitForVisible(driver, locator, timeoutMs);
  await driver.wait(until.elementIsEnabled(element), timeoutMs);
  await element.click();
  return element;
};

const runSmoke = async (driver: WebDriver) => {
  log("waiting for first-run close-to-tray prompt");
  const closeDialogButton = await waitForVisible(
    driver,
    By.css('button[aria-label="Close dialog"]'),
    BOOT_TIMEOUT_MS,
  );
  await closeDialogButton.click();
  await driver.wait(until.stalenessOf(closeDialogButton), DEFAULT_TIMEOUT_MS);

  log("opening Settings");
  await clickVisible(driver, By.css('button[aria-label="open settings"]'));

  await waitForVisible(
    driver,
    By.xpath(
      "//*[self::h1 or self::h2 or self::h3 or self::h4][normalize-space()='General']",
    ),
  );
  await waitForVisible(
    driver,
    By.xpath(
      "//*[self::h1 or self::h2 or self::h3 or self::h4][normalize-space()='About']",
    ),
  );

  const versionElement = await waitForVisible(
    driver,
    By.xpath(
      "//p[starts-with(normalize-space(), 'Version:') and string-length(normalize-space()) > string-length('Version: ')]",
    ),
  );
  const versionText = await versionElement.getText();

  if (!/^Version:\s+\S+/.test(versionText)) {
    throw new Error(
      `Expected non-empty native app version, got: ${versionText}`,
    );
  }

  await saveCapture(driver, SUCCESS_CAPTURE);
  log(`saved capture: ${path.relative(REPO_ROOT, SUCCESS_CAPTURE)}`);
};

const main = async () => {
  let driver: WebDriver | null = null;
  let tauriDriver: ChildProcess | null = null;
  let tauriDriverStartupError: Error | null = null;
  let tauriDriverExitError: Error | null = null;

  try {
    await cleanE2eAppData();
    await ensureDriverPortFree();
    const tauriDriverPath = resolveTauriDriver();
    buildE2eApp();
    const application = resolveAppBinary();

    const driverArgs = tauriDriverArgs();
    log(
      `starting tauri-driver: ${tauriDriverPath}${driverArgs.length ? ` ${driverArgs.join(" ")}` : ""}`,
    );
    tauriDriver = spawn(tauriDriverPath, driverArgs, {
      cwd: REPO_ROOT,
      detached: process.platform !== "win32",
      env: e2eEnv,
      stdio: ["ignore", "inherit", "inherit"],
    });

    tauriDriver.once("error", (error) => {
      tauriDriverStartupError = error;
    });
    tauriDriver.once("exit", (code, signal) => {
      if (!driver) {
        tauriDriverExitError = new Error(
          `tauri-driver exited before WebDriver session started (code=${code}, signal=${signal})`,
        );
      }
    });

    await waitForDriver(() => tauriDriverStartupError ?? tauriDriverExitError);

    const capabilities = new Capabilities();
    capabilities.set("tauri:options", { application });
    capabilities.setBrowserName("wry");

    log(`starting WebDriver session for ${application}`);
    driver = await new Builder()
      .withCapabilities(capabilities)
      .usingServer(DRIVER_URL)
      .build();
    await driver.manage().setTimeouts({
      implicit: 0,
      pageLoad: BOOT_TIMEOUT_MS,
      script: DEFAULT_TIMEOUT_MS,
    });

    await runSmoke(driver);
  } catch (error) {
    console.error("[native-e2e] smoke failed:", error);

    try {
      if (await saveCapture(driver, FAILURE_CAPTURE)) {
        log(
          `saved failure capture: ${path.relative(REPO_ROOT, FAILURE_CAPTURE)}`,
        );
      }
    } catch (captureError) {
      console.error(
        "[native-e2e] failed to save failure capture:",
        captureError,
      );
    }

    process.exitCode = 1;
  } finally {
    if (driver) {
      try {
        await driver.quit();
      } catch (error) {
        console.error("[native-e2e] failed to quit WebDriver session:", error);
      }
    }

    if (tauriDriver) {
      terminateProcessTree(tauriDriver, "SIGTERM");

      if (!(await waitForChildExit(tauriDriver, 5_000))) {
        terminateProcessTree(tauriDriver, "SIGKILL");
        await waitForChildExit(tauriDriver, 2_000);
      }
    }
  }
};

await main();
