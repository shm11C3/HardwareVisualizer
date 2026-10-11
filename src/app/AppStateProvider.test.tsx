import { act, cleanup, render, screen } from "@testing-library/react";
import { atom, useAtomValue, useSetAtom } from "jotai";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AppStateProvider, useAppStateReset } from "./AppStateProvider";

const counterAtom = atom(0);

const mounted = vi.fn();
let reset: () => void = () => {};

const Child = () => {
  const value = useAtomValue(counterAtom);
  const setValue = useSetAtom(counterAtom);
  reset = useAppStateReset();
  return (
    <button type="button" onClick={() => setValue(42)}>
      {`counter:${value}`}
    </button>
  );
};

const MountProbe = () => {
  mounted();
  return null;
};

describe("AppStateProvider", () => {
  afterEach(cleanup);

  it("returns atoms to their initial value and remounts the subtree on reset", () => {
    mounted.mockClear();
    render(
      <AppStateProvider>
        <Child />
        <MountProbe />
      </AppStateProvider>,
    );
    expect(screen.getByRole("button")).toHaveTextContent("counter:0");

    act(() => screen.getByRole("button").click());
    expect(screen.getByRole("button")).toHaveTextContent("counter:42");
    const mountsBeforeReset = mounted.mock.calls.length;

    act(() => reset());

    expect(screen.getByRole("button")).toHaveTextContent("counter:0");
    expect(mounted.mock.calls.length).toBeGreaterThan(mountsBeforeReset);
  });

  it("throws when useAppStateReset is used outside the provider", () => {
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    expect(() => render(<Child />)).toThrow(
      "useAppStateReset must be used within AppStateProvider",
    );
    spy.mockRestore();
  });
});
