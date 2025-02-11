import { convertFileToBase64 } from "@/lib/file";
import { describe, expect, it } from "vitest";

describe("convertFileToBase64", () => {
  it("should convert a valid file to a base64 string", async () => {
    const file = new File(["test, test, test"], "test.txt", {
      type: "text/plain",
    });
    const base64Str = await convertFileToBase64(file);
    expect(base64Str).toMatch(/^data:text\/plain;base64,/);
  });
});
