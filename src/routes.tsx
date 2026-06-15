import { createBrowserRouter } from "react-router";
import { SetupGate } from "./features/onboarding/setup-gate";

export const router = createBrowserRouter([
  {
    index: true,
    element: <SetupGate />,
  },
]);
