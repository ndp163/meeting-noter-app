import { Copy, Check } from "lucide-react";
import { useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

interface Props {
  content?: string;
}

export const MeetingSummary = ({ content }: Props) => {
  const [copied, setCopied] = useState(false);

  const copyToClipboard = async () => {
    try {
      await navigator.clipboard.writeText(summaryContent);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error("Failed to copy:", err);
    }
  };

  // Default demo content if no content is provided
  const summaryContent =
    content ||
    `## Key Points

- Discussed Q4 goals and objectives for the engineering team, focusing on scalability improvements
- Reviewed current sprint progress and identified blockers in the authentication module
- Assigned action items for database optimization and performance testing
- Scheduled follow-up meetings with stakeholders for next week

## Action Items

- [ ] Complete API documentation (Assigned to: John Doe • Due: Dec 8)
- [ ] Review pull requests for authentication module (Assigned to: Jane Smith • Due: Dec 6)
- [ ] Set up performance monitoring dashboard (Assigned to: Mike Johnson • Due: Dec 10)

## Decisions Made

- **Approved migration to PostgreSQL 15**
  Migration will begin in January 2025 with a two-week timeline

- **Adopted new code review process**
  All PRs require at least two approvals before merging

## Next Steps

1. Schedule architecture review session
2. Update project roadmap with new timelines
3. Share meeting notes with stakeholders
4. Book conference room for next planning session`;

  return (
    <div className="max-w-4xl mx-auto">
      <div className="bg-white rounded-lg border border-gray-200 p-6">
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-2xl font-bold text-gray-900">Summary</h2>
          <button
            onClick={copyToClipboard}
            className="flex items-center gap-1.5 px-3 py-1.5 text-sm text-gray-600 hover:text-gray-900 hover:bg-gray-100 rounded-lg transition-colors"
            title="Copy as markdown"
          >
            {copied ? (
              <>
                <Check className="w-4 h-4 text-green-600" />
                <span className="text-green-600">Copied!</span>
              </>
            ) : (
              <>
                <Copy className="w-4 h-4" />
                <span>Copy</span>
              </>
            )}
          </button>
        </div>
        <div className="prose prose-sm max-w-none">
          <ReactMarkdown remarkPlugins={[remarkGfm]}>
            {summaryContent}
          </ReactMarkdown>
        </div>
      </div>
    </div>
  );
};
