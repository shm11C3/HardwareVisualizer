import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { LoadFailure } from "@/components/LoadFailure";
import { Skeleton } from "@/components/ui/skeleton";
import { useHardwareInfoAtom } from "@/features/hardware/hooks/useHardwareInfoAtom";
import type { NetworkInfo } from "@/rspc/bindings";
import { formatLinkSpeed } from "./formatLinkSpeed";

/** Link state changes rarely (cable, Wi-Fi roaming), so a slow poll is enough. */
const NETWORK_POLL_INTERVAL_MS = 10_000;

const LinkSpeedValue = ({
  network,
}: {
  network: Pick<NetworkInfo, "receiveLinkSpeedBps" | "transmitLinkSpeedBps">;
}) => {
  const { t, i18n } = useTranslation();
  const rx = network.receiveLinkSpeedBps;
  const tx = network.transmitLinkSpeedBps;

  const format = (bps: number) => {
    const { value, unit } = formatLinkSpeed(bps, i18n.language);
    return t(`pages.performance.network.${unit}`, { value });
  };

  if (rx == null && tx == null) {
    // Not reported is not zero: keep the row and say so.
    return (
      <span className="font-sans text-sm text-muted-foreground">
        {t("pages.performance.network.linkSpeedNotReported")}
      </span>
    );
  }

  if (rx != null && tx != null && rx !== tx) {
    return (
      <>
        <span className="block text-sm">
          {t("pages.performance.network.receiveLinkSpeed", {
            value: format(rx),
          })}
        </span>
        <span className="block text-sm">
          {t("pages.performance.network.transmitLinkSpeed", {
            value: format(tx),
          })}
        </span>
      </>
    );
  }

  return <span className="text-sm">{format((rx ?? tx) as number)}</span>;
};

/**
 * Row keys unique within one read. A MAC alone is not unique (a Linux bridge
 * shares its member's MAC) and the list order is not stable across reads, so
 * the key combines name and MAC and numbers any remaining repeats.
 */
const withRowKeys = (networks: NetworkInfo[]) => {
  const seen = new Map<string, number>();
  return networks.map((network) => {
    const base = `${network.description ?? ""}|${network.macAddress ?? ""}`;
    const occurrence = seen.get(base) ?? 0;
    seen.set(base, occurrence + 1);
    return { network, key: occurrence === 0 ? base : `${base}|${occurrence}` };
  });
};

/**
 * Negotiated link speed per network interface. The speed is the adapter's
 * current link rate, not measured throughput, and is shown here rather than in
 * the specification sheet because it changes with the link.
 */
export const NetworkPanel = () => {
  const { t } = useTranslation();
  const { networkInfo, networkLoadFailed, initNetwork } = useHardwareInfoAtom();
  const [hasRead, setHasRead] = useState(false);
  // Retry goes through the same guarded read as the timer, so a retry and a
  // scheduled poll never race to write the shared network atoms.
  const readRef = useRef<() => void>(() => undefined);
  // biome-ignore lint/correctness/useExhaustiveDependencies: `initNetwork` only writes stable atom setters
  useEffect(() => {
    let mounted = true;
    let inFlight = false;

    const poll = async () => {
      if (inFlight) {
        return;
      }
      inFlight = true;
      try {
        await initNetwork();
      } finally {
        inFlight = false;
        if (mounted) {
          setHasRead(true);
        }
      }
    };

    readRef.current = () => void poll();
    void poll();
    const timer = setInterval(() => void poll(), NETWORK_POLL_INTERVAL_MS);

    return () => {
      mounted = false;
      readRef.current = () => undefined;
      clearInterval(timer);
    };
  }, []);

  // Checked before the cached list: the shared read keeps the last adapters on
  // failure, and showing their rates would present stale link state as current.
  if (networkLoadFailed) {
    return (
      <LoadFailure
        className="h-auto px-4 pb-4"
        message={t("pages.dashboard.systemSpecifications.networkLoadFailed")}
        onRetry={() => readRef.current()}
      />
    );
  }

  if (networkInfo.length === 0) {
    if (!hasRead) {
      return (
        <div className="px-4 pb-4">
          <Skeleton
            className="h-16 w-full rounded-md"
            data-testid="network-panel-loading"
          />
        </div>
      );
    }

    return (
      <p className="px-4 pb-4 text-sm text-muted-foreground">
        {t("pages.dashboard.systemSpecifications.networkUnavailable")}
      </p>
    );
  }

  return (
    <ul className="space-y-2 p-4 pt-2" data-testid="network-panel">
      {withRowKeys(networkInfo).map(({ network, key }) => {
        const address = network.ipv4[0] ?? network.ipv6[0];
        return (
          <li key={key} className="flex items-start justify-between gap-4">
            <div className="min-w-0">
              <p className="truncate text-sm" title={network.description ?? ""}>
                {network.description}
              </p>
              {address != null && (
                <p className="truncate font-mono text-xs text-muted-foreground tabular-nums">
                  {address}
                </p>
              )}
            </div>
            <div className="shrink-0 text-right font-mono tabular-nums">
              <LinkSpeedValue network={network} />
              <p className="font-sans text-[11px] text-muted-foreground">
                {t("pages.performance.network.linkSpeed")}
              </p>
            </div>
          </li>
        );
      })}
    </ul>
  );
};
