import { message } from "@tauri-apps/plugin-dialog";
import { renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useErrorModalListener } from "@/hooks/useTauriEventListener";

// --- Mock setup ---
// Variable to hold the event listener callback within tests
let registeredCallback:
  | ((event: { payload: { title: string; message: string } }) => void)
  | undefined;

// Mock function for unsubscribing from event listener
const offMock = vi.fn();

// Mock Tauri's listen
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(
    (
      _eventName: string,
      handler: (event: { payload: { title: string; message: string } }) => void,
    ) => {
      // Store the registered callback function
      registeredCallback = handler;
      // Return a Promise that resolves to the off function
      return Promise.resolve(offMock);
    },
  ),
}));

// Mock Tauri's message
vi.mock("@tauri-apps/plugin-dialog", () => ({
  message: vi.fn(),
}));

describe("useErrorModalListener", () => {
  beforeEach(() => {
    // Reset mock state before each test
    vi.clearAllMocks();
    registeredCallback = undefined;
  });

  it("Dialog is displayed correctly when error event is received", async () => {
    // Execute the hook
    renderHook(() => useErrorModalListener());

    // Check if registered callback exists
    if (!registeredCallback) {
      throw new Error("Event listener was not registered");
    }

    // Simulate error event
    const errorPayload = { title: "Error Title", message: "An error occurred" };
    registeredCallback({ payload: errorPayload });

    // Wait for async processing in event handler to complete
    await Promise.resolve();

    // Verify that error content is correctly passed to the message function for dialog display
    expect(message).toHaveBeenCalledWith("An error occurred", {
      title: "Error Title",
      kind: "error",
    });
  });

  it("Event listener is unsubscribed when component unmounts", async () => {
    const { unmount } = renderHook(() => useErrorModalListener());

    // Execute hook cleanup (unmount)
    unmount();

    // Wait for async processing to complete (Promise resolution)
    await Promise.resolve();

    // Verify that the off function for unsubscribing from event listener was called
    expect(offMock).toHaveBeenCalled();
  });
});
