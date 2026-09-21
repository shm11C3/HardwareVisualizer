// Runs detached from the "Start CI telemetry" step for the rest of the job.
// The monitor must not become the workload (AGENTS.md), so every tick stays
// synchronous and allocation-light: one os.cpus() scan, one appendFileSync.

const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");

const samplesFile = process.argv[2];
const intervalMs = Number(process.argv[3]) || 5000;

// GitHub's hosted-job time limit is 6h. Self-terminating at that ceiling
// means a sampler whose post step never runs (runner killed, job hard-
// cancelled) cannot outlive the runner as an orphan process.
const MAX_LIFETIME_MS = 6 * 60 * 60 * 1000;
const deadline = Date.now() + MAX_LIFETIME_MS;

function readVmStatPages(output, label) {
  const match = new RegExp(`${label}:\\s+(\\d+)\\.`).exec(output);
  if (!match) throw new Error(`vm_stat missing "${label}"`);
  return Number(match[1]);
}

// os.freemem() on macOS only counts pages on the free list, not pages the
// kernel could reclaim instantly (inactive/purgeable), so it makes the
// machine look permanently almost full. vm_stat's active + wired +
// compressor pages is what Activity Monitor calls "Memory Used".
function readDarwinMemUsedBytes() {
  const output = execFileSync("vm_stat", { encoding: "utf8", timeout: 2000 });
  const pageSizeMatch = /page size of (\d+) bytes/.exec(output);
  const pageSize = pageSizeMatch ? Number(pageSizeMatch[1]) : 4096;
  const active = readVmStatPages(output, "Pages active");
  const wired = readVmStatPages(output, "Pages wired down");
  const compressed = readVmStatPages(output, "Pages occupied by compressor");
  return (active + wired + compressed) * pageSize;
}

function readLinuxMemUsedBytes() {
  const text = fs.readFileSync("/proc/meminfo", "utf8");
  const total = /MemTotal:\s+(\d+)/.exec(text);
  const available = /MemAvailable:\s+(\d+)/.exec(text);
  if (!total || !available) throw new Error("/proc/meminfo missing fields");
  return (Number(total[1]) - Number(available[1])) * 1024;
}

function readMemUsedBytes() {
  try {
    if (process.platform === "linux") return readLinuxMemUsedBytes();
    if (process.platform === "darwin") return readDarwinMemUsedBytes();
    return os.totalmem() - os.freemem();
  } catch {
    // win32 and any platform-specific read failure: the generic estimate is
    // still better than reporting no memory data at all.
    return os.totalmem() - os.freemem();
  }
}

function readDisk() {
  try {
    const target = process.env.GITHUB_WORKSPACE || process.cwd();
    const stats = fs.statfsSync(target);
    return {
      disk_total: stats.blocks * stats.bsize,
      disk_used: (stats.blocks - stats.bfree) * stats.bsize,
    };
  } catch {
    return { disk_total: null, disk_used: null };
  }
}

function tick() {
  try {
    let idle = 0;
    let total = 0;
    for (const cpu of os.cpus()) {
      const times = cpu.times;
      idle += times.idle;
      total += times.user + times.nice + times.sys + times.idle + times.irq;
    }

    const disk = readDisk();
    const sample = {
      t: Date.now(),
      cpu_idle: idle,
      cpu_total: total,
      mem_used: readMemUsedBytes(),
      mem_total: os.totalmem(),
      disk_used: disk.disk_used,
      disk_total: disk.disk_total,
    };
    fs.appendFileSync(samplesFile, `${JSON.stringify(sample)}\n`);
  } catch {
    // A single failed tick must not kill the sampling loop.
  }
}

tick();
const timer = setInterval(() => {
  if (Date.now() >= deadline) {
    clearInterval(timer);
    process.exit(0);
  }
  tick();
}, intervalMs);
