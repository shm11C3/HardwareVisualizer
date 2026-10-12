import { atom, createStore } from "jotai";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createSharedRead } from "@/features/hardware/store/sharedRead";

type Deferred<T> = {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (reason: Error) => void;
};

const defer = <T>(): Deferred<T> => {
  let resolve: (value: T) => void = () => {};
  let reject: (reason: Error) => void = () => {};
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
};

const flush = async () => {
  for (let i = 0; i < 5; i++) {
    await Promise.resolve();
  }
};

/**
 * One shared read keyed by `days`, with every request handed back as a
 * deferred promise so a test decides when and in which order they land.
 */
const setup = () => {
  const daysAtom = atom<90 | 365 | null>(90);
  const requests: { days: number; deferred: Deferred<string> }[] = [];
  const fetch = vi.fn((days: number) => {
    const deferred = defer<string>();
    requests.push({ days, deferred });
    return deferred.promise;
  });
  const readAtom = createSharedRead<90 | 365, string>({
    params: daysAtom,
    fetch,
  });
  const store = createStore();
  const subscribe = () => store.sub(readAtom, () => {});
  const settle = async (index: number, value: string) => {
    requests[index]?.deferred.resolve(value);
    await flush();
  };
  return { daysAtom, readAtom, requests, fetch, store, subscribe, settle };
};

