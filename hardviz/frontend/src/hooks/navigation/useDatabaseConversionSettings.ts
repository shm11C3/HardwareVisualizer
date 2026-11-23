import { useAtom, useSetAtom } from "jotai";
import { useEffect, useRef, useState } from "react";
import { commands } from "@/rspc/bindings";
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

/**
 * Whether to offer the way into the conversion section. A build without
 * `duckdb-archive` also answers native-only reads with `needsNativeDatabase`
 * (ADR 0030 decision 6), but reports `notSupported` here and renders no such
 * section. False until the answer arrives. A failed read does not prove the
 * build cannot convert, so it keeps the entry point.
 */
export const useOffersDatabaseConversionSettings = () => {
  const [offered, setOffered] = useState(false);

  useEffect(() => {
    let active = true;
    const read = async () => {
      let supported = true;
      try {
        const state = await commands.getDatabaseConversionState();
        supported = state.kind !== "notSupported";
      } catch (error) {
        console.error("Failed to read database conversion support:", error);
      }
      if (active) setOffered(supported);
    };
    void read();

    return () => {
      active = false;
    };
  }, []);

  return offered;
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
