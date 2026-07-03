import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from "react";
import { clsx } from "./util";

export type ButtonVariant = "primary" | "ghost" | "accent" | "danger";
export type ButtonSize = "sm" | "md" | "lg";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** Visual style. @default "ghost" */
  variant?: ButtonVariant;
  /** Padding scale. @default "md" */
  size?: ButtonSize;
  /** Fully rounded (pill) corners. */
  pill?: boolean;
  /** Stretch to full container width. */
  block?: boolean;
  /** Optional leading icon. */
  icon?: ReactNode;
  children?: ReactNode;
}

/** Primary text/action button. Three variants, three sizes, optional pill + icon. */
export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ variant = "ghost", size = "md", pill, block, icon, className, children, ...rest }, ref) => (
    <button
      ref={ref}
      className={clsx(
        "ds-btn",
        `ds-btn--${variant}`,
        size !== "md" && `ds-btn--${size}`,
        pill && "ds-btn--pill",
        block && "ds-btn--block",
        className,
      )}
      {...rest}
    >
      {icon}
      {children}
    </button>
  ),
);
Button.displayName = "Button";
