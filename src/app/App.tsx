import { Suspense, useCallback, useEffect, useState } from "react";
import {
  ChartTemplate,
  CpuUsages,
  Dashboard,
  Insights,
  Performance,
  prefetchScreen,
  Settings,
  SystemSpecifications,
} from "./lazyScreens";
import "@/index.css";
import type { CSSProperties, ErrorInfo, JSX } from "react";
import { ErrorBoundary } from "react-error-boundary";
import { useHardwareEventListener } from "@/features/hardware/hooks/useHardwareEventListener";
import { useSelectedGpuPersistence } from "@/features/hardware/hooks/useSelectedGpuPersistence";
import { useSelectedStorageDevicePersistence } from "@/features/hardware/hooks/useSelectedStorageDevicePersistence";
import { SideMenu } from "@/features/menu/SideMenu";
import { useBackgroundImage } from "@/hooks/appearance/useBgImage";
import { useColorTheme } from "@/hooks/appearance/useColorTheme";
import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";
import { useTauriDialog } from "@/hooks/tauri/useTauriDialog";
import { useErrorModalListener } from "@/hooks/tauri/useTauriEventListener";
import { useDocumentVisibilityClass } from "@/hooks/window/useDocumentVisibilityClass";
import type { SelectedDisplayType } from "@/types/ui";
import { ElevationUnavailableNotice } from "./ElevationUnavailableNotice";
import ErrorFallback from "./ErrorFallback";
import { RootErrorFallback } from "./RootErrorFallback";
import { ScreenTemplate } from "./ScreenTemplate";
import "@/lib/i18n";
import {
  ChartLineIcon,
  ComputerTowerIcon,
  CpuIcon,
  GearIcon,
  SquaresFourIcon,
} from "@phosphor-icons/react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { platform } from "@tauri-apps/plugin-os";
import { useAtomValue } from "jotai";
import { useTranslation } from "react-i18next";
import { useHardwareInfoAtom } from "@/features/hardware/hooks/useHardwareInfoAtom";
import { TrayWidgetFlyout } from "@/features/tray/TrayWidgetFlyout";
import { AppUpdate } from "@/features/updater/AppUpdate";
import { useTitleIconVisualSelector } from "@/hooks/appearance/useTitleIconVisualSelector";
import { useTauriStore } from "@/hooks/tauri/useTauriStore";
import { useFullScreenMode } from "@/hooks/window/useFullScreenMode";
import { useKeydown } from "@/hooks/window/useInputListener";
import type { PerformanceGuidanceContext } from "@/lib/externalComponentGuidance";
import { cn } from "@/lib/utils";
import { displayTargetAtom } from "@/store/navigation";
import { AppStateProvider, useAppStateReset } from "./AppStateProvider";
import { FullScreenButton } from "./FullScreenButton";
import { FullscreenExitButton } from "./FullScreenExit";
import { resetAppState } from "./resetAppState";
import { StartupDialogs } from "./StartupDialogs";

const onRootError = (error: unknown, info: ErrorInfo) => {
  console.error(
    "root.error.message",
    error instanceof Error ? error.message : String(error),
  );
  console.error(
    "root.info.componentStack:",
    info.componentStack ?? "No stack trace available",
  );
};

const onError = (error: unknown, info: ErrorInfo) => {
  console.error(
    "error.message",
    error instanceof Error ? error.message : String(error),
  );
  console.error(
    "info.componentStack:",
    info.componentStack ?? "No stack trace available",
  );
};

/** Outer wrapper to catch initialization errors */
export const App = () => {
  if (getCurrentWindowLabel() === "tray-widget-flyout") {
    return (
      <ErrorBoundary
        FallbackComponent={RootErrorFallback}
        onError={onRootError}
      >
        <AppStateProvider>
          <TrayWidgetFlyout />
        </AppStateProvider>
      </ErrorBoundary>
    );
  }

  return (
    <ErrorBoundary FallbackComponent={RootErrorFallback} onError={onRootError}>
      <AppStateProvider>
        <AppContent />
      </AppStateProvider>
    </ErrorBoundary>
  );
};

const getCurrentWindowLabel = () => {
  try {
    return getCurrentWindow().label;
  } catch {
    return "main";
  }
};

