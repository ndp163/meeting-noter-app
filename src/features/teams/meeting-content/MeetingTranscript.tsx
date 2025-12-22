export const MeetingTranscript = () => {
  return (
    <div className="max-w-4xl mx-auto">
      <div className="bg-white rounded-lg border border-gray-200 p-6">
        <div className="space-y-6">
          {/* Transcript Entry */}
          <div className="flex gap-4">
            <div className="flex flex-col items-center shrink-0">
              <div className="w-8 h-8 bg-indigo-100 text-indigo-600 rounded-full flex items-center justify-center text-xs font-semibold">
                JD
              </div>
              <div className="w-px h-full bg-gray-200 mt-2" />
            </div>
            <div className="flex-1 pb-6">
              <div className="flex items-center gap-2 mb-2">
                <span className="font-medium text-gray-900">John Doe</span>
                <span className="text-xs text-gray-500">00:02:15</span>
              </div>
              <p className="text-gray-700">
                Good morning everyone. Let's start with a quick recap of where
                we are with the Q4 planning. As you all know, we've been
                focusing on three main areas: performance optimization, security
                improvements, and user experience enhancements.
              </p>
            </div>
          </div>

          {/* Transcript Entry */}
          <div className="flex gap-4">
            <div className="flex flex-col items-center shrink-0">
              <div className="w-8 h-8 bg-purple-100 text-purple-600 rounded-full flex items-center justify-center text-xs font-semibold">
                JS
              </div>
              <div className="w-px h-full bg-gray-200 mt-2" />
            </div>
            <div className="flex-1 pb-6">
              <div className="flex items-center gap-2 mb-2">
                <span className="font-medium text-gray-900">Jane Smith</span>
                <span className="text-xs text-gray-500">00:03:42</span>
              </div>
              <p className="text-gray-700">
                Thanks John. I wanted to highlight that the authentication
                module is nearly complete. We've addressed most of the security
                concerns raised in the last review. However, we still need to
                finalize the OAuth integration with Microsoft and Google.
              </p>
            </div>
          </div>

          {/* Transcript Entry */}
          <div className="flex gap-4">
            <div className="flex flex-col items-center shrink-0">
              <div className="w-8 h-8 bg-green-100 text-green-600 rounded-full flex items-center justify-center text-xs font-semibold">
                MJ
              </div>
              <div className="w-px h-full bg-gray-200 mt-2" />
            </div>
            <div className="flex-1 pb-6">
              <div className="flex items-center gap-2 mb-2">
                <span className="font-medium text-gray-900">Mike Johnson</span>
                <span className="text-xs text-gray-500">00:05:18</span>
              </div>
              <p className="text-gray-700">
                On the performance side, we've made significant progress. The
                database query optimization has reduced our average response
                time by 40%. I'll be setting up the monitoring dashboard this
                week so we can track these improvements in real-time.
              </p>
            </div>
          </div>

          {/* Transcript Entry */}
          <div className="flex gap-4">
            <div className="flex flex-col items-center shrink-0">
              <div className="w-8 h-8 bg-orange-100 text-orange-600 rounded-full flex items-center justify-center text-xs font-semibold">
                SL
              </div>
              <div className="w-px h-full bg-gray-200 mt-2" />
            </div>
            <div className="flex-1 pb-6">
              <div className="flex items-center gap-2 mb-2">
                <span className="font-medium text-gray-900">Sarah Lee</span>
                <span className="text-xs text-gray-500">00:07:05</span>
              </div>
              <p className="text-gray-700">
                That's excellent news, Mike. From the UX perspective, we've
                completed the user testing for the new dashboard design. The
                feedback has been overwhelmingly positive. Users particularly
                love the simplified navigation and the new dark mode option.
              </p>
            </div>
          </div>

          {/* Transcript Entry */}
          <div className="flex gap-4">
            <div className="flex flex-col items-center shrink-0">
              <div className="w-8 h-8 bg-indigo-100 text-indigo-600 rounded-full flex items-center justify-center text-xs font-semibold">
                JD
              </div>
            </div>
            <div className="flex-1">
              <div className="flex items-center gap-2 mb-2">
                <span className="font-medium text-gray-900">John Doe</span>
                <span className="text-xs text-gray-500">00:09:30</span>
              </div>
              <p className="text-gray-700">
                Great updates all around. Let's make sure we document all action
                items and follow up next week. I'll send out the meeting notes
                by end of day today.
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
