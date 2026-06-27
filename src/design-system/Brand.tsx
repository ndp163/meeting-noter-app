import { type ReactNode } from "react";
import { Mic } from "lucide-react";
import { clsx } from "./util";

export interface BrandProps {
  /** Product name shown beside the mark. @default "Meeting Noter" */
  name?: string;
  /** Hide the wordmark, show only the glyph. */
  glyphOnly?: boolean;
  /** Override the glyph. */
  logo?: ReactNode;
  className?: string;
}

/** App brand lockup — gradient glyph + wordmark. */
export const Brand = ({ name = "Meeting Noter", glyphOnly, logo, className }: BrandProps) => (
  <span className={clsx("ds-brand", className)}>
    <span className="ds-brand__logo">{logo ?? <Mic size={16} />}</span>
    {!glyphOnly && <span className="ds-brand__name">{name}</span>}
  </span>
);
