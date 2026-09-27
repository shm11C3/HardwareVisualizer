import { act, renderHook } from "@testing-library/react";
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  type Mock,
  vi,
} from "vitest";

let commands: {
  getDatabaseConversionState: Mock;
  startDatabaseConversion: Mock;
  cancelDatabaseConversion: Mock;
  rebuildNativeDatabaseFromSqlite: Mock;
};
let useDatabaseConversion: () => ReturnType<
  typeof import("./useDatabaseConversion").useDatabaseConversion
>;

describe("useDatabaseConversion", () => {
  beforeEach(async () => {
    vi.useFakeTimers();
    // Reload the module (and its module-level, cross-mount Jotai atoms - see
    // `lastObservedDatabaseConversionKindAtom` and
    // `databaseConversionJustCompletedAtom`) fresh for each test, the same
    // isolation pattern `useDatabaseConversionNoticeShown`'s own test uses -
    // otherwise an atom's in-memory value would leak between tests.
    vi.resetModules();

    commands = {
      getDatabaseConversionState: vi.fn(),
      startDatabaseConversion: vi.fn(),
      cancelDatabaseConversion: vi.fn(),
      rebuildNativeDatabaseFromSqlite: vi.fn(),
    };
    vi.doMock("@/rspc/bindings", () => ({ commands }));

    const module = await import("./useDatabaseConversion");
    useDatabaseConversion = module.useDatabaseConversion;
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
    vi.resetModules();
  });

  it("fetches the state once on mount", async () => {
    (commands.getDatabaseConversionState as Mock).mockResolvedValue({
      kind: "sqliteAuthoritative",
    });

    const { result } = renderHook(() => useDatabaseConversion());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });

    expect(result.current.state).toEqual({ kind: "sqliteAuthoritative" });
    expect(commands.getDatabaseConversionState).toHaveBeenCalledTimes(1);
  });

  it("settles once the first state read completes", async () => {
    (commands.getDatabaseConversionState as Mock).mockResolvedValue({
      kind: "notSupported",
    });

    const { result } = renderHook(() => useDatabaseConversion());
    expect(result.current.settled).toBe(false);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });

    expect(result.current.settled).toBe(true);
  });

  it("polls while converting and stops once it leaves that state", async () => {
    (commands.getDatabaseConversionState as Mock)
      .mockResolvedValueOnce({
        kind: "converting",
        step: "preflight",
      })
      .mockResolvedValueOnce({
        kind: "converting",
        step: "buildingCandidate",
      })
      .mockResolvedValue({ kind: "nativeAuthoritative" });

    const { result } = renderHook(() => useDatabaseConversion());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(result.current.state).toEqual({
      kind: "converting",
      step: "preflight",
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(800);
    });
    expect(result.current.state).toEqual({
      kind: "converting",
      step: "buildingCandidate",
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(800);
    });
    expect(result.current.state).toEqual({ kind: "nativeAuthoritative" });
    expect(result.current.justCompleted).toBe(true);

    const callsAfterCompletion = (commands.getDatabaseConversionState as Mock)
      .mock.calls.length;
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000);
    });
    // No further polling once no longer converting.
    expect(
      (commands.getDatabaseConversionState as Mock).mock.calls.length,
    ).toBe(callsAfterCompletion);
  });

  it("acknowledgeCompletion clears justCompleted", async () => {
    (commands.getDatabaseConversionState as Mock)
      .mockResolvedValueOnce({ kind: "converting", step: "preflight" })
      .mockResolvedValue({ kind: "nativeAuthoritative" });

    const { result } = renderHook(() => useDatabaseConversion());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(result.current.state.kind).toBe("converting");

    await act(async () => {
      await vi.advanceTimersByTimeAsync(800);
    });
    expect(result.current.justCompleted).toBe(true);

    act(() => {
      result.current.acknowledgeCompletion();
    });
    expect(result.current.justCompleted).toBe(false);
  });

  it("start() surfaces a command error without throwing", async () => {
    (commands.getDatabaseConversionState as Mock).mockResolvedValue({
      kind: "sqliteAuthoritative",
    });
    (commands.startDatabaseConversion as Mock).mockResolvedValue({
      status: "error",
      error: "boom",
    });

    const { result } = renderHook(() => useDatabaseConversion());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });

    let started: boolean | undefined;
    await act(async () => {
      started = await result.current.start();
    });

    expect(started).toBe(false);
    expect(result.current.error).toBe("boom");
  });

  it("start() arms polling from a non-converting initial state, not just at mount", async () => {
    (commands.getDatabaseConversionState as Mock)
      .mockResolvedValueOnce({ kind: "sqliteAuthoritative" })
      .mockResolvedValueOnce({ kind: "converting", step: "preflight" })
      .mockResolvedValueOnce({
        kind: "converting",
        step: "buildingCandidate",
      })
      .mockResolvedValue({ kind: "nativeAuthoritative" });
    (commands.startDatabaseConversion as Mock).mockResolvedValue({
      status: "ok",
      data: null,
    });

    const { result } = renderHook(() => useDatabaseConversion());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(result.current.state).toEqual({ kind: "sqliteAuthoritative" });

    // start()'s own refresh() moves state to "converting" - this must, by
    // itself, arm the polling interval rather than leaving the UI stuck on
    // the first observed step.
    await act(async () => {
      await result.current.start();
    });
    expect(result.current.state).toEqual({
      kind: "converting",
      step: "preflight",
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(800);
    });
    expect(result.current.state).toEqual({
      kind: "converting",
      step: "buildingCandidate",
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(800);
    });
    expect(result.current.state).toEqual({ kind: "nativeAuthoritative" });
    expect(result.current.justCompleted).toBe(true);
  });

  it("start() counts an immediate nativeAuthoritative result as its own completion, even without an observed converting poll", async () => {
    (commands.getDatabaseConversionState as Mock)
      .mockResolvedValueOnce({ kind: "sqliteAuthoritative" })
      // The very first refresh() inside start() already reports
      // nativeAuthoritative - a fast completion this mount's own polling
      // never caught mid-flight as "converting".
      .mockResolvedValue({ kind: "nativeAuthoritative" });
    (commands.startDatabaseConversion as Mock).mockResolvedValue({
      status: "ok",
      data: null,
    });

    const { result } = renderHook(() => useDatabaseConversion());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(result.current.state).toEqual({ kind: "sqliteAuthoritative" });
    expect(result.current.justCompleted).toBe(false);

    await act(async () => {
      await result.current.start();
    });

    expect(result.current.state).toEqual({ kind: "nativeAuthoritative" });
    expect(result.current.justCompleted).toBe(true);
  });

  it("start() ignores a stale pre-start read on its first refresh and keeps polling until a later state arrives", async () => {
    (commands.getDatabaseConversionState as Mock)
      .mockResolvedValueOnce({ kind: "sqliteAuthoritative" }) // initial mount
      // The backend's own point-in-time disk read raced the driver's
      // first progress write and still reports the pre-start state - the
      // #2246 audit race this hook must not surface.
      .mockResolvedValueOnce({ kind: "sqliteAuthoritative" })
      .mockResolvedValueOnce({
        kind: "converting",
        step: "buildingCandidate",
      })
      .mockResolvedValue({ kind: "nativeAuthoritative" });
    (commands.startDatabaseConversion as Mock).mockResolvedValue({
      status: "ok",
      data: null,
    });

    const { result } = renderHook(() => useDatabaseConversion());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(result.current.state).toEqual({ kind: "sqliteAuthoritative" });

    await act(async () => {
      await result.current.start();
    });
    // The stale read must not overwrite the optimistic `converting` state
    // start() already set, and polling must stay armed so real progress is
    // still observed once it lands.
    expect(result.current.state.kind).toBe("converting");

    await act(async () => {
      await vi.advanceTimersByTimeAsync(800);
    });
    expect(result.current.state).toEqual({
      kind: "converting",
      step: "buildingCandidate",
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(800);
    });
    expect(result.current.state).toEqual({ kind: "nativeAuthoritative" });
    expect(result.current.justCompleted).toBe(true);
  });

  it("a deferred mount read released after start()'s own refresh must not regress the state (React Strict Mode double-invoke)", async () => {
    // Simulates React Strict Mode double-invoking the mount effect: the
    // mount's own `getDatabaseConversionState` call is still in flight
    // (deliberately never auto-resolved) when `start()` runs and its own
    // refresh already observes real progress. The mount read is released
    // only afterward, reporting the stale pre-start state.
    let resolveMountRead: (value: { kind: string }) => void = () => {};
    const mountReadPromise = new Promise<{ kind: string }>((resolve) => {
      resolveMountRead = resolve;
    });

    (commands.getDatabaseConversionState as Mock)
      .mockImplementationOnce(() => mountReadPromise) // mount read - held open
      .mockResolvedValueOnce({ kind: "converting", step: "preflight" }) // start()'s own refresh
      .mockResolvedValue({ kind: "nativeAuthoritative" });
    (commands.startDatabaseConversion as Mock).mockResolvedValue({
      status: "ok",
      data: null,
    });

    const { result } = renderHook(() => useDatabaseConversion());
    // The mount effect issued its read; it never resolves in this act(),
    // so the hook is left at its initial state.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });

    await act(async () => {
      await result.current.start();
    });
    expect(result.current.state).toEqual({
      kind: "converting",
      step: "preflight",
    });

    // The stale mount read finally resolves, reporting the pre-start
    // state. It must be discarded outright - it predates this start()'s
    // own generation - rather than overwrite the real progress already
    // observed above.
    await act(async () => {
      resolveMountRead({ kind: "sqliteAuthoritative" });
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(result.current.state).toEqual({
      kind: "converting",
      step: "preflight",
    });

    // Polling must still be intact after the stale read resolved.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(800);
    });
    expect(result.current.state).toEqual({ kind: "nativeAuthoritative" });
    expect(result.current.justCompleted).toBe(true);
  });

  it("cancel() refreshes state after a successful call", async () => {
    (commands.getDatabaseConversionState as Mock)
      .mockResolvedValueOnce({ kind: "converting", step: "reconciling" })
      .mockResolvedValue({ kind: "sqliteAuthoritative" });
    (commands.cancelDatabaseConversion as Mock).mockResolvedValue({
      status: "ok",
      data: null,
    });

    const { result } = renderHook(() => useDatabaseConversion());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(result.current.state.kind).toBe("converting");

    await act(async () => {
      await result.current.cancel();
    });

    expect(result.current.state).toEqual({ kind: "sqliteAuthoritative" });
  });

  // #2267: `justCompleted` used to live entirely in a per-mount `useRef`, so
  // only the exact hook instance that watched `converting` turn into
  // `nativeAuthoritative` ever set it. A conversion started from Settings
  // that finished after the user navigated away (Settings unmounted) was
  // first observed by the *next* Settings mount as already
  // `nativeAuthoritative`, with no local memory of `converting` - so the
  // one-time retention notice never appeared. These three cases match the
  // shared, cross-mount fix: an observer that stays mounted through the
  // transition (modeling the always-mounted app-root prompt dialog) records
  // it for a later, unrelated mount (modeling Settings) to read.
  describe("#2267 cross-mount completion detection", () => {
    it("records a transition observed by one mount and surfaces it to a mount created afterward", async () => {
      (commands.getDatabaseConversionState as Mock)
        .mockResolvedValueOnce({ kind: "converting", step: "preflight" })
        .mockResolvedValue({ kind: "nativeAuthoritative" });

      // Models the always-mounted app-root prompt dialog: it is the only
      // instance alive while the conversion actually finishes.
      const observer = renderHook(() => useDatabaseConversion());
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0);
      });
      expect(observer.result.current.state.kind).toBe("converting");

      await act(async () => {
        await vi.advanceTimersByTimeAsync(800);
      });
      expect(observer.result.current.state).toEqual({
        kind: "nativeAuthoritative",
      });
      expect(observer.result.current.justCompleted).toBe(true);

      // Models the Settings screen mounting later - e.g. the user started
      // the conversion from Settings, navigated away before it finished, and
      // comes back after. Its own first read is `nativeAuthoritative`
      // directly, with no `converting` poll of its own.
      const settingsMount = renderHook(() => useDatabaseConversion());
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0);
      });

      expect(settingsMount.result.current.state).toEqual({
        kind: "nativeAuthoritative",
      });
      expect(settingsMount.result.current.justCompleted).toBe(true);
    });

    it("never treats a fresh install's already-native first read as a completion", async () => {
      // A fresh install starts native-authoritative directly - there is no
      // prior `converting` state for any mount, this session or a previous
      // one, to have observed (#2203).
      (commands.getDatabaseConversionState as Mock).mockResolvedValue({
        kind: "nativeAuthoritative",
      });

      const first = renderHook(() => useDatabaseConversion());
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0);
      });
      expect(first.result.current.state).toEqual({
        kind: "nativeAuthoritative",
      });
      expect(first.result.current.justCompleted).toBe(false);

      // A second mount (e.g. opening Settings) must not retroactively treat
      // the already-native state as a completion either.
      const second = renderHook(() => useDatabaseConversion());
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0);
      });
      expect(second.result.current.justCompleted).toBe(false);
    });

    it("still reports completion for the startup-prompt path: a single mount watching the whole transition", async () => {
      (commands.getDatabaseConversionState as Mock)
        .mockResolvedValueOnce({ kind: "sqliteAuthoritative" })
        .mockResolvedValueOnce({ kind: "converting", step: "preflight" })
        .mockResolvedValue({ kind: "nativeAuthoritative" });
      (commands.startDatabaseConversion as Mock).mockResolvedValue({
        status: "ok",
        data: null,
      });

      // The app-root prompt dialog: one instance, mounted before the user
      // starts the conversion and still mounted when it completes.
      const { result } = renderHook(() => useDatabaseConversion());
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0);
      });
      expect(result.current.state).toEqual({ kind: "sqliteAuthoritative" });

      await act(async () => {
        await result.current.start();
      });
      expect(result.current.state.kind).toBe("converting");

      await act(async () => {
        await vi.advanceTimersByTimeAsync(800);
      });
      expect(result.current.state).toEqual({ kind: "nativeAuthoritative" });
      expect(result.current.justCompleted).toBe(true);
    });
  });
});
