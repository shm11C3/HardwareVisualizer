import { useTauriStore } from "@/hooks/useTauriStore";
import type { SelectedDisplayType } from "@/types/ui";
import { CaretDoubleLeft, CaretDoubleRight } from "@phosphor-icons/react";
import { atom, useAtom } from "jotai";
import { memo, useCallback, useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { tv } from "tailwind-variants";

const buttonClasses = tv({
  base: "fixed top-0 rounded-xl hover:bg-zinc-300 dark:hover:bg-gray-700 p-2 transition-all cursor-pointer",
  variants: {
    open: {
      true: "left-64",
      false: "left-0",
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

const menuItemClasses = tv({
  base: "mb-2 rounded-lg transition-colors",
  variants: {
    selected: {
      true: "dark:text-white font-bold",
      false:
        "text-neutral-700 dark:text-slate-400 hover:text-slate-400 dark:hover:text-neutral-200",
    },
  },
});

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

    return (
      displayTarget &&
      isOpen && (
        <li
          className={menuItemClasses({
            selected: displayTarget === type,
          })}
        >
          <button
            type="button"
            className="p-2 w-full h-full text-left cursor-pointer"
            onClick={() => handleMenuClick(type)}
            aria-expanded={isOpen}
            aria-label={isOpen ? "Close menu" : "Open menu"}
          >
            {menuTitles[type]}
          </button>
        </li>
      )
    );
  });

  const menuItems = useMemo(
    () => (
      <>
        <MenuItem type="dashboard" />
        <MenuItem type="usage" />
        <MenuItem type="insights" />
        <MenuItem type="settings" />
      </>
    ),
    [MenuItem],
  );

  return (
    isOpen != null && (
      <div className="inset-0">
        <div className="fixed z-50">
          <button
            type="button"
            className={buttonClasses({ open: isOpen })}
            onClick={toggleMenu}
            aria-label="Toggle Side Menu"
          >
            {caretIcon}
          </button>
          <div className={sideMenuClasses({ open: isOpen })}>
            <ul className="p-4">
              <li className="mb-4">
                <h2 className="text-xl font-bold">HardwareVisualizer</h2>
              </li>
              {menuItems}
            </ul>
          </div>
        </div>
      </div>
    )
  );
});
