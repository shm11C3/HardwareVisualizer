import { ImageSquareIcon, SpinnerIcon, UploadSimpleIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "../../../components/ui/form";
import { Input } from "../../../components/ui/input";
import { useUploadImage } from "../hooks/useUploadImageForm";

export const UploadImage = () => {
  const { form, picture, onSubmit, isSubmitting, fileName, displayUrl } =
    useUploadImage();
  const { t } = useTranslation();

  return (
    <Form {...form}>
      <form onSubmit={form.handleSubmit(onSubmit)}>
        <FormField
          control={form.control}
          name="picture"
          render={({ field: { onChange, value, ...rest } }) => (
            <>
              <FormItem>
                <div className="flex items-center gap-4">
                  <div className="h-28">
                    <FormLabel className="mr-2 mb-2 block text-lg">
                      {t("pages.settings.backgroundImage.upload.name")}
                    </FormLabel>
                    <FormControl>
                      <div className="relative h-10 w-80">
                        <Input
                          type="file"
                          accept="image/png, image/jpeg"
                          className="absolute inset-0 h-full w-full cursor-pointer opacity-0"
                          {...rest}
                          onChange={(e) => {
                            onChange(e.target.files?.[0]);
                          }}
                        />
                        <Button
                          className="flex h-full w-full items-center justify-start"
                          type="button"
                          variant="secondary"
                        >
                          {picture ? (
                            <span className="truncate">{fileName}</span>
                          ) : (
                            t(
                              "pages.settings.backgroundImage.upload.pleaseSelectAFile",
                            )
                          )}
                          {displayUrl ? (
                            <img
                              src={displayUrl}
                              alt=""
                              className="h-full w-full object-contain object-right"
                            />
                          ) : (
                            <ImageSquareIcon className="ml-auto object-contain" />
                          )}
                        </Button>
                      </div>
                    </FormControl>
                    <FormMessage />
                    <FormDescription className="my-2">
                      {t("pages.settings.backgroundImage.upload.description")}
                    </FormDescription>
                  </div>
                  <div className="flex h-28 items-center">
                    <Button type="submit" disabled={!picture || isSubmitting}>
                      {t("pages.settings.backgroundImage.upload.confirm")}
                      {isSubmitting ? (
                        <SpinnerIcon className="ml-1 animate-spin" />
                      ) : (
                        <UploadSimpleIcon className="ml-1" />
                      )}
                    </Button>
                  </div>
                </div>
              </FormItem>
            </>
          )}
        />
      </form>
    </Form>
  );
};
