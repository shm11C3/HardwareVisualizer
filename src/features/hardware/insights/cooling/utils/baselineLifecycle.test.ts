import { describe, expect, it } from "vitest";
import { resolveBaselineLifecycle } from "./baselineLifecycle";

describe("resolveBaselineLifecycle", () => {
  it("reports loading when the source has not been fetched yet", () => {
    expect(resolveBaselineLifecycle(null)).toEqual({ kind: "loading" });
  });

  it("surfaces Core's qualifying/required day counts as-is while establishing", () => {
    expect(
      resolveBaselineLifecycle({
        status: "establishing",
        qualifyingDays: 4,
        requiredDays: 7,
      }),
    ).toEqual({ kind: "establishing", qualifyingDays: 4, requiredDays: 7 });
  });

  it("reports ready once the baseline is established", () => {
    expect(resolveBaselineLifecycle({ status: "established" })).toEqual({
      kind: "ready",
    });
  });
});
