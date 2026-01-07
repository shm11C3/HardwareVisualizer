type NeedResult = "success" | "failure" | "cancelled" | "skipped" | string;

type NeedsObject = Record<
  string,
  {
    result?: NeedResult;
    outputs?: Record<string, string>;
  }
>;

function parseNeedsJson(): NeedsObject {
  const raw = process.env.NEEDS_JSON;
  if (!raw) {
    throw new Error(
      // biome-ignore lint/suspicious/noTemplateCurlyInString: this message is intended to be used in GitHub Actions
      "NEEDS_JSON is not set. Pass env: NEEDS_JSON: ${{ toJson(needs) }}",
    );
  }
  try {
    return JSON.parse(raw) as NeedsObject;
  } catch (e) {
    throw new Error(`Failed to parse NEEDS_JSON: ${(e as Error).message}`);
  }
}

function main() {
  const needs = parseNeedsJson();

  const targets = Object.keys(needs);
  if (targets.length === 0) {
    throw new Error(
      "No jobs found in NEEDS_JSON. Check merge-gate job's 'needs:' in the workflow.",
    );
  }

  let failed = false;

  console.log("=== job results ===");
  for (const name of targets) {
    const result = needs[name]?.result ?? "unknown";
    console.log(`${name}: ${result}`);

    // success and skipped are OK; other results (failure/cancelled/unknown, etc.) are NG
    if (result !== "success" && result !== "skipped") {
      console.error(`NG  - ${name} (${result})`);
      failed = true;
    } else {
      console.log(`OK  - ${name} (${result})`);
    }
  }

  if (failed) {
    console.error("Merge gate failed.");
    process.exit(1);
  }

  console.log("Merge gate passed.");
}

main();
