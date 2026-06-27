import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from "react";
import { clsx } from "./util";

export interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** Icon element (e.g. a lucide-react icon). */
  icon: ReactNode;
  /** Accessible label — also used as the tooltip. */
  label: string;
}

/** Square, transparent icon-only button. Subtle hover fill. */
export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(
  ({ icon, label, className, ...rest }, ref) => (
    <button
      ref={ref}
      className={clsx("ds-icon-btn", className)}
      aria-label={label}
      title={label}
      {...rest}
    >
      {icon}
    </button>
  ),
);
IconButton.displayName = "IconButton";
