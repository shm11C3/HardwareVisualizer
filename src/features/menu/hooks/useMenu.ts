import { atom, useAtom } from "jotai";
import { useCallback, useEffect } from "react";
import { useTauriStore } from "@/hooks/useTauriStore";
import type { NavigationLayout } from "@/rspc/bindings";
import type { SelectedDisplayType } from "@/types/ui";

export const displayTargetAtom = atom<SelectedDisplayType | null>(null);
export const sideMenuOpenAtom = atom<boolean | null>(null);
export const navigationLayoutFocusRequestedAtom = atom(false);
export const DEFAULT_DISPLAY_TARGET = "dashboard" satisfies SelectedDisplayType;

const classicDisplayTargets: SelectedDisplayType[] = [
  "dashboard",
  "usage",
  "cpuDetail",
  "insights",
  "settings",
];

const groupedDisplayTargets: SelectedDisplayType[] = [
  "performance",
  "insights",
  "settings",
];

export const normalizeDisplayTarget = (
  displayTarget: SelectedDisplayType,
  navigationLayout: NavigationLayout,
): SelectedDisplayType => {
  const allowedTargets =
    navigationLayout === "grouped"
      ? groupedDisplayTargets
      : classicDisplayTargets;

  if (allowedTargets.includes(displayTarget)) {
    return displayTarget;
  }

  return navigationLayout === "grouped" ? "performance" : "dashboard";
};

export const useMenu = (
  navigationLayout: NavigationLayout,
  settingsLoaded: boolean,
) => {
  const [displayTargetValue, setDisplayTargetAtom] = useAtom(displayTargetAtom);
  const [, setSideMenuOpenAtom] = useAtom(sideMenuOpenAtom);
  const [isOpen, setMenuOpen] = useTauriStore("sideMenuOpen", false);
  const [displayTarget, setDisplayTarget, isDisplayPending] =
    useTauriStore<SelectedDisplayType>("display", DEFAULT_DISPLAY_TARGET);

  useEffect(() => {
    if (displayTarget && !isDisplayPending) {
      if (!settingsLoaded) {
        setDisplayTargetAtom(displayTarget);
        return;
      }

      const normalizedTarget = normalizeDisplayTarget(
        displayTarget,
        navigationLayout,
      );

      if (normalizedTarget !== displayTarget) {
        setDisplayTarget(normalizedTarget);
      }
      setDisplayTargetAtom(normalizedTarget);
    }
  }, [
    displayTarget,
    isDisplayPending,
    navigationLayout,
    settingsLoaded,
    setDisplayTarget,
    setDisplayTargetAtom,
  ]);

  useEffect(() => {
    if (isOpen != null) {
      setSideMenuOpenAtom(isOpen);
    }
  }, [isOpen, setSideMenuOpenAtom]);

  const toggleMenu = useCallback(() => {
    const nextIsOpen = !isOpen;
    setMenuOpen(nextIsOpen);
    setSideMenuOpenAtom(nextIsOpen);
  }, [isOpen, setMenuOpen, setSideMenuOpenAtom]);

  const handleMenuClick = useCallback(
    (type: SelectedDisplayType) => {
      setDisplayTarget(type);
      setDisplayTargetAtom(type);
    },
    [setDisplayTarget, setDisplayTargetAtom],
  );

  return {
    isOpen,
    toggleMenu,
    handleMenuClick,
    displayTarget: displayTargetValue,
  };
};
