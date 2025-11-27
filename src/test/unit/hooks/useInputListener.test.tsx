import { cleanup, render } from "@testing-library/react";
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  type Mock,
  vi,
} from "vitest";
import { useKeydown } from "@/hooks/useInputListener";
import { commands } from "@/rspc/bindings";

// --- Mock definitions ---
// Mock the error function from useTauriDialog
const errorMock = vi.fn();

// Mock returning [state, setState] as the return value of useTauriStore
let storeValue = false;
const setDecoratedMock = vi.fn((newVal: boolean) => {
  storeValue = newVal;
  return Promise.resolve();
});

// Mock useTauriDialog
vi.mock("@/hooks/useTauriDialog", () => ({
  useTauriDialog: () => ({
    error: errorMock,
  }),
}));
// Mock useTauriStore
vi.mock("@/hooks/useTauriStore", () => ({
  useTauriStore: (_key: string, defaultValue: boolean) => {
    storeValue = defaultValue;
    return [storeValue, setDecoratedMock];
  },
}));

// Mock commands.setDecoration
vi.mock("@/rspc/bindings", () => ({
  commands: {
    setDecoration: vi.fn(() => Promise.resolve()),
  },
}));

// Test component
function TestComponent() {
  // Pass the required arguments to useKeydown
  const isDecorated = storeValue;
  const keydownHandler = useKeydown({
    isDecorated,
    setDecorated: setDecoratedMock,
  });
  return (
    <>
      {keydownHandler}
      <div>Test Component</div>
    </>
  );
}

describe("useKeydown", () => {
  beforeEach(() => {
    // Reset initial state
    storeValue = false;
    errorMock.mockReset();
    setDecoratedMock.mockReset();
    (commands.setDecoration as Mock).mockClear();
  });

  afterEach(() => {
    cleanup();
  });

  it("When F11 key is pressed, commands.setDecoration and setDecorated are called", async () => {
    render(<TestComponent />);
    // Dispatch F11 key event
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "F11" }));
    // Wait for async processing to complete (wait for microtasks)
    await new Promise((resolve) => setTimeout(resolve, 0));

    // Since initial state is false, verify that !false → true is passed
    expect(commands.setDecoration).toHaveBeenCalledWith(true);
    expect(setDecoratedMock).toHaveBeenCalledWith(true);
  });

  it("When a key other than F11 is pressed, nothing is executed", async () => {
    render(<TestComponent />);
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter" }));
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(commands.setDecoration).not.toHaveBeenCalled();
    expect(setDecoratedMock).not.toHaveBeenCalled();
  });

  it("When commands.setDecoration throws an error, error handler and console.error are called", async () => {
    // Configure commands.setDecoration to return an error
    const errorMessage = "Test error";
    (commands.setDecoration as Mock).mockImplementationOnce(() =>
      Promise.reject(new Error(errorMessage)),
    );
    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    render(<TestComponent />);
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "F11" }));
    await new Promise((resolve) => setTimeout(resolve, 0));

    // Verify that error mock and console.error were called
    expect(errorMock).toHaveBeenCalled();
    expect(consoleErrorSpy).toHaveBeenCalledWith(
      "Failed to toggle window decoration:",
      expect.any(Error),
    );

    consoleErrorSpy.mockRestore();
  });

  it("Event listener is removed when component unmounts", async () => {
    const { unmount } = render(<TestComponent />);
    unmount();

    // Verify that dispatching F11 event after unmount does nothing
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "F11" }));
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(commands.setDecoration).not.toHaveBeenCalled();
    expect(setDecoratedMock).not.toHaveBeenCalled();
  });
});
