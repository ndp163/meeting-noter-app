import type { Meeting, TranscriptMessage } from "@/types/meeting";

/**
 * Case- and accent-insensitive text search, shared by the sidebar's global
 * search and the in-meeting find bar. Folding keeps the UTF-16 length, so a
 * match offset in the folded text is valid in the original.
 */
export const foldText = (text: string): string => {
  let out = "";
  for (const ch of text) out += foldChar(ch);
  return out;
};

const foldChar = (ch: string): string => {
  if (ch === "đ" || ch === "Đ") return "d";
  const lower = ch.toLowerCase();
  if (lower.length !== ch.length) return ch;
  // Strip diacritics from Latin letters only — kana dakuten (が→か) carry meaning.
  const base = lower.normalize("NFD")[0];
  return base.charCodeAt(0) < 0x250 ? base : lower;
};

/** Start offsets of every non-overlapping match of an already-folded query. */
export const findMatches = (folded: string, foldedQuery: string): number[] => {
  const hits: number[] = [];
  if (!foldedQuery) return hits;
  let i = folded.indexOf(foldedQuery);
  while (i !== -1) {
    hits.push(i);
    i = folded.indexOf(foldedQuery, i + foldedQuery.length);
  }
  return hits;
};

/** A snippet around a match, split so the match itself can be highlighted. */
export interface Snippet {
  before: string;
  match: string;
  after: string;
}

const SNIPPET_CONTEXT = 30;

const makeSnippet = (text: string, start: number, length: number): Snippet => {
  const from = Math.max(0, start - SNIPPET_CONTEXT);
  const to = Math.min(text.length, start + length + SNIPPET_CONTEXT);
  return {
    before: (from > 0 ? "…" : "") + text.slice(from, start),
    match: text.slice(start, start + length),
    after: text.slice(start + length, to) + (to < text.length ? "…" : ""),
  };
};

export interface MeetingHit {
  meeting: Meeting;
  /** Matches in the transcript (a title-only hit has 0). */
  count: number;
  snippet?: Snippet;
}

// Folded transcript lines, cached per message object. The store is immutable
// (immer), so an edited message is a new object and misses the cache.
const foldCache = new WeakMap<TranscriptMessage, string>();
const foldedContent = (m: TranscriptMessage): string => {
  let folded = foldCache.get(m);
  if (folded === undefined) {
    folded = foldText(m.content);
    foldCache.set(m, folded);
  }
  return folded;
};

/** Meetings whose title or transcript contains `query`, in input order. */
export const searchMeetings = (
  meetings: Meeting[],
  query: string,
): MeetingHit[] => {
  const q = foldText(query.trim());
  if (!q) return meetings.map((meeting) => ({ meeting, count: 0 }));

  const hits: MeetingHit[] = [];
  for (const meeting of meetings) {
    let count = 0;
    let snippet: Snippet | undefined;
    for (const message of meeting.transcript) {
      const offsets = findMatches(foldedContent(message), q);
      if (offsets.length === 0) continue;
      count += offsets.length;
      snippet ??= makeSnippet(message.content, offsets[0], q.length);
    }
    if (count > 0 || foldText(meeting.title).includes(q)) {
      hits.push({ meeting, count, snippet });
    }
  }
  return hits;
};
