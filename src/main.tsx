import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";

// TEMP DIAGNOSTIC for #1960 — remove before commit.
// Beacons page-visibility / event-arrival / rAF activity to a local collector
// every 5s, and closes the window after 45s to exercise the real
// close-to-tray hide path (requires HARDVIZ_CLOSE_TO_BACKGROUND=1).
(async () => {
  const { listen } = await import("@tauri-apps/api/event");
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  if (getCurrentWindow().label !== "main") return;
  let evCount = 0;
  let rafCount = 0;
  let timerCount = 0;
  await listen("hardware-monitor-update", () => {
    evCount++;
  });
  const rafTick = () => {
    rafCount++;
    requestAnimationFrame(rafTick);
  };
  requestAnimationFrame(rafTick);
  setInterval(() => {
    timerCount++;
  }, 1000);
  setInterval(() => {
    void fetch("http://127.0.0.1:9876/beacon", {
      method: "POST",
      body: JSON.stringify({
        t: new Date().toISOString().slice(11, 19),
        vis: document.visibilityState,
        ev: evCount,
        raf: rafCount,
        timer: timerCount,
      }),
    }).catch(() => {});
    evCount = 0;
    rafCount = 0;
    timerCount = 0;
  }, 5000);
  setTimeout(() => {
    void getCurrentWindow().close();
  }, 45000);
})();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
