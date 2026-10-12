// src/hooks/appearance/useBgImage.test.ts
import { act, renderHook, waitFor } from "@testing-library/react";
import { Provider } from "jotai";
import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";

// ----------------------
// Mock setup
// ----------------------

// Failures are state for the control that owns them, never an OS dialog.
vi.mock("@/hooks/tauri/useTauriDialog", () => {
  throw new Error("useBgImage must not open a dialog");
});

// Manage settings state and update function with global variables for useSettingsAtom mock
let settingsMock: { selectedBackgroundImg: string | null } = {
  selectedBackgroundImg: null,
};
let updateSettingAtomMock = vi.fn();

// Reset state for each test
beforeEach(() => {
  vi.clearAllMocks();
  settingsMock = { selectedBackgroundImg: null };
  updateSettingAtomMock = vi.fn();
});

// Mock useSettingsAtom (references global variables that can be changed per test)
vi.mock("@/hooks/settings/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: settingsMock,
    updateSettingAtom: updateSettingAtomMock,
  }),
}));

// Mock file conversion function (returns vi.fn() directly without depending on top-level variables)
vi.mock("@/lib/file", () => ({
  convertFileToBase64: vi.fn(),
}));

// Mock command groups
vi.mock("@/rspc/bindings", () => ({
  commands: {
    getBackgroundImage: vi.fn(),
    saveBackgroundImage: vi.fn(),
    deleteBackgroundImage: vi.fn(),
    getBackgroundImages: vi.fn(),
  },
}));

// ----------------------
// Import target modules and use vi.mocked to utilize mock functions
// ----------------------
import { convertFileToBase64 } from "@/lib/file";

const convertFileToBase64Mock = vi.mocked(convertFileToBase64);

import {
  useBackgroundImage,
  useBackgroundImageList,
} from "@/hooks/appearance/useBgImage";
import { commands } from "@/rspc/bindings";

