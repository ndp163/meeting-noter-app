import { brandMarkImage } from "./brand-mark-image";

interface BrandMarkProps {
  size?: number;
  className?: string;
}

/**
 * App mark — retro cassette in the vintage palette (cream squircle shell, two
 * reels, a waveform underneath). Mirrors the native macOS app icon
 * (`src-tauri/icons/icon.png`); the source bitmap lives in
 * `brand-mark-image.ts`. The container rounds the corners.
 */
export const BrandMark = ({ size = 32, className }: BrandMarkProps) => (
  <img
    src={brandMarkImage}
    width={size}
    height={size}
    className={className}
    alt=""
    aria-hidden="true"
    draggable={false}
  />
);
