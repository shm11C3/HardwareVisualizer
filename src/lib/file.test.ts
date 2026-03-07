import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { convertFileToBase64 } from "@/lib/file";

describe("convertFileToBase64", () => {
  it("should convert a valid file to a base64 string", async () => {
    const file = new File(["test, test, test"], "test.txt", {
      type: "text/plain",
    });
    const base64Str = await convertFileToBase64(file);
    expect(base64Str).toMatch(/^data:text\/plain;base64,/);
  });

  describe("onerror path", () => {
    const OriginalFileReader = global.FileReader;

    beforeEach(() => {
      // Replace FileReader with one that fires onerror after handlers are assigned
      class ErrorFileReader {
        onload: (() => void) | null = null;
        onerror: ((e: ProgressEvent) => void) | null = null;
        result: string | ArrayBuffer | null = null;
        readAsDataURL(_: Blob) {
          // Defer to allow synchronous handler assignment before firing
          Promise.resolve().then(() => {
            if (this.onerror) {
              this.onerror(new ProgressEvent("error"));
            }
          });
        }
        addEventListener() {}
        removeEventListener() {}
      }
      vi.stubGlobal("FileReader", ErrorFileReader);
    });

    afterEach(() => {
      vi.stubGlobal("FileReader", OriginalFileReader);
    });

    it("should reject when FileReader fires onerror", async () => {
      const file = new File(["data"], "test.txt", { type: "text/plain" });
      await expect(convertFileToBase64(file)).rejects.toBeInstanceOf(
        ProgressEvent,
      );
    });
  });
});
