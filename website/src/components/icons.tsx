import React from "react";
import { siteConfig } from "@/config/site";

interface LogoProps extends React.SVGProps<SVGSVGElement> {
  size?: number;
  className?: string;
}

export function Logo({ size = 24, className, ...props }: LogoProps) {
  // A minimalist geometric graph/node representation for OmniContext
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      {...props}
    >
      {/* Dynamic Graph Nodes & Edges */}
      <circle
        cx="12"
        cy="12"
        r="9"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeDasharray="2 3"
        opacity="0.4"
      />
      {/* Central Core */}
      <circle cx="12" cy="12" r="3" fill="currentColor" />
      {/* Data Ingestion Edges */}
      <path
        d="M4 12H9"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
      <path
        d="M7 6L10 10"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
      <path
        d="M7 18L10 14"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
      {/* Context Extraction Edges */}
      <path
        d="M15 12H20"
        stroke="#22c55e"
        strokeWidth="1.5"
        strokeLinecap="round"
      />{" "}
      {/* Emerald accent */}
      <path
        d="M14 10L17 6"
        stroke="#22c55e"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
      <path
        d="M14 14L17 18"
        stroke="#22c55e"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
      {/* Peripheral Nodes (Outputs) */}
      <circle cx="20" cy="12" r="1.5" fill="#22c55e" />
      <circle cx="17" cy="6" r="1.5" fill="#22c55e" />
      <circle cx="17" cy="18" r="1.5" fill="#22c55e" />
      {/* Inputs */}
      <circle cx="4" cy="12" r="1.5" fill="currentColor" opacity="0.8" />
      <circle cx="7" cy="6" r="1.5" fill="currentColor" opacity="0.8" />
      <circle cx="7" cy="18" r="1.5" fill="currentColor" opacity="0.8" />
    </svg>
  );
}

export function SiteIdentity({
  className,
  withText = true,
}: {
  className?: string;
  withText?: boolean;
}) {
  return (
    <div className={`flex items-center gap-2 ${className || ""}`}>
      <Logo size={18} />
      {withText && (
        <span className="font-semibold text-sm tracking-tight">
          {siteConfig.name}
        </span>
      )}
    </div>
  );
}
