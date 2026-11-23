import { describe, expect, it } from "vitest";
import { shallowEqualArray, shallowEqualRecord } from "@/lib/shallowEqual";

describe("shallowEqualRecord", () => {
  it("treats two empty records as equal", () => {
    expect(shallowEqualRecord({}, {})).toBe(true);
  });

  it("is equal for the same keys and Object.is-equal values", () => {
    expect(shallowEqualRecord({ a: 1, b: "x" }, { a: 1, b: "x" })).toBe(true);
    expect(shallowEqualRecord({ a: null }, { a: null })).toBe(true);
  });

  it("is not equal when a value differs", () => {
    expect(shallowEqualRecord({ a: 1 }, { a: 2 })).toBe(false);
  });

  it("does not treat NaN as different from NaN, nor 0 as the same as -0", () => {
    expect(shallowEqualRecord({ a: Number.NaN }, { a: Number.NaN })).toBe(true);
    expect(shallowEqualRecord({ a: 0 }, { a: -0 })).toBe(false);
  });

  it("is not equal when a key is added, removed or swapped", () => {
    expect(shallowEqualRecord<object>({ a: 1 }, { a: 1, b: 2 })).toBe(false);
    expect(shallowEqualRecord<object>({ a: 1, b: 2 }, { a: 1 })).toBe(false);
    expect(shallowEqualRecord<object>({ a: undefined }, { b: undefined })).toBe(
      false,
    );
  });

  it("is not equal when only the key order differs", () => {
    expect(shallowEqualRecord({ a: 1, b: 2 }, { b: 2, a: 1 })).toBe(false);
  });

  it("compares nested objects by reference unless told otherwise", () => {
    expect(shallowEqualRecord({ g: { v: 1 } }, { g: { v: 1 } })).toBe(false);
    expect(
      shallowEqualRecord({ g: { v: 1 } }, { g: { v: 1 } }, shallowEqualRecord),
    ).toBe(true);
    expect(
      shallowEqualRecord({ g: { v: 1 } }, { g: { v: 2 } }, shallowEqualRecord),
    ).toBe(false);
  });
});

describe("shallowEqualArray", () => {
  it("treats two empty arrays as equal", () => {
    expect(shallowEqualArray([], [])).toBe(true);
  });

  it("is equal for the same items in the same order", () => {
    expect(shallowEqualArray([1, 2, 3], [1, 2, 3])).toBe(true);
  });

  it("is not equal when length, an item or the order differs", () => {
    expect(shallowEqualArray([1, 2], [1, 2, 3])).toBe(false);
    expect(shallowEqualArray([1, 2, 3], [1, 2, 4])).toBe(false);
    expect(shallowEqualArray([1, 2], [2, 1])).toBe(false);
  });

  it("compares items by reference unless told otherwise", () => {
    expect(shallowEqualArray([{ n: 1 }], [{ n: 1 }])).toBe(false);
    expect(shallowEqualArray([{ n: 1 }], [{ n: 1 }], shallowEqualRecord)).toBe(
      true,
    );
    expect(shallowEqualArray([{ n: 1 }], [{ n: 2 }], shallowEqualRecord)).toBe(
      false,
    );
  });
});
