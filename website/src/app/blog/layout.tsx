import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Blog | OmniContext",
  description:
    "Engineering insights, release notes, and deep dives into the OmniContext code context engine.",
};

export default function BlogLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return <>{children}</>;
}
