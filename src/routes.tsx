import { createBrowserRouter } from "react-router";
import { HomePage } from "./pages/home";
import { TeamsPage } from "./pages/teams";

export const router = createBrowserRouter([
  {
    index: true,
    element: <HomePage />,
  },
  {
    path: "/teams",
    element: <TeamsPage />,
  },
]);
