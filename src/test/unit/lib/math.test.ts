import { describe, expect, it, vi } from "vitest";
import { randInt } from "@/lib/math";

describe("math utilities", () => {
  describe("randInt", () => {
    it("should return integer within given range", () => {
      const result = randInt(1, 10);
      expect(result).toBeGreaterThanOrEqual(1);
      expect(result).toBeLessThanOrEqual(10);
      expect(Number.isInteger(result)).toBe(true);
    });

    it("should return min when min equals max", () => {
      const result = randInt(5, 5);
      expect(result).toBe(5);
    });

    it("should handle negative ranges", () => {
      const result = randInt(-10, -5);
      expect(result).toBeGreaterThanOrEqual(-10);
      expect(result).toBeLessThanOrEqual(-5);
      expect(Number.isInteger(result)).toBe(true);
    });

    it("should use Math.random and Math.floor correctly", () => {
      const mathRandomSpy = vi.spyOn(Math, "random").mockReturnValue(0.5);
      const mathFloorSpy = vi.spyOn(Math, "floor");

      randInt(1, 10);

      expect(mathRandomSpy).toHaveBeenCalled();
      expect(mathFloorSpy).toHaveBeenCalled();

      mathRandomSpy.mockRestore();
      mathFloorSpy.mockRestore();
    });
  });
});
