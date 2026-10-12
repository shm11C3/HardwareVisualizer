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
  const [fileName, setFilename] = useState<string>("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const { saveBackgroundImage, failure } = useBackgroundImage();

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
    saveFailed: failure === "upload",
    fileName,
    displayUrl,
  };
};
