const { Octokit } = require("@octokit/rest");

// GitHub Actionsから取得したバージョン
const version = process.env.VERSION;
if (!version) {
  console.error("VERSION environment variable is not set.");
  process.exit(1);
}

// GitHubトークンを取得
const githubToken = process.env.GITHUB_TOKEN;
if (!githubToken) {
  console.error("GITHUB_TOKEN is not set.");
  process.exit(1);
}

const octokit = new Octokit({ auth: githubToken });

(async () => {
  try {
    const owner = "shm11C3";
    const repo = "HardwareVisualizer";

    const releases = await octokit.repos.listReleases({ owner, repo });
    const existingVersions = releases.data.map((release) => release.tag_name);

    if (existingVersions.includes(version)) {
      console.error(`Version ${version} already exists.`);
      process.exit(1);
    }

    console.log("Version check passed.");
  } catch (error) {
    console.error("Error:", error.message);
    process.exit(1);
  }
})();
