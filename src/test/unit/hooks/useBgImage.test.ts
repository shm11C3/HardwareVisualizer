// src/test/unit/hooks/useBgImage.test.ts
import { act, renderHook } from "@testing-library/react";
import { Provider } from "jotai";
import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";

// ----------------------
// Mock setup
// ----------------------

// Mock Tauri dialog error function
const errorMock = vi.fn();
vi.mock("@/hooks/useTauriDialog", () => ({
  useTauriDialog: () => ({ error: errorMock }),
}));

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
vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
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

import { useBackgroundImage, useBackgroundImageList } from "@/hooks/useBgImage";
import { commands } from "@/rspc/bindings";

// ----------------------
// Test body
// ----------------------
describe("useBackgroundImage", () => {
  beforeEach(() => {
    errorMock.mockClear();
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

    it("Calls error when getBackgroundImage returns error", async () => {
      settingsMock.selectedBackgroundImg = "file123";
      const errorMsg = "Error loading image";
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

      expect(errorMock).toHaveBeenCalledWith(errorMsg);
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

    it("Calls error and does not update settings when saveBackgroundImage returns error", async () => {
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

      await act(async () => {
        await result.current.saveBackgroundImage(file);
      });

      expect(errorMock).toHaveBeenCalledWith(errorMsg);
      expect(updateSettingAtomMock).not.toHaveBeenCalled();
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

    it("Calls error dialog when deletion throws error", async () => {
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

      expect(errorMock).toHaveBeenCalledWith(errorMsg);
    });
  });
});

describe("useBackgroundImageList", () => {
  beforeEach(() => {
    errorMock.mockClear();
  });

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

  it("initBackgroundImages: Calls error when getBackgroundImages returns error", async () => {
    const errorMsg = "Failed to load images";
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

    expect(errorMock).toHaveBeenCalledWith(errorMsg);
    expect(result.current.backgroundImageList).toEqual([]);
  });
});
