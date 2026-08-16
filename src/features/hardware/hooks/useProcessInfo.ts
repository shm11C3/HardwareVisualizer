import { atom, useAtomValue, useStore } from "jotai";
import { useEffect, useRef } from "react";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import { commands, type ProcessInfo } from "@/rspc/bindings";

const PROCESS_POLL_INTERVAL_MS = 3000;
const processesAtom = atom<ProcessInfo[]>([]);
const disabledProcessesAtom = atom<ProcessInfo[]>([]);

type ErrorReporter = (pollingError: unknown) => void;
type Store = ReturnType<typeof useStore>;

class ProcessPollingCoordinator {
  private readonly consumers = new Map<symbol, ErrorReporter>();
  private intervalId: ReturnType<typeof setInterval> | undefined;
  private requestInFlight = false;
  private pendingDemandRefresh = false;
  private demandRefreshScheduled = false;
  private generation = 0;

  constructor(private readonly store: Store) {}

  subscribe(reportError: ErrorReporter) {
    const consumerId = Symbol("process-polling-consumer");
    const isFirstConsumer = this.consumers.size === 0;
    this.consumers.set(consumerId, reportError);

    if (isFirstConsumer) {
      document.addEventListener(
        "visibilitychange",
        this.handleVisibilityChange,
      );
      if (!document.hidden) {
        this.startInterval();
        this.scheduleDemandRefresh();
      }
    }

    return () => {
      if (!this.consumers.delete(consumerId) || this.consumers.size > 0) {
        return;
      }

      document.removeEventListener(
        "visibilitychange",
        this.handleVisibilityChange,
      );
      this.pause();
    };
  }

  private readonly handleVisibilityChange = () => {
    if (document.hidden) {
      this.pause();
      return;
    }

    if (this.consumers.size > 0) {
      this.startInterval();
      this.scheduleDemandRefresh();
    }
  };

  private startInterval() {
    if (this.intervalId !== undefined) {
      return;
    }

    this.intervalId = setInterval(() => {
      void this.refreshProcesses("periodic");
    }, PROCESS_POLL_INTERVAL_MS);
  }

  private pause() {
    this.generation += 1;
    this.pendingDemandRefresh = false;

    if (this.intervalId !== undefined) {
      clearInterval(this.intervalId);
      this.intervalId = undefined;
    }
  }

  private scheduleDemandRefresh() {
    if (this.demandRefreshScheduled) {
      return;
    }

    this.demandRefreshScheduled = true;
    // One microtask of deferral collapses rapid subscribe/unsubscribe cycles
    // (StrictMode remounts, visibility flaps) into a single request.
    void Promise.resolve().then(() => {
      this.demandRefreshScheduled = false;
      if (this.hasVisibleDemand()) {
        void this.refreshProcesses("demand");
      }
    });
  }

  private async refreshProcesses(trigger: "demand" | "periodic") {
    if (!this.hasVisibleDemand()) {
      return;
    }

    if (this.requestInFlight) {
      if (trigger === "demand") {
        this.pendingDemandRefresh = true;
      }
      return;
    }

    this.requestInFlight = true;
    const requestGeneration = this.generation;

    try {
      const processesData = await commands.getProcessList();
      if (this.isCurrentRequest(requestGeneration)) {
        this.store.set(processesAtom, processesData);
      }
    } catch (pollingError) {
      if (this.isCurrentRequest(requestGeneration)) {
        // All reporters open the same shared dialog; any surviving one works.
        this.consumers.values().next().value?.(pollingError);
        console.error("Failed to fetch processes:", pollingError);
      }
    } finally {
      this.requestInFlight = false;

      if (this.pendingDemandRefresh) {
        this.pendingDemandRefresh = false;
        this.scheduleDemandRefresh();
      }
    }
  }

  private hasVisibleDemand() {
    return this.consumers.size > 0 && !document.hidden;
  }

  private isCurrentRequest(requestGeneration: number) {
    return requestGeneration === this.generation && this.hasVisibleDemand();
  }
}

const coordinators = new WeakMap<Store, ProcessPollingCoordinator>();

const getCoordinator = (store: Store) => {
  let coordinator = coordinators.get(store);
  if (!coordinator) {
    coordinator = new ProcessPollingCoordinator(store);
    coordinators.set(store, coordinator);
  }
  return coordinator;
};

export const useProcessInfo = ({
  enabled = true,
}: {
  enabled?: boolean;
} = {}) => {
  const { error } = useTauriDialog();
  const errorRef = useRef(error);
  const store = useStore();
  const processes = useAtomValue(
    enabled ? processesAtom : disabledProcessesAtom,
  );

  useEffect(() => {
    errorRef.current = error;
  }, [error]);

  useEffect(() => {
    if (!enabled) {
      return;
    }

    return getCoordinator(store).subscribe((pollingError) => {
      errorRef.current(pollingError as string);
    });
  }, [store, enabled]);

  return processes;
};
