import { Provider } from "jotai";
import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useState,
} from "react";

const AppStateResetContext = createContext<(() => void) | null>(null);

/**
 * Mounts the one jotai `Provider` of a window, at the composition root.
 *
 * The Provider creates its own store on mount, so the store is the unit of
 * reset: bumping the epoch remounts the Provider and its whole subtree, which
 * returns every atom to its initial value and abandons store-keyed state
 * (`WeakMap<Store, ...>`) together with the old store.
 */
export const AppStateProvider = ({ children }: { children: ReactNode }) => {
  const [epoch, setEpoch] = useState(0);
  const resetAppStateEpoch = useCallback(() => setEpoch((n) => n + 1), []);

  return (
    <AppStateResetContext.Provider value={resetAppStateEpoch}>
      <Provider key={epoch}>{children}</Provider>
    </AppStateResetContext.Provider>
  );
};

/** Returns a function that restarts the window's app state from scratch. */
export const useAppStateReset = (): (() => void) => {
  const reset = useContext(AppStateResetContext);
  if (reset === null) {
    throw new Error("useAppStateReset must be used within AppStateProvider");
  }
  return reset;
};
