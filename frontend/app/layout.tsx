import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";
import { ThemeProvider } from "./providers/theme-provider";
import { WorkspaceProvider } from "./providers/WorkspaceProvider";
import { ToastProvider } from "./providers/ToastProvider";
import { NavBar } from "./components/NavBar";
import { CommandPalette } from "./components/CommandPalette";
import { KeyboardShortcuts } from "./components/KeyboardShortcuts";
import { ErrorBoundary } from "./components/ErrorBoundary";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "Sanctifier | Security Dashboard",
  description: "Visualize Soroban security analysis results",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  const themeBootstrapScript = `
    (() => {
      const storageKey = "theme";
      const root = document.documentElement;
      let preference = "system";

      try {
        const stored = window.localStorage.getItem(storageKey);
        if (
          stored === "light" ||
          stored === "dark" ||
          stored === "system" ||
          stored === "high-contrast"
        ) {
          preference = stored;
        }
      } catch {
        preference = "system";
      }

      const resolved =
        preference === "system"
          ? (window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light")
          : preference;

      root.dataset.theme = resolved;
      root.classList.toggle("dark", resolved === "dark");
      root.classList.toggle("theme-high-contrast", resolved === "high-contrast");
      root.style.colorScheme = resolved === "high-contrast" ? "dark" : resolved;
    })();
  `;

  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <script dangerouslySetInnerHTML={{ __html: themeBootstrapScript }} />
      </head>
      <body
        className={`${geistSans.variable} ${geistMono.variable} antialiased`}
      >
        <ErrorBoundary>
          <ThemeProvider>
            <ToastProvider>
              <WorkspaceProvider>
                <NavBar />
                <CommandPalette />
                <KeyboardShortcuts />
                {children}
              </WorkspaceProvider>
            </ToastProvider>
          </ThemeProvider>
        </ErrorBoundary>
      </body>
    </html>
  );
}
