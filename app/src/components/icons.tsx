import type { ReactNode } from "react";

interface IconProps {
  className?: string;
}

function Svg({ className = "size-4", children }: IconProps & { children: ReactNode }) {
  return (
    <svg
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.6}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      className={className}
    >
      {children}
    </svg>
  );
}

export function Mark({ className = "size-7" }: IconProps) {
  // Pixel-hinted 32 px drawing of the app icon; brand colours are fixed, not themed.
  return (
    <svg viewBox="0 0 32 32" aria-hidden="true" focusable="false" className={className}>
      <path
        d="M11.94 3H20.06C22.63 3 23.92 3 25.31 3.44C26.82 3.99 28.01 5.18 28.56 6.69C29 8.08 29 9.37 29 11.94V20.06C29 22.63 29 23.92 28.56 25.31C28.01 26.82 26.82 28.01 25.31 28.56C23.92 29 22.63 29 20.06 29H11.94C9.37 29 8.08 29 6.69 28.56C5.18 28.01 3.99 26.82 3.44 25.31C3 23.92 3 22.63 3 20.06V11.94C3 9.37 3 8.08 3.44 6.69C3.99 5.18 5.18 3.99 6.69 3.44C8.08 3 9.37 3 11.94 3Z"
        fill="#15171C"
      />
      <path d="M12 6A14.5 14.5 0 0 0 12 26L15 26A26 26 0 0 1 15 6Z" fill="#F2EDE4" />
      <path d="M20 6A14.5 14.5 0 0 1 20 26L17 26A26 26 0 0 0 17 6Z" fill="#FF5A36" />
    </svg>
  );
}

export const StatusIcon = (p: IconProps) => (
  <Svg {...p}>
    <circle cx="10" cy="10" r="6.5" />
    <path d="M3.5 10h13" />
    <path d="M5.2 6.5A6.5 6.5 0 0 1 14.8 6.5" />
  </Svg>
);

export const RulesIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="m3.5 5.5 1.5 1.5 2.5-3" />
    <path d="M10 5.5h6.5" />
    <path d="m3.5 13 1.5 1.5 2.5-3" />
    <path d="M10 13h6.5" />
  </Svg>
);

export const CategoriesIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M3.5 4.5h5l7.5 7.5-4.5 4.5-7.5-7.5z" />
    <circle cx="6.75" cy="7.75" r="1" />
  </Svg>
);

export const ActivityIcon = (p: IconProps) => (
  <Svg {...p}>
    <circle cx="10" cy="10" r="6.5" />
    <path d="M10 6.5V10l2.5 1.5" />
  </Svg>
);

export const SettingsIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M3.5 6h8M15.5 6h1M3.5 14h1M8.5 14h8" />
    <circle cx="13.5" cy="6" r="2" />
    <circle cx="6.5" cy="14" r="2" />
  </Svg>
);

export const PlusIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M10 4.5v11M4.5 10h11" />
  </Svg>
);

export const TrashIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M4 6h12M8 6V4.5h4V6M5.5 6l.8 9.5h7.4l.8-9.5" />
  </Svg>
);

export const ChevronRight = (p: IconProps) => (
  <Svg {...p}>
    <path d="m8 5 5 5-5 5" />
  </Svg>
);

export const ChevronLeft = (p: IconProps) => (
  <Svg {...p}>
    <path d="m12 5-5 5 5 5" />
  </Svg>
);

export const ExternalIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M11.5 4h4.5v4.5M16 4l-7 7M14 11.5V16H4V6h4.5" />
  </Svg>
);

export const TabIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M3 16V6.5h5.5L10 4.5h7V16z" />
  </Svg>
);

export const DocumentIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M5 3h6.5L15 6.5V17H5z" />
    <path d="M11.5 3v3.5H15" />
  </Svg>
);

export const WindowIcon = (p: IconProps) => (
  <Svg {...p}>
    <rect x="3" y="4" width="14" height="12" rx="1.5" />
    <path d="M3 7.5h14" />
  </Svg>
);

export const AppIcon = (p: IconProps) => (
  <Svg {...p}>
    <rect x="4" y="4" width="12" height="12" rx="3" />
  </Svg>
);

export const PlayIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M6.5 4.5v11l9-5.5z" />
  </Svg>
);

export const RestoreIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M7 8H3.5V4.5" />
    <path d="M3.8 8A6.5 6.5 0 1 1 4 12.5" />
  </Svg>
);

export const AlertIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M10 3.5 17 16H3z" />
    <path d="M10 8.5v3M10 13.8v.2" />
  </Svg>
);

export const CloseIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="m5.5 5.5 9 9M14.5 5.5l-9 9" />
  </Svg>
);
