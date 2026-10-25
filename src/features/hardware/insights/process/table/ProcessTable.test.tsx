import {
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import "@/lib/i18n";
import { ProcessTable } from "@/features/hardware/insights/process/table/ProcessTable";
import type { ProcessStat } from "@/features/hardware/insights/types/processStats";

vi.mock("@/hooks/settings/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: {
      selectedBackgroundImg: null,
      backgroundImgOpacity: 50,
      language: "en",
    },
  }),
}));
vi.mock("@/hooks/window/useStickyObserver", () => ({
  useStickyObserver: () => ({ sentinelRef: { current: null }, isStuck: false }),
}));
afterEach(cleanup);

const records = (count: number): ProcessStat[] =>
  Array.from({ length: count }, (_, index) => ({
    pid: index + 1,
    process_name: `process-${String(index + 1).padStart(4, "0")}`,
    avg_cpu_usage: (index + 1) / 10,
    avg_memory_usage: (index + 1) * 1024,
    total_execution_sec: 60 * (index + 1),
    latest_timestamp: "2026-10-15T16:14:12.661Z",
  }));
const rows = () =>
  within(screen.getByRole("table")).getAllByRole("row").slice(1);
const firstPid = () => within(rows()[0]).getAllByRole("cell")[0].textContent;
const selectPage = (page: number) =>
  fireEvent.change(screen.getByRole("combobox", { name: "Page" }), {
    target: { value: String(page) },
  });
const sortCpu = () =>
  fireEvent.click(screen.getByRole("columnheader", { name: "Avg CPU Usage" }));

describe("ProcessTable pagination", () => {
  it("renders at most 100 rows and reaches the final partial page", () => {
    render(
      <ProcessTable processStats={records(237)} loading={false} period={60} />,
    );
    expect(rows()).toHaveLength(100);
    expect(screen.getByText("1–100 of 237 entries")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Previous" })).toBeDisabled();

    const region = screen.getByRole("region", { name: "Process" });
    region.scrollTop = 200;
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(firstPid()).toBe("101");
    expect(region.scrollTop).toBe(0);
    expect(screen.getByText("101–200 of 237 entries")).toBeInTheDocument();

    selectPage(3);
    expect(rows()).toHaveLength(37);
    expect(firstPid()).toBe("201");
    expect(screen.getByText("201–237 of 237 entries")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Next" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Previous" }));
    expect(firstPid()).toBe("101");
  });

  it("sorts the entire result before paging and returns to page one on sort", () => {
    render(
      <ProcessTable
        processStats={records(237).reverse()}
        loading={false}
        period={60}
      />,
    );
    expect(firstPid()).toBe("237");
    sortCpu();
    expect(firstPid()).toBe("1");
    selectPage(2);
    expect(firstPid()).toBe("101");
    sortCpu();
    expect(screen.getByRole("combobox", { name: "Page" })).toHaveValue("1");
    expect(firstPid()).toBe("237");
    expect(within(rows()[99]).getAllByRole("cell")[0]).toHaveTextContent("138");
  });

  it("preserves page, sorting, and scroll position across loading and data refresh", () => {
    const data = records(237);
    const { rerender } = render(
      <ProcessTable processStats={data} loading={false} period={60} />,
    );
    sortCpu();
    selectPage(2);
    const region = screen.getByRole("region", { name: "Process" });
    region.scrollTop = 120;
    rerender(<ProcessTable processStats={data} loading={true} period={60} />);
    expect(screen.getByRole("combobox", { name: "Page" })).toHaveValue("2");
    expect(region.scrollTop).toBe(120);

    rerender(
      <ProcessTable
        processStats={records(238).reverse()}
        loading={false}
        period={60}
      />,
    );
    expect(firstPid()).toBe("101");
    expect(screen.getByRole("combobox", { name: "Page" })).toHaveValue("2");
    expect(region.scrollTop).toBe(120);
    expect(screen.getByText("101–200 of 238 entries")).toBeInTheDocument();
  });

  it("resets the page and scroll on period changes while preserving the sort", () => {
    const data = records(237);
    const { rerender } = render(
      <ProcessTable processStats={data} loading={false} period={60} />,
    );
    sortCpu();
    sortCpu();
    selectPage(3);
    screen.getByRole("region", { name: "Process" }).scrollTop = 120;
    rerender(
      <ProcessTable processStats={data} loading={false} period={1440} />,
    );
    expect(screen.getByRole("combobox", { name: "Page" })).toHaveValue("1");
    expect(firstPid()).toBe("237");
    expect(screen.getByRole("region", { name: "Process" }).scrollTop).toBe(0);
  });

  it("clamps a removed page without restoring it on growth and handles an empty result", () => {
    const { rerender } = render(
      <ProcessTable processStats={records(237)} loading={false} period={60} />,
    );
    selectPage(3);
    rerender(
      <ProcessTable processStats={records(150)} loading={false} period={60} />,
    );
    expect(rows()).toHaveLength(50);
    expect(screen.getByRole("combobox", { name: "Page" })).toHaveValue("2");
    rerender(
      <ProcessTable processStats={records(237)} loading={false} period={60} />,
    );
    expect(screen.getByRole("combobox", { name: "Page" })).toHaveValue("2");

    rerender(<ProcessTable processStats={[]} loading={false} period={60} />);
    expect(rows()).toHaveLength(0);
    expect(screen.getByText("0–0 of 0 entries")).toBeInTheDocument();
    expect(screen.queryByRole("combobox")).not.toBeInTheDocument();
    rerender(
      <ProcessTable processStats={records(237)} loading={false} period={60} />,
    );
    expect(screen.getByRole("combobox", { name: "Page" })).toHaveValue("1");
  });

  it("keeps separate archived names with the same PID when repeatedly sorting", () => {
    const data = records(3);
    data[0] = { ...data[0], pid: 42, process_name: "z-worker" };
    data[1] = { ...data[1], pid: 42, process_name: "a-worker" };
    data[2].process_name = "m-worker";
    render(<ProcessTable processStats={data} loading={false} period={60} />);
    const names = () =>
      rows().map((row) => within(row).getAllByRole("cell")[1].textContent);
    const sortName = () =>
      fireEvent.click(screen.getByRole("columnheader", { name: "Name" }));
    sortName();
    expect(names()).toEqual(["a-worker", "m-worker", "z-worker"]);
    sortName();
    expect(names()).toEqual(["z-worker", "m-worker", "a-worker"]);
    expect(rows()).toHaveLength(3);
  });

  it("waits for the initial result without presenting it as an empty table", () => {
    render(<ProcessTable processStats={null} loading={true} period={60} />);
    expect(screen.queryByRole("table")).not.toBeInTheDocument();
    expect(screen.queryByText(/0–0/)).not.toBeInTheDocument();
  });
});
