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
import { useDatabaseConversion } from "./useDatabaseConversion";

vi.mock("@/rspc/bindings", () => ({
  commands: {
    getDatabaseConversionState: vi.fn(),
    startDatabaseConversion: vi.fn(),
    cancelDatabaseConversion: vi.fn(),
  },
}));

import { commands } from "@/rspc/bindings";

describe("useDatabaseConversion", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
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
});
