import { clsx } from "./util";

export interface TabItem {
  /** Stable id returned by onChange. */
  id: string;
  /** Visible label. */
  label: string;
}

export interface TabsProps {
  /** Tabs to render, left to right. */
  items: TabItem[];
  /** Id of the active tab. */
  value: string;
  /** Fired with the id of the clicked tab. */
  onChange?: (id: string) => void;
  className?: string;
}

/** Segmented tab control. Active tab gets a raised surface chip. */
export const Tabs = ({ items, value, onChange, className }: TabsProps) => (
  <div className={clsx("ds-tabs", className)} role="tablist">
    {items.map((t) => (
      <button
        key={t.id}
        role="tab"
        aria-selected={t.id === value}
        className={clsx("ds-tab", t.id === value && "ds-tab--active")}
        onClick={() => onChange?.(t.id)}
      >
        {t.label}
      </button>
    ))}
  </div>
);
