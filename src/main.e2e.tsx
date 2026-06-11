import React from "react";
import ReactDOM from "react-dom/client";
import { installTauriMocks } from "./e2e/mocks/installTauriMocks";

const bootstrap = async () => {
  installTauriMocks();

  const { App } = await import("./App");

  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
};

bootstrap().catch((error) => {
  console.error("Failed to bootstrap the E2E app:", error);
});
