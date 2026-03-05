import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useUploadImage } from "@/features/settings/hooks/useUploadImageForm";

if (!URL.createObjectURL) {
  URL.createObjectURL = () => "blob:testurl";
}

// --- Dependency module mock setup ---
// react-i18next
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

// Mock useTauriDialog
const errorMock = vi.fn();
vi.mock("@/hooks/useTauriDialog", () => ({
  useTauriDialog: () => ({
    error: errorMock,
  }),
}));

// Mock useBackgroundImage
const saveBackgroundImageMock = vi.fn();
vi.mock("@/hooks/useBgImage", () => ({
  useBackgroundImage: () => ({
    saveBackgroundImage: saveBackgroundImageMock,
  }),
}));

// --- Mock URL.createObjectURL ---
const createObjectURLSpy = vi
  .spyOn(URL, "createObjectURL")
  .mockReturnValue("blob:testurl");

describe("useUploadImage", () => {
  beforeEach(() => {
    errorMock.mockReset();
    saveBackgroundImageMock.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("Initial state has empty fileName, null displayUrl, and false isSubmitting", () => {
    const { result } = renderHook(() => useUploadImage());
    expect(result.current.fileName).toBe("");
    expect(result.current.displayUrl).toBeNull();
    expect(result.current.isSubmitting).toBe(false);
  });

  it("fileName and displayUrl are updated when file is set to picture", async () => {
    const fakeFile = new File(["dummy content"], "test.png", {
      type: "image/png",
    });
    const { result } = renderHook(() => useUploadImage());

    // form is react-hook-form object so update picture with setValue
    act(() => {
      result.current.form.setValue("picture", fakeFile);
    });

    // Wait for useEffect async update
    await waitFor(() => result.current.fileName !== "");

    expect(result.current.fileName).toBe("test.png");
    // Also verify correct file was passed to URL.createObjectURL (mock return value "blob:testurl")
    expect(result.current.displayUrl).toBe("blob:testurl");
    expect(createObjectURLSpy).toHaveBeenCalledWith(fakeFile);
  });

  it("Image is saved successfully on onSubmit call and form is reset", async () => {
    const fakeFile = new File(["dummy content"], "test.jpg", {
      type: "image/jpeg",
    });
    const { result } = renderHook(() => useUploadImage());

    // First, set value to picture
    act(() => {
      result.current.form.setValue("picture", fakeFile);
    });

    // isSubmitting is false before submit
    expect(result.current.isSubmitting).toBe(false);

    // Call onSubmit (success case)
    await act(async () => {
      await result.current.onSubmit({ picture: fakeFile });
    });

    // Verify saveBackgroundImageMock was called correctly
    expect(saveBackgroundImageMock).toHaveBeenCalledWith(fakeFile);
    // Form is reset and picture value should be undefined (defaultValues)
    expect(result.current.form.getValues("picture")).toBeUndefined();
    // isSubmitting returns to false after processing completes
    expect(result.current.isSubmitting).toBe(false);
  });

  it("Error handler is called when saveBackgroundImage returns error in onSubmit", async () => {
    const fakeFile = new File(["dummy content"], "error.png", {
      type: "image/png",
    });
    // Override saveBackgroundImageMock to return error
    saveBackgroundImageMock.mockRejectedValue(new Error("Test error"));
    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    const { result } = renderHook(() => useUploadImage());

    // Set value to picture
    act(() => {
      result.current.form.setValue("picture", fakeFile);
    });

    await act(async () => {
      await result.current.onSubmit({ picture: fakeFile });
    });

    // Verify error handler was called on error
    expect(errorMock).toHaveBeenCalledWith(new Error("Test error"));
    expect(consoleErrorSpy).toHaveBeenCalledWith(
      "Error saveBackgroundImage:",
      expect.any(Error),
    );
    // isSubmitting returns to false
    expect(result.current.isSubmitting).toBe(false);
    consoleErrorSpy.mockRestore();
  });

  it("form validation rejects files with unsupported type (e.g. image/gif)", async () => {
    const gifFile = new File(["dummy content"], "test.gif", {
      type: "image/gif",
    });
    const { result } = renderHook(() => useUploadImage());
    const submitCallback = vi.fn();

    act(() => {
      result.current.form.setValue("picture", gifFile);
    });

    // handleSubmit runs validation; because refine rejects gif, submitCallback
    // must NOT be called (invalid form state).
    await act(async () => {
      await result.current.form.handleSubmit(submitCallback)();
    });

    expect(submitCallback).not.toHaveBeenCalled();
    expect(saveBackgroundImageMock).not.toHaveBeenCalled();
  });

  it("onSubmit returns early (no-op) when a submission is already in progress", async () => {
    const fakeFile = new File(["dummy content"], "test.png", {
      type: "image/png",
    });
    // Make saveBackgroundImage never resolve so isSubmitting stays true.
    saveBackgroundImageMock.mockReturnValue(new Promise(() => {}));

    const { result } = renderHook(() => useUploadImage());

    // Start first submission (isSubmitting becomes true)
    act(() => {
      result.current.onSubmit({ picture: fakeFile });
    });

    // Call onSubmit a second time while the first is still in-flight.
    await act(async () => {
      await result.current.onSubmit({ picture: fakeFile });
    });

    // saveBackgroundImage must have been called exactly once (first call only).
    expect(saveBackgroundImageMock).toHaveBeenCalledTimes(1);
  });
});
