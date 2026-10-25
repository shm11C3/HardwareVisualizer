import { CaretDownIcon, CaretUpIcon } from "@phosphor-icons/react";
import {
  type JSX,
  memo,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import { tv } from "tailwind-variants";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { minOpacity } from "@/consts/style";
import type { ProcessStat } from "@/features/hardware/insights/types/processStats";
import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";
import { useStickyObserver } from "@/hooks/window/useStickyObserver";
import { formatBytes, formatDuration } from "@/lib/formatter";

const PAGE_SIZE = 100;

export const ProcessTable = ({
  processStats,
  loading,
  period,
}: {
  processStats: ProcessStat[] | null;
  loading: boolean;
  period: number;
}) => {
  const { settings } = useSettingsAtom();
  const { t } = useTranslation();
  const scrollRef = useRef<HTMLElement>(null);
  const [pageState, setPageState] = useState({ period, page: 1 });

  const [sortConfig, setSortConfig] = useState<{
    key: keyof ProcessStat;
    direction: "ascending" | "descending";
  } | null>(null);

  const sortedProcesses = useMemo(() => {
    if (!processStats) return [];
    if (sortConfig == null) {
      return processStats;
    }

    return [...processStats].sort((a, b) => {
      const aValue = a[sortConfig.key];
      const bValue = b[sortConfig.key];

      // Compare directly for numbers
      if (typeof aValue === "number" && typeof bValue === "number") {
        return sortConfig.direction === "ascending"
          ? aValue - bValue
          : bValue - aValue;
      }

      // Convert to number if stored as string
      if (typeof aValue === "string" && typeof bValue === "string") {
        const aNumber = Number.parseFloat(aValue);
        const bNumber = Number.parseFloat(bValue);

        // Compare as numbers if not NaN to avoid string comparison of decimals
        if (!Number.isNaN(aNumber) && !Number.isNaN(bNumber)) {
          return sortConfig.direction === "ascending"
            ? aNumber - bNumber
            : bNumber - aNumber;
        }

        // Compare as strings if not numbers
        return sortConfig.direction === "ascending"
          ? aValue.localeCompare(bValue)
          : bValue.localeCompare(aValue);
      }

      // Don't change order if types differ
      return 0;
    });
  }, [processStats, sortConfig]);

  const pageCount = Math.max(1, Math.ceil(sortedProcesses.length / PAGE_SIZE));
  const page =
    pageState.period === period ? Math.min(pageState.page, pageCount) : 1;
  // Commit the correction so later archive growth cannot restore an invalid page.
  if (pageState.period !== period || pageState.page !== page) {
    setPageState({ period, page });
  }
  const pageNumbers = Array.from(
    { length: pageCount },
    (_, index) => index + 1,
  );
  const pageStart = (page - 1) * PAGE_SIZE;
  const pageProcesses = useMemo(
    () => sortedProcesses.slice(pageStart, pageStart + PAGE_SIZE),
    [sortedProcesses, pageStart],
  );

  const previousNavigation = useRef({ page, period, sortConfig });
  useLayoutEffect(() => {
    const previous = previousNavigation.current;
    if (
      previous.page !== page ||
      previous.period !== period ||
      previous.sortConfig !== sortConfig
    ) {
      if (scrollRef.current) scrollRef.current.scrollTop = 0;
      previousNavigation.current = { page, period, sortConfig };
    }
  }, [page, period, sortConfig]);

  if (processStats == null || (loading && processStats.length === 0)) {
    return (
      <Skeleton className="m-4 h-[400px] w-full xl:h-[600px] 2xl:h-[800px]" />
    );
  }

  const requestSort = (key: keyof ProcessStat) => {
    let direction: "ascending" | "descending" = "ascending";
    if (
      sortConfig &&
      sortConfig.key === key &&
      sortConfig.direction === "ascending"
    ) {
      direction = "descending";
    }
    setSortConfig({ key, direction });
    setPageState({ period, page: 1 });
  };

  return (
    <div
      className="rounded-md bg-card-non-transparent p-4 text-foreground shadow-md"
      style={{
        opacity:
          settings.selectedBackgroundImg != null
            ? Math.max(
                (1 - settings.backgroundImgOpacity / 100) ** 2,
                minOpacity,
              )
            : 1,
      }}
    >
      <section
        ref={scrollRef}
        aria-label={t("pages.insights.process.title")}
        // biome-ignore lint/a11y/noNoninteractiveTabindex: Keyboard users need to focus and scroll the table region.
        tabIndex={0}
        className="max-h-[400px] overflow-auto xl:max-h-[600px] 2xl:max-h-[800px]"
      >
        <InfoTable
          processes={pageProcesses}
          sortConfig={sortConfig}
          requestSort={requestSort}
        />
      </section>
      <div className="mt-4 flex flex-wrap items-center justify-between gap-3 text-sm">
        <p aria-live="polite">
          {t("pages.insights.process.pagination.range", {
            start: sortedProcesses.length === 0 ? 0 : pageStart + 1,
            end: Math.min(pageStart + PAGE_SIZE, sortedProcesses.length),
            total: sortedProcesses.length,
          })}
        </p>
        {pageCount > 1 && (
          <nav
            aria-label={t("pages.insights.process.pagination.label")}
            className="flex flex-wrap items-center gap-2"
          >
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={page === 1}
              onClick={() => setPageState({ period, page: page - 1 })}
            >
              {t("pages.insights.process.pagination.previous")}
            </Button>
            <select
              aria-label={t("pages.insights.process.pagination.page")}
              className="h-9 rounded-md border border-border bg-background px-2 text-foreground"
              value={page}
              onChange={(event) =>
                setPageState({ period, page: Number(event.target.value) })
              }
            >
              {pageNumbers.map((pageNumber) => (
                <option key={pageNumber} value={pageNumber}>
                  {t("pages.insights.process.pagination.pageNumber", {
                    page: pageNumber,
                    pages: pageCount,
                  })}
                </option>
              ))}
            </select>
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={page === pageCount}
              onClick={() => setPageState({ period, page: page + 1 })}
            >
              {t("pages.insights.process.pagination.next")}
            </Button>
          </nav>
        )}
      </div>
    </div>
  );
};

const InfoTable = ({
  processes,
  sortConfig,
  requestSort,
}: {
  processes: ProcessStat[];
  requestSort: (key: keyof ProcessStat) => void;
  sortConfig: {
    key: keyof ProcessStat;
    direction: "ascending" | "descending";
  } | null;
}) => {
  const { t } = useTranslation();
  const { sentinelRef, isStuck } = useStickyObserver();

  const sortIcon: Record<"ascending" | "descending", JSX.Element> = {
    ascending: <CaretUpIcon className="ml-1" size={18} />,
    descending: <CaretDownIcon className="ml-1" size={18} />,
  };

  return (
    <>
      <div ref={sentinelRef} className="h-1" />
      <table className="w-full whitespace-nowrap text-left">
        <thead className="sticky top-[-1px] bg-card-non-transparent">
          <tr className="border-gray-700 border-b">
            <th
              className="cursor-pointer py-2 pr-4 dark:text-gray-400"
              onClick={() => requestSort("pid")}
              onKeyDown={() => requestSort("pid")}
            >
              <div className={tableHeaderVariants({ isStuck })}>
                <span>PID</span>
                {sortConfig &&
                  sortConfig.key === "pid" &&
                  sortIcon[sortConfig.direction]}
              </div>
            </th>
            <th
              className="cursor-pointer py-2 pr-4 dark:text-gray-400"
              onClick={() => requestSort("process_name")}
              onKeyDown={() => requestSort("process_name")}
            >
              <div className={tableHeaderVariants({ isStuck })}>
                <span>{t("shared.name")}</span>
                {sortConfig &&
                  sortConfig.key === "process_name" &&
                  sortIcon[sortConfig.direction]}
              </div>
            </th>
            <th
              className="cursor-pointer py-2 pr-4 dark:text-gray-400"
              onClick={() => requestSort("avg_cpu_usage")}
              onKeyDown={() => requestSort("avg_cpu_usage")}
            >
              <div className={tableHeaderVariants({ isStuck })}>
                <span>{t("shared.avgCpuUsage")}</span>
                {sortConfig &&
                  sortConfig.key === "avg_cpu_usage" &&
                  sortIcon[sortConfig.direction]}
              </div>
            </th>
            <th
              className="cursor-pointer py-2 pr-4 dark:text-gray-400"
              onClick={() => requestSort("avg_memory_usage")}
              onKeyDown={() => requestSort("avg_memory_usage")}
            >
              <div className={tableHeaderVariants({ isStuck })}>
                <span>{t("shared.avgMemoryUsageValue")}</span>
                {sortConfig &&
                  sortConfig.key === "avg_memory_usage" &&
                  sortIcon[sortConfig.direction]}
              </div>
            </th>
            <th
              className="cursor-pointer py-2 pr-4 dark:text-gray-400"
              onClick={() => requestSort("total_execution_sec")}
              onKeyDown={() => requestSort("total_execution_sec")}
            >
              <div className={tableHeaderVariants({ isStuck })}>
                <span>{t("shared.totalExecTime")}</span>
                {sortConfig &&
                  sortConfig.key === "total_execution_sec" &&
                  sortIcon[sortConfig.direction]}
              </div>
            </th>
            <th
              className="cursor-pointer py-2 pr-4 dark:text-gray-400"
              onClick={() => requestSort("latest_timestamp")}
              onKeyDown={() => requestSort("latest_timestamp")}
            >
              <div className={tableHeaderVariants({ isStuck })}>
                <span>{t("shared.latestExecTime")}</span>
                {sortConfig &&
                  sortConfig.key === "latest_timestamp" &&
                  sortIcon[sortConfig.direction]}
              </div>
            </th>
          </tr>
        </thead>
        <TableBody processes={processes} />
      </table>
    </>
  );
};

const TableBody = memo(({ processes }: { processes: ProcessStat[] }) => {
  const { settings } = useSettingsAtom();

  return (
    <tbody>
      {processes.map((process) => {
        return (
          // Archive identity is (pid, process_name); a PID can be reused.
          <tr
            key={JSON.stringify([process.pid, process.process_name])}
            className="border-gray-700 border-b"
          >
            <td className="py-2 pr-4">{process.pid}</td>
            <td className="py-2 pr-4">{process.process_name}</td>
            <td className="py-2 pr-4">
              {Number.parseFloat(process.avg_cpu_usage.toFixed(2))}%
            </td>
            <td className="py-2 pr-4">
              {formatBytes(process.avg_memory_usage * 1024).join(" ")}
            </td>
            <td className="py-2 pr-4">
              {formatDuration(
                process.total_execution_sec,
                settings.language === "ja" ? "ja-JP" : "en-US",
              )}
            </td>
            <td className="py-2 pr-4">
              {new Date(process.latest_timestamp).toLocaleString()}
            </td>
          </tr>
        );
      })}
    </tbody>
  );
});

const tableHeaderVariants = tv({
  base: "flex transition-all duration-300",
  variants: {
    isStuck: {
      true: "items-end h-20 mb-2",
      false: "items-center h-14",
    },
  },
});
