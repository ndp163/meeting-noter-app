import { useCallback, useEffect, useState } from "react";
import { HomePage } from "@/pages/home";
import { modelsReady } from "@/services/setup";
import { Onboarding } from "./onboarding";

type GateState = "checking" | "needs-setup" | "ready";

/**
 * Gates the app behind a one-time model download. If the models are already
 * cached the home page renders immediately; otherwise the onboarding screen
 * runs the download first.
 */
export const SetupGate = () => {
  const [state, setState] = useState<GateState>("checking");

  useEffect(() => {
    modelsReady()
      .then((ready) => setState(ready ? "ready" : "needs-setup"))
      .catch(() => setState("needs-setup"));
  }, []);

  const handleDone = useCallback(() => setState("ready"), []);

  if (state === "checking") return null;
  if (state === "needs-setup") return <Onboarding onDone={handleDone} />;
  return <HomePage />;
};
