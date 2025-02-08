/**
 * リリース用のtauri.conf.jsonに更新する
 *
 * @param {*} signCommand
 */
function updateTauriConfig(signCommand) {
  const fs = require("node:fs");
  const configPath = "src-tauri/tauri.conf.json";

  const config = JSON.parse(fs.readFileSync(configPath, "utf8"));

  config.bundle.windows.signCommand = signCommand;

  fs.writeFileSync(configPath, JSON.stringify(config, null, 2));
  console.log("tauri.conf.json has been updated");
}

updateTauriConfig(process.argv[2]);
