import { act, renderHook } from "@testing-library/react";
import type React from "react";
import type { RefObject } from "react";
import { describe, expect, it } from "vitest";
import { useScatterChartZoom } from "@/features/hardware/insights/process/chart/hooks/useScatterChartZoom";

describe("useScatterChartZoom", () => {
  const data = [
    { x: 0, y: 0 },
    { x: 100, y: 100 },
  ];

  const rect = {
    left: 0,
    top: 0,
    width: 1000,
    height: 500,
    right: 1000,
    bottom: 500,
    x: 0,
    y: 0,
    toJSON: () => ({}),
  } as DOMRect;

  it("initializes with default domains and cursor", () => {
    const { result } = renderHook(() => useScatterChartZoom(data));

    expect(result.current.isZoomed).toBe(false);
    expect(result.current.xDomain).toEqual([0, "auto"]);
    expect(result.current.yDomain).toEqual([0, 100]);
    expect(result.current.cursorStyle).toEqual({ cursor: "zoom-in" });
  });

  it("zooms in on click and centers around click position", () => {
    const { result } = renderHook(() => useScatterChartZoom(data));

    // Provide a mock container size for click calculations
    (result.current.containerRef as RefObject<HTMLDivElement | null>).current =
      {
        getBoundingClientRect: () => rect,
      } as HTMLDivElement;

    // Mouse down and up without moving beyond threshold triggers zoom
    act(() => {
      result.current.handleMouseDown({
        clientX: 500,
        clientY: 250,
      } as React.MouseEvent);
      result.current.handleMouseUp({
        clientX: 500,
        clientY: 250,
      } as React.MouseEvent);
    });

    // With ZOOM_FACTOR=0.7 and data range [0,100], centered click yields [15,85]
    expect(result.current.isZoomed).toBe(true);
    expect(result.current.xDomain).toEqual([15, 85]);
    expect(result.current.yDomain).toEqual([15, 85]);
    expect(result.current.cursorStyle).toEqual({ cursor: "grab" });
  });

  it("toggles zoom off on second click", () => {
    const { result } = renderHook(() => useScatterChartZoom(data));

    (result.current.containerRef as RefObject<HTMLDivElement | null>).current =
      {
        getBoundingClientRect: () => rect,
      } as HTMLDivElement;

    // First click to zoom in
    act(() => {
      result.current.handleMouseDown({
        clientX: 500,
        clientY: 250,
      } as React.MouseEvent);
      result.current.handleMouseUp({
        clientX: 500,
        clientY: 250,
      } as React.MouseEvent);
    });

    expect(result.current.isZoomed).toBe(true);

    // Second click to reset
    act(() => {
      result.current.handleMouseDown({
        clientX: 500,
        clientY: 250,
      } as React.MouseEvent);
      result.current.handleMouseUp({
        clientX: 500,
        clientY: 250,
      } as React.MouseEvent);
    });

    expect(result.current.isZoomed).toBe(false);
    expect(result.current.xDomain).toEqual([0, "auto"]);
    expect(result.current.yDomain).toEqual([0, 100]);
    expect(result.current.cursorStyle).toEqual({ cursor: "zoom-in" });
  });

  it("pans domains on drag when zoomed (x uses pixel width)", () => {
    const { result } = renderHook(() => useScatterChartZoom(data));

    (result.current.containerRef as RefObject<HTMLDivElement | null>).current =
      {
        getBoundingClientRect: () => rect,
      } as HTMLDivElement;

    // Zoom in to enable dragging
    act(() => {
      result.current.handleMouseDown({
        clientX: 500,
        clientY: 250,
      } as React.MouseEvent);
      result.current.handleMouseUp({
        clientX: 500,
        clientY: 250,
      } as React.MouseEvent);
    });

    expect(result.current.xDomain).toEqual([15, 85]);
    expect(result.current.yDomain).toEqual([15, 85]);

    // Start dragging and move to the right by 10px (deltaX=+10)
    act(() => {
      result.current.handleMouseDown({
        clientX: 100,
        clientY: 100,
      } as React.MouseEvent);
      result.current.handleMouseMove({
        clientX: 110,
        clientY: 100,
      } as React.MouseEvent);
    });

    // X domain should shift left by ~0.7 and round to integers -> [14,84]
    expect(result.current.xDomain).toEqual([14, 84]);
    // Y domain unchanged because deltaY=0
    expect(result.current.yDomain).toEqual([15, 85]);

    // Mouse up after dragging should not toggle zoom off
    act(() => {
      result.current.handleMouseUp({
        clientX: 110,
        clientY: 100,
      } as React.MouseEvent);
    });
    expect(result.current.isZoomed).toBe(true);
  });

  it("clamps Y domain within [0,100] during drag", () => {
    const { result } = renderHook(() => useScatterChartZoom(data));

    (result.current.containerRef as RefObject<HTMLDivElement | null>).current =
      {
        getBoundingClientRect: () => rect,
      } as HTMLDivElement;

    // Zoom in to enable dragging
    act(() => {
      result.current.handleMouseDown({
        clientX: 500,
        clientY: 250,
      } as React.MouseEvent);
      result.current.handleMouseUp({
        clientX: 500,
        clientY: 250,
      } as React.MouseEvent);
    });

    // Drag upward with a large deltaY to push y domain below 0
    act(() => {
      result.current.handleMouseDown({
        clientX: 100,
        clientY: 200,
      } as React.MouseEvent);
      result.current.handleMouseMove({
        clientX: 100,
        clientY: -9800,
      } as React.MouseEvent);
      result.current.handleMouseUp({
        clientX: 100,
        clientY: -9800,
      } as React.MouseEvent);
    });

    // Y domain should clamp to [0,70] (range 70 kept from [15,85])
    expect(result.current.yDomain).toEqual([0, 70]);
  });
});
