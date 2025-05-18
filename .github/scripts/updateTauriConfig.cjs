/**
 * リリース用のtauri.conf.jsonに更新する
 *
 * @param {string[]} args
 */
function updateTauriConfig(args) {
  const fs = require("node:fs");
  const configPath = "src-tauri/tauri.conf.json";

  const signCommand = args[0];
  const pubkey = args[1];

  const config = JSON.parse(fs.readFileSync(configPath, "utf8"));

  config.bundle.windows.signCommand = signCommand;
  config.bundle.createUpdaterArtifacts = true;
  config.plugins.updater = {
    dialog: true,
    pubkey,
    endpoints: [
      "https://github.com/shm11C3/HardwareVisualizer/releases/latest/download/latest.json",
    ],
    windows: {
      installMode: "passive",
    },
  };

  fs.writeFileSync(configPath, JSON.stringify(config, null, 2));
  console.log("tauri.conf.json has been updated");
}

updateTauriConfig(process.argv.slice(2));
