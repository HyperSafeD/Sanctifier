import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";
import { ThemeProvider } from "./providers/theme-provider";
import { NavBar } from "./components/NavBar";

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
      let theme = "light";

      try {
        const stored = window.localStorage.getItem(storageKey);
        if (stored === "light" || stored === "dark") {
          theme = stored;
        } else if (window.matchMedia("(prefers-color-scheme: dark)").matches) {
          theme = "dark";
        }
      } catch (error) {
        if (window.matchMedia("(prefers-color-scheme: dark)").matches) {
          theme = "dark";
        }
      }

      root.dataset.theme = theme;
      root.classList.toggle("dark", theme === "dark");
      root.style.colorScheme = theme;
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
        <ThemeProvider>
          <NavBar />
          {children}
        </ThemeProvider>
      </body>
    </html>
  );
}
