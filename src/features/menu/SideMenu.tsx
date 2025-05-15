import { useTauriStore } from "@/hooks/useTauriStore";
import { cn } from "@/lib/utils";
import type { SelectedDisplayType } from "@/types/ui";
import { CaretDoubleLeft, CaretDoubleRight } from "@phosphor-icons/react";
import { ChartLine, Cpu, Gear, SquaresFour } from "@phosphor-icons/react";
import { atom, useAtom } from "jotai";
import { type JSX, memo, useCallback, useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { tv } from "tailwind-variants";

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

const ClosedSideMenu = ({
  type,
  handleMenuClick,
}: {
  type: SelectedDisplayType;
  handleMenuClick: (type: SelectedDisplayType) => void;
}) => {
  const menuIcons: Record<SelectedDisplayType, JSX.Element> = {
    dashboard: <SquaresFour size={24} />,
    usage: <Cpu size={24} />,
    insights: <ChartLine size={24} />,
    settings: <Gear size={24} />,
  };

  return (
    <li
      className={menuItemClasses({
        selected: false,
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

export const displayTargetAtom = atom<SelectedDisplayType | null>(null);
export const SideMenu = memo(() => {
  const [, setDisplayTargetAtom] = useAtom(displayTargetAtom);
  const [isOpen, setMenuOpen] = useTauriStore("sideMenuOpen", false);
  const [displayTarget, setDisplayTarget] = useTauriStore<SelectedDisplayType>(
    "display",
    "dashboard",
  );

  useEffect(() => {
    if (displayTarget) {
      setDisplayTargetAtom(displayTarget);
    }
  }, [displayTarget, setDisplayTargetAtom]);

  const toggleMenu = () => setMenuOpen(!isOpen);

  const handleMenuClick = useCallback(
    (type: SelectedDisplayType) => {
      setDisplayTarget(type);
      setDisplayTargetAtom(type);
    },
    [setDisplayTarget, setDisplayTargetAtom],
  );

  const caretIcon = useMemo(
    () => (isOpen ? <CaretDoubleLeft /> : <CaretDoubleRight />),
    [isOpen],
  );

  const MenuItem = memo(({ type }: { type: SelectedDisplayType }) => {
    const { t } = useTranslation();

    const menuTitles: Record<SelectedDisplayType, string> = {
      dashboard: t("pages.dashboard.name"),
      usage: t("pages.usage.name"),
      insights: t("pages.insights.name"),
      settings: t("pages.settings.name"),
    };

    const menuIcons: Record<SelectedDisplayType, JSX.Element> = {
      dashboard: <SquaresFour size={20} />,
      usage: <Cpu size={20} />,
      insights: <ChartLine size={20} />,
      settings: <Gear size={20} />,
    };

    return (
      displayTarget &&
      isOpen && (
        <li
          className={menuItemClasses({
            selected: displayTarget === type,
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
            aria-expanded={isOpen}
            aria-label={isOpen ? "Close menu" : "Open menu"}
          >
            {menuIcons[type]}
            <span className="ml-1">{menuTitles[type]}</span>
          </button>
        </li>
      )
    );
  });

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
                <MenuItem type="dashboard" />
                <MenuItem type="usage" />
                <MenuItem type="insights" />
              </ul>
              <ul className="absolute bottom-0 h-14 w-full border-slate-200 border-t-1 p-3 dark:border-zinc-60">
                <MenuItem type="settings" />
              </ul>
            </div>
          </div>
          {/** Closed */}
          <div className={closedSideMenuClasses({ open: isOpen })}>
            <div className="relative flex h-full flex-col">
              <ul className="pt-2 ">
                <ClosedSideMenu
                  type="dashboard"
                  handleMenuClick={handleMenuClick}
                />
                <ClosedSideMenu
                  type="usage"
                  handleMenuClick={handleMenuClick}
                />
                <ClosedSideMenu
                  type="insights"
                  handleMenuClick={handleMenuClick}
                />
              </ul>
              <ul className="absolute bottom-0 h-14 w-full border-slate-200 border-t-1 p-3 dark:border-zinc-60">
                <ClosedSideMenu
                  type="settings"
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
