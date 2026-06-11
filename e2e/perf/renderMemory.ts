import fs from "node:fs/promises";
import path from "node:path";
import type { CDPSession } from "@playwright/test";

export type MemorySample = {
  phase: "warmup" | "measure";
  elapsedMs: number;
  jsHeapUsedBytes: number;
  jsHeapTotalBytes: number;
  runtimeHeapUsedBytes: number;
  runtimeHeapTotalBytes: number;
  documents: number;
  nodes: number;
  jsEventListeners: number;
};

export type MemoryThresholds = {
  jsHeapGrowthBytes: number;
  jsHeapSlopeBytesPerMinute: number;
  domNodeGrowth: number;
  listenerGrowth: number;
};

export type MemoryGrowthAnalysis = {
  status: "ok" | "warn";
  baselineElapsedMs: number;
  finalElapsedMs: number;
  jsHeapGrowthBytes: number;
  runtimeHeapGrowthBytes: number;
  domNodeGrowth: number;
  documentGrowth: number;
  listenerGrowth: number;
  tailSlopeBytesPerMinute: number;
  reasons: string[];
};

export type MemoryReport = {
  generatedAt: string;
  scenario: string;
  config: {
    warmupMs: number;
    durationMs: number;
    sampleIntervalMs: number;
    tailWindowMs: number;
    streamIntervalMs: number;
    emittedCount?: number;
  };
  thresholds: MemoryThresholds;
  analysis: MemoryGrowthAnalysis;
  samples: MemorySample[];
};

type RuntimeHeapUsage = {
  usedSize: number;
  totalSize: number;
};

type PerformanceMetricsResult = {
  metrics: Array<{ name: string; value: number }>;
};

type DomCounters = {
  documents: number;
  nodes: number;
  jsEventListeners: number;
};

const REPORT_DIR = path.join("test-results", "render-memory");
const REPORT_PATH = path.join(REPORT_DIR, "render-memory-results.json");

export const createMemorySession = async (cdp: CDPSession) => {
  await cdp.send("Performance.enable");
};

export const collectMemorySample = async (
  cdp: CDPSession,
  phase: MemorySample["phase"],
  elapsedMs: number,
): Promise<MemorySample> => {
  await collectGarbage(cdp);
  const [runtimeHeap, performance, domCounters] = await Promise.all([
    cdp.send("Runtime.getHeapUsage") as Promise<RuntimeHeapUsage>,
    cdp.send("Performance.getMetrics") as Promise<PerformanceMetricsResult>,
    cdp.send("Memory.getDOMCounters") as Promise<DomCounters>,
  ]);
  const metrics = new Map(
    performance.metrics.map((metric) => [metric.name, metric.value]),
  );

  return {
    phase,
    elapsedMs,
    jsHeapUsedBytes: Math.round(metrics.get("JSHeapUsedSize") ?? 0),
    jsHeapTotalBytes: Math.round(metrics.get("JSHeapTotalSize") ?? 0),
    runtimeHeapUsedBytes: Math.round(runtimeHeap.usedSize),
    runtimeHeapTotalBytes: Math.round(runtimeHeap.totalSize),
    documents: domCounters.documents,
    nodes: domCounters.nodes,
    jsEventListeners: domCounters.jsEventListeners,
  };
};

export const collectMemorySamples = async (
  cdp: CDPSession,
  config: Pick<
    MemoryReport["config"],
    "warmupMs" | "durationMs" | "sampleIntervalMs"
  >,
): Promise<MemorySample[]> => {
  const samples: MemorySample[] = [];
  const startedAt = Date.now();
  let nextElapsedMs = 0;

  while (nextElapsedMs <= config.durationMs) {
    const elapsedMs = Date.now() - startedAt;
    samples.push(
      await collectMemorySample(
        cdp,
        elapsedMs < config.warmupMs ? "warmup" : "measure",
        elapsedMs,
      ),
    );

    nextElapsedMs += config.sampleIntervalMs;
    const waitMs = Math.min(
      config.sampleIntervalMs,
      Math.max(0, config.durationMs - (Date.now() - startedAt)),
    );
    if (waitMs === 0) break;
    await sleep(waitMs);
  }

  const lastSample = samples.at(-1);
  const elapsedMs = Date.now() - startedAt;
  if (!lastSample || lastSample.elapsedMs < config.durationMs) {
    samples.push(
      await collectMemorySample(
        cdp,
        "measure",
        Math.max(elapsedMs, config.warmupMs),
      ),
    );
  }

  return samples;
};

