import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { DatabaseConversionState } from "@/rspc/bindings";

interface FakeStore {
  data: Record<string, unknown>;
  has: (key: string) => Promise<boolean>;
  get: <T = unknown>(key: string) => Promise<T | undefined>;
  set: <T>(key: string, value: T) => Promise<void>;
  save: () => Promise<void>;
}

let fakeStore: FakeStore;

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, unknown>) => {
      if (options && "days" in options) {
        return `${key}:${options["days"]}`;
      }
      return key;
    },
  }),
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: { hardwareArchive: { retentionDays: 30 } },
    setHardwareArchiveRetentionDays: vi.fn(async () => true),
  }),
}));

/**
 * Regression coverage for a CodeRabbit finding on #2220: the Settings
 * entry point and the app-root prompt dialog can both be mounted at once
 * (each drives its own `useDatabaseConversion` instance), and each used
 * to read/write its own independent copy of the persisted
 * "notice already shown" flag - so the notice could render in both, and
 * dismissing it in one left it showing in the other.
 * `useDatabaseConversionNoticeShown` shares one Jotai-backed value across
 * every mount instead; this exercises that through two real mounted
 * `DatabaseConversionStateBody` instances (not mocked), only faking the
 * underlying Tauri Store plugin.
 */
describe("DatabaseConversionStateBody notice sharing across mounts", () => {
  beforeEach(() => {
    vi.resetModules();
    fakeStore = {
      data: {},
      has: vi.fn((key: string) => Promise.resolve(key in fakeStore.data)),
      get: vi
        .fn()
        .mockImplementation((key: string) =>
          Promise.resolve(fakeStore.data[key]),
        ) as <T = unknown>(key: string) => Promise<T | undefined>,
      set: vi.fn((key: string, value: unknown) => {
        fakeStore.data[key] = value;
        return Promise.resolve();
      }) as <T>(key: string, value: T) => Promise<void>,
      save: vi.fn(() => Promise.resolve()),
    };
    vi.doMock("@tauri-apps/plugin-store", () => ({
      load: vi.fn(() => Promise.resolve(fakeStore)),
    }));
  });

  afterEach(() => {
    cleanup();
    vi.resetModules();
    vi.clearAllMocks();
  });

  it("dismissing the notice in one mounted body hides it in the other", async () => {
    const { DatabaseConversionStateBody } = await import(
      "./DatabaseConversionStateBody"
    );
    const state: DatabaseConversionState = { kind: "nativeAuthoritative" };
    const commonProps = {
      state,
      error: null,
      start: vi.fn(async () => true),
      recover: vi.fn(async () => true),
      cancel: vi.fn(async () => true),
      justCompleted: true,
      acknowledgeCompletion: vi.fn(),
    };

    render(
      <>
        <div data-testid="first">
          <DatabaseConversionStateBody {...commonProps} />
        </div>
        <div data-testid="second">
          <DatabaseConversionStateBody {...commonProps} />
        </div>
      </>,
    );

    const keepButtons = () =>
      screen.queryAllByText(
        "pages.settings.insights.databaseConversion.notice.keep",
      );

    await waitFor(() => expect(keepButtons().length).toBeGreaterThan(0));

    fireEvent.click(keepButtons()[0]);

    await waitFor(() => expect(keepButtons().length).toBe(0));
  });
});
