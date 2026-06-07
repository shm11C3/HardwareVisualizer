import React from "react";
import ReactDOM from "react-dom/client";

const bootstrap = async () => {
  // E2E-only: install Tauri IPC/event mocks before any app module runs.
  // The condition is statically false in production builds, so the mock
  // chunk is dead-code eliminated and never shipped.
  if (import.meta.env.VITE_E2E_MOCK === "true") {
    const { installTauriMocks } = await import("./e2e/mocks/installTauriMocks");
    installTauriMocks();
  }

  const { App } = await import("./App");

  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
};

bootstrap().catch((error) => {
  console.error("Failed to bootstrap the app:", error);
});
