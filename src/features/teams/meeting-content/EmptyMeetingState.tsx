import { FileText } from "lucide-react";

export const EmptyMeetingState = () => {
  return (
    <div className="flex flex-col items-center justify-center h-full text-center">
      <FileText className="w-16 h-16 text-gray-300 mb-4" />
      <h3 className="text-xl font-semibold text-gray-900 mb-2">
        No Meeting Selected
      </h3>
      <p className="text-gray-600">
        Select a meeting from the sidebar to view its details
      </p>
    </div>
  );
};
