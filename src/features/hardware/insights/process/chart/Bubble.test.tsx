import { cleanup, render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ProcessBubbleChart } from "@/features/hardware/insights/process/chart/Bubble";
import type { ProcessStat } from "@/features/hardware/insights/types/processStats";

const mocks = vi.hoisted(() => ({ scatter: {} as Record<string, unknown> }));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, values?: { displayed: number; omitted: number }) =>
      values
        ? `Showing ${values.displayed}; others ${values.omitted}. See table.`
        : key,
  }),
}));
vi.mock("recharts", async (importOriginal) => {
  const { getNiceTickValues } =
    await importOriginal<typeof import("recharts")>();
  const Container = ({ children }: { children: ReactNode }) => (
    <div>{children}</div>
  );
  return {
    getNiceTickValues,
    ResponsiveContainer: Container,
    ScatterChart: Container,
    CartesianGrid: () => null,
    YAxis: () => null,
    Tooltip: () => null,
    XAxis: ({ domain }: { domain: unknown }) => (
      <div data-testid="x-domain">{JSON.stringify(domain)}</div>
    ),
    ZAxis: ({ domain }: { domain: unknown }) => (
      <div data-testid="z-domain">{JSON.stringify(domain)}</div>
    ),
    Scatter: (props: Record<string, unknown>) => {
      mocks.scatter = props;
      return null;
    },
  };
});

afterEach(cleanup);

const records = (): ProcessStat[] =>
  Array.from({ length: 750 }, (_, i) => ({
    pid: i + 1,
    process_name: `process-${i + 1}`,
    avg_cpu_usage: i < 250 ? 80 : 0.01,
    avg_memory_usage: i >= 250 && i < 500 ? 2_000_000 : 1,
    total_execution_sec: i === 500 ? 1_200_000 : 60,
    latest_timestamp: "2026-10-15T16:14:12.661Z",
  }));

describe("ProcessBubbleChart bounded display", () => {
  it("reports omitted entries while retaining full-result scales and default animation", () => {
    render(<ProcessBubbleChart processStats={records()} loading={false} />);

    expect(
      screen.getByText("Showing 500; others 250. See table."),
    ).toBeInTheDocument();
    expect(mocks.scatter["data"]).toHaveLength(500);
    expect(mocks.scatter["isAnimationActive"]).toBeUndefined();
    expect(screen.getByTestId("x-domain")).toHaveTextContent("[0,20000]");
    expect(screen.getByTestId("z-domain")).toHaveTextContent(
      JSON.stringify([0, Math.sqrt(12_000)]),
    );
  });

  it("does not show an omission message for a complete result", () => {
    render(
      <ProcessBubbleChart
        processStats={records().slice(0, 2)}
        loading={false}
      />,
    );
    expect(screen.queryByText(/others/)).not.toBeInTheDocument();
    expect(mocks.scatter["data"]).toHaveLength(2);
  });

  it("handles the initial unavailable result without reporting omitted entries", () => {
    render(<ProcessBubbleChart processStats={null} loading={true} />);
    expect(screen.queryByText(/others/)).not.toBeInTheDocument();
    expect(mocks.scatter["data"]).toEqual([]);
  });
});
