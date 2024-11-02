import { zodResolver } from "@hookform/resolvers/zod";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

const convertFileToBase64 = (file: File): Promise<string> => {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.readAsDataURL(file);
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = (error) => reject(error);
  });
};

const formSchema = z.object({
  picture: z
    .instanceof(File, { message: "Please select a file" })
    .refine((file) => ["image/jpeg", "image/png"].includes(file.type), {
      message: "Please select JPEG or PNG format images",
    }),
});

//　画像アップロードカスタムフック
export const useUploadImage = () => {
  const [displayUrl, setDisplayUrl] = useState<string | null>(null);
  const [fileName, setFilename] = useState<string>("");

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      picture: undefined,
    },
  });

  const onSubmit = async (values: z.infer<typeof formSchema>) => {
    try {
      const base64String = await convertFileToBase64(values.picture);
      console.log(base64String);

      form.reset();
    } catch (error) {
      console.error("Error converting file to base64:", error);
    }
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
    fileName,
    displayUrl,
  };
};
