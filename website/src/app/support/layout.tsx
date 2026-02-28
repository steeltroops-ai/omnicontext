import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Support | OmniContext",
  description:
    "Get help with OmniContext. Report bugs, join community discussions, read documentation, or contact enterprise support.",
};

export default function SupportLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return <>{children}</>;
}
