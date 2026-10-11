import { describe, expect, it } from "vitest";
import { chartConfig } from "@/consts/chart";
import { RingBuffer } from "@/features/hardware/live/ringBuffer";

describe("RingBuffer", () => {
  it("defaults to the chart history length", () => {
    expect(new RingBuffer<number>().capacity).toBe(
      chartConfig.historyLengthSec,
    );
  });

  it("rejects a capacity that cannot hold a value", () => {
    expect(() => new RingBuffer<number>(0)).toThrow(RangeError);
    expect(() => new RingBuffer<number>(1.5)).toThrow(RangeError);
  });

  it("holds values oldest to newest while it fills", () => {
    const buffer = new RingBuffer<number>(3);
    expect(buffer.size).toBe(0);
    expect(buffer.latest()).toBeUndefined();
    expect(buffer.oldest()).toBeUndefined();

    buffer.push(1);
    buffer.push(2);

    expect(buffer.size).toBe(2);
    expect(buffer.toArray()).toEqual([1, 2]);
    expect(buffer.oldest()).toBe(1);
    expect(buffer.latest()).toBe(2);
  });

  it("overwrites the oldest value once full, across several wraparounds", () => {
    const buffer = new RingBuffer<number>(3);
    for (let value = 1; value <= 8; value += 1) {
      buffer.push(value);
    }

    expect(buffer.size).toBe(3);
    expect(buffer.toArray()).toEqual([6, 7, 8]);
    expect(buffer.oldest()).toBe(6);
    expect(buffer.latest()).toBe(8);
  });

  it("pads on the left up to capacity, and not at all when full", () => {
    const buffer = new RingBuffer<number>(4);
    expect(buffer.toPaddedArray(null)).toEqual([null, null, null, null]);

    buffer.push(10);
    buffer.push(20);
    expect(buffer.toPaddedArray(null)).toEqual([null, null, 10, 20]);

    buffer.push(30);
    buffer.push(40);
    buffer.push(50);
    expect(buffer.toPaddedArray(null)).toEqual([20, 30, 40, 50]);
  });

  it("keeps null as a value, distinct from the padding", () => {
    const buffer = new RingBuffer<number | null>(3);
    buffer.push(1);
    buffer.push(null);

    expect(buffer.size).toBe(2);
    expect(buffer.toPaddedArray(0)).toEqual([0, 1, null]);
  });

  it("returns a fresh array each time, so a caller cannot reach the slots", () => {
    const buffer = new RingBuffer<number>(2);
    buffer.push(1);
    const first = buffer.toPaddedArray(null);
    first[1] = 99;

    expect(buffer.toPaddedArray(null)).toEqual([null, 1]);
    expect(buffer.toArray()).not.toBe(buffer.toArray());
  });

  it("starts over after clear", () => {
    const buffer = new RingBuffer<number>(3);
    for (let value = 1; value <= 5; value += 1) {
      buffer.push(value);
    }

    buffer.clear();

    expect(buffer.size).toBe(0);
    expect(buffer.latest()).toBeUndefined();
    expect(buffer.toArray()).toEqual([]);
    buffer.push(7);
    expect(buffer.toPaddedArray(null)).toEqual([null, null, 7]);
  });
});
