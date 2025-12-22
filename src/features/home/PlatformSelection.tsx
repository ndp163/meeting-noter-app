import { ArrowRight } from "lucide-react";

export interface PlatformCard {
  id: string;
  name: string;
  color: string;
  bgColor: string;
  borderColor: string;
  icon: string;
}

interface Props {
  platform: PlatformCard;
  onPlatformSelect: (platformId: string) => void;
}

export const PlatformSelection = ({ platform, onPlatformSelect }: Props) => {
  return (
    <button
      onClick={() => onPlatformSelect(platform.id)}
      className={`bg-white rounded-xl border-2 ${platform.borderColor} p-8 text-center transition-all hover:shadow-lg hover:scale-105 active:scale-100 cursor-pointer group`}
    >
      <div
        className={`w-20 h-20 ${platform.bgColor} rounded-xl flex items-center justify-center mb-4 text-4xl mx-auto`}
      >
        <img className="w-1/2" src={platform.icon} />
      </div>

      <h3 className="text-gray-900 mb-4 group-hover:text-indigo-600 transition-colors">
        {platform.name}
      </h3>

      <div
        className={`inline-flex items-center gap-2 ${platform.color} group-hover:gap-3 transition-all`}
      >
        <span>Select</span>
        <ArrowRight className="w-4 h-4" />
      </div>
    </button>
  );
};
