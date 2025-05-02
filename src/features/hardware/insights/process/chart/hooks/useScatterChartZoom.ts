import { useCallback, useMemo, useRef, useState } from "react";

/** クリックとドラッグを判別するためのピクセル距離の閾値 */
const CLICK_THRESHOLD = 5;

/** ズーム係数 */
const ZOOM_FACTOR = 0.7;

export const useScatterChartZoom = (data: { x: number; y: number }[]) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const [isZoomed, setIsZoomed] = useState(false);
  const [xDomain, _setXDomain] = useState<[number, number] | [0, "auto"]>([
    0,
    "auto",
  ]);
  const [yDomain, _setYDomain] = useState<[number, number]>([0, 100]);
  const dragStart = useRef<{ x: number; y: number } | null>(null);
  const [isDragging, setIsDragging] = useState(false);
  const moved = useRef(false);

  const setXDomain = useCallback((domain: [number, number] | [0, "auto"]) => {
    if (domain[1] === "auto") {
      _setXDomain([0, "auto"]);
      return;
    }
    const rawStart = Math.round(domain[0]);
    const rawEnd = Math.round(domain[1]);

    const originalWidth = Math.max(1, rawEnd - rawStart); // 移動前の幅を記録

    const start = Math.max(0, rawStart);
    const end = start + originalWidth; // 常に固定幅を維持

    _setXDomain([start, end]);
  }, []);

  const setYDomain = useCallback((domain: [number, number]) => {
    const rawStart = Math.round(domain[0]);
    const rawEnd = Math.round(domain[1]);

    const range = rawEnd - rawStart;
    const maxStart = 100 - range;
    const start = Math.min(Math.max(0, rawStart), maxStart); // 0以上かつ上限を超えない
    const end = start + range;

    _setYDomain([start, end]);
  }, []);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    dragStart.current = { x: e.clientX, y: e.clientY };
    moved.current = false;
    setIsDragging(true);
  }, []);

  const handleMouseMove = useCallback(
    (e: React.MouseEvent) => {
      if (!isZoomed || !dragStart.current || xDomain[1] === "auto") return;

      const deltaX = e.clientX - dragStart.current.x;
      const deltaY = e.clientY - dragStart.current.y;

      if (
        Math.abs(deltaX) > CLICK_THRESHOLD ||
        Math.abs(deltaY) > CLICK_THRESHOLD
      ) {
        moved.current = true;

        const pixelWidth = containerRef.current?.getBoundingClientRect().width;

        setXDomain(calcDomain(deltaX, xDomain, "x", pixelWidth));
        setYDomain(calcDomain(deltaY, yDomain, "y"));

        dragStart.current = { x: e.clientX, y: e.clientY };
      }
    },
    [isZoomed, xDomain, yDomain, setXDomain, setYDomain],
  );

  const handleMouseUp = useCallback(
    (e: React.MouseEvent) => {
      setIsDragging(false);

      // ドラッグが発生した場合はズームを無効にする
      if (!dragStart.current || moved.current) {
        dragStart.current = null;
        moved.current = false;
        return;
      }

      if (!containerRef.current) return;

      // ズーム処理
      const rect = containerRef.current.getBoundingClientRect();
      const clickXRatio = (e.clientX - rect.left) / rect.width;
      const clickYRatio = 1 - (e.clientY - rect.top) / rect.height;

      const xMin = Math.min(...data.map((d) => d.x));
      const xMax = Math.max(...data.map((d) => d.x));
      const yMin = Math.min(...data.map((d) => d.y));
      const yMax = Math.max(...data.map((d) => d.y));

      const clickX = xMin + clickXRatio * (xMax - xMin);
      const clickY = yMin + clickYRatio * (yMax - yMin);

      if (!isZoomed) {
        setXDomain([
          clickX - ((xMax - xMin) * ZOOM_FACTOR) / 2,
          clickX + ((xMax - xMin) * ZOOM_FACTOR) / 2,
        ]);
        setYDomain([
          clickY - ((yMax - yMin) * ZOOM_FACTOR) / 2,
          clickY + ((yMax - yMin) * ZOOM_FACTOR) / 2,
        ]);
        setIsZoomed(true);
      } else {
        setXDomain([0, "auto"]);
        setYDomain([0, 100]);
        setIsZoomed(false);
      }

      dragStart.current = null;
    },
    [data, isZoomed, setXDomain, setYDomain],
  );

  const cursorStyle = useMemo(() => {
    if (isZoomed) {
      return {
        cursor: isDragging ? "grabbing" : "grab",
      };
    }

    return {
      cursor: "zoom-in",
    };
  }, [isZoomed, isDragging]);

  return {
    containerRef,
    xDomain,
    yDomain,
    handleMouseDown,
    handleMouseMove,
    handleMouseUp,
    cursorStyle,
    isZoomed,
  };
};

const calcDomain = (
  delta: number,
  currentDomain: [number, number],
  direction: "x" | "y",
  pixelWidth?: number,
): [number, number] => {
  const scale = currentDomain[1] - currentDomain[0];

  if (pixelWidth) {
    return [
      currentDomain[0] -
        (delta / pixelWidth) * scale * (direction === "x" ? 1 : -1),
      currentDomain[1] -
        (delta / pixelWidth) * scale * (direction === "x" ? 1 : -1),
    ];
  }

  const deltaScale = direction === "x" ? 0.0005 : 0.001;

  return [
    currentDomain[0] -
      delta * deltaScale * scale * (direction === "x" ? 1 : -1),
    currentDomain[1] -
      delta * deltaScale * scale * (direction === "x" ? 1 : -1),
  ];
};
