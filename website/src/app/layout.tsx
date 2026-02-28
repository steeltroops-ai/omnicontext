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
    // Defaulting to dark mode to match user preference/design screenshots
    <html lang="en" className="dark">
      <body className="antialiased min-h-screen bg-background text-foreground flex flex-col font-sans">
        {children}
      </body>
    </html>
  );
}
