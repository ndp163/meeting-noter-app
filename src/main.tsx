import ReactDOM from "react-dom/client";
import { RouterProvider } from "react-router/dom";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { router } from "./routes";
import { AlertWindow } from "./features/alert/alert-window";
import { CaptionWindow } from "./features/caption/caption-window";
import { UpdateBanner } from "./features/update/update-banner";
import "./styles/App.css";
import "@/design-system/styles.css";

const windowLabel = getCurrentWindow().label;
const isAlertWindow = windowLabel === "alert";
const isCaptionWindow = windowLabel === "caption";

if (isAlertWindow || isCaptionWindow) {
  // These overlays are transparent, borderless windows; let their card float
  // over whatever is behind them instead of the app's background.
  document.documentElement.style.background = "transparent";
  document.body.style.background = "transparent";
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  isAlertWindow ? (
    <AlertWindow />
  ) : isCaptionWindow ? (
    <CaptionWindow />
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
