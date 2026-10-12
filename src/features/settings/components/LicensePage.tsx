import { ArrowLeft, ExternalLink } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ActionFailure } from "@/components/ActionFailure";
import { LoadFailure } from "@/components/LoadFailure";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { commands } from "@/rspc/bindings";
import { isError, isOk } from "@/types/result";

interface LicensePageProps {
  onBack: () => void;
}

export const LicensePage = ({ onBack }: LicensePageProps) => {
  const { t } = useTranslation();
  const [licenseContent, setLicenseContent] = useState("");
  const [thirdPartyContent, setThirdPartyContent] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [licenseFailed, setLicenseFailed] = useState(false);
  const [thirdPartyFailed, setThirdPartyFailed] = useState(false);
  const [openFailed, setOpenFailed] = useState(false);

  // Open the directory where files exist
  const openLicenseInExplorer = async () => {
    setOpenFailed(false);
    try {
      const result = await commands.openLicenseFilePath();
      if (isError(result)) {
        console.error("Failed to open LICENSE file in explorer:", result.error);
        setOpenFailed(true);
      }
    } catch (e) {
      console.error("Failed to open LICENSE file in explorer:", e);
      setOpenFailed(true);
    }
  };

  const loadLicenseFiles = useCallback(async () => {
    setIsLoading(true);
    setLicenseFailed(false);
    setThirdPartyFailed(false);
    try {
      const [licenseResult, thirdPartyResult] = await Promise.all([
        commands.readLicenseFile(),
        commands.readThirdPartyNoticesFile(),
      ]);

      if (isOk(licenseResult)) {
        setLicenseContent(licenseResult.data);
      } else {
        console.error("Failed to load license file:", licenseResult.error);
        setLicenseFailed(true);
      }

      if (isOk(thirdPartyResult)) {
        setThirdPartyContent(thirdPartyResult.data);
      } else {
        console.error(
          "Failed to load third party notices file:",
          thirdPartyResult.error,
        );
        setThirdPartyFailed(true);
      }
    } catch (e) {
      console.error("Failed to load license files:", e);
      setLicenseFailed(true);
      setThirdPartyFailed(true);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadLicenseFiles();
  }, [loadLicenseFiles]);

  // A failed read is one panel's failure, not an empty license: the tab shows
  // the failure state with a retry instead of an empty text.
  const renderContent = (content: string, failed: boolean) => {
    if (isLoading) {
      return (
        <div className="flex h-32 items-center justify-center">
          <p className="text-muted-foreground">{t("shared.loading")}</p>
        </div>
      );
    }
    if (failed) {
      return (
        <LoadFailure
          message={t("pages.settings.license.loadFailed")}
          onRetry={() => void loadLicenseFiles()}
          className="h-32"
        />
      );
    }
    return (
      <pre className="whitespace-pre-wrap font-mono text-foreground text-sm leading-relaxed">
        {content}
      </pre>
    );
  };

  return (
    <div className="flex h-full w-full flex-col">
      {/* Header with back button */}
      <div className="flex items-center justify-between border-b py-6">
        <div className="flex items-center gap-4">
          <Button variant="ghost" size="sm" onClick={onBack} className="gap-2">
            <ArrowLeft size={24} />
          </Button>
          <h1 className="font-bold text-2xl">
            {t("pages.settings.about.license")}
          </h1>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 p-6">
        <Tabs defaultValue="license" className="flex h-full w-full flex-col">
          <TabsList className="grid w-full max-w-md grid-cols-2">
            <TabsTrigger value="license">
              {t("pages.settings.license.applicationLicense")}
            </TabsTrigger>
            <TabsTrigger value="third-party">
              {t("pages.settings.license.thirdPartyLicenses")}
            </TabsTrigger>
          </TabsList>

          <TabsContent value="license" className="mt-6 flex-1">
            <div className="h-full rounded-lg border bg-card">
              <div className="flex items-center justify-between border-b p-4">
                <h3 className="font-semibold">
                  {t("pages.settings.license.applicationLicense")}
                </h3>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={openLicenseInExplorer}
                  className="gap-2"
                >
                  <ExternalLink size={16} />
                  {t("shared.showInExplorer")}
                </Button>
              </div>
              {openFailed && (
                <ActionFailure
                  message={t("pages.settings.license.openFailed")}
                  className="border-b px-4 py-2"
                />
              )}
              <ScrollArea className="h-[calc(100vh-340px)] p-6">
                {renderContent(licenseContent, licenseFailed)}
              </ScrollArea>
            </div>
          </TabsContent>

          <TabsContent value="third-party" className="mt-6 flex-1">
            <div className="h-full rounded-lg border bg-card">
              <div className="flex items-center justify-between border-b p-4">
                <h3 className="font-semibold">
                  {t("pages.settings.license.thirdPartyLicenses")}
                </h3>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={openLicenseInExplorer}
                  className="gap-2"
                >
                  <ExternalLink size={16} />
                  {t("shared.showInExplorer")}
                </Button>
              </div>
              {openFailed && (
                <ActionFailure
                  message={t("pages.settings.license.openFailed")}
                  className="border-b px-4 py-2"
                />
              )}
              <ScrollArea className="h-[calc(100vh-340px)] p-6">
                {renderContent(thirdPartyContent, thirdPartyFailed)}
              </ScrollArea>
            </div>
          </TabsContent>
        </Tabs>
      </div>
    </div>
  );
};