// ----------------------
// Test body
// ----------------------
describe("useBackgroundImage", () => {
  beforeEach(() => {
    updateSettingAtomMock.mockClear();
    convertFileToBase64Mock.mockClear();
    (commands.getBackgroundImages as Mock).mockResolvedValue({ data: [] });
  });

  describe("initBackgroundImage", () => {
    it("When settings.selectedBackgroundImg exists and getBackgroundImage returns ok, sets backgroundImage", async () => {
      // When background image ID is set in settings
      settingsMock.selectedBackgroundImg = "file123";
      // getBackgroundImage returns success result
      (commands.getBackgroundImage as Mock).mockResolvedValue({
        status: "ok",
        data: "base64data",
      });

      const { result } = renderHook(() => useBackgroundImage(), {
        wrapper: Provider,
      });

      await act(async () => {
        await result.current.initBackgroundImage();
      });

      expect(result.current.backgroundImage).toEqual(
        "data:image/png;base64,base64data",
      );
    });

    it("Only logs when getBackgroundImage returns error (automatic read)", async () => {
      settingsMock.selectedBackgroundImg = "file123";
      const errorMsg = "Error loading image";
      const consoleError = vi
        .spyOn(console, "error")
        .mockImplementation(() => {});
      (commands.getBackgroundImage as Mock).mockResolvedValue({
        status: "error",
        error: errorMsg,
      });

      const { result } = renderHook(() => useBackgroundImage(), {
        wrapper: Provider,
      });

      await act(async () => {
        await result.current.initBackgroundImage();
      });

      expect(result.current.failures.size).toBe(0);
      expect(consoleError).toHaveBeenCalledWith(
        "Failed to load background image:",
        errorMsg,
      );
      consoleError.mockRestore();
      // Background image is not set (null state)
      expect(result.current.backgroundImage).toBeNull();
    });

    it("Sets backgroundImage to null when selectedBackgroundImg is not set", async () => {
      settingsMock.selectedBackgroundImg = null;

      const { result } = renderHook(() => useBackgroundImage(), {
        wrapper: Provider,
      });

      await act(async () => {
        await result.current.initBackgroundImage();
      });

      expect(result.current.backgroundImage).toBeNull();
    });
  });

  describe("saveBackgroundImage", () => {
    it("Settings are updated after successful save", async () => {
      // Generate dummy file object (in test environment where File constructor is available)
      const file = new File(["dummy content"], "dummy.png", {
        type: "image/png",
      });

      // convertFileToBase64 returns base64 string
      convertFileToBase64Mock.mockResolvedValue("base64image");

      // saveBackgroundImage returns success result
      (commands.saveBackgroundImage as Mock).mockResolvedValue({
        status: "ok",
        data: "newFileId",
      });

      // Since useBackgroundImage uses initBackgroundImages from useBackgroundImageList internally,
      // spy on whether that function was called
      const { result } = renderHook(() => useBackgroundImage(), {
        wrapper: Provider,
      });

      await act(async () => {
        await result.current.saveBackgroundImage(file);
      });

      // Verify that updateSettingAtom is called to set new background image ID
      expect(updateSettingAtomMock).toHaveBeenCalledWith(
        "selectedBackgroundImg",
        "newFileId",
      );
    });

    it("Records an upload failure, reports false and does not update settings when saveBackgroundImage returns error", async () => {
      const file = new File(["dummy content"], "dummy.png", {
        type: "image/png",
      });

      convertFileToBase64Mock.mockResolvedValue("base64image");

      const errorMsg = "Save failed";
      (commands.saveBackgroundImage as Mock).mockResolvedValue({
        status: "error",
        error: errorMsg,
      });

      const { result } = renderHook(() => useBackgroundImage(), {
        wrapper: Provider,
      });

      let saved = true;
      await act(async () => {
        saved = await result.current.saveBackgroundImage(file);
      });

      expect(saved).toBe(false);
      expect([...result.current.failures]).toEqual(["upload"]);
      expect(updateSettingAtomMock).not.toHaveBeenCalled();
    });

    it("Reports a rejected file conversion as an upload failure instead of throwing", async () => {
      const file = new File(["dummy content"], "dummy.png", {
        type: "image/png",
      });
      convertFileToBase64Mock.mockRejectedValue(new Error("unreadable"));

      const { result } = renderHook(() => useBackgroundImage(), {
        wrapper: Provider,
      });

      let saved = true;
      await act(async () => {
        saved = await result.current.saveBackgroundImage(file);
      });

      expect(saved).toBe(false);
      expect([...result.current.failures]).toEqual(["upload"]);
      expect(commands.saveBackgroundImage).not.toHaveBeenCalled();
    });

    it("A later successful save clears the upload failure", async () => {
      const file = new File(["dummy content"], "dummy.png", {
        type: "image/png",
      });
      convertFileToBase64Mock.mockResolvedValue("base64image");
      (commands.saveBackgroundImage as Mock)
        .mockResolvedValueOnce({ status: "error", error: "Save failed" })
        .mockResolvedValueOnce({ status: "ok", data: "newFileId" });

      const { result } = renderHook(() => useBackgroundImage(), {
        wrapper: Provider,
      });

      await act(async () => {
        await result.current.saveBackgroundImage(file);
      });
      expect([...result.current.failures]).toEqual(["upload"]);

      let saved = false;
      await act(async () => {
        saved = await result.current.saveBackgroundImage(file);
      });
      expect(saved).toBe(true);
      expect(result.current.failures.size).toBe(0);
    });
  });

  describe("deleteBackgroundImage", () => {
    it("Verify selected background image is deleted and settings are reset", async () => {
      // Set state where deletion target is selected in settings
      settingsMock.selectedBackgroundImg = "fileToDelete";

      // Set initial background image list
      const initialList = [
        { fileId: "fileToDelete", imageData: "data:image/png;base64,oldData" },
        { fileId: "otherFile", imageData: "data:image/png;base64,otherData" },
      ];

      // Render useBackgroundImageList hook separately to manipulate list
      const { result: listResult } = renderHook(
        () => useBackgroundImageList(),
        { wrapper: Provider },
      );
      act(() => {
        listResult.current.setBackgroundImageList(initialList);
      });

      // Set deleteBackgroundImage to succeed
      (commands.deleteBackgroundImage as Mock).mockResolvedValue({});

      const { result } = renderHook(() => useBackgroundImage(), {
        wrapper: Provider,
      });

      await act(async () => {
        await result.current.deleteBackgroundImage("fileToDelete");
      });

      // Since it was selected, updateSettingAtom sets null
      expect(updateSettingAtomMock).toHaveBeenCalledWith(
        "selectedBackgroundImg",
        null,
      );
    });

    it("Records a delete failure and keeps the list when deletion throws", async () => {
      // Initial background image list
      const initialList = [
        { fileId: "fileToDelete", imageData: "data:image/png;base64,oldData" },
      ];

      const { result: listResult } = renderHook(
        () => useBackgroundImageList(),
        { wrapper: Provider },
      );
      act(() => {
        listResult.current.setBackgroundImageList(initialList);
      });

      const errorMsg = "Deletion failed";
      // Set deleteBackgroundImage to throw exception
      (commands.deleteBackgroundImage as Mock).mockRejectedValue(errorMsg);

      const { result } = renderHook(() => useBackgroundImage(), {
        wrapper: Provider,
      });

      await act(async () => {
        await result.current.deleteBackgroundImage("fileToDelete");
      });

      expect([...result.current.failures]).toEqual(["delete"]);
    });

    it("Records a delete failure and keeps the image when the command returns an error", async () => {
      (commands.getBackgroundImages as Mock).mockResolvedValue({
        status: "ok",
        data: [{ fileId: "keep", imageData: "dataA" }],
      });
      (commands.deleteBackgroundImage as Mock).mockResolvedValue({
        status: "error",
        error: "in use",
      });

      const { result } = renderHook(
        () => ({
          bg: useBackgroundImage(),
          list: useBackgroundImageList(),
        }),
        { wrapper: Provider },
      );
      await waitFor(() =>
        expect(result.current.list.backgroundImageList).toHaveLength(1),
      );

      await act(async () => {
        await result.current.bg.deleteBackgroundImage("keep");
      });

      expect([...result.current.bg.failures]).toEqual(["delete"]);
      expect(result.current.list.backgroundImageList).toHaveLength(1);
    });

    it("Keeps the image selected when the delete fails, and deselects only after it succeeds", async () => {
      settingsMock.selectedBackgroundImg = "selected";
      (commands.deleteBackgroundImage as Mock)
        .mockResolvedValueOnce({ status: "error", error: "in use" })
        .mockResolvedValueOnce({ status: "ok", data: null });

      const { result } = renderHook(() => useBackgroundImage(), {
        wrapper: Provider,
      });

      await act(async () => {
        await result.current.deleteBackgroundImage("selected");
      });
      expect(updateSettingAtomMock).not.toHaveBeenCalled();

      await act(async () => {
        await result.current.deleteBackgroundImage("selected");
      });
      expect(updateSettingAtomMock).toHaveBeenCalledWith(
        "selectedBackgroundImg",
        null,
      );
    });

    it("Holds delete and upload failures independently", async () => {
      const file = new File(["dummy content"], "dummy.png", {
        type: "image/png",
      });
      convertFileToBase64Mock.mockResolvedValue("base64image");
      (commands.deleteBackgroundImage as Mock).mockResolvedValue({
        status: "error",
        error: "in use",
      });
      (commands.saveBackgroundImage as Mock)
        .mockResolvedValueOnce({ status: "error", error: "disk full" })
        .mockResolvedValueOnce({ status: "ok", data: "newFileId" });

      const { result } = renderHook(() => useBackgroundImage(), {
        wrapper: Provider,
      });

      await act(async () => {
        await result.current.deleteBackgroundImage("any");
      });
      await act(async () => {
        await result.current.saveBackgroundImage(file);
      });
      // The upload failure does not hide the pending delete failure.
      expect([...result.current.failures].sort()).toEqual(["delete", "upload"]);

      await act(async () => {
        await result.current.saveBackgroundImage(file);
      });
      // A successful upload clears only its own action.
      expect([...result.current.failures]).toEqual(["delete"]);
    });

    it("Removes deleted file from backgroundImageList via filter callback", async () => {
      // Populate the list via the hook's own auto-init effect so the filter
      // callback `(v) => v.fileId !== fileId` is invoked during deletion.
      (commands.getBackgroundImages as Mock).mockResolvedValue({
        status: "ok",
        data: [
          { fileId: "keep", imageData: "dataA" },
          { fileId: "remove", imageData: "dataB" },
        ],
      });
      (commands.deleteBackgroundImage as Mock).mockResolvedValue({});

      // Render both hooks under the same Provider so they share the same atom store.
      const { result } = renderHook(
        () => ({
          bg: useBackgroundImage(),
          list: useBackgroundImageList(),
        }),
        { wrapper: Provider },
      );

      // Wait for the auto-init effect to populate the list (2 items)
      await act(async () => {
        await waitFor(
          () => result.current.list.backgroundImageList.length === 2,
        );
      });

      // Delete one item — exercises the filter callback at line 68 of useBgImage.ts
      await act(async () => {
        await result.current.bg.deleteBackgroundImage("remove");
      });

      expect(result.current.list.backgroundImageList).toEqual([
        { fileId: "keep", imageData: "data:image/png;base64,dataA" },
      ]);
    });
  });
});

