import { chartConfig } from "@/consts/chart";

/**
 * A fixed-capacity window over a stream: the newest `capacity` values, oldest
 * first. Pushing past capacity overwrites the oldest value in place, so a
 * buffer never allocates after construction.
 *
 * It is plain mutable data. Whoever owns it decides who may mutate it and how
 * readers hear about a change (see `store/liveMetrics.ts`).
 */
export class RingBuffer<T> {
  readonly capacity: number;
  readonly #slots: (T | undefined)[];
  /** Index the next push writes to. */
  #next = 0;
  #size = 0;

  constructor(capacity: number = chartConfig.historyLengthSec) {
    if (!Number.isInteger(capacity) || capacity < 1) {
      throw new RangeError(`RingBuffer capacity must be >= 1, got ${capacity}`);
    }
    this.capacity = capacity;
    this.#slots = new Array<T | undefined>(capacity);
  }

  /** Number of values currently held, at most `capacity`. */
  get size(): number {
    return this.#size;
  }

  push(value: T): void {
    this.#slots[this.#next] = value;
    this.#next = (this.#next + 1) % this.capacity;
    if (this.#size < this.capacity) {
      this.#size += 1;
    }
  }

  /** The newest value, or `undefined` when empty. */
  latest(): T | undefined {
    if (this.#size === 0) {
      return undefined;
    }
    return this.#slots[(this.#next - 1 + this.capacity) % this.capacity];
  }

  /** The oldest value still held, or `undefined` when empty. */
  oldest(): T | undefined {
    if (this.#size === 0) {
      return undefined;
    }
    return this.#slots[
      (this.#next - this.#size + this.capacity) % this.capacity
    ];
  }

  clear(): void {
    this.#slots.fill(undefined);
    this.#next = 0;
    this.#size = 0;
  }

  /** The held values, oldest to newest. Always a fresh array. */
  toArray(): T[] {
    const start = (this.#next - this.#size + this.capacity) % this.capacity;
    return Array.from(
      { length: this.#size },
      (_, offset) => this.#slots[(start + offset) % this.capacity] as T,
    );
  }

  /**
   * The held values, oldest to newest, left-padded with `fill` up to
   * `capacity`: the fixed-width series a chart draws. Always a fresh array.
   */
  toPaddedArray<F>(fill: F): (T | F)[] {
    const padding = this.capacity - this.#size;
    const padded: (T | F)[] = Array.from({ length: padding }, () => fill);
    for (const value of this.toArray()) {
      padded.push(value);
    }
    return padded;
  }
}
