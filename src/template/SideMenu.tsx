import { useSettingsAtom } from "@/atom/useSettingsAtom";
import { useTauriStore } from "@/hooks/useTauriStore";
import type { SelectedDisplayType } from "@/types/ui";
import { CaretDoubleLeft, CaretDoubleRight } from "@phosphor-icons/react";
import { memo, useCallback } from "react";
import { tv } from "tailwind-variants";

const buttonClasses = tv({
  base: "fixed top-0 rounded-xl hover:bg-zinc-300 dark:hover:bg-gray-700 p-2 transition-all",
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

const menuTitles: Record<SelectedDisplayType, string> = {
  dashboard: "Dashboard",
  usage: "Usage",
  settings: "Settings",
};

const SideMenu = () => {
  const [isOpen, setMenuOpen] = useTauriStore("sideMenuOpen", false);
  const { settings, updateStateAtom } = useSettingsAtom();

  const toggleMenu = () => setMenuOpen(!isOpen);

  const handleMenuClick = useCallback(
    (type: SelectedDisplayType) => {
      updateStateAtom("display", type);
    },
    [updateStateAtom],
  );

  const MenuItem = memo(({ type }: { type: SelectedDisplayType }) => {
    return (
      <li
        className={menuItemClasses({
          selected: settings.state.display === type,
        })}
      >
        <button
          type="button"
          className="p-2 w-full h-full text-left"
          onClick={() => handleMenuClick(type)}
          aria-expanded={isOpen}
          aria-label={isOpen ? "Close menu" : "Open menu"}
        >
          {menuTitles[type]}
        </button>
      </li>
    );
  });

  return (
    <div className="inset-0">
      <div className="fixed z-50">
        <button
          type="button"
          className={buttonClasses({ open: isOpen })}
          onClick={toggleMenu}
        >
          {isOpen ? <CaretDoubleLeft /> : <CaretDoubleRight />}
        </button>
        <div className={sideMenuClasses({ open: isOpen })}>
          <ul className="p-4">
            <li className="mb-4">
              <h2 className="text-xl font-bold">Hardware Monitor</h2>
            </li>
            <MenuItem type="dashboard" />
            <MenuItem type="usage" />
            <MenuItem type="settings" />
          </ul>
        </div>
      </div>
    </div>
  );
};

export default SideMenu;
