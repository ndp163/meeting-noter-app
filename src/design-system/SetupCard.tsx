import { type ReactNode } from "react";
import { Mic } from "lucide-react";
import { clsx } from "./util";

export interface SetupCardProps {
  /** Heading. @default "Welcome to Meeting Noter" */
  title?: string;
  /** Sub-heading line. */
  subtitle?: string;
  /** Step rows (compose <Step> elements). */
  children?: ReactNode;
  /** Optional footer slot (e.g. a continue Button). */
  footer?: ReactNode;
  /** Override the brand glyph. */
  logo?: ReactNode;
  className?: string;
}

/** Centered onboarding / setup-gate card with brand mark, steps and footer. */
export const SetupCard = ({
  title = "Welcome to Meeting Noter",
  subtitle,
  children,
  footer,
  logo,
  className,
}: SetupCardProps) => (
  <div className={clsx("ds-setup", className)}>
    <div className="ds-setup__logo">{logo ?? <Mic size={26} />}</div>
    <h2 className="ds-setup__title">{title}</h2>
    {subtitle && <p className="ds-setup__sub">{subtitle}</p>}
    <div className="ds-setup__steps">{children}</div>
    {footer}
  </div>
);
