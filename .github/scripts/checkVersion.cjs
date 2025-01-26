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

/**
 * タグ名のフォーマット（`publish.yaml` で設定されている値）
 */
const TAG_NAME_FORMAT = "v{VERSION}";
const OWNER = "shm11C3";
const REPO = "HardwareVisualizer";

(async () => {
  try {
    const releases = await octokit.repos.listReleases({
      owner: OWNER,
      repo: REPO,
    });
    const existingVersions = releases.data.map((release) => release.tag_name);

    console.log("existingVersions: ", existingVersions.join(", "));

    if (
      existingVersions.includes(TAG_NAME_FORMAT.replace("{VERSION}", version))
    ) {
      console.error(`Version ${version} already exists.`);
      process.exit(1);
    }

    console.log("Version check passed.");
  } catch (error) {
    console.error("Error:", error.message);
    process.exit(1);
  }
})();
