/**
 * The icon set, inline.
 *
 * A diagram tool needs perhaps a dozen glyphs; an icon package would be a
 * dependency, a bundle chapter and a licence for that. These are drawn on a
 * 16-unit grid and inherit `currentColor`.
 */

import type { ReactNode } from "react";

type IconProps = { size?: number; className?: string };

function Svg({ size = 16, className, children }: IconProps & { children: ReactNode }) {
  return (
    <svg
      className={className}
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.4"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {children}
    </svg>
  );
}

export function TableIcon(props: IconProps) {
  return (
    <Svg {...props}>
      <rect x="2" y="2.5" width="12" height="11" rx="1.5" />
      <path d="M2 6h12M6 6v7.5" />
    </Svg>
  );
}

export function SearchIcon(props: IconProps) {
  return (
    <Svg {...props}>
      <circle cx="7" cy="7" r="4.5" />
      <path d="M10.5 10.5 14 14" />
    </Svg>
  );
}

export function EyeIcon(props: IconProps) {
  return (
    <Svg {...props}>
      <path d="M1.5 8S4 3.5 8 3.5 14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8Z" />
      <circle cx="8" cy="8" r="1.8" />
    </Svg>
  );
}

export function EyeOffIcon(props: IconProps) {
  return (
    <Svg {...props}>
      <path d="M2.5 5.5C1.9 6.4 1.5 8 1.5 8S4 12.5 8 12.5c1 0 1.9-.3 2.7-.7" />
      <path d="M6.3 4c.5-.3 1.1-.5 1.7-.5 4 0 6.5 4.5 6.5 4.5s-.6 1.1-1.7 2.2" />
      <path d="M2 2l12 12" />
    </Svg>
  );
}

export function SidebarIcon(props: IconProps) {
  return (
    <Svg {...props}>
      <rect x="2" y="3" width="12" height="10" rx="1.5" />
      <path d="M6.5 3v10" />
    </Svg>
  );
}

export function MinusIcon(props: IconProps) {
  return (
    <Svg {...props}>
      <path d="M3.5 8h9" />
    </Svg>
  );
}

export function PlusIcon(props: IconProps) {
  return (
    <Svg {...props}>
      <path d="M8 3.5v9M3.5 8h9" />
    </Svg>
  );
}

export function FitViewIcon(props: IconProps) {
  return (
    <Svg {...props}>
      <path d="M2 5.5V3a1 1 0 0 1 1-1h2.5M10.5 2H13a1 1 0 0 1 1 1v2.5M14 10.5V13a1 1 0 0 1-1 1h-2.5M5.5 14H3a1 1 0 0 1-1-1v-2.5" />
    </Svg>
  );
}

export function TidyIcon(props: IconProps) {
  return (
    <Svg {...props}>
      <rect x="2" y="2.5" width="5" height="5" rx="1" />
      <rect x="9" y="2.5" width="5" height="5" rx="1" />
      <rect x="2" y="9" width="5" height="4.5" rx="1" />
      <rect x="9" y="9" width="5" height="4.5" rx="1" />
    </Svg>
  );
}

export function LinkIcon(props: IconProps) {
  return (
    <Svg {...props}>
      <path d="M6.5 9.5a2.5 2.5 0 0 0 3.5 0l2-2a2.5 2.5 0 0 0-3.5-3.5l-.7.7" />
      <path d="M9.5 6.5a2.5 2.5 0 0 0-3.5 0l-2 2a2.5 2.5 0 0 0 3.5 3.5l.7-.7" />
    </Svg>
  );
}

export function CommentIcon(props: IconProps) {
  return (
    <Svg {...props}>
      <path d="M13.5 9.5a1.5 1.5 0 0 1-1.5 1.5H6l-3 2.5V4a1.5 1.5 0 0 1 1.5-1.5h7.5A1.5 1.5 0 0 1 13.5 4Z" />
    </Svg>
  );
}

export function CloseIcon(props: IconProps) {
  return (
    <Svg {...props}>
      <path d="M4 4l8 8M12 4l-8 8" />
    </Svg>
  );
}

export function CheckIcon(props: IconProps) {
  return (
    <Svg {...props}>
      <path d="M3 8.5 6.5 12 13 4.5" />
    </Svg>
  );
}

export function DownloadIcon(props: IconProps) {
  return (
    <Svg {...props}>
      <path d="M8 2v8" />
      <path d="M4.5 7 8 10.5 11.5 7" />
      <path d="M2.5 13.5h11" />
    </Svg>
  );
}

/** The PK marker: a key, in the accent colour. */
export function KeyIcon(props: IconProps) {
  return (
    <Svg {...props}>
      <circle cx="5.5" cy="10.5" r="2.5" />
      <path d="M7.3 8.7 13 3m-2 2 1.5 1.5M9.5 6.5 11 8" />
    </Svg>
  );
}

/** The "has a value" marker: filled for NOT NULL, hollow for nullable. */
export function DiamondIcon({ filled, ...props }: IconProps & { filled: boolean }) {
  return (
    <Svg {...props}>
      <path d="M8 2.8 13.2 8 8 13.2 2.8 8Z" fill={filled ? "currentColor" : "none"} />
    </Svg>
  );
}
