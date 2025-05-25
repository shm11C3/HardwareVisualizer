import { cn } from "@/lib/utils";
import type { SelectedDisplayType } from "@/types/ui";
import {
  CaretDoubleLeft,
  CaretDoubleRight,
  ComputerTowerIcon,
  CpuIcon,
} from "@phosphor-icons/react";
import { ChartLine, Gear, SquaresFour } from "@phosphor-icons/react";
import { type JSX, memo, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { tv } from "tailwind-variants";
import { useMenu } from "./hooks/useMenu";

const menuTypes = [
  "dashboard",
  "usage",
  "cpuDetail",
  "insights",
  "settings",
] as const;

const buttonClasses = tv({
  base: "fixed top-0 rounded-xl hover:bg-zinc-300 dark:hover:bg-gray-700 p-2 transition-all cursor-pointer z-20",
  variants: {
    open: {
      true: "left-64",
      false: "left-16",
    },
  },
});

const sideMenuClasses = tv({
  base: "fixed top-0 left-0 h-full bg-zinc-300 dark:bg-gray-800 dark:text-white w-64 transform transition-transform duration-300 ease-in-out",
  variants: {
    open: {
      true: "translate-x-0",
      false: "-translate-x-full",
    },
  },
});

const closedSideMenuClasses = tv({
  base: "fixed top-0 left-0 h-full bg-zinc-300 dark:bg-gray-800 dark:text-white w-16 transform transition-transform duration-300 ease-in-out",
  variants: {
    open: {
      true: "-translate-x-full",
      false: "translate-x-0",
    },
  },
});

const menuItemClasses = tv({
  base: "rounded-lg transition-colors",
  variants: {
    selected: {
      true: "dark:text-white font-bold",
      false:
        "text-neutral-700 dark:text-slate-400 hover:text-slate-400 dark:hover:text-neutral-200",
    },
    isBottom: {
      true: "h-full",
      false: "mb-2",
    },
  },
});

const MenuItem = memo(
  ({
    type,
    selected,
    handleMenuClick,
  }: {
    type: SelectedDisplayType;
    selected: boolean;
    handleMenuClick: (type: SelectedDisplayType) => void;
  }) => {
    const { t } = useTranslation();

    const menuTitles: Record<SelectedDisplayType, string> = {
      dashboard: t("pages.dashboard.name"),
      usage: t("pages.usage.name"),
      cpuDetail: "CPU",
      insights: t("pages.insights.name"),
      settings: t("pages.settings.name"),
    };

    const menuIcons: Record<SelectedDisplayType, JSX.Element> = {
      dashboard: <SquaresFour size={20} />,
      usage: <ComputerTowerIcon size={20} />,
      cpuDetail: <CpuIcon size={20} />,
      insights: <ChartLine size={20} />,
      settings: <Gear size={20} />,
    };

    return (
      <li
        className={menuItemClasses({
          selected: selected,
          isBottom: type === "settings",
        })}
      >
        <button
          type="button"
          className={cn(
            "flex h-full w-full cursor-pointer items-center text-left",
            type === "settings" ? "" : "p-2",
          )}
          onClick={() => handleMenuClick(type)}
          aria-label={`${menuTitles[type]} tab`}
          aria-selected={selected}
          role="tab"
        >
          {menuIcons[type]}
          <span className="ml-1">{menuTitles[type]}</span>
        </button>
      </li>
    );
  },
);

const ClosedSideMenu = ({
  type,
  selected,
  handleMenuClick,
}: {
  type: SelectedDisplayType;
  selected: boolean;
  handleMenuClick: (type: SelectedDisplayType) => void;
}) => {
  const menuIcons: Record<SelectedDisplayType, JSX.Element> = {
    dashboard: <SquaresFour size={24} />,
    usage: <ComputerTowerIcon size={24} />,
    cpuDetail: <CpuIcon size={24} />,
    insights: <ChartLine size={24} />,
    settings: <Gear size={24} />,
  };

  return (
    <li
      className={menuItemClasses({
        selected,
        isBottom: type === "settings",
      })}
    >
      <button
        type="button"
        className={cn(
          "flex h-full w-full cursor-pointer items-center justify-center text-left",
          type === "settings" ? "" : "p-2",
        )}
        onClick={() => handleMenuClick(type)}
        aria-label={`open ${type}`}
      >
        {menuIcons[type]}
      </button>
    </li>
  );
};

export const SideMenu = memo(() => {
  const { isOpen, displayTarget, handleMenuClick, toggleMenu } = useMenu();

  const caretIcon = useMemo(
    () => (isOpen ? <CaretDoubleLeft /> : <CaretDoubleRight />),
    [isOpen],
  );

  return (
    isOpen != null && (
      <div className="inset-0">
        <div className="fixed z-60">
          {/** カーソルが近づいた時だけアイコンを表示する */}
          <button
            type="button"
            className={buttonClasses({ open: isOpen })}
            onClick={toggleMenu}
          >
            {caretIcon}
          </button>
          {/** Opened */}
          <div className={sideMenuClasses({ open: isOpen })}>
            <div className="relative flex h-full flex-col">
              <ul className="p-4 pb-16">
                {/* bottom space for settings */}
                <li className="mb-4">
                  {isOpen && (
                    <h2 className="font-bold text-xl">HardwareVisualizer</h2>
                  )}
                </li>
                {menuTypes
                  .filter((v) => v !== "settings")
                  .map((type) => (
                    <MenuItem
                      key={type}
                      type={type}
                      handleMenuClick={handleMenuClick}
                      selected={displayTarget === type}
                    />
                  ))}
              </ul>
              <ul className="absolute bottom-0 h-14 w-full border-slate-200 border-t-1 p-3 dark:border-zinc-60">
                <MenuItem
                  type="settings"
                  handleMenuClick={handleMenuClick}
                  selected={displayTarget === "settings"}
                />
              </ul>
            </div>
          </div>
          {/** Closed */}
          <div className={closedSideMenuClasses({ open: isOpen })}>
            <div className="relative flex h-full flex-col">
              <ul className="pt-2 ">
                {menuTypes
                  .filter((v) => v !== "settings")
                  .map((type) => (
                    <ClosedSideMenu
                      key={type}
                      type={type}
                      selected={displayTarget === type}
                      handleMenuClick={handleMenuClick}
                    />
                  ))}
              </ul>
              <ul className="absolute bottom-0 h-14 w-full border-slate-200 border-t-1 p-3 dark:border-zinc-60">
                <ClosedSideMenu
                  type="settings"
                  selected={displayTarget === "settings"}
                  handleMenuClick={handleMenuClick}
                />
              </ul>
            </div>
          </div>
        </div>
      </div>
    )
  );
});
