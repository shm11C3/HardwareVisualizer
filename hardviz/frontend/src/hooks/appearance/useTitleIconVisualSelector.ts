import { useAtomValue, useSetAtom } from "jotai";
import { useCallback } from "react";
import { showTitleIconAtom } from "@/store/titleIcon";
import type { SelectedDisplayType } from "@/types/ui";

export const useTitleIconVisualSelector = () => {
  const setShowTitleIcon = useSetAtom(showTitleIconAtom);
  const visibleTypes = useAtomValue(showTitleIconAtom);

  const isTitleIconVisible = useCallback(
    (type: SelectedDisplayType): boolean => {
      return visibleTypes.includes(type);
    },
    [visibleTypes],
  );

  const toggleTitleIconVisibility = useCallback(
    (type: SelectedDisplayType, visible: boolean) => {
      setShowTitleIcon((prev) => {
        const isVisible = prev.includes(type);

        if (visible) {
          if (!isVisible) {
            return [...prev, type];
          }
          return prev;
        }

        if (!isVisible) {
          return prev;
        }

        return prev.filter((t) => t !== type);
      });
    },
    [setShowTitleIcon],
  );

  return { visibleTypes, isTitleIconVisible, toggleTitleIconVisibility };
};
