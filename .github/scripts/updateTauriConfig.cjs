/**
 * Update `tauri.conf.json` for release
 *
 * @param {string[]} args
 */
function updateTauriConfig(args) {
  const fs = require("node:fs");
  const configPath = "src-tauri/tauri.conf.json";

  const getArg = (name) => {
    const i = args.indexOf(name);
    if (i === -1) return null;
    return args[i + 1] ?? null;
  };

  const version = getArg("--version");
  const signCommand = getArg("--sign");

  const config = JSON.parse(fs.readFileSync(configPath, "utf8"));

  if (version) {
    config.version = version;
  }
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

updateTauriConfig(process.argv.slice(2));
