import { createBrowserRouter } from "react-router";
import { HomePage } from "./pages/home";

export const router = createBrowserRouter([
  {
    index: true,
    element: <HomePage />,
  },
]);
