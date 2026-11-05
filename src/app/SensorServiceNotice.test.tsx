import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "@/lib/i18n";
import type { SensorGuidance } from "@/features/hardware/store/sensors";
import { SensorServiceNotice } from "./SensorServiceNotice";

const mocks = vi.hoisted(() => ({
  diagnostics: [] as SensorGuidance[],
  elevated: false as boolean | null,
  settings: { elevatedStartupMode: false },
  acknowledge: vi.fn(),
  defer: vi.fn(),
  status: vi.fn(),
  open: vi.fn(),
}));
vi.mock("jotai", async (importOriginal) => ({
  ...(await importOriginal<typeof import("jotai")>()),
  useAtomValue: () => mocks.diagnostics,
}));
vi.mock("@tauri-apps/plugin-os", () => ({ platform: () => "windows" }));
vi.mock("@/hooks/settings/useSettingsAtom", () => ({
  useSettingsAtom: () => ({ settings: mocks.settings }),
}));
vi.mock("@/hooks/settings/useElevationAvailability", () => ({
  useProcessElevated: () => mocks.elevated,
}));
vi.mock("@/hooks/navigation/useSensorServiceSettings", () => ({
  useSensorServiceSettings: () => mocks.open,
}));
vi.mock("@/rspc/bindings", () => ({
  commands: {
    acknowledgeExternalComponentGuidanceKey: mocks.acknowledge,
    deferExternalComponentGuidanceForSession: mocks.defer,
    getSensorServiceStatus: mocks.status,
  },
}));

const cpu: SensorGuidance = {
  key: "pawnio:cpu-package-temperature:v1",
  component: "pawnio",
  usage: "cpuPackageTemperature",
};
const board: SensorGuidance = {
  key: "pawnio:motherboard-sensors:v1",
  component: "pawnio",
  usage: "motherboardSensors",
};
const context = {
  settled: true,
  cpuTemperatureVisible: true,
  motherboardSensorsVisible: true,
  expanded: false,
};
const props = {
  settingsLoaded: true,
  displayTarget: "performance" as const,
  performanceContext: context,
};

describe("SensorServiceNotice", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.diagnostics = [];
    mocks.elevated = false;
    mocks.settings.elevatedStartupMode = false;
    mocks.acknowledge.mockResolvedValue({ status: "ok", data: null });
    mocks.defer.mockResolvedValue({ status: "ok", data: null });
  });
  afterEach(cleanup);

  it("does not infer missing installation from an absent temperature", () => {
    render(<SensorServiceNotice {...props} />);
    expect(screen.queryByRole("complementary")).toBeNull();
    expect(mocks.status).not.toHaveBeenCalled();
  });

  it("renders real failed-attempt diagnostics without a modal or a status query", async () => {
    mocks.diagnostics = [cpu, board];
    render(<SensorServiceNotice {...props} />);
    expect(
      screen.getByText(
        /CPU package temperature.*Motherboard temperature sensors/,
      ),
    ).toBeInTheDocument();
    expect(screen.queryByRole("alertdialog")).toBeNull();
    await userEvent.click(
      screen.getByRole("button", { name: "Open sensor settings" }),
    );
    expect(mocks.open).toHaveBeenCalledOnce();
    expect(mocks.status).not.toHaveBeenCalled();
  });

  it("follows late failures and recovery directly from the monitor data", () => {
    const { rerender } = render(<SensorServiceNotice {...props} />);
    mocks.diagnostics = [cpu];
    rerender(<SensorServiceNotice {...props} />);
    expect(screen.getByRole("complementary")).toBeInTheDocument();
    mocks.diagnostics = [];
    rerender(<SensorServiceNotice {...props} />);
    expect(screen.queryByRole("complementary")).toBeNull();
    expect(mocks.status).not.toHaveBeenCalled();
  });

  it.each([false, true])(
    "hides the notice for the launch and preserves Core keys (permanent=%s)",
    async (permanent) => {
      mocks.diagnostics = [cpu, board];
      const { rerender } = render(<SensorServiceNotice {...props} />);
      await userEvent.click(
        screen.getByRole("button", {
          name: permanent
            ? "Never show again"
            : "Hide sensor guidance for this session",
        }),
      );
      const save = permanent ? mocks.acknowledge : mocks.defer;
      expect(save.mock.calls.map(([key]) => key)).toEqual([cpu.key, board.key]);
      mocks.diagnostics = [board];
      rerender(<SensorServiceNotice {...props} />);
      expect(screen.queryByRole("complementary")).toBeNull();
    },
  );

  it("does not offer a migration CTA solely because elevated startup is enabled", () => {
    mocks.elevated = true;
    mocks.settings.elevatedStartupMode = true;
    render(<SensorServiceNotice {...props} />);
    expect(screen.queryByRole("complementary")).toBeNull();
    expect(mocks.status).not.toHaveBeenCalled();
    expect(mocks.defer).not.toHaveBeenCalled();
  });

  it("does not duplicate the service section on Settings", () => {
    mocks.diagnostics = [cpu];
    mocks.elevated = true;
    mocks.settings.elevatedStartupMode = true;
    render(<SensorServiceNotice {...props} displayTarget="settings" />);
    expect(screen.queryByRole("complementary")).toBeNull();
    expect(mocks.status).not.toHaveBeenCalled();
  });

  it("uses the current visible readings without waiting for a startup timer", () => {
    mocks.diagnostics = [board];
    const { rerender } = render(
      <SensorServiceNotice
        {...props}
        performanceContext={{ ...context, motherboardSensorsVisible: false }}
      />,
    );
    expect(screen.queryByRole("complementary")).toBeNull();
    rerender(<SensorServiceNotice {...props} />);
    expect(screen.getByRole("complementary")).toBeInTheDocument();
    rerender(
      <SensorServiceNotice
        {...props}
        performanceContext={{ ...context, expanded: true }}
      />,
    );
    expect(screen.queryByRole("complementary")).toBeNull();
  });

  it("keeps guidance visible when saving dismissal fails", async () => {
    mocks.diagnostics = [cpu];
    mocks.acknowledge.mockResolvedValue({
      status: "error",
      error: "write failed",
    });
    const error = vi.spyOn(console, "error").mockImplementation(() => {});
    render(<SensorServiceNotice {...props} />);
    await userEvent.click(
      screen.getByRole("button", { name: "Never show again" }),
    );
    expect(
      await screen.findByText("Failed to save this guidance preference."),
    ).toBeInTheDocument();
    expect(screen.getByRole("complementary")).toBeInTheDocument();
    error.mockRestore();
  });
});
