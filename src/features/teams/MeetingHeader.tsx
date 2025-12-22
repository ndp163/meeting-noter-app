import { useBoundStore } from "@/store";
import { Calendar, Clock, Mic } from "lucide-react";

interface Props {
  isCapturing: boolean;
  onStartCapture: () => void;
}

export const MeetingHeader = ({ isCapturing, onStartCapture }: Props) => {
  const { meetingDetail } = useBoundStore.use.teams();
  return (
    <div className="bg-white border-b border-gray-200 p-4">
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center justify-center">
            <span
              className={`inline-flex items-center text-sm px-3 py-1 rounded-full border bg-blue-100 text-blue-700 border-blue-200 mr-2`}
            >
              Microsoft Teams
            </span>
            <h1 className="text-2xl font-bold text-gray-900">
              {meetingDetail?.title || "Select a meeting"}
            </h1>
          </div>

          {meetingDetail && (
            <div className="flex items-center gap-4 mt-2 text-sm text-gray-600">
              <span className="flex items-center gap-1.5">
                <Calendar className="w-4 h-4" />
                {new Date(meetingDetail.date).toLocaleDateString("en-US", {
                  weekday: "long",
                  year: "numeric",
                  month: "long",
                  day: "numeric",
                })}
              </span>
              <span className="flex items-center gap-1.5">
                <Clock className="w-4 h-4" />
                {meetingDetail.duration}
              </span>
            </div>
          )}
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={onStartCapture}
            className={`flex items-center gap-2 px-4 py-2 rounded-lg font-medium transition-all ${
              isCapturing
                ? "bg-red-600 hover:bg-red-700 text-white animate-pulse"
                : "bg-cyan-500 hover:bg-cyan-700 text-white"
            }`}
          >
            <Mic className="w-4 h-4" />
            {isCapturing ? "Stop Capture" : "Start Capture"}
          </button>
        </div>
      </div>
    </div>
  );
};