/** Main app content with reset-capable error boundary */
const AppContent = () => {
  const { settings, loadSettings } = useSettingsAtom();
  useColorTheme(settings.theme);
  const { backgroundImage: nextImage, initBackgroundImage } =
    useBackgroundImage();
  const { t, i18n } = useTranslation();
  const { error } = useTauriDialog();
  const [isDecorated, setDecorated] = useTauriStore("window_decorated", true);

  const [currentImage, setCurrentImage] = useState(nextImage);
  const [opacity, setOpacity] = useState(1);
  const [settingsLoaded, setSettingsLoaded] = useState(false);
  const [settingsLoadFailed, setSettingsLoadFailed] = useState(false);
  const [performanceContext, setPerformanceContext] =
    useState<PerformanceGuidanceContext>({
      settled: false,
      cpuTemperatureVisible: false,
      motherboardSensorsVisible: false,
      expanded: false,
    });

  useErrorModalListener();
  useDocumentVisibilityClass();
  useHardwareEventListener();
  useSelectedGpuPersistence();
  useSelectedStorageDevicePersistence();
  const { hardwareInfo } = useHardwareInfoAtom();

  useEffect(() => {
    i18n.changeLanguage(settings.language);
    document.documentElement.lang = settings.language;
  }, [settings.language, i18n]);

  useEffect(() => {
    document.documentElement.classList.toggle(
      "user-select-none",
      !settings.textSelectable,
    );
  }, [settings.textSelectable]);

  useEffect(() => {
    setOpacity(0);
    const fadeOutTimeout = setTimeout(() => {
      setCurrentImage(nextImage);
      setOpacity(1);
    }, 500);

    return () => clearTimeout(fadeOutTimeout);
  }, [nextImage]);

  useEffect(() => {
    let isCancelled = false;

    const initialize = async () => {
      const [didLoadSettings, backgroundResult] = await Promise.allSettled([
        loadSettings(),
        initBackgroundImage(),
      ]);

      if (!isCancelled) {
        const loaded =
          didLoadSettings.status === "fulfilled" && didLoadSettings.value;
        setSettingsLoaded(loaded);
        setSettingsLoadFailed(!loaded);
      }

      if (backgroundResult.status === "rejected") {
        throw backgroundResult.reason;
      }
    };

    initialize();

    return () => {
      isCancelled = true;
    };
  }, [loadSettings, initBackgroundImage]);

  // Settings that cannot be read are an app-level failure (the screens run on
  // defaults), so this stays a dialog. Reported once per failed load.
  useEffect(() => {
    if (!settingsLoadFailed) return;
    setSettingsLoadFailed(false);
    void error(t("shared.settingsLoadFailed"));
  }, [settingsLoadFailed, error, t]);

  const displayTarget = useAtomValue(displayTargetAtom);

  const { visibleTypes } = useTitleIconVisualSelector();

  useKeydown({ isDecorated: Boolean(isDecorated), setDecorated });
  const { isFullScreen, toggleFullScreen } = useFullScreenMode();
  const isMacOS = platform() === "macos";
  const windowOpacityRatio = settings.transparentUi
    ? settings.windowOpacity / 100
    : 1;
  const transparentSurfaceOpacity = Math.min(
    70,
    Math.max(34, settings.windowOpacity * 0.64),
  );

  const restartAppState = useAppStateReset();
  const handleReset = useCallback(
    () => resetAppState(restartAppState),
    [restartAppState],
  );

  const displayTargets: Record<SelectedDisplayType, JSX.Element> = {
    dashboard: (
      <ScreenTemplate
        icon={
          visibleTypes.includes("dashboard") ? (
            <SquaresFourIcon size={32} />
          ) : undefined
        }
        title={
          visibleTypes.includes("dashboard")
            ? t("pages.dashboard.name")
            : undefined
        }
        enabledBurnInShift
        isFullScreen={Boolean(isFullScreen)}
      >
        <Dashboard />
      </ScreenTemplate>
    ),
    performance: (
      <Performance
        isFullScreen={Boolean(isFullScreen)}
        showTitle={visibleTypes.includes("dashboard")}
        onGuidanceContextChange={setPerformanceContext}
      />
    ),
    systemSpecifications: (
      <ScreenTemplate
        icon={
          visibleTypes.includes("dashboard") ? (
            <ComputerTowerIcon size={32} />
          ) : undefined
        }
        title={
          visibleTypes.includes("dashboard")
            ? t("navigation.systemSpecifications")
            : undefined
        }
        enabledBurnInShift
        isFullScreen={Boolean(isFullScreen)}
      >
        <SystemSpecifications />
      </ScreenTemplate>
    ),
    usage: <ChartTemplate isFullScreen={Boolean(isFullScreen)} />,
    cpuDetail: (
      <ScreenTemplate
        icon={
          visibleTypes.includes("cpuDetail") ? <CpuIcon size={32} /> : undefined
        }
        title={
          visibleTypes.includes("cpuDetail")
            ? hardwareInfo.cpu?.name || "CPU"
            : undefined
        }
        enabledBurnInShift
        isFullScreen={Boolean(isFullScreen)}
      >
        <CpuUsages />
      </ScreenTemplate>
    ),
    insights: (
      <ScreenTemplate
        icon={
          visibleTypes.includes("insights") ? (
            <ChartLineIcon size={32} />
          ) : undefined
        }
        title={
          visibleTypes.includes("insights")
            ? t("pages.insights.name")
            : undefined
        }
        enabledBurnInShift
        isFullScreen={Boolean(isFullScreen)}
      >
        <Insights />
      </ScreenTemplate>
    ),
    settings: (
      <ScreenTemplate
        icon={
          visibleTypes.includes("settings") ? <GearIcon size={32} /> : undefined
        }
        title={
          visibleTypes.includes("settings")
            ? t("pages.settings.name")
            : undefined
        }
      >
        <Settings />
      </ScreenTemplate>
    ),
  };

  return (
    <ErrorBoundary
      FallbackComponent={ErrorFallback}
      onError={onError}
      onReset={handleReset}
    >
      <div
        className={cn(
          "min-h-screen bg-background bg-cover text-foreground duration-300 ease-in-out",
          settings.transparentUi && "transparent-ui",
          settings.transparentUi && isMacOS && "native-vibrancy",
        )}
        style={
          {
            "--window-opacity-percent": `${settings.windowOpacity}%`,
            "--transparent-surface-opacity-percent": `${transparentSurfaceOpacity}%`,
            "--transparent-backdrop-blur-px": `${settings.glassBlur}px`,
          } as CSSProperties
        }
      >
        <div
          className="transparent-theme-backdrop fixed inset-0 bg-background bg-cover transition-opacity duration-300"
          style={{
            backgroundImage: "var(--background-gradient)",
            opacity: windowOpacityRatio,
          }}
        />
        <div
          className="fixed inset-0 bg-center bg-cover transition-opacity duration-500"
          style={{
            backgroundImage: `url(${currentImage})`,
            backgroundAttachment: "fixed",
            backgroundSize: "cover",
            opacity: currentImage
              ? opacity *
                (settings.backgroundImgOpacity / 100) *
                windowOpacityRatio
              : 0,
          }}
        />
        {settings.transparentUi && (
          <div className="transparent-app-backdrop pointer-events-none fixed inset-0" />
        )}
        <div className="relative z-10">
          <SideMenu
            isFullScreen={isFullScreen || false}
            navigationLayout={settings.navigationLayout}
            settingsLoaded={settingsLoaded}
            onPrefetchScreen={prefetchScreen}
          />
          <Suspense>
            {displayTarget ? (
              displayTargets[displayTarget]
            ) : (
              <div className="min-h-screen text-foreground" />
            )}
          </Suspense>
          <AppUpdate />
          <StartupDialogs
            settingsLoaded={settingsLoaded}
            displayTarget={displayTarget}
            performanceContext={performanceContext}
            nsisMigrationNoticeDismissed={settings.nsisMigrationNoticeDismissed}
          />
          <ElevationUnavailableNotice settingsLoaded={settingsLoaded} />
        </div>
      </div>
      <FullscreenExitButton
        isDecorated={Boolean(isDecorated)}
        setDecorated={setDecorated}
      />
      <FullScreenButton
        isFullScreen={isFullScreen || false}
        onToggleFullScreen={toggleFullScreen}
      />
    </ErrorBoundary>
  );
};
