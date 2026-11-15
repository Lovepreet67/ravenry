import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { RouterProvider } from "@tanstack/react-router";
import { Toaster } from "sonner";

import { AppProviders } from "@/app/providers";
import { router } from "@/app/router";

import "@/index.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <AppProviders>
      <RouterProvider router={router} />
      <Toaster position="top-right" richColors />
    </AppProviders>
  </StrictMode>,
);