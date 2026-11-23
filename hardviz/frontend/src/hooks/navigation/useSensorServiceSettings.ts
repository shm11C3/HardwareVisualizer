import { useAtom, useSetAtom } from "jotai";
import { useEffect, useRef } from "react";
import {
  displayTargetAtom,
  sensorServiceSettingsRequestedAtom,
} from "@/store/navigation";

/** Open the Settings destination and focus its existing service content section. */
export const useSensorServiceSettings = () => {
  const setDisplayTarget = useSetAtom(displayTargetAtom);
  const requestSection = useSetAtom(sensorServiceSettingsRequestedAtom);

  return () => {
    requestSection(true);
    setDisplayTarget("settings");
  };
};

export const useSensorServiceSettingsFocus = (enabled: boolean) => {
  const ref = useRef<HTMLElement>(null);
  const [requested, setRequested] = useAtom(sensorServiceSettingsRequestedAtom);

  useEffect(() => {
    if (!enabled || !requested || !ref.current) return;
    ref.current.focus({ preventScroll: true });
    ref.current.scrollIntoView({ block: "start" });
    setRequested(false);
  }, [enabled, requested, setRequested]);

  return ref;
};
