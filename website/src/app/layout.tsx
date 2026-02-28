import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "OmniContext | Universal Code Context Engine",
  description:
    "The foundation for AI coding agents. A highly parallel, local, and universal code context engine written in Rust.",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <head>
        <link
          href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500;600&display=swap"
          rel="stylesheet"
        />
      </head>
      <body>{children}</body>
    </html>
  );
}
