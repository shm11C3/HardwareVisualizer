export type SnapshotPeriod = {
  start: string;
  end: string;
};

export type UsageRange = {
  type: "cpu" | "memory";
  value: [number, number];
};
