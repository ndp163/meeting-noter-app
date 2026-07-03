import { type ReactNode, useEffect, useId, useRef, useState } from "react";
import { ChevronDown } from "lucide-react";
import { clsx } from "./util";

export interface SelectOption {
  /** Stable id returned by onChange. */
  id: string;
  /** Primary label (e.g. native language name). */
  label: string;
  /** Secondary line under the label. */
  hint?: string;
  /** Right-aligned content (badge, size, status icon). */
  trailing?: ReactNode;
}

export interface SelectProps {
  options: SelectOption[];
  /** Id of the selected option. */
  value: string;
  onChange?: (id: string) => void;
  disabled?: boolean;
  /** Shown when no option matches `value`. */
  placeholder?: string;
  /** Menu heading (optional). */
  label?: string;
  className?: string;
}

/**
 * Token-styled dropdown with full keyboard support (listbox + aria-activedescendant
 * pattern): Arrow/Home/End move the active option, Enter/Space select, Escape
 * closes, and typing jumps to a matching label. Closes on Escape or outside
 * click. Presentational — the caller owns `value`.
 */
export const Select = ({
  options,
  value,
  onChange,
  disabled,
  placeholder = "Select…",
  label,
  className,
}: SelectProps) => {
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(0);
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const typeahead = useRef({ query: "", at: 0 });
  const baseId = useId();
  const optionId = (i: number) => `${baseId}-opt-${i}`;
  const selectedIndex = options.findIndex((o) => o.id === value);
  const selected = selectedIndex >= 0 ? options[selectedIndex] : undefined;

  const openMenu = () => {
    setActiveIndex(selectedIndex >= 0 ? selectedIndex : 0);
    setOpen(true);
  };
  const close = (focusTrigger = true) => {
    setOpen(false);
    if (focusTrigger) triggerRef.current?.focus();
  };
  const commit = (i: number) => {
    const opt = options[i];
    if (opt) onChange?.(opt.id);
    close();
  };

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    };
    window.addEventListener("mousedown", onDown);
    return () => window.removeEventListener("mousedown", onDown);
  }, [open]);

  // Keep the active option scrolled into view.
  useEffect(() => {
    if (!open) return;
    document
      .getElementById(optionId(activeIndex))
      ?.scrollIntoView({ block: "nearest" });
  }, [open, activeIndex]);

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (!open) {
      if (e.key === "ArrowDown" || e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        openMenu();
      }
      return;
    }
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        setActiveIndex((i) => Math.min(options.length - 1, i + 1));
        break;
      case "ArrowUp":
        e.preventDefault();
        setActiveIndex((i) => Math.max(0, i - 1));
        break;
      case "Home":
        e.preventDefault();
        setActiveIndex(0);
        break;
      case "End":
        e.preventDefault();
        setActiveIndex(options.length - 1);
        break;
      case "Enter":
      case " ":
        e.preventDefault();
        commit(activeIndex);
        break;
      case "Escape":
        e.preventDefault();
        close();
        break;
      case "Tab":
        setOpen(false);
        break;
      default:
        // Type-ahead: accumulate typed chars, jump to first matching label.
        if (e.key.length === 1) {
          const now = Date.now();
          const q =
            (now - typeahead.current.at < 600 ? typeahead.current.query : "") +
            e.key.toLowerCase();
          typeahead.current = { query: q, at: now };
          const hit = options.findIndex((o) =>
            o.label.toLowerCase().startsWith(q),
          );
          if (hit >= 0) setActiveIndex(hit);
        }
    }
  };

  return (
    <div ref={rootRef} className={clsx("ds-select", className)}>
      <button
        ref={triggerRef}
        type="button"
        className="ds-select__trigger"
        disabled={disabled}
        role="combobox"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={`${baseId}-list`}
        aria-activedescendant={open ? optionId(activeIndex) : undefined}
        onClick={() => (open ? setOpen(false) : openMenu())}
        onKeyDown={onKeyDown}
      >
        <span className="ds-select__value">
          {selected?.label ?? placeholder}
        </span>
        <ChevronDown
          className={clsx("ds-select__chevron", open && "ds-select__chevron--open")}
        />
      </button>

      {open && (
        <div className="ds-select__menu" id={`${baseId}-list`} role="listbox">
          {label && <div className="ds-select__heading">{label}</div>}
          {options.map((o, i) => (
            <div
              key={o.id}
              id={optionId(i)}
              role="option"
              aria-selected={o.id === value}
              className={clsx(
                "ds-select__opt",
                i === activeIndex && "ds-select__opt--active",
              )}
              onMouseEnter={() => setActiveIndex(i)}
              onClick={() => commit(i)}
            >
              <span className="ds-select__opt-text">
                <span className="ds-select__opt-label">{o.label}</span>
                {o.hint && <span className="ds-select__opt-hint">{o.hint}</span>}
              </span>
              {o.trailing && (
                <span className="ds-select__opt-trailing">{o.trailing}</span>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
