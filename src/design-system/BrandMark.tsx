interface BrandMarkProps {
  size?: number;
  className?: string;
}

/**
 * App mark — retro cassette in the vintage palette (terracotta→amber squircle,
 * cream shell, two reels reading as a face). Mirrors the native macOS app icon
 * (`src-tauri/icons/icon.svg`). viewBox is cropped to the squircle bounds so the
 * mark fills its container edge-to-edge; the container rounds the corners.
 */
export const BrandMark = ({ size = 28, className }: BrandMarkProps) => (
  <svg
    width={size}
    height={size}
    viewBox="100 100 824 824"
    xmlns="http://www.w3.org/2000/svg"
    className={className}
    aria-hidden="true"
  >
    <defs>
      <linearGradient id="ds-brandmark-warm" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stopColor="#b5552f" />
        <stop offset="1" stopColor="#d08a4f" />
      </linearGradient>
    </defs>
    <rect x="100" y="100" width="824" height="824" fill="url(#ds-brandmark-warm)" />
    <rect x="222.3" y="325.3" width="579.2" height="373.3" rx="54.7" fill="#fbf6ec" />
    <rect x="273.8" y="380.0" width="476.3" height="193.1" rx="25.7" fill="#7a3a1f" />
    <circle cx="405.7" cy="476.5" r="64.4" fill="#fbf6ec" />
    <circle cx="405.7" cy="476.5" r="20.9" fill="#7a3a1f" />
    <circle cx="618.1" cy="476.5" r="64.4" fill="#fbf6ec" />
    <circle cx="618.1" cy="476.5" r="20.9" fill="#7a3a1f" />
    <path
      d="M437.9 534.4 q74.0 48.3 148.0 0"
      fill="none"
      stroke="#d08a4f"
      strokeWidth="16.1"
      strokeLinecap="round"
    />
    <rect x="341.4" y="624.5" width="341.1" height="25.7" rx="12.9" fill="#b5552f" />
  </svg>
);
