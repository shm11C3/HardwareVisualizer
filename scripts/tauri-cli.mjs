import { spawn } from "node:child_process";
import { existsSync } from "node:fs";

const DEV_CONFIG = "src-tauri/tauri.dev.conf.json";
const LOCAL_TAURI_BIN =
  process.platform === "win32"
    ? "node_modules/.bin/tauri.cmd"
    : "node_modules/.bin/tauri";

const args = process.argv.slice(2);

const hasConfigArg = args.some(
  (arg, index) =>
    arg === "--config" ||
    arg.startsWith("--config=") ||
    args[index - 1] === "--config",
);

const tauriArgs =
  args[0] === "dev" && !hasConfigArg
    ? ["dev", "--config", DEV_CONFIG, ...args.slice(1)]
    : args;

const command = existsSync(LOCAL_TAURI_BIN) ? LOCAL_TAURI_BIN : "tauri";

const child = spawn(command, tauriArgs, {
  stdio: "inherit",
  shell: process.platform === "win32",
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }

  process.exit(code ?? 1);
});

child.on("error", (error) => {
  console.error(error);
  process.exit(1);
});
