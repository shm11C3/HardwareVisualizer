import { cn } from "@/lib/utils";
import { describe, expect, it } from "vitest";

describe("cn", () => {
  it("should merge class names and remove duplicates", () => {
    const result = cn("bg-red-500", "text-white", "bg-red-500");
    const classes = result.split(/\s+/).sort();
    expect(classes).toEqual(["bg-red-500", "text-white"]);
  });

  it("should handle conditional class names", () => {
    const result = cn("foo", { bar: true, baz: false }, [
      "qux",
      { quux: true },
    ]);
    // Order of classes should match clsx/twMerge behavior
    expect(result).toBe("foo bar qux quux");
  });
});
