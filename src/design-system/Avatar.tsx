import { clsx } from "./util";

export type AvatarSize = "sm" | "md" | "lg";

export interface AvatarProps {
  /** Name used to derive the initial and the colour. */
  name: string;
  /** Diameter. @default "md" */
  size?: AvatarSize;
  /** Override the background colour (defaults to a hash of the name). */
  color?: string;
  className?: string;
}

/** Speaker avatar — a coloured circle with the name's first initial. */
const PALETTE = ["#2f5bea", "#e08f2f", "#2e9e6b", "#9b51e0", "#e0484d", "#0f9bb0"];

function colorFor(name: string): string {
  let h = 0;
  for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) >>> 0;
  return PALETTE[h % PALETTE.length];
}

export const Avatar = ({ name, size = "md", color, className }: AvatarProps) => (
  <span
    className={clsx("ds-avatar", size !== "md" && `ds-avatar--${size}`, className)}
    style={{ background: color ?? colorFor(name) }}
    aria-hidden
  >
    {name.trim().charAt(0).toUpperCase() || "?"}
  </span>
);
