import { type ReactNode } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { clsx } from "./util";

export interface ProseProps {
  /** Markdown source to render (GFM supported). Ignored if children are given. */
  markdown?: string;
  /** Pre-rendered content; takes precedence over `markdown`. */
  children?: ReactNode;
  className?: string;
}

/** Styled long-form text block for AI summaries (TL;DR, key points, action items). */
export const Prose = ({ markdown, children, className }: ProseProps) => (
  <div className={clsx("ds-prose", className)}>
    {children ?? (markdown ? <ReactMarkdown remarkPlugins={[remarkGfm]}>{markdown}</ReactMarkdown> : null)}
  </div>
);
