import { RGB2HEX } from "@/lib/color";
import { describe, expect, it } from "vitest";

describe("RGB2HEX", () => {
  it.each([
    { input: "0,0,0", expected: "#000000" },
    { input: "255,255,255", expected: "#ffffff" },
    { input: "255,0,0", expected: "#ff0000" },
    { input: "0,255,0", expected: "#00ff00" },
  ])("RGB文字列をHEX文字列に変換する", ({ input, expected }) => {
    const result = RGB2HEX(input);
    expect(result).toBe(expected);
  });
});
