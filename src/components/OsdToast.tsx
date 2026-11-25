import React, { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  Headphones,
  Speaker,
  MicOff,
  Mic,
  VolumeX,
  Volume1,
  Volume2,
} from "lucide-react";
import { OsdToastPayload } from "../types";

export const OsdToast: React.FC = () => {
  const [payload, setPayload] = useState<OsdToastPayload | null>(null);

  useEffect(() => {
    document.body.style.background = "transparent";
    const unlistenPromise = listen<OsdToastPayload>("osd-toast", (event) => {
      setPayload(event.payload);
    });
    return () => {
      unlistenPromise.then((u) => u());
    };
  }, []);

  if (!payload) {
    return null;
  }

  const renderIcon = () => {
    switch (payload.icon) {
      case "headphones":
        return <Headphones className="h-5 w-5 text-sky-400" />;
      case "speaker":
        return <Speaker className="h-5 w-5 text-indigo-400" />;
      case "mic-off":
        return <MicOff className="h-5 w-5 text-rose-400" />;
      case "mic-on":
        return <Mic className="h-5 w-5 text-emerald-400" />;
      case "mute":
        return <VolumeX className="h-5 w-5 text-rose-400" />;
      case "duck":
        return <Volume1 className="h-5 w-5 text-amber-400" />;
      default:
        return <Volume2 className="h-5 w-5 text-sky-400" />;
    }
  };

  return (
    <div className="flex h-full w-full items-center justify-center p-1.5">
      <div className="flex w-full items-center gap-3 rounded-2xl border border-white/15 bg-[#0d111a]/95 px-4 py-3 shadow-2xl backdrop-blur-xl">
        <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-white/[0.06] border border-white/10">
          {renderIcon()}
        </div>
        <div className="min-w-0 flex-1">
          <div className="truncate text-xs font-semibold text-white">
            {payload.title}
          </div>
          <div className="truncate text-[11px] text-slate-400">
            {payload.subtitle}
          </div>
        </div>
      </div>
    </div>
  );
};
