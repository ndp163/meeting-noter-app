interface Props {
  activeTab: "summary" | "transcript";
  onTabChange: (tab: "summary" | "transcript") => void;
}

export const MeetingTabs = ({ activeTab, onTabChange }: Props) => {
  return (
    <div className="bg-white border-b border-gray-200">
      <div className="flex gap-1 px-4">
        <button
          onClick={() => onTabChange("summary")}
          className={`px-6 py-3 font-medium text-sm transition-colors relative ${
            activeTab === "summary"
              ? "text-indigo-600"
              : "text-gray-600 hover:text-gray-900"
          }`}
        >
          Summary
          {activeTab === "summary" && (
            <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-indigo-600" />
          )}
        </button>
        <button
          onClick={() => onTabChange("transcript")}
          className={`px-6 py-3 font-medium text-sm transition-colors relative ${
            activeTab === "transcript"
              ? "text-indigo-600"
              : "text-gray-600 hover:text-gray-900"
          }`}
        >
          Transcript
          {activeTab === "transcript" && (
            <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-indigo-600" />
          )}
        </button>
      </div>
    </div>
  );
};
