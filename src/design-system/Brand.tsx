import { type ReactNode } from "react";
import { BrandMark } from "./BrandMark";
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

/** App brand lockup — glyph + two-tone wordmark (last word accented). */
export const Brand = ({
  name = "Meeting Noter",
  glyphOnly,
  logo,
  className,
}: BrandProps) => {
  const words = name.trim().split(/\s+/);
  const last = words.pop();
  return (
    <span
      className={clsx("ds-brand", className)}
      aria-label={glyphOnly ? name : undefined}
      role={glyphOnly ? "img" : undefined}
    >
      <span className={clsx("ds-brand__logo", !logo && "ds-brand__logo--mark")}>
        {logo ?? <BrandMark size={40} />}
      </span>
      {!glyphOnly && (
        <span className="ds-brand__name">
          {words.length > 0 && <>{words.join(" ")} </>}
          <span className="ds-brand__name-accent">{last}</span>
        </span>
      )}
    </span>
  );
};