describe("createSharedRead", () => {
  beforeEach(() => {
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("reads loading, then the data together with the params it was requested for", async () => {
    const { readAtom, store, subscribe, settle } = setup();
    subscribe();

    expect(store.get(readAtom)).toEqual({ kind: "loading" });

    await settle(0, "d90");

    expect(store.get(readAtom)).toEqual({
      kind: "ready",
      params: 90,
      data: "d90",
    });
  });

  it("reads loading after a period change instead of the previous period's data", async () => {
    const { daysAtom, readAtom, store, subscribe, settle } = setup();
    subscribe();
    await settle(0, "d90");
    expect(store.get(readAtom).kind).toBe("ready");

    store.set(daysAtom, 365);

    // The 365-day request is in flight: nothing may show the 90-day data
    // under the new period's label.
    expect(store.get(readAtom)).toEqual({ kind: "loading" });

    await settle(1, "d365");

    expect(store.get(readAtom)).toEqual({
      kind: "ready",
      params: 365,
      data: "d365",
    });
  });

  it("keeps the last data visible while a same-params refresh is in flight", async () => {
    const { readAtom, requests, store, subscribe, settle } = setup();
    subscribe();
    await settle(0, "first");

    store.set(readAtom, "refresh");
    await flush();

    expect(requests).toHaveLength(2);
    expect(store.get(readAtom)).toEqual({
      kind: "ready",
      params: 90,
      data: "first",
    });

    await settle(1, "second");

    expect(store.get(readAtom)).toEqual({
      kind: "ready",
      params: 90,
      data: "second",
    });
  });

  it("replaces the data with a failure that names its params, without rejecting", async () => {
    const { readAtom, requests, store, subscribe, settle } = setup();
    subscribe();
    await settle(0, "first");

    store.set(readAtom, "refresh");
    requests[1]?.deferred.reject(new Error("db locked"));
    await flush();

    expect(store.get(readAtom)).toEqual({ kind: "failed", params: 90 });
    expect(console.error).toHaveBeenCalledWith(
      expect.objectContaining({ message: "db locked" }),
    );
  });

  it("recovers from a failure when the retry lands", async () => {
    const { readAtom, requests, store, subscribe, settle } = setup();
    subscribe();
    requests[0]?.deferred.reject(new Error("db locked"));
    await flush();
    expect(store.get(readAtom)).toEqual({ kind: "failed", params: 90 });

    store.set(readAtom, "refresh");
    // The retry gives "Try again" a visible response instead of leaving the
    // failure on screen until the result lands.
    expect(store.get(readAtom)).toEqual({ kind: "loading" });
    await settle(1, "second");

    expect(store.get(readAtom)).toEqual({
      kind: "ready",
      params: 90,
      data: "second",
    });
  });

  it("drops a superseded request that lands after the newer one", async () => {
    const { readAtom, requests, store, subscribe, settle } = setup();
    subscribe();
    await settle(0, "first");

    store.set(readAtom, "refresh");
    await flush();
    store.set(readAtom, "refresh");
    await flush();
    expect(requests).toHaveLength(3);

    await settle(2, "newest");
    await settle(1, "superseded");

    expect(store.get(readAtom)).toEqual({
      kind: "ready",
      params: 90,
      data: "newest",
    });
  });

  it("drops a late failure from a request the period change superseded", async () => {
    const { daysAtom, readAtom, requests, store, subscribe, settle } = setup();
    subscribe();

    store.set(daysAtom, 365);
    await settle(1, "d365");
    requests[0]?.deferred.reject(new Error("late failure"));
    await flush();

    expect(store.get(readAtom)).toEqual({
      kind: "ready",
      params: 365,
      data: "d365",
    });
  });

  it("neither fetches nor claims data while the params are null", async () => {
    const { daysAtom, readAtom, fetch, store, subscribe, settle } = setup();
    store.set(daysAtom, null);
    subscribe();
    await flush();

    expect(fetch).not.toHaveBeenCalled();
    expect(store.get(readAtom)).toEqual({ kind: "loading" });

    store.set(daysAtom, 90);
    await settle(0, "d90");

    expect(store.get(readAtom).kind).toBe("ready");
  });

  it("serves two subscribers with one request", async () => {
    const { fetch, store, readAtom, subscribe, settle } = setup();
    subscribe();
    subscribe();
    await settle(0, "d90");

    expect(fetch).toHaveBeenCalledTimes(1);
    expect(store.get(readAtom).kind).toBe("ready");
  });

  it("does not duplicate a request that is still in flight when the subscribers leave and return", async () => {
    const { fetch, store, readAtom, subscribe, settle } = setup();
    const unsubscribe = subscribe();
    unsubscribe();
    subscribe();
    await flush();

    expect(fetch).toHaveBeenCalledTimes(1);

    await settle(0, "d90");

    expect(store.get(readAtom).kind).toBe("ready");
    expect(fetch).toHaveBeenCalledTimes(1);
  });

  it("does not refetch on the first subscription when the response beat the subscriber", async () => {
    // A component reads during render and subscribes in an effect; a fast
    // response can land between the two.
    const { fetch, store, readAtom, subscribe, settle } = setup();
    store.get(readAtom);
    await settle(0, "d90");

    subscribe();
    await flush();

    expect(fetch).toHaveBeenCalledTimes(1);
    expect(store.get(readAtom).kind).toBe("ready");
  });

  it("does not fetch while nothing subscribes", async () => {
    const { fetch, store, readAtom, subscribe, settle } = setup();
    const unsubscribe = subscribe();
    await settle(0, "d90");
    unsubscribe();

    // Reading the settled state later is not a reason to ask again.
    expect(store.get(readAtom).kind).toBe("ready");
    await flush();

    expect(fetch).toHaveBeenCalledTimes(1);
  });

  it("refetches a settled read on return, keeping the previous data visible meanwhile", async () => {
    const { fetch, store, readAtom, subscribe, settle } = setup();
    const unsubscribe = subscribe();
    await settle(0, "visit one");
    unsubscribe();

    subscribe();
    await flush();

    expect(fetch).toHaveBeenCalledTimes(2);
    expect(store.get(readAtom)).toEqual({
      kind: "ready",
      params: 90,
      data: "visit one",
    });

    await settle(1, "visit two");

    expect(store.get(readAtom)).toEqual({
      kind: "ready",
      params: 90,
      data: "visit two",
    });
  });

  it("refetches on return when the request the subscribers left behind settled meanwhile", async () => {
    const { fetch, subscribe, settle } = setup();
    const unsubscribe = subscribe();
    unsubscribe();
    await settle(0, "settled while away");

    subscribe();
    await flush();

    expect(fetch).toHaveBeenCalledTimes(2);
  });
});
