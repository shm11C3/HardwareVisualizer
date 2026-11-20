import { useAtom, useSetAtom } from "jotai";
import { useEffect, useRef } from "react";
import {
  databaseConversionSettingsRequestedAtom,
  displayTargetAtom,
} from "@/store/navigation";

/**
 * Open the Settings destination and focus its existing native database
 * conversion section. It only navigates; the conversion stays an explicit
 * action in that section (ADR 0030 decision 4).
 */
export const useDatabaseConversionSettings = () => {
  const setDisplayTarget = useSetAtom(displayTargetAtom);
  const requestSection = useSetAtom(databaseConversionSettingsRequestedAtom);

  return () => {
    requestSection(true);
    setDisplayTarget("settings");
  };
};

export const useDatabaseConversionSettingsFocus = (enabled: boolean) => {
  const ref = useRef<HTMLDivElement>(null);
  const [requested, setRequested] = useAtom(
    databaseConversionSettingsRequestedAtom,
  );

  useEffect(() => {
    if (!enabled || !requested || !ref.current) return;
    ref.current.focus({ preventScroll: true });
    ref.current.scrollIntoView({ block: "start" });
    setRequested(false);
  }, [enabled, requested, setRequested]);

  return ref;
};
