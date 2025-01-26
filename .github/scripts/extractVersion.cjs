const fs = require("node:fs");
const path = require("node:path");
const toml = require("toml");

const tauriDir = path.join(__dirname, "../../src-tauri");
let version = null;

function parseJsonFile(filepath) {
  const contents = fs.readFileSync(filepath, "utf-8");
  return JSON.parse(contents);
}

function parseTomlFile(filepath) {
  const contents = fs.readFileSync(filepath, "utf-8");
  return toml.parse(contents);
}

if (fs.existsSync(path.join(tauriDir, "tauri.conf.json"))) {
  const config = parseJsonFile(path.join(tauriDir, "tauri.conf.json"));
  version = config.version;
} else if (fs.existsSync(path.join(tauriDir, "tauri.conf.json5"))) {
  const config = parseJsonFile(path.join(tauriDir, "tauri.conf.json5"));
  version = config.version;
} else if (fs.existsSync(path.join(tauriDir, "Tauri.toml"))) {
  const config = parseTomlFile(path.join(tauriDir, "Tauri.toml"));
  version = config.version;
}

if (!version) {
  console.error("Version not found in Tauri configuration files.");
  process.exit(1);
}

console.log(`Version: ${version}`);

// GitHub Actionsの出力として設定
const outputFilePath = process.env.GITHUB_OUTPUT;
fs.appendFileSync(outputFilePath, `version=${version}\n`);
