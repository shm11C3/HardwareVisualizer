// src/hooks/useColorTheme.test.ts
import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useColorTheme } from "@/hooks/useColorTheme";
import type { Theme } from "@/rspc/bindings";

// Create mock functions that can be reconfigured per test
const mockTheme = vi.fn();
const mockOnThemeChanged = vi.fn();

// Mock Tauri API
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    theme: mockTheme,
    onThemeChanged: mockOnThemeChanged,
  }),
}));

describe("useColorTheme", () => {
  // Reset document class list before each test execution
  beforeEach(() => {
    document.documentElement.classList.remove("dark", "light");
    document.documentElement.dataset.theme = "";
    vi.clearAllMocks();

    // Set default mock behavior
    mockTheme.mockResolvedValue("light");
    mockOnThemeChanged.mockResolvedValue(() => {});
  });

  it("Initialized with 'light' by default", () => {
    renderHook(() => useColorTheme("light"));
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("Correctly initialized when 'dark' is passed as initial value", () => {
    renderHook(() => useColorTheme("dark"));
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });

  it("State changes to specified theme", () => {
    const { rerender } = renderHook(({ theme }) => useColorTheme(theme), {
      initialProps: { theme: "light" as Theme },
    });
    expect(document.documentElement.classList.contains("dark")).toBe(false);

    rerender({ theme: "dark" as Theme });
    expect(document.documentElement.classList.contains("dark")).toBe(true);

    rerender({ theme: "light" as Theme });
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("Themes included in darkClasses add 'dark' class", () => {
    const darkThemes: Theme[] = ["dark", "nebula", "espresso"];

    darkThemes.forEach((theme) => {
      renderHook(() => useColorTheme(theme));
      expect(document.documentElement.classList.contains("dark")).toBe(true);
      document.documentElement.classList.remove("dark"); // Reset for next loop
    });
  });

  it("Themes not included in darkClasses do not add 'dark' class", () => {
    const nonDarkThemes: Theme[] = [
      "light",
      "ocean",
      "grove",
      "sunset",
      "orbit",
      "cappuccino",
    ];

    nonDarkThemes.forEach((theme) => {
      renderHook(() => useColorTheme(theme));
      expect(document.documentElement.classList.contains("dark")).toBe(false);
    });
  });

  it("'light' class is added when 'system' theme and system theme is 'light'", async () => {
    renderHook(() => useColorTheme("system"));

    await waitFor(() => {
      expect(document.documentElement.classList.contains("light")).toBe(true);
    });
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("'dark' class is added when 'system' theme and system theme is 'dark'", async () => {
    // Set mock to return dark theme for this test
    mockTheme.mockResolvedValue("dark");

    renderHook(() => useColorTheme("system"));

    await waitFor(() => {
      expect(document.documentElement.classList.contains("dark")).toBe(true);
    });
    expect(document.documentElement.classList.contains("light")).toBe(false);
  });
});