export const analyzeMemoryGrowth = (
  samples: MemorySample[],
  thresholds: MemoryThresholds,
  tailWindowMs: number,
): MemoryGrowthAnalysis => {
  if (samples.length < 2) {
    throw new Error("At least two memory samples are required for analysis.");
  }

  const baseline = samples.find((sample) => sample.phase === "measure");
  if (!baseline) {
    throw new Error("At least one post-warmup memory sample is required.");
  }

  const finalSample = samples[samples.length - 1];
  const tailStartMs = finalSample.elapsedMs - tailWindowMs;
  const tailSamples = samples.filter(
    (sample) => sample.phase === "measure" && sample.elapsedMs >= tailStartMs,
  );
  const tailSlopeBytesPerMinute =
    tailSamples.length >= 2 ? slopeBytesPerMinute(tailSamples) : 0;

  const analysis = {
    status: "ok" as const,
    baselineElapsedMs: baseline.elapsedMs,
    finalElapsedMs: finalSample.elapsedMs,
    jsHeapGrowthBytes: finalSample.jsHeapUsedBytes - baseline.jsHeapUsedBytes,
    runtimeHeapGrowthBytes:
      finalSample.runtimeHeapUsedBytes - baseline.runtimeHeapUsedBytes,
    domNodeGrowth: finalSample.nodes - baseline.nodes,
    documentGrowth: finalSample.documents - baseline.documents,
    listenerGrowth: finalSample.jsEventListeners - baseline.jsEventListeners,
    tailSlopeBytesPerMinute,
    reasons: [] as string[],
  };

  if (
    analysis.jsHeapGrowthBytes > thresholds.jsHeapGrowthBytes &&
    analysis.tailSlopeBytesPerMinute > thresholds.jsHeapSlopeBytesPerMinute
  ) {
    analysis.reasons.push(
      "JS heap grew past the threshold and kept rising in the tail window.",
    );
  }

  if (analysis.domNodeGrowth > thresholds.domNodeGrowth) {
    analysis.reasons.push("DOM node count grew past the threshold.");
  }

  if (analysis.listenerGrowth > thresholds.listenerGrowth) {
    analysis.reasons.push(
      "JavaScript event listener count grew past the threshold.",
    );
  }

  return {
    ...analysis,
    status: analysis.reasons.length > 0 ? "warn" : "ok",
  };
};

export const writeMemoryReport = async (report: MemoryReport) => {
  await fs.mkdir(REPORT_DIR, { recursive: true });
  await fs.writeFile(REPORT_PATH, `${JSON.stringify(report, null, 2)}\n`);
};

const slopeBytesPerMinute = (samples: MemorySample[]) => {
  const firstElapsedMs = samples[0].elapsedMs;
  const points = samples.map((sample) => ({
    x: (sample.elapsedMs - firstElapsedMs) / 60_000,
    y: sample.jsHeapUsedBytes,
  }));
  const meanX = average(points.map((point) => point.x));
  const meanY = average(points.map((point) => point.y));
  const numerator = points.reduce(
    (sum, point) => sum + (point.x - meanX) * (point.y - meanY),
    0,
  );
  const denominator = points.reduce(
    (sum, point) => sum + (point.x - meanX) ** 2,
    0,
  );

  return denominator === 0 ? 0 : Math.round(numerator / denominator);
};

const average = (values: number[]) =>
  values.reduce((sum, value) => sum + value, 0) / values.length;

const collectGarbage = async (cdp: CDPSession) => {
  try {
    await cdp.send("HeapProfiler.collectGarbage");
  } catch {
    // Best-effort only; metrics are still useful when GC collection is unavailable.
  }
};

const sleep = (ms: number) =>
  new Promise<void>((resolve) => {
    setTimeout(resolve, ms);
  });
