import { act, renderHook } from "@testing-library/react";
import { createStore, Provider, useAtomValue } from "jotai";
import type { ReactNode } from "react";
import { createElement } from "react";
import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";

// The two write shapes (`persistOptimistic`, `persistThenApply`) live inside
// the hook, so they are proved through the mutators that use them. This file
// is separate from useSettingsAtom.test.ts, which pins each mutator's own
// behavior and must keep passing unchanged.

vi.mock("@/rspc/bindings", () => ({
  commands: {
    setTheme: vi.fn(),
    setDisplayTargets: vi.fn(),
    setTrayWidgetSettings: vi.fn(),
    setCloseToTrayPreference: vi.fn(),
    setLineGraphColor: vi.fn(),
  },
}));

import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";
import { commands } from "@/rspc/bindings";
import { settingsWriteFailuresAtom } from "@/store/settings";

const setup = () => {
  const store = createStore();
  // Counts every change of the failure state, so "exactly one outcome per
  // write" is an observable fact and not only a final-state check.
  let changes = 0;
  store.sub(settingsWriteFailuresAtom, () => {
    changes += 1;
  });
  const wrapper = ({ children }: { children: ReactNode }) =>
    createElement(Provider, { store }, children);
  const hook = renderHook(
    () => ({
      ...useSettingsAtom(),
      failures: useAtomValue(settingsWriteFailuresAtom),
    }),
    { wrapper },
  );
  return { hook, changes: () => changes };
};

const ok = { status: "ok", data: null };
const fail = (error: string) => ({ status: "error", error });

describe("persistOptimistic (updateSettingAtom)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  it("applies first, and a successful write clears the failure and resolves true", async () => {
    (commands.setTheme as Mock)
      .mockResolvedValueOnce(fail("disk full"))
      .mockResolvedValueOnce(ok);
    const { hook } = setup();

    await act(async () => {
      await hook.result.current.updateSettingAtom("theme", "dark");
    });
    expect(hook.result.current.failures.has("theme")).toBe(true);

    let saved = false;
    await act(async () => {
      saved = await hook.result.current.updateSettingAtom("theme", "dark");
    });
    expect(saved).toBe(true);
    expect(hook.result.current.failures.size).toBe(0);
    expect(hook.result.current.settings.theme).toBe("dark");
  });

  it("an error result records the failure, rolls back and resolves false", async () => {
    (commands.setTheme as Mock).mockResolvedValue(fail("disk full"));
    const { hook } = setup();
    const before = hook.result.current.settings.theme;

    let saved = true;
    await act(async () => {
      saved = await hook.result.current.updateSettingAtom("theme", "dark");
    });

    expect(saved).toBe(false);
    expect(hook.result.current.failures).toEqual(
      new Map([["theme", "disk full"]]),
    );
    expect(hook.result.current.settings.theme).toBe(before);
  });

  it("a rejected call records the failure, rolls back and rethrows", async () => {
    const rejection = new Error("ipc closed");
    (commands.setTheme as Mock).mockRejectedValue(rejection);
    const { hook } = setup();
    const before = hook.result.current.settings.theme;

    let thrown: unknown;
    await act(async () => {
      thrown = await hook.result.current
        .updateSettingAtom("theme", "dark")
        .catch((err: unknown) => err);
    });

    expect(thrown).toBe(rejection);
    expect(hook.result.current.failures).toEqual(
      new Map([["theme", "ipc closed"]]),
    );
    expect(hook.result.current.settings.theme).toBe(before);
  });
});

describe("persistThenApply (toggleDisplayTarget)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  it("applies only after the write succeeded, and clears the failure", async () => {
    (commands.setDisplayTargets as Mock)
      .mockResolvedValueOnce(fail("disk full"))
      .mockResolvedValueOnce(ok);
    const { hook } = setup();

    await act(async () => {
      await hook.result.current.toggleDisplayTarget("cpu");
    });
    expect(hook.result.current.settings.displayTargets).toEqual([]);
    expect(hook.result.current.failures.has("displayTargets")).toBe(true);

    await act(async () => {
      await hook.result.current.toggleDisplayTarget("cpu");
    });
    expect(hook.result.current.settings.displayTargets).toEqual(["cpu"]);
    expect(hook.result.current.failures.size).toBe(0);
  });

  it("a rejected call records the failure, applies nothing and rethrows", async () => {
    const rejection = new Error("ipc closed");
    (commands.setDisplayTargets as Mock).mockRejectedValue(rejection);
    const { hook } = setup();

    let thrown: unknown;
    await act(async () => {
      thrown = await hook.result.current
        .toggleDisplayTarget("cpu")
        .catch((err: unknown) => err);
    });

    expect(thrown).toBe(rejection);
    expect(hook.result.current.settings.displayTargets).toEqual([]);
    expect(hook.result.current.failures).toEqual(
      new Map([["displayTargets", "ipc closed"]]),
    );
  });
});

describe("a mutator records exactly one outcome per write", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  it("closeToTray: two backend writes, one recorded failure", async () => {
    // The widget save succeeds, the preference save fails, and so does the
    // widget undo: three backend results, still a single recorded outcome.
    (commands.setTrayWidgetSettings as Mock)
      .mockResolvedValueOnce(ok)
      .mockResolvedValueOnce(fail("undo failed"));
    (commands.setCloseToTrayPreference as Mock).mockResolvedValue(
      fail("preference failed"),
    );
    const { hook, changes } = setup();

    await act(async () => {
      await hook.result.current.setCloseToTrayPreferenceAtom(true);
    });

    expect(hook.result.current.failures).toEqual(
      new Map([["closeToTray", "preference failed"]]),
    );
    expect(changes()).toBe(1);
  });

  it("closeToTray: a successful pair of writes is one clear, not two", async () => {
    (commands.setCloseToTrayPreference as Mock)
      .mockResolvedValueOnce(fail("preference failed"))
      .mockResolvedValueOnce(ok);
    (commands.setTrayWidgetSettings as Mock).mockResolvedValue(ok);
    const { hook, changes } = setup();

    await act(async () => {
      await hook.result.current.setCloseToTrayPreferenceAtom(true);
    });
    expect(changes()).toBe(1);

    await act(async () => {
      await hook.result.current.setCloseToTrayPreferenceAtom(true);
    });
    expect(hook.result.current.failures.size).toBe(0);
    expect(changes()).toBe(2);
  });

  it("lineGraphColor.reset: several failing colors record one outcome", async () => {
    (commands.setLineGraphColor as Mock).mockImplementation(
      async (key: string) =>
        key === "gpu"
          ? { status: "ok", data: "rgb(1, 2, 3)" }
          : fail(`${key} failed`),
    );
    const { hook, changes } = setup();

    await act(async () => {
      await hook.result.current.resetLineGraphColors();
    });

    expect(hook.result.current.failures.size).toBe(1);
    expect(hook.result.current.failures.has("lineGraphColor.reset")).toBe(true);
    expect(changes()).toBe(1);
  });
});
