import { transpose } from "@/lib/array";
import { describe, expect, it } from "vitest";

describe("transpose", () => {
  it("should return an empty array when given an empty matrix", () => {
    expect(transpose([])).toEqual([]);
  });

  it("should transpose a matrix", () => {
    const matrix = [
      [1, 2, 3],
      [4, 5, 6],
    ];
    const result = transpose(matrix);
    expect(result).toEqual([
      [1, 4],
      [2, 5],
      [3, 6],
    ]);
  });
});
