import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import {
  useCallback,
  useEffect,
  useOptimistic,
  useRef,
  useState,
  useTransition,
} from "react";
import { useTranslation } from "react-i18next";
import { ActionFailure } from "@/components/ActionFailure";
import { LoadFailure } from "@/components/LoadFailure";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";

export const AutoStartToggle = () => {
  const { t } = useTranslation();
  // The last value read from or written to the OS. The switch shows it
  // through `useOptimistic`, so a write that fails needs no rollback code:
  // the optimistic value falls away when the transition ends.
  const [confirmed, setConfirmed] = useState<boolean | null>(null);
  const [readFailed, setReadFailed] = useState(false);
  const [toggleFailed, setToggleFailed] = useState(false);
  const [shown, showOptimistically] = useOptimistic(
    confirmed,
    (_current, next: boolean) => next,
  );
  const [isPending, startTransition] = useTransition();
  const readIdRef = useRef(0);

  const readAutoStart = useCallback(async () => {
    const readId = readIdRef.current + 1;
    readIdRef.current = readId;

    try {
      const enabled = await isEnabled();
      if (readIdRef.current === readId) {
        setConfirmed(enabled);
        setReadFailed(false);
      }
    } catch (err) {
      console.error("Failed to read autostart:", err);
      // A superseded read must not flip the state.
      if (readIdRef.current === readId) {
        setReadFailed(true);
      }
    }
  }, []);

  const retryRead = () => {
    setReadFailed(false);
    void readAutoStart();
  };

  const toggleAutoStart = (value: boolean) => {
    setToggleFailed(false);

    startTransition(async () => {
      showOptimistically(value);

      try {
        value ? await enable() : await disable();
        // Updates after an `await` leave the action's transition, so the
        // confirmed value is committed together with the optimistic one
        // falling away.
        startTransition(() => {
          setConfirmed(value);
        });
      } catch (err) {
        console.error("Failed to set autostart:", err);
        setToggleFailed(true);
      }
    });
  };

  useEffect(() => {
    void readAutoStart();

    return () => {
      readIdRef.current += 1;
    };
  }, [readAutoStart]);

  return (
    <div className="w-full py-6 xl:w-1/3">
      <div className="flex w-full items-center justify-between space-x-4">
        <div className="space-y-0.5">
          <Label htmlFor="insight" className="text-lg">
            {t("pages.settings.general.autostart.name")}
          </Label>
        </div>

        {readFailed ? (
          <LoadFailure
            message={t("shared.loadFailed")}
            onRetry={retryRead}
            className="h-auto w-auto items-end"
          />
        ) : shown != null ? (
          // A write in flight blocks further toggles: two overlapping OS
          // writes could finish in either order and leave the switch showing
          // a value the OS does not have.
          <Switch
            checked={shown}
            disabled={isPending}
            onCheckedChange={toggleAutoStart}
          />
        ) : (
          <Skeleton className="h-[24px] w-[44px] rounded-full" />
        )}
      </div>
      {toggleFailed && (
        <ActionFailure
          message={t("pages.settings.general.autostart.error")}
          className="mt-2"
        />
      )}
    </div>
  );
};
