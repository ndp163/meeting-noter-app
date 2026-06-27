import { type HTMLAttributes, type ReactNode } from "react";
import { clsx } from "./util";

export interface CardProps extends HTMLAttributes<HTMLDivElement> {
  /** Drop the shadow for a flat, hairline-only surface. */
  flat?: boolean;
  /** Apply standard interior padding. */
  padded?: boolean;
  children?: ReactNode;
}

/** Generic surface container — rounded, hairline border, soft shadow. */
export const Card = ({ flat, padded, className, children, ...rest }: CardProps) => (
  <div
    className={clsx("ds-card", flat && "ds-card--flat", padded && "ds-card--pad", className)}
    {...rest}
  >
    {children}
  </div>
);
