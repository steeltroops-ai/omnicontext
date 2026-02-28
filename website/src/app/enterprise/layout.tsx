import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Enterprise | OmniContext",
  description:
    "Deploy OmniContext as a hosted API for your engineering organization. Team-wide knowledge sharing, SSO, audit logs, and SLA guarantees.",
};

export default function EnterpriseLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return <>{children}</>;
}
