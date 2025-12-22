import { Video } from "lucide-react";
import { useNavigate } from "react-router";
import {
  PlatformSelection,
  PlatformCard,
} from "@/features/home/PlatformSelection";
import TeamsLogo from "@assets/teams-logo.svg";
import ZoomLogo from "@assets/zoom-logo.svg";
import GoogleMeetLogo from "@assets/google-meet-logo.svg";

export const HomePage = () => {
  const navigate = useNavigate();

  const handlePlatformClick = (platformId: string) => {
    navigate(`/${platformId}`);
  };

  const platforms: PlatformCard[] = [
    {
      id: "teams",
      name: "Microsoft Teams",
      color: "text-blue-600",
      bgColor: "bg-blue-50",
      borderColor: "border-blue-200",
      icon: TeamsLogo,
    },
    {
      id: "zoom",
      name: "Zoom",
      color: "text-sky-600",
      bgColor: "bg-sky-50",
      borderColor: "border-sky-200",
      icon: ZoomLogo,
    },
    {
      id: "meet",
      name: "Google Meet",
      color: "text-green-600",
      bgColor: "bg-green-50",
      borderColor: "border-green-200",
      icon: GoogleMeetLogo,
    },
  ];

  return (
    <div className="min-h-screen bg-linear-to-br from-indigo-50 via-white to-purple-50 flex items-center justify-center p-8">
      <div className="max-w-4xl w-full">
        <div className="text-center mb-12">
          <div className="inline-flex items-center justify-center w-20 h-20 bg-indigo-600 rounded-2xl mb-6">
            <Video className="w-10 h-10 text-white" />
          </div>
          <h1 className="text-gray-900 mb-4 text-2xl">
            Welcome to Meeting Noter
          </h1>
          <p className="text-gray-600 max-w-2xl mx-auto">
            Select the platform you want to capture captions from
          </p>
        </div>

        {/* Platform Cards */}
        <div className="grid md:grid-cols-3 gap-6">
          {platforms.map((platform) => (
            <PlatformSelection
              key={platform.id}
              platform={platform}
              onPlatformSelect={handlePlatformClick}
            />
          ))}
        </div>
      </div>
    </div>
  );
};
