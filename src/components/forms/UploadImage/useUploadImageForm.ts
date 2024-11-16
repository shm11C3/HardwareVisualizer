import { useBackgroundImage } from "@/hooks/useBgImage";
import { zodResolver } from "@hookform/resolvers/zod";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { z } from "zod";

//　画像アップロードカスタムフック
export const useUploadImage = () => {
  const { t } = useTranslation();
  const [displayUrl, setDisplayUrl] = useState<string | null>(null);
  const [fileName, setFilename] = useState<string>("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const { saveBackgroundImage } = useBackgroundImage();

  const formSchema = z.object({
    picture: z
      .instanceof(File, {
        message: t("pages.settings.backgroundImage.upload.errors.notSelected"),
      })
      .refine((file) => ["image/jpeg", "image/png"].includes(file.type), {
        message: t("pages.settings.backgroundImage.upload.errors.format"),
      }),
  });

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      picture: undefined,
    },
  });

  const onSubmit = async (values: z.infer<typeof formSchema>) => {
    if (isSubmitting) return;
    setIsSubmitting(true);

    try {
      await saveBackgroundImage(values.picture);
      form.reset();
    } catch (error) {
      console.error("Error saveBackgroundImage:", error);
    }

    setIsSubmitting(false);
  };

  const picture = form.watch("picture");

  useEffect(() => {
    if (picture) {
      setFilename(picture.name);
      setDisplayUrl(URL.createObjectURL(picture));
    } else {
      setFilename("");
      setDisplayUrl(null);
    }
  }, [picture]);

  return {
    form,
    picture,
    onSubmit,
    isSubmitting,
    fileName,
    displayUrl,
  };
};
