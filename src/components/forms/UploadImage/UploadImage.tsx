import { ImageSquare, UploadSimple } from "@phosphor-icons/react";
import { Button } from "../../ui/button";
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "../../ui/form";
import { Input } from "../../ui/input";
import { useUploadImage } from "./useUploadImageForm";

export const UploadImage = () => {
  const { form, picture, onSubmit, fileName, displayUrl } = useUploadImage();

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
                    <FormLabel className="block text-lg mb-2 mr-2">
                      Upload Image
                    </FormLabel>
                    <FormControl>
                      <div className="relative w-80 h-10">
                        <Input
                          type="file"
                          accept="image/png, image/jpeg"
                          className="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
                          {...rest}
                          onChange={(e) => {
                            onChange(e.target.files?.[0]);
                          }}
                        />
                        <Button
                          className="w-full h-full flex items-center justify-start"
                          type="button"
                          variant="secondary"
                        >
                          {picture ? fileName : "Please select a file"}
                          {displayUrl ? (
                            <img
                              src={displayUrl}
                              alt=""
                              className="w-full h-full object-contain object-right"
                            />
                          ) : (
                            <ImageSquare className="ml-auto object-contain" />
                          )}
                        </Button>
                      </div>
                    </FormControl>
                    <FormDescription className="mt-2">
                      The file will only be saved on your device
                    </FormDescription>
                    <FormMessage />
                  </div>
                  <div className="h-28 flex items-center">
                    <Button type="submit" disabled={!picture}>
                      Confirm
                      <UploadSimple className="ml-1" />
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
