import { zodResolver } from "@hookform/resolvers/zod";
import { useEffect, useState } from "react";
import { useForm, useWatch } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { z } from "zod";
import { useBackgroundImage } from "@/hooks/appearance/useBgImage";

// Custom hook for image upload
export const useUploadImage = () => {
  const { t } = useTranslation();
  const [displayUrl, setDisplayUrl] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const { saveBackgroundImage, failures } = useBackgroundImage();

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
    defaultValues: {},
  });

  const onSubmit = async (values: z.infer<typeof formSchema>) => {
    if (isSubmitting) return;
    setIsSubmitting(true);

    try {
      // Keep the chosen file when the save failed, so the user can submit
      // again; the failure line under the control says why nothing changed.
      if (await saveBackgroundImage(values.picture)) {
        form.reset();
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  const picture = useWatch({
    control: form.control,
    name: "picture",
  });

  const fileName = picture?.name ?? "";

  // The object URL holds the file in memory until it is revoked, so the Effect
  // that creates it also releases it when the picture changes or unmounts.
  useEffect(() => {
    if (!picture) {
      setDisplayUrl(null);
      return;
    }

    const url = URL.createObjectURL(picture);
    setDisplayUrl(url);
    return () => {
      URL.revokeObjectURL(url);
    };
  }, [picture]);

  return {
    form,
    picture,
    onSubmit,
    isSubmitting,
    saveFailed: failures.has("upload"),
    fileName,
    displayUrl,
  };
};
