import { useAtom, useAtomValue, useSetAtom } from "jotai";
import { useCallback } from "react";
import {
  availableUpdateAtom,
  updateDialogOpenAtom,
  updateIndicatorVersionAtom,
  updateInstallStartedAtom,
} from "@/store/appUpdate";

/** The update the updater owns: what it found and whether it is installing. */
export const useAvailableUpdate = () => {
  const [availableUpdate, setAvailableUpdate] = useAtom(availableUpdateAtom);
  const setInstallStarted = useSetAtom(updateInstallStartedAtom);
  return { availableUpdate, setAvailableUpdate, setInstallStarted };
};

/** Open state of the update confirmation dialog. */
export const useUpdateDialog = () => {
  const [isUpdateDialogOpen, setUpdateDialogOpen] =
    useAtom(updateDialogOpenAtom);
  return { isUpdateDialogOpen, setUpdateDialogOpen };
};

/** What the side menu needs to offer an available update. */
export const useUpdateIndicator = () => {
  const version = useAtomValue(updateIndicatorVersionAtom);
  const setUpdateDialogOpen = useSetAtom(updateDialogOpenAtom);
  const openUpdateDialog = useCallback(
    () => setUpdateDialogOpen(true),
    [setUpdateDialogOpen],
  );
  return { version, openUpdateDialog };
};
