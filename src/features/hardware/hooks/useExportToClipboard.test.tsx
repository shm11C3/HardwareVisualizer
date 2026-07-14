import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { act, renderHook } from "@testing-library/react";
import { Provider } from "jotai";
import { useHydrateAtoms } from "jotai/utils";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useExportToClipboard } from "@/features/hardware/dashboard/hooks/useExportToClipboard";
import { processorsUsageHistoryAtom } from "@/features/hardware/store/chart";
import type {
  DiskKind,
  GraphicInfo,
  MemoryInfo,
  NetworkInfo,
  SizeUnit,
  SysInfo,
} from "@/rspc/bindings";

vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({
  writeText: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

const mockUseHardwareInfoAtom = vi.fn();
vi.mock("@/features/hardware/hooks/useHardwareInfoAtom", () => ({
  useHardwareInfoAtom: () => mockUseHardwareInfoAtom(),
}));

const mockUseProcessInfo = vi.fn();
vi.mock("@/features/hardware/hooks/useProcessInfo", () => ({
  useProcessInfo: (options?: { enabled?: boolean }) =>
    mockUseProcessInfo(options),
}));

// ── Fixture data ──────────────────────────────────────────────────────────────

const defaultSysInfo: SysInfo = {
  cpu: null,
  memory: null,
  gpus: null,
  storage: [],
  motherboard: null,
};

const fullCpuInfo = {
  name: "Intel Core i7-12700K",
  vendor: "Intel",
  coreCount: 12,
  clock: 3600,
  clockUnit: "MHz",
  cpuName: "i7-12700K",
};

const fullMemoryDetailed: MemoryInfo = {
  size: "32 GB",
  clock: 3200,
  clockUnit: "MHz",
  memoryCount: 2,
  totalSlots: 4,
  memoryType: "DDR4",
  isDetailed: true,
};

const memoryBasic: MemoryInfo = { ...fullMemoryDetailed, isDetailed: false };

const gpuInfo: GraphicInfo = {
  id: "gpu-0",
  name: "NVIDIA RTX 3080",
  vendorName: "NVIDIA",
  clock: 1440,
  memorySize: "10 GB",
  memorySizeDedicated: "10 GB",
  coreCount: "8704",
};

const motherboardWithVersion = {
  manufacturer: "ASUS",
  product: "ROG STRIX Z690-E",
  version: "1.0",
  serialNumber: "SN12345",
  biosVendor: "American Megatrends",
  biosVersion: "3802",
  biosReleaseDate: "2022-01-01",
};

const motherboardNoVersion = { ...motherboardWithVersion, version: "" };

const networkWithLinkLocal: NetworkInfo = {
  description: "Ethernet",
  macAddress: "AA:BB:CC:DD:EE:FF",
  ipv4: ["192.168.1.100"],
  ipv6: ["2001:db8::1"],
  linkLocalIpv6: ["fe80::1"],
  ipSubnet: ["255.255.255.0"],
  defaultIpv4Gateway: ["192.168.1.1"],
  defaultIpv6Gateway: ["fe80::1"],
};

const networkNoLinkLocal: NetworkInfo = {
  ...networkWithLinkLocal,
  linkLocalIpv6: [],
};

const makeStorageItem = (
  name: string,
  storageType: DiskKind,
  fileSystem = "NTFS",
) => ({
  name,
  size: 512,
  sizeUnit: "GB" as SizeUnit,
  free: 200,
  freeUnit: "GB" as SizeUnit,
  storageType,
  fileSystem,
});

// ── Helpers ───────────────────────────────────────────────────────────────────

const makeWrapper =
  (processorHistory: number[][] = []) =>
  ({ children }: { children: ReactNode }) => {
    const HydrateAtoms = ({ children }: { children: ReactNode }) => {
      useHydrateAtoms([[processorsUsageHistoryAtom, processorHistory]]);
      return <>{children}</>;
    };
    return (
      <Provider>
        <HydrateAtoms>{children}</HydrateAtoms>
      </Provider>
    );
  };

const getWrittenContent = () =>
  (writeText as ReturnType<typeof vi.fn>).mock.calls[0][0] as string;

// ── Tests ─────────────────────────────────────────────────────────────────────

describe("useExportToClipboard", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseProcessInfo.mockReturnValue([]);
    mockUseHardwareInfoAtom.mockReturnValue({
      hardwareInfo: defaultSysInfo,
      networkInfo: [],
    });
  });

  it("calls writeText when exportToClipboard is invoked", async () => {
    const { result } = renderHook(() => useExportToClipboard());
    await act(async () => {
      await result.current.exportToClipboard();
    });
    expect(writeText).toHaveBeenCalledWith(expect.any(String));
  });

  // ── CPU branches ──────────────────────────────────────────────────────────

  it("uses fallback text when cpu is null", async () => {
    const { result } = renderHook(() => useExportToClipboard());
    await act(async () => {
      await result.current.exportToClipboard();
    });
    expect(getWrittenContent()).toContain("No CPU information available");
  });

  it("includes CPU details when cpu is present", async () => {
    mockUseHardwareInfoAtom.mockReturnValue({
      hardwareInfo: { ...defaultSysInfo, cpu: fullCpuInfo },
      networkInfo: [],
    });
    const { result } = renderHook(() => useExportToClipboard(), {
      wrapper: makeWrapper([[1, 2, 3, 4]]),
    });
    await act(async () => {
      await result.current.exportToClipboard();
    });
    const content = getWrittenContent();
    expect(content).toContain("Intel Core i7-12700K");
    expect(content).toContain("Intel");
    expect(content).not.toContain("No CPU information available");
  });

  it("uses processorsUsageHistory[0].length for thread count when available", async () => {
    mockUseHardwareInfoAtom.mockReturnValue({
      hardwareInfo: { ...defaultSysInfo, cpu: fullCpuInfo },
      networkInfo: [],
    });
    const { result } = renderHook(() => useExportToClipboard(), {
      wrapper: makeWrapper([[100, 200, 300]]),
    });
    await act(async () => {
      await result.current.exportToClipboard();
    });
    expect(getWrittenContent()).toContain("3");
  });

  it("omits runtime-derived CPU fields from the specifications report", async () => {
    mockUseHardwareInfoAtom.mockReturnValue({
      hardwareInfo: { ...defaultSysInfo, cpu: fullCpuInfo },
      networkInfo: [],
    });
    mockUseProcessInfo.mockReturnValue([
      { pid: 1, name: "process1", cpuUsage: 10, memoryUsage: 200 },
    ]);
    const { result } = renderHook(
      () => useExportToClipboard({ includeRuntimeStats: false }),
      {
        wrapper: makeWrapper([[100, 200, 300]]),
      },
    );

    await act(async () => {
      await result.current.exportToClipboard();
    });

    expect(mockUseProcessInfo).toHaveBeenCalledWith({ enabled: false });
    expect(getWrittenContent()).not.toContain("shared.threadCount");
    expect(getWrittenContent()).not.toContain("shared.processCount");
  });

  it("falls back to 0 threads when processorsUsageHistory is empty", async () => {
    mockUseHardwareInfoAtom.mockReturnValue({
      hardwareInfo: { ...defaultSysInfo, cpu: fullCpuInfo },
      networkInfo: [],
    });
    const { result } = renderHook(() => useExportToClipboard(), {
      wrapper: makeWrapper([]),
    });
    await act(async () => {
      await result.current.exportToClipboard();
    });
    expect(getWrittenContent()).toContain("Intel Core i7-12700K");
  });

  // ── Memory branches ───────────────────────────────────────────────────────

  it("uses fallback text when memory is null", async () => {
    const { result } = renderHook(() => useExportToClipboard());
    await act(async () => {
      await result.current.exportToClipboard();
    });
    expect(getWrittenContent()).toContain("No memory information available");
  });

  it("includes detailed memory fields when isDetailed is true", async () => {
    mockUseHardwareInfoAtom.mockReturnValue({
      hardwareInfo: { ...defaultSysInfo, memory: fullMemoryDetailed },
      networkInfo: [],
    });
    const { result } = renderHook(() => useExportToClipboard());
    await act(async () => {
      await result.current.exportToClipboard();
    });
    const content = getWrittenContent();
    expect(content).toContain("32 GB");
    expect(content).toContain("DDR4");
    expect(content).toContain("2/4");
    expect(content).not.toContain("No memory information available");
  });

  it("omits detailed memory fields when isDetailed is false", async () => {
    mockUseHardwareInfoAtom.mockReturnValue({
      hardwareInfo: { ...defaultSysInfo, memory: memoryBasic },
      networkInfo: [],
    });
    const { result } = renderHook(() => useExportToClipboard());
    await act(async () => {
      await result.current.exportToClipboard();
    });
    const content = getWrittenContent();
    expect(content).toContain("32 GB");
    expect(content).not.toContain("2/4");
    expect(content).not.toContain("No memory information available");
  });

  // ── GPU branches ──────────────────────────────────────────────────────────

  it("uses fallback text when gpus is null", async () => {
    const { result } = renderHook(() => useExportToClipboard());
    await act(async () => {
      await result.current.exportToClipboard();
    });
    expect(getWrittenContent()).toContain("No GPU information available");
  });

  it("includes GPU info when gpus array is present", async () => {
    mockUseHardwareInfoAtom.mockReturnValue({
      hardwareInfo: { ...defaultSysInfo, gpus: [gpuInfo] },
      networkInfo: [],
    });
    const { result } = renderHook(() => useExportToClipboard());
    await act(async () => {
      await result.current.exportToClipboard();
    });
    const content = getWrittenContent();
    expect(content).toContain("NVIDIA RTX 3080");
    expect(content).not.toContain("No GPU information available");
  });

  // ── Storage branches ──────────────────────────────────────────────────────

  it("includes all storage types (ssd, hdd, other)", async () => {
    mockUseHardwareInfoAtom.mockReturnValue({
      hardwareInfo: {
        ...defaultSysInfo,
        storage: [
          makeStorageItem("SSD Drive", "ssd"),
          makeStorageItem("HDD Drive", "hdd"),
          makeStorageItem("Other Drive", "other", "exFAT"),
        ],
      },
      networkInfo: [],
    });
    const { result } = renderHook(() => useExportToClipboard());
    await act(async () => {
      await result.current.exportToClipboard();
    });
    const content = getWrittenContent();
    expect(content).toContain("SSD");
    expect(content).toContain("HDD");
    expect(content).toContain("SSD Drive");
    expect(content).toContain("HDD Drive");
    expect(content).toContain("Other Drive");
  });

  // ── Motherboard branches ──────────────────────────────────────────────────

  it("includes version field when motherboard.version is non-empty", async () => {
    mockUseHardwareInfoAtom.mockReturnValue({
      hardwareInfo: { ...defaultSysInfo, motherboard: motherboardWithVersion },
      networkInfo: [],
    });
    const { result } = renderHook(() => useExportToClipboard());
    await act(async () => {
      await result.current.exportToClipboard();
    });
    const content = getWrittenContent();
    expect(content).toContain("ASUS");
    expect(content).toContain("ROG STRIX Z690-E");
    expect(content).toContain("1.0");
  });

  it("omits version field when motherboard.version is empty", async () => {
    mockUseHardwareInfoAtom.mockReturnValue({
      hardwareInfo: { ...defaultSysInfo, motherboard: motherboardNoVersion },
      networkInfo: [],
    });
    const { result } = renderHook(() => useExportToClipboard());
    await act(async () => {
      await result.current.exportToClipboard();
    });
    const content = getWrittenContent();
    expect(content).toContain("ASUS");
    expect(content).toContain("ROG STRIX Z690-E");
  });

  // ── Network branches ──────────────────────────────────────────────────────

  it("includes linkLocalIpv6 section when present", async () => {
    mockUseHardwareInfoAtom.mockReturnValue({
      hardwareInfo: defaultSysInfo,
      networkInfo: [networkWithLinkLocal],
    });
    const { result } = renderHook(() => useExportToClipboard());
    await act(async () => {
      await result.current.exportToClipboard();
    });
    const content = getWrittenContent();
    expect(content).toContain("AA:BB:CC:DD:EE:FF");
    expect(content).toContain("fe80::1");
  });

  it("omits linkLocalIpv6 section when array is empty", async () => {
    mockUseHardwareInfoAtom.mockReturnValue({
      hardwareInfo: defaultSysInfo,
      networkInfo: [networkNoLinkLocal],
    });
    const { result } = renderHook(() => useExportToClipboard());
    await act(async () => {
      await result.current.exportToClipboard();
    });
    const content = getWrittenContent();
    expect(content).toContain("AA:BB:CC:DD:EE:FF");
  });

  // ── Process count ─────────────────────────────────────────────────────────

  it("reflects process count from useProcessInfo", async () => {
    mockUseProcessInfo.mockReturnValue([
      { pid: 1, name: "proc1" },
      { pid: 2, name: "proc2" },
    ]);
    mockUseHardwareInfoAtom.mockReturnValue({
      hardwareInfo: { ...defaultSysInfo, cpu: fullCpuInfo },
      networkInfo: [],
    });
    const { result } = renderHook(() => useExportToClipboard());
    await act(async () => {
      await result.current.exportToClipboard();
    });
    expect(getWrittenContent()).toContain("2");
  });
});
