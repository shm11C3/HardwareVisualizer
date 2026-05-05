import { describe, expect, it } from "vitest";
import { isError, isOk, isResult } from "./result";

describe("result helpers", () => {
  it("returns false for null and primitive values", () => {
    expect(isResult(null)).toBe(false);
    expect(isResult(undefined)).toBe(false);
    expect(isResult("error")).toBe(false);
    expect(isResult(1)).toBe(false);
    expect(isResult(true)).toBe(false);
  });

  it("identifies ok and error results", () => {
    const ok = { status: "ok", data: "value" } as const;
    const error = { status: "error", error: "message" } as const;

    expect(isResult(ok)).toBe(true);
    expect(isOk(ok)).toBe(true);
    expect(isError(ok)).toBe(false);

    expect(isResult(error)).toBe(true);
    expect(isOk(error)).toBe(false);
    expect(isError(error)).toBe(true);
  });
});
