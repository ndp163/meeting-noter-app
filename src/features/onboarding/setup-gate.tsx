import { useCallback, useEffect, useState } from "react";
import { Loader2 } from "lucide-react";
import { Button } from "@/design-system";
import { HomePage } from "@/pages/home";
import { modelsReady } from "@/services/setup";
import { Onboarding } from "./onboarding";

type GateState = "checking" | "needs-setup" | "ready" | "error";

/**
 * Gates the app behind a one-time model download. If the models are already
 * cached the home page renders immediately; otherwise the onboarding screen
 * runs the download first. A failed check offers a retry rather than assuming
 * a fresh setup (which would wrongly push an installed user into onboarding).
 */
export const SetupGate = () => {
  const [state, setState] = useState<GateState>("checking");

  const check = useCallback(() => {
    setState("checking");
    modelsReady()
      .then((ready) => setState(ready ? "ready" : "needs-setup"))
      .catch((e) => {
        console.error("Model readiness check failed", e);
        setState("error");
      });
  }, []);

  useEffect(check, [check]);

  const handleDone = useCallback(() => setState("ready"), []);

  if (state === "checking") {
    return (
      <div className="flex h-screen items-center justify-center bg-[var(--ds-bg)]">
        <Loader2 className="w-6 h-6 animate-spin text-[var(--ds-accent)]" />
      </div>
    );
  }

  if (state === "error") {
    return (
      <div className="flex h-screen flex-col items-center justify-center gap-3 bg-[var(--ds-bg)] text-center px-6">
        <p className="text-sm text-[var(--ds-text-2)]">
          Couldn&apos;t check the installed models.
        </p>
        <Button variant="ghost" pill onClick={check}>
          Try again
        </Button>
      </div>
    );
  }

  if (state === "needs-setup") return <Onboarding onDone={handleDone} />;
  return <HomePage />;
};
