import type { NextConfig } from "next";

const isDev = process.env.NODE_ENV === "development";
const cspReportOnly = process.env.CSP_REPORT_ONLY === "1";

const nextConfig: NextConfig = {
  reactCompiler: true,
  turbopack: {
    root: __dirname,
  },
  async headers() {
    const cspDirectives = [
      "default-src 'self'",
      // Allow inline styles for Tailwind v4 in dev mode only
      isDev ? "style-src 'self' 'unsafe-inline'" : "style-src 'self'",
      "script-src 'self'",
      "img-src 'self' data: https:",
      "font-src 'self' data:",
      "connect-src 'self'",
      "frame-ancestors 'none'",
      "base-uri 'self'",
      "form-action 'self'",
    ].join("; ");

    const cspHeader = cspReportOnly
      ? "Content-Security-Policy-Report-Only"
      : "Content-Security-Policy";

    return [
      {
        source: "/:path*",
        headers: [
          {
            key: cspHeader,
            value: cspDirectives,
          },
          {
            key: "Strict-Transport-Security",
            value: "max-age=31536000; includeSubDomains",
          },
          {
            key: "X-Frame-Options",
            value: "DENY",
          },
          {
            key: "X-Content-Type-Options",
            value: "nosniff",
          },
          {
            key: "Referrer-Policy",
            value: "strict-origin-when-cross-origin",
          },
          {
            key: "Permissions-Policy",
            value: "camera=(), microphone=(), geolocation=()",
          },
        ],
      },
    ];
  },
};

export default nextConfig;


