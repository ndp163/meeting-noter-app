import { useRef } from "react";
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

/** Segmented tab control. Active tab gets a raised surface chip. Arrow keys
 *  move between tabs (roving tabindex); Home/End jump to the ends. */
export const Tabs = ({ items, value, onChange, className }: TabsProps) => {
  const refs = useRef<(HTMLButtonElement | null)[]>([]);

  const move = (to: number) => {
    const i = (to + items.length) % items.length;
    onChange?.(items[i].id);
    refs.current[i]?.focus();
  };

  const onKeyDown = (e: React.KeyboardEvent, index: number) => {
    if (e.key === "ArrowRight" || e.key === "ArrowDown") {
      e.preventDefault();
      move(index + 1);
    } else if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
      e.preventDefault();
      move(index - 1);
    } else if (e.key === "Home") {
      e.preventDefault();
      move(0);
    } else if (e.key === "End") {
      e.preventDefault();
      move(items.length - 1);
    }
  };

  return (
    <div className={clsx("ds-tabs", className)} role="tablist">
      {items.map((t, i) => (
        <button
          key={t.id}
          ref={(el) => {
            refs.current[i] = el;
          }}
          role="tab"
          aria-selected={t.id === value}
          tabIndex={t.id === value ? 0 : -1}
          className={clsx("ds-tab", t.id === value && "ds-tab--active")}
          onClick={() => onChange?.(t.id)}
          onKeyDown={(e) => onKeyDown(e, i)}
        >
          {t.label}
        </button>
      ))}
    </div>
  );
};
