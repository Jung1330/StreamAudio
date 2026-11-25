import React from "react";

interface PeakBarProps {
  peak: number;
  muted?: boolean;
  accent?: "cyan" | "emerald" | "amber";
}

export const PeakBar: React.FC<PeakBarProps> = ({
  peak,
  muted = false,
  accent = "cyan",
}) => {
  const pct = muted ? 0 : Math.min(100, Math.max(0, Math.round(peak * 100)));

  const gradientClass =
    accent === "emerald"
      ? "from-emerald-500 via-teal-400 to-amber-400"
      : accent === "amber"
      ? "from-amber-500 via-orange-400 to-rose-500"
      : "from-emerald-400 via-sky-400 to-rose-500";

  return (
    <div className="relative h-1.5 w-full overflow-hidden rounded-full bg-white/[0.06]">
      <div
        className={`h-full rounded-full bg-gradient-to-r ${gradientClass} transition-[width] duration-75 ease-out`}
        style={{ width: `${pct}%` }}
      />
    </div>
  );
};