describe("useBackgroundImageList", () => {
  it("initBackgroundImages: Updates backgroundImageList when getBackgroundImages returns success", async () => {
    // getBackgroundImages returns multiple image data
    const images = [
      { fileId: "img1", imageData: "imgData1" },
      { fileId: "img2", imageData: "imgData2" },
    ];
    (commands.getBackgroundImages as Mock).mockResolvedValue({
      status: "ok",
      data: images,
    });

    const { result } = renderHook(() => useBackgroundImageList(), {
      wrapper: Provider,
    });

    await act(async () => {
      await result.current.initBackgroundImages();
    });

    // Verify that each returned image's imageData has prefix added
    expect(result.current.backgroundImageList).toEqual([
      { fileId: "img1", imageData: "data:image/png;base64,imgData1" },
      { fileId: "img2", imageData: "data:image/png;base64,imgData2" },
    ]);
  });

  it("initBackgroundImages: Only logs when getBackgroundImages returns error (automatic read)", async () => {
    const errorMsg = "Failed to load images";
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    (commands.getBackgroundImages as Mock).mockResolvedValue({
      status: "error",
      error: errorMsg,
    });

    const { result } = renderHook(() => useBackgroundImageList(), {
      wrapper: Provider,
    });
    // Set initial state to empty list
    act(() => {
      result.current.setBackgroundImageList([]);
    });

    await act(async () => {
      await result.current.initBackgroundImages();
    });

    expect(consoleError).toHaveBeenCalledWith(
      "Failed to load background images:",
      errorMsg,
    );
    consoleError.mockRestore();
    expect(result.current.backgroundImageList).toEqual([]);
  });
});
