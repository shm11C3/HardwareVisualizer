import { useEffect, useState } from "react";
import { Dashboard } from "./features/hardware/dashboard/Dashboard";
import { ChartTemplate } from "./features/hardware/usage/Usage";
import "./index.css";
import type { ErrorInfo, JSX } from "react";
import { ErrorBoundary } from "react-error-boundary";
import {
  useHardwareUpdater,
  useUsageUpdater,
} from "@/features/hardware/hooks/useHardwareData";
import { useErrorModalListener } from "@/hooks/useTauriEventListener";
import ErrorFallback from "./components/ErrorFallback";
import { ScreenTemplate } from "./components/shared/ScreenTemplate";
import { SideMenu } from "./features/menu/SideMenu";
import { useSettingsAtom } from "./features/settings/hooks/useSettingsAtom";
import { Settings } from "./features/settings/Settings";
import { useBackgroundImage } from "./hooks/useBgImage";
import { useColorTheme } from "./hooks/useColorTheme";
import type { SelectedDisplayType } from "./types/ui";
import "@/lib/i18n";
import {
  ChartLineIcon,
  CpuIcon,
  GearIcon,
  SquaresFourIcon,
} from "@phosphor-icons/react";
import { useAtom } from "jotai";
import { useTranslation } from "react-i18next";
import { FullscreenExitButton } from "./components/ui/FullScreenExit";
import { useHardwareInfoAtom } from "./features/hardware/hooks/useHardwareInfoAtom";
import { Insights } from "./features/hardware/insights/Insights";
import { CpuUsages } from "./features/hardware/usage/cpu/CpuUsage";
import { displayTargetAtom } from "./features/menu/hooks/useMenu";
import { useKeydown } from "./hooks/useInputListener";
import { useTauriStore } from "./hooks/useTauriStore";

const onError = (error: Error, info: ErrorInfo) => {
  console.error("error.message", error.message);
  console.error(
    "info.componentStack:",
    info.componentStack ?? "No stack trace available",
  );
};

export const App = () => {
  const { settings, loadSettings } = useSettingsAtom();
  useColorTheme(settings.theme);
  const { backgroundImage: nextImage, initBackgroundImage } =
    useBackgroundImage();
  const { t, i18n } = useTranslation();
  const [isDecorated, setDecorated] = useTauriStore("window_decorated", false);

  const [currentImage, setCurrentImage] = useState(nextImage);
  const [opacity, setOpacity] = useState(1);

  useErrorModalListener();
  useUsageUpdater("cpu");
  useUsageUpdater("memory");
  useUsageUpdater("gpu");
  useUsageUpdater("processors");
  useHardwareUpdater("gpu", "temp");
  useHardwareUpdater("gpu", "fan");
  const { hardwareInfo } = useHardwareInfoAtom();

  useEffect(() => {
    i18n.changeLanguage(settings.language);
  }, [settings.language, i18n]);

  useEffect(() => {
    setOpacity(0);
    const fadeOutTimeout = setTimeout(() => {
      setCurrentImage(nextImage);
      setOpacity(1);
    }, 500);

    return () => clearTimeout(fadeOutTimeout);
  }, [nextImage]);

  useEffect(() => {
    loadSettings();
    initBackgroundImage();
  }, [loadSettings, initBackgroundImage]);

  const [displayTarget] = useAtom(displayTargetAtom);

  useKeydown({ isDecorated: Boolean(isDecorated), setDecorated });

  const displayTargets: Record<SelectedDisplayType, JSX.Element> = {
    dashboard: (
      <ScreenTemplate
        icon={<SquaresFourIcon size={32} />}
        title={t("pages.dashboard.name")}
      >
        <Dashboard />
      </ScreenTemplate>
    ),
    usage: <ChartTemplate />,
    cpuDetail: (
      <ScreenTemplate
        icon={<CpuIcon size={32} />}
        title={hardwareInfo.cpu?.name || "CPU"}
      >
        <CpuUsages />
      </ScreenTemplate>
    ),
    insights: (
      <ScreenTemplate
        icon={<ChartLineIcon size={32} />}
        title={t("pages.insights.name")}
      >
        <Insights />
      </ScreenTemplate>
    ),
    settings: (
      <ScreenTemplate
        icon={<GearIcon size={32} />}
        title={t("pages.settings.name")}
      >
        <Settings />
      </ScreenTemplate>
    ),
  };

  return (
    <ErrorBoundary FallbackComponent={ErrorFallback} onError={onError}>
      <div
        className="min-h-screen bg-background bg-cover text-foreground duration-300 ease-in-out"
        style={{ backgroundImage: "var(--background-gradient)" }}
      >
        <div
          className="fixed inset-0 bg-center bg-cover transition-opacity duration-500"
          style={{
            backgroundImage: `url(${currentImage})`,
            backgroundAttachment: "fixed",
            backgroundSize: "cover",
            opacity: currentImage
              ? opacity * (settings.backgroundImgOpacity / 100)
              : 0,
          }}
        />
        <div className="relative z-10">
          <SideMenu />
          {displayTarget ? (
            displayTargets[displayTarget]
          ) : (
            <div
              className="min-h-screen bg-background bg-cover text-foreground"
              style={{ backgroundImage: "var(--background-gradient)" }}
            />
          )}
        </div>
      </div>
      <FullscreenExitButton
        isDecorated={Boolean(isDecorated)}
        setDecorated={setDecorated}
      />
    </ErrorBoundary>
  );
};
