import React, { useState, useEffect } from "react";
import { Keyboard, X } from "lucide-react";

interface ShortcutInputProps {
  value: string;
  onChange: (shortcut: string) => void;
  onRecordingChange?: (recording: boolean) => void;
  placeholder?: string;
  pressKeysLabel: string;
  recordLabel: string;
  clearLabel: string;
}

function formatKeyEvent(e: KeyboardEvent): string | null {
  const ignored = ["Control", "Shift", "Alt", "Meta", "OS"];
  if (ignored.includes(e.key)) return null;

  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push("Super");

  let key = e.key;
  if (key === " ") key = "Space";
  else if (key.length === 1) key = key.toUpperCase();
  else if (key.startsWith("Arrow")) key = key.replace("Arrow", "");

  // Require at least one modifier unless it's a Function key (F1-F12)
  const isFnKey = /^F([1-9]|1[0-2])$/.test(key);
  if (parts.length === 0 && !isFnKey) return null;

  parts.push(key);
  return parts.join("+");
}

export const ShortcutInput: React.FC<ShortcutInputProps> = ({
  value,
  onChange,
  onRecordingChange,
  placeholder = "Ctrl+Alt+S",
  pressKeysLabel,
  recordLabel,
  clearLabel,
}) => {
  const [recording, setRecording] = useState(false);

  useEffect(() => {
    onRecordingChange?.(recording);
  }, [recording, onRecordingChange]);

  useEffect(() => {
    if (!recording) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      if (e.key === "Escape") {
        setRecording(false);
        return;
      }

      const combo = formatKeyEvent(e);
      if (combo) {
        onChange(combo);
        setRecording(false);
      }
    };

    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [recording, onChange]);

  return (
    <div className="flex items-center gap-1.5">
      <button
        type="button"
        onClick={() => setRecording((r) => !r)}
        className={`flex items-center gap-1.5 rounded-lg border px-2.5 py-1.5 text-xs font-medium transition-all ${
          recording
            ? "border-sky-400 bg-sky-500/20 text-sky-200 shadow-[0_0_12px_rgba(56,189,248,0.25)]"
            : "border-white/10 bg-[#0d111a] text-slate-200 hover:border-white/20 hover:bg-white/[0.06]"
        }`}
        title={recordLabel}
      >
        <Keyboard className="h-3.5 w-3.5 text-sky-400" />
        <span className="font-mono tracking-tight">
          {recording ? pressKeysLabel : value || placeholder}
        </span>
      </button>
      {value && !recording && (
        <button
          type="button"
          onClick={() => onChange("")}
          className="rounded-md p-1 text-slate-400 transition hover:bg-white/10 hover:text-rose-400"
          title={clearLabel}
        >
          <X className="h-3.5 w-3.5" />
        </button>
      )}
    </div>
  );
};
