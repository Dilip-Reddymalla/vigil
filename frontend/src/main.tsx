import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "./App";
import { ThemeProvider } from "@/lib/theme";
import { DateProvider } from "@/lib/date";
import "./styles/globals.css";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { staleTime: 30_000 },
  },
});

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ThemeProvider>
      <DateProvider>
        <QueryClientProvider client={queryClient}>
          <App />
        </QueryClientProvider>
      </DateProvider>
    </ThemeProvider>
  </React.StrictMode>,
);
