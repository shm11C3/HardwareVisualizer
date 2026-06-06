import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { useDocumentVisibilityClass } from "@/hooks/useDocumentVisibilityClass";

const setDocumentHidden = (hidden: boolean) => {
  Object.defineProperty(document, "hidden", {
    configurable: true,
    value: hidden,
  });
};

describe("useDocumentVisibilityClass", () => {
  beforeEach(() => {
    setDocumentHidden(false);
    document.documentElement.classList.remove("hardviz-document-hidden");
  });

  afterEach(() => {
    document.documentElement.classList.remove("hardviz-document-hidden");
  });

  it("reflects document visibility on the root element", () => {
    renderHook(() => useDocumentVisibilityClass());

    expect(
      document.documentElement.classList.contains("hardviz-document-hidden"),
    ).toBe(false);

    setDocumentHidden(true);
    act(() => {
      document.dispatchEvent(new Event("visibilitychange"));
    });

    expect(
      document.documentElement.classList.contains("hardviz-document-hidden"),
    ).toBe(true);

    setDocumentHidden(false);
    act(() => {
      document.dispatchEvent(new Event("visibilitychange"));
    });

    expect(
      document.documentElement.classList.contains("hardviz-document-hidden"),
    ).toBe(false);
  });
});
