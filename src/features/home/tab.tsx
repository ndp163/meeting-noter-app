import { cn } from '@/lib/utils';

interface TabProps {
  label: string;
  isActive: boolean;
  onClick: () => void;
}

export const Tab = ({ label, isActive, onClick }: TabProps) => {
  return (
    <button
      onClick={onClick}
      className={cn(
        "flex items-center justify-center px-1 py-0.5 text-base transition-colors",
        isActive 
          ? "border-b-2 border-[#404a4c] text-[#404a4c]" 
          : "text-[#404a4c]/70 hover:text-[#404a4c]"
      )}
    >
      {label}
    </button>
  );
};
