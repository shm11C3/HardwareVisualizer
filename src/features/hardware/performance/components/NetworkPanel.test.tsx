import { act, cleanup, render, screen } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import { afterEach, describe, expect, it, vi } from "vitest";
import en from "@/lang/en.json";
import type { NetworkInfo } from "@/rspc/bindings";

const mocks = vi.hoisted(() => ({ getNetworkInfo: vi.fn() }));

vi.mock("@/rspc/bindings", () => ({
  commands: { getNetworkInfo: mocks.getNetworkInfo },
}));

// Resolve real English copy so assertions read what the user would see.
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    i18n: { language: "en" },
    t: (key: string, options?: Record<string, string>) => {
      const template = key
        .split(".")
        .reduce<unknown>(
          (node, part) => (node as Record<string, unknown> | undefined)?.[part],
          en,
        );
      return typeof template === "string"
        ? template.replace(
            /{{(\w+)}}/g,
            (_, name: string) => options?.[name] ?? "",
          )
        : key;
    },
  }),
}));

import { NetworkPanel } from "./NetworkPanel";

const adapter = (overrides: Partial<NetworkInfo> = {}): NetworkInfo => ({
  description: "Ethernet",
  macAddress: "aa:bb:cc:00:00:01",
  ipv4: ["192.168.0.10"],
  ipv6: [],
  linkLocalIpv6: [],
  ipSubnet: [],
  defaultIpv4Gateway: [],
  defaultIpv6Gateway: [],
  receiveLinkSpeedBps: 1_000_000_000,
  transmitLinkSpeedBps: 1_000_000_000,
  ...overrides,
});

const ok = (data: NetworkInfo[]) => ({ status: "ok", data });

const renderPanel = () =>
  render(
    <Provider store={createStore()}>
      <NetworkPanel />
    </Provider>,
  );

describe("NetworkPanel", () => {
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it("shows one value when receive and transmit rates are equal", async () => {
    mocks.getNetworkInfo.mockResolvedValue(
      ok([
        adapter({
          receiveLinkSpeedBps: 2_500_000_000,
          transmitLinkSpeedBps: 2_500_000_000,
        }),
      ]),
    );
    renderPanel();

    expect(screen.getByTestId("network-panel-loading")).toBeVisible();
    expect(await screen.findByText("2.5 Gb/s")).toBeVisible();
    expect(screen.getByText("Link speed")).toBeVisible();
    expect(screen.getByText("192.168.0.10")).toBeVisible();
    expect(screen.queryByText(/^Receive/)).toBeNull();
  });

  it("labels receive and transmit in text when they differ", async () => {
    mocks.getNetworkInfo.mockResolvedValue(
      ok([
        adapter({
          description: "Wi-Fi",
          receiveLinkSpeedBps: 866_700_000,
          transmitLinkSpeedBps: 433_300_000,
        }),
      ]),
    );
    renderPanel();

    expect(await screen.findByText("Receive 867 Mb/s")).toBeVisible();
    expect(screen.getByText("Transmit 433 Mb/s")).toBeVisible();
  });

  it("keeps the row and says Not reported instead of showing zero", async () => {
    mocks.getNetworkInfo.mockResolvedValue(
      ok([
        adapter({
          description: "Virtual",
          receiveLinkSpeedBps: null,
          transmitLinkSpeedBps: null,
        }),
      ]),
    );
    renderPanel();

    expect(await screen.findByText("Not reported")).toBeVisible();
    expect(screen.getByText("Virtual")).toBeVisible();
    expect(screen.queryByText(/0 Mb\/s/)).toBeNull();
  });

  it("shows a single value when only one direction is reported", async () => {
    mocks.getNetworkInfo.mockResolvedValue(
      ok([
        adapter({
          receiveLinkSpeedBps: 100_000_000,
          transmitLinkSpeedBps: null,
        }),
      ]),
    );
    renderPanel();

    expect(await screen.findByText("100 Mb/s")).toBeVisible();
    expect(screen.queryByText("Not reported")).toBeNull();
  });

  it("keeps each adapter's own rate (attribution is not mixed)", async () => {
    mocks.getNetworkInfo.mockResolvedValue(
      ok([
        adapter({
          description: "Ethernet",
          macAddress: "aa:00",
          receiveLinkSpeedBps: 1_000_000_000,
          transmitLinkSpeedBps: 1_000_000_000,
        }),
        adapter({
          description: "Wi-Fi",
          macAddress: "bb:00",
          ipv4: [],
          ipv6: ["fd00::2"],
          receiveLinkSpeedBps: 54_000_000,
          transmitLinkSpeedBps: 54_000_000,
        }),
      ]),
    );
    renderPanel();

    const rows = await screen.findAllByRole("listitem");
    expect(rows).toHaveLength(2);
    expect(rows[0]).toHaveTextContent("Ethernet");
    expect(rows[0]).toHaveTextContent("1 Gb/s");
    expect(rows[0]).not.toHaveTextContent("54 Mb/s");
    expect(rows[1]).toHaveTextContent("Wi-Fi");
    expect(rows[1]).toHaveTextContent("fd00::2");
    expect(rows[1]).toHaveTextContent("54 Mb/s");
    expect(rows[1]).not.toHaveTextContent("1 Gb/s");
  });

  it("shows a failed read as a failure with retry, not an empty panel", async () => {
    mocks.getNetworkInfo.mockResolvedValue({ status: "error", error: "boom" });
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    renderPanel();

    expect(await screen.findByTestId("load-failure")).toHaveTextContent(
      en.pages.dashboard.systemSpecifications.networkLoadFailed,
    );
    expect(
      screen.queryByText(
        en.pages.dashboard.systemSpecifications.networkUnavailable,
      ),
    ).toBeNull();
    consoleError.mockRestore();
  });

  it("shows the unavailable copy when the read succeeds with no adapters", async () => {
    mocks.getNetworkInfo.mockResolvedValue(ok([]));
    renderPanel();

    expect(
      await screen.findByText(
        en.pages.dashboard.systemSpecifications.networkUnavailable,
      ),
    ).toBeVisible();
    expect(screen.queryByTestId("load-failure")).toBeNull();
  });

  it("re-reads every 10 seconds while mounted and stops after unmount", async () => {
    vi.useFakeTimers();
    mocks.getNetworkInfo.mockResolvedValue(ok([adapter()]));
    const { unmount } = renderPanel();
    await act(async () => {});
    expect(mocks.getNetworkInfo).toHaveBeenCalledTimes(1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(10_000);
    });
    expect(mocks.getNetworkInfo).toHaveBeenCalledTimes(2);

    unmount();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_000);
    });
    expect(mocks.getNetworkInfo).toHaveBeenCalledTimes(2);
  });

  it("does not start a new read while the previous one is still running", async () => {
    vi.useFakeTimers();
    let resolve: (value: unknown) => void = () => {};
    mocks.getNetworkInfo.mockImplementation(
      () => new Promise((r) => (resolve = r)),
    );
    renderPanel();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(35_000);
    });
    expect(mocks.getNetworkInfo).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolve(ok([adapter()]));
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(10_000);
    });
    expect(mocks.getNetworkInfo).toHaveBeenCalledTimes(2);
  });
});
