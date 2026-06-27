import ReactDOM from "react-dom/client";
import { RouterProvider } from "react-router/dom";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { router } from "./routes";
import { AlertWindow } from "./features/alert/alert-window";
import { UpdateBanner } from "./features/update/update-banner";
import "./styles/App.css";
import "@/design-system/styles.css";

const isAlertWindow = getCurrentWindow().label === "alert";

if (isAlertWindow) {
  // The overlay is a transparent, borderless window; let its rounded card
  // float over whatever is behind it instead of the app's white background.
  document.documentElement.style.background = "transparent";
  document.body.style.background = "transparent";
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  isAlertWindow ? (
    <AlertWindow />
  ) : (
    <div
      className="ds-root ds-theme-vintage"
      style={{ minHeight: "100vh", background: "var(--ds-bg)" }}
    >
      <UpdateBanner />
      <RouterProvider router={router} />
    </div>
  ),
);
