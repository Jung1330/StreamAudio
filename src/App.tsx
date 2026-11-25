import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  ArrowLeftRight,
  AudioLines,
  Check,
  Headphones,
  Keyboard,
  Mic,
  MicOff,
  Monitor,
  Pin,
  PinOff,
  Plus,
  Power,
  Sliders,
  Speaker,
  Trash2,
  Volume1,
  Volume2,
  VolumeX,
  X,
} from "lucide-react";
import {
  AppConfig,
  AppShortcutRule,
  AudioDeviceInfo,
  AudioSessionInfo,
  AudioSnapshot,
  Language,
} from "./types";
import { t } from "./i18n";
import { PeakBar } from "./components/PeakBar";
import { ShortcutInput } from "./components/ShortcutInput";

type TabId = "mixer" | "devices" | "shortcuts";

const DEFAULT_CONFIG: AppConfig = {
  language: "tr",
  pinned: false,
  switchShortcut: "Ctrl+Alt+S",
  micMuteShortcut: "Ctrl+Alt+M",
  favoriteDeviceA: null,
  favoriteDeviceB: null,
  appShortcuts: [],
};

function DeviceIcon({
  kind,
  className = "h-4 w-4",
}: {
  kind: AudioDeviceInfo["kind"];
  className?: string;
}) {
  switch (kind) {
    case "headphones":
      return <Headphones className={className} />;
    case "monitor":
      return <Monitor className={className} />;
    case "speaker":
    default:
      return <Speaker className={className} />;
  }
}

function getSliderBackground(value: number, muted: boolean, ducked = false) {
  const pct = Math.round(value * 100);
  const fill = muted ? "#475569" : ducked ? "#f59e0b" : "#38bdf8";
  return {
    background: `linear-gradient(to right, ${fill} 0%, ${fill} ${pct}%, rgba(255,255,255,0.09) ${pct}%, rgba(255,255,255,0.09) 100%)`,
  };
}

function getAppBadgeColor(name: string): string {
  const palette = [
    "from-sky-500/25 to-indigo-500/25 text-sky-300 border-sky-400/25",
    "from-emerald-500/25 to-teal-500/25 text-emerald-300 border-emerald-400/25",
    "from-violet-500/25 to-fuchsia-500/25 text-violet-300 border-violet-400/25",
    "from-amber-500/25 to-orange-500/25 text-amber-300 border-amber-400/25",
    "from-rose-500/25 to-pink-500/25 text-rose-300 border-rose-400/25",
  ];
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = name.charCodeAt(i) + ((hash << 5) - hash);
  }
  return palette[Math.abs(hash) % palette.length];
}

export const App: React.FC = () => {
  const [tab, setTab] = useState<TabId>("mixer");
  const [config, setConfig] = useState<AppConfig>(DEFAULT_CONFIG);
  const [snapshot, setSnapshot] = useState<AudioSnapshot>({
    outputDevices: [],
    defaultOutputId: null,
    mic: null,
    sessions: [],
  });
  const [isRecordingShortcut, setIsRecordingShortcut] = useState(false);
  const [statusMsg, setStatusMsg] = useState<string | null>(null);

  // Optimistic slider overrides while dragging to prevent jitter
  const [draggingVolumes, setDraggingVolumes] = useState<Record<string, number>>({});
  const draggingRef = useRef<Record<string, number>>({});

  // New per-app shortcut form state
  const [newRuleApp, setNewRuleApp] = useState("");
  const [newRuleShortcut, setNewRuleShortcut] = useState("Ctrl+Alt+D");
  const [newRuleMode, setNewRuleMode] = useState<"mute" | "duck">("duck");
  const [newRuleDuckPct, setNewRuleDuckPct] = useState(20);

  const lang: Language = config.language;

  const flashStatus = useCallback((msg: string) => {
    setStatusMsg(msg);
    window.setTimeout(() => {
      setStatusMsg((cur) => (cur === msg ? null : cur));
    }, 1800);
  }, []);

  const fetchSnapshot = useCallback(async () => {
    try {
      const data = await invoke<AudioSnapshot>("get_audio_snapshot");
      setSnapshot(data);
    } catch {
      // Ignore transient COM errors during device hot-swap
    }
  }, []);

  // Load config & initial snapshot
  useEffect(() => {
    invoke<AppConfig>("get_config")
      .then((cfg) => setConfig(cfg))
      .catch(() => {});
    fetchSnapshot();

    const unlistenPromise = listen("audio-changed", () => {
      fetchSnapshot();
    });

    return () => {
      unlistenPromise.then((u) => u());
    };
  }, [fetchSnapshot]);

  // Real-time WASAPI polling (120ms when visible, paused when hidden)
  useEffect(() => {
    const id = window.setInterval(() => {
      if (!document.hidden) {
        fetchSnapshot();
      }
    }, 120);
    return () => window.clearInterval(id);
  }, [fetchSnapshot]);

  // Esc to hide window (when not recording shortcut)
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !isRecordingShortcut) {
        invoke("hide_main_window").catch(() => {});
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [isRecordingShortcut]);

  const persistConfig = useCallback(
    async (next: AppConfig, notify = false) => {
      setConfig(next);
      try {
        await invoke("save_config", { newConfig: next });
        if (notify) {
          flashStatus(t(next.language, "saved"));
        }
      } catch (err) {
        console.error("save_config failed:", err);
      }
    },
    [flashStatus]
  );

  const defaultDevice = useMemo(
    () =>
      snapshot.outputDevices.find((d) => d.isDefault) ??
      snapshot.outputDevices[0] ??
      null,
    [snapshot.outputDevices]
  );

  const activeSessionCount = useMemo(
    () => snapshot.sessions.filter((s) => s.isActive).length,
    [snapshot.sessions]
  );

  // Slider helpers
  const startDrag = (key: string, val: number) => {
    draggingRef.current[key] = val;
    setDraggingVolumes((prev) => ({ ...prev, [key]: val }));
  };

  const endDrag = (key: string) => {
    delete draggingRef.current[key];
    setDraggingVolumes((prev) => {
      const next = { ...prev };
      delete next[key];
      return next;
    });
  };

  const getEffectiveVolume = (key: string, actual: number) =>
    key in draggingVolumes ? draggingVolumes[key] : actual;

  // Audio actions
  const handleMasterVolume = (deviceId: string | null, val: number) => {
    const key = `dev:${deviceId ?? "default"}`;
    startDrag(key, val);
    invoke("set_output_volume", { deviceId, volume: val }).catch(() => {});
  };

  const handleMasterMute = (deviceId: string | null, muted: boolean) => {
    invoke("set_output_mute", { deviceId, muted })
      .then(fetchSnapshot)
      .catch(() => {});
  };

  const handleSetDefaultDevice = (deviceId: string) => {
    invoke("set_default_device", { deviceId })
      .then(fetchSnapshot)
      .catch(() => {});
  };

  const handleSwitchFavorite = () => {
    invoke("switch_favorite_device")
      .then(fetchSnapshot)
      .catch((err) => {
        if (typeof err === "string") flashStatus(err);
      });
  };

  const handleMicVolume = (val: number) => {
    startDrag("mic", val);
    invoke("set_mic_volume", { volume: val }).catch(() => {});
  };

  const handleMicMute = () => {
    invoke("toggle_mic_mute")
      .then(fetchSnapshot)
      .catch(() => {});
  };

  const handleAppVolume = (appKey: string, val: number) => {
    startDrag(`app:${appKey}`, val);
    invoke("set_app_volume", { appKey, volume: val }).catch(() => {});
  };

  const handleAppMute = (session: AudioSessionInfo) => {
    invoke("set_app_mute", { appKey: session.appKey, muted: !session.muted })
      .then(fetchSnapshot)
      .catch(() => {});
  };

  const handleAppDuck = (session: AudioSessionInfo) => {
    invoke("toggle_app_duck", { appKey: session.appKey, duckPercent: 20 })
      .then(fetchSnapshot)
      .catch(() => {});
  };

  const handleAddAppRule = () => {
    const cleaned = newRuleApp.trim();
    if (!cleaned || !newRuleShortcut.trim()) return;

    const matchedSession = snapshot.sessions.find(
      (s) =>
        s.appKey.toLowerCase() === cleaned.toLowerCase() ||
        s.displayName.toLowerCase() === cleaned.toLowerCase()
    );

    const appKey = matchedSession
      ? matchedSession.appKey
      : cleaned.toLowerCase().endsWith(".exe") || cleaned === "__system_sounds__"
      ? cleaned.toLowerCase()
      : `${cleaned.toLowerCase()}.exe`;

    const appName = matchedSession
      ? matchedSession.displayName
      : cleaned.replace(/\.exe$/i, "");

    const rule: AppShortcutRule = {
      id: `${Date.now()}`,
      appKey,
      appName,
      shortcut: newRuleShortcut.trim(),
      mode: newRuleMode,
      duckPercent: newRuleDuckPct,
    };

    const next: AppConfig = {
      ...config,
      appShortcuts: [...config.appShortcuts, rule],
    };
    persistConfig(next, true);
    setNewRuleApp("");
  };

  const handleDeleteAppRule = (ruleId: string) => {
    const next: AppConfig = {
      ...config,
      appShortcuts: config.appShortcuts.filter((r) => r.id !== ruleId),
    };
    persistConfig(next, true);
  };

  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden bg-[#0b0e14] text-slate-100 border border-white/10">
      {/* Title Bar */}
      <header
        data-tauri-drag-region
        className="flex shrink-0 items-center justify-between border-b border-white/[0.07] bg-[#0e131d]/95 px-3.5 py-2.5"
      >
        <div
          data-tauri-drag-region
          className="flex items-center gap-2 pointer-events-none"
        >
          <div className="flex h-7 w-7 items-center justify-center rounded-lg bg-gradient-to-br from-sky-500 to-indigo-600 shadow-[0_0_14px_rgba(56,189,248,0.35)]">
            <AudioLines className="h-4 w-4 text-white" />
          </div>
          <div className="flex items-center gap-1.5">
            <span className="text-sm font-bold tracking-tight text-white">
              {t(lang, "appTitle")}
            </span>
            <span className="rounded-md bg-sky-500/15 px-1.5 py-0.5 text-[9px] font-semibold tracking-wider text-sky-300 border border-sky-400/20">
              WASAPI
            </span>
          </div>
        </div>

        <div className="flex items-center gap-1">
          {/* Language Switcher */}
          <button
            type="button"
            onClick={() =>
              persistConfig({
                ...config,
                language: lang === "tr" ? "en" : "tr",
              })
            }
            className="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-[10px] font-semibold tracking-wider text-slate-300 transition hover:border-sky-400/40 hover:text-white"
            title="TR / EN"
          >
            {lang.toUpperCase()}
          </button>

          {/* Pin / Unpin */}
          <button
            type="button"
            onClick={() =>
              persistConfig({
                ...config,
                pinned: !config.pinned,
              })
            }
            className={`rounded-md p-1.5 transition ${
              config.pinned
                ? "bg-sky-500/20 text-sky-300 hover:bg-sky-500/30"
                : "text-slate-400 hover:bg-white/[0.07] hover:text-slate-200"
            }`}
            title={t(lang, config.pinned ? "unpinTooltip" : "pinTooltip")}
          >
            {config.pinned ? (
              <Pin className="h-3.5 w-3.5" />
            ) : (
              <PinOff className="h-3.5 w-3.5" />
            )}
          </button>

          {/* Hide to tray */}
          <button
            type="button"
            onClick={() => invoke("hide_main_window").catch(() => {})}
            className="rounded-md p-1.5 text-slate-400 transition hover:bg-white/[0.07] hover:text-white"
            title={t(lang, "hideTooltip")}
          >
            <X className="h-3.5 w-3.5" />
          </button>
        </div>
      </header>

      {/* Tabs */}
      <nav className="grid shrink-0 grid-cols-3 gap-1 border-b border-white/[0.06] bg-[#0d1119] px-3 py-2">
        {(
          [
            { id: "mixer", icon: Sliders, label: t(lang, "tabMixer") },
            { id: "devices", icon: Headphones, label: t(lang, "tabDevices") },
            { id: "shortcuts", icon: Keyboard, label: t(lang, "tabShortcuts") },
          ] as const
        ).map((item) => {
          const Icon = item.icon;
          const active = tab === item.id;
          return (
            <button
              key={item.id}
              type="button"
              onClick={() => setTab(item.id)}
              className={`flex items-center justify-center gap-1.5 rounded-lg py-1.5 text-xs font-medium transition-all ${
                active
                  ? "bg-gradient-to-r from-sky-500/20 to-indigo-500/20 text-sky-200 border border-sky-400/25 shadow-sm"
                  : "text-slate-400 hover:bg-white/[0.04] hover:text-slate-200 border border-transparent"
              }`}
            >
              <Icon className="h-3.5 w-3.5" />
              <span>{item.label}</span>
            </button>
          );
        })}
      </nav>

      {/* Main Content */}
      <main className="flex-1 overflow-y-auto px-3 py-3 space-y-3">
        {tab === "mixer" && (
          <>
            {/* Master Output Card */}
            {defaultDevice && (
              <section className="rounded-xl border border-white/[0.08] bg-[#121722] p-3 shadow-md">
                <div className="mb-2 flex items-center justify-between gap-2">
                  <div className="flex min-w-0 items-center gap-2">
                    <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-sky-500/15 text-sky-400">
                      <DeviceIcon kind={defaultDevice.kind} className="h-4 w-4" />
                    </div>
                    <div className="min-w-0">
                      <div className="text-[10px] font-semibold uppercase tracking-wider text-slate-400">
                        {t(lang, "masterOutput")}
                      </div>
                      <div
                        className="truncate text-xs font-semibold text-white"
                        title={defaultDevice.name}
                      >
                        {defaultDevice.name}
                      </div>
                    </div>
                  </div>

                  {snapshot.outputDevices.length > 1 && (
                    <button
                      type="button"
                      onClick={handleSwitchFavorite}
                      className="flex shrink-0 items-center gap-1 rounded-lg border border-sky-400/25 bg-sky-500/15 px-2 py-1 text-[11px] font-medium text-sky-200 transition hover:bg-sky-500/25"
                      title={config.switchShortcut || t(lang, "quickSwitch")}
                    >
                      <ArrowLeftRight className="h-3 w-3" />
                      <span>{t(lang, "quickSwitch")}</span>
                    </button>
                  )}
                </div>

                {(() => {
                  const devKey = `dev:${defaultDevice.id}`;
                  const vol = getEffectiveVolume(devKey, defaultDevice.volume);
                  return (
                    <div className="space-y-1.5">
                      <div className="flex items-center gap-2.5">
                        <button
                          type="button"
                          onClick={() =>
                            handleMasterMute(defaultDevice.id, !defaultDevice.muted)
                          }
                          className={`flex h-7 w-7 shrink-0 items-center justify-center rounded-lg border transition ${
                            defaultDevice.muted
                              ? "border-rose-500/30 bg-rose-500/15 text-rose-400"
                              : "border-white/10 bg-white/[0.04] text-slate-200 hover:bg-white/[0.08]"
                          }`}
                          title={t(lang, "muteTooltip")}
                        >
                          {defaultDevice.muted || vol === 0 ? (
                            <VolumeX className="h-4 w-4" />
                          ) : vol < 0.5 ? (
                            <Volume1 className="h-4 w-4" />
                          ) : (
                            <Volume2 className="h-4 w-4" />
                          )}
                        </button>

                        <input
                          type="range"
                          min={0}
                          max={1}
                          step={0.01}
                          value={vol}
                          onChange={(e) =>
                            handleMasterVolume(
                              defaultDevice.id,
                              parseFloat(e.target.value)
                            )
                          }
                          onMouseUp={() => endDrag(devKey)}
                          onTouchEnd={() => endDrag(devKey)}
                          className={`audio-slider flex-1 ${
                            defaultDevice.muted ? "muted" : ""
                          }`}
                          style={getSliderBackground(vol, defaultDevice.muted)}
                        />

                        <span className="w-9 text-right font-mono text-xs font-semibold text-slate-200">
                          {Math.round(vol * 100)}%
                        </span>
                      </div>

                      <PeakBar
                        peak={defaultDevice.peak}
                        muted={defaultDevice.muted}
                        accent="cyan"
                      />
                    </div>
                  );
                })()}
              </section>
            )}

            {/* Microphone Strip */}
            <section className="rounded-xl border border-white/[0.08] bg-[#121722] px-3 py-2.5">
              {snapshot.mic ? (
                (() => {
                  const mic = snapshot.mic;
                  const vol = getEffectiveVolume("mic", mic.volume);
                  return (
                    <div className="space-y-1.5">
                      <div className="flex items-center justify-between gap-2">
                        <div className="flex min-w-0 items-center gap-1.5">
                          <span className="text-[10px] font-semibold uppercase tracking-wider text-slate-400">
                            {t(lang, "microphone")}
                          </span>
                          <span className="truncate text-[11px] text-slate-300">
                            • {mic.name}
                          </span>
                        </div>
                        {config.micMuteShortcut && (
                          <span className="shrink-0 rounded bg-white/[0.06] px-1.5 py-0.5 font-mono text-[9px] text-slate-400">
                            {config.micMuteShortcut}
                          </span>
                        )}
                      </div>

                      <div className="flex items-center gap-2.5">
                        <button
                          type="button"
                          onClick={handleMicMute}
                          className={`flex h-7 w-7 shrink-0 items-center justify-center rounded-lg border transition ${
                            mic.muted
                              ? "border-rose-500/30 bg-rose-500/15 text-rose-400"
                              : "border-emerald-500/25 bg-emerald-500/15 text-emerald-300 hover:bg-emerald-500/25"
                          }`}
                          title={t(lang, "muteTooltip")}
                        >
                          {mic.muted ? (
                            <MicOff className="h-4 w-4" />
                          ) : (
                            <Mic className="h-4 w-4" />
                          )}
                        </button>

                        <input
                          type="range"
                          min={0}
                          max={1}
                          step={0.01}
                          value={vol}
                          onChange={(e) =>
                            handleMicVolume(parseFloat(e.target.value))
                          }
                          onMouseUp={() => endDrag("mic")}
                          onTouchEnd={() => endDrag("mic")}
                          className={`audio-slider flex-1 ${
                            mic.muted ? "muted" : ""
                          }`}
                          style={getSliderBackground(vol, mic.muted)}
                        />

                        <span className="w-9 text-right font-mono text-xs font-semibold text-slate-200">
                          {Math.round(vol * 100)}%
                        </span>
                      </div>

                      <PeakBar
                        peak={mic.peak}
                        muted={mic.muted}
                        accent="emerald"
                      />
                    </div>
                  );
                })()
              ) : (
                <div className="flex items-center gap-2 text-xs text-slate-400">
                  <MicOff className="h-4 w-4 text-slate-500" />
                  <span>{t(lang, "noMic")}</span>
                </div>
              )}
            </section>

            {/* Applications Mixer */}
            <section className="space-y-2">
              <div className="flex items-center justify-between px-0.5">
                <span className="text-[11px] font-semibold uppercase tracking-wider text-slate-400">
                  {t(lang, "applications")}
                </span>
                <span className="rounded-full bg-white/[0.06] px-2 py-0.5 text-[10px] font-medium text-slate-300">
                  {activeSessionCount} {t(lang, "activeCount")}
                </span>
              </div>

              {snapshot.sessions.length === 0 ? (
                <div className="rounded-xl border border-dashed border-white/10 bg-[#121722]/60 px-4 py-8 text-center">
                  <AudioLines className="mx-auto mb-2 h-6 w-6 text-slate-500" />
                  <div className="text-xs font-medium text-slate-300">
                    {t(lang, "noApps")}
                  </div>
                  <div className="mt-1 text-[11px] text-slate-500">
                    {t(lang, "noAppsSub")}
                  </div>
                </div>
              ) : (
                <div className="space-y-2">
                  {snapshot.sessions.map((session) => {
                    const key = `app:${session.appKey}`;
                    const vol = getEffectiveVolume(key, session.volume);
                    const label = session.isSystemSounds
                      ? t(lang, "systemSounds")
                      : session.displayName;
                    const initial = label.charAt(0).toUpperCase();

                    return (
                      <div
                        key={session.appKey}
                        className={`rounded-xl border p-2.5 transition ${
                          session.isDucked
                            ? "border-amber-500/30 bg-amber-500/[0.06]"
                            : "border-white/[0.07] bg-[#121722] hover:border-white/[0.12]"
                        }`}
                      >
                        <div className="mb-1.5 flex items-center justify-between gap-2">
                          <div className="flex min-w-0 items-center gap-2">
                            <div
                              className={`flex h-6 w-6 shrink-0 items-center justify-center rounded-md border bg-gradient-to-br text-[11px] font-bold ${getAppBadgeColor(
                                label
                              )}`}
                            >
                              {initial}
                            </div>
                            <span
                              className="truncate text-xs font-medium text-slate-100"
                              title={session.exeName}
                            >
                              {label}
                            </span>
                            {session.isActive && session.peak > 0.01 && (
                              <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-emerald-400 shadow-[0_0_6px_#34d399]" />
                            )}
                            {session.isDucked && (
                              <span className="shrink-0 rounded bg-amber-500/20 px-1.5 py-0.5 text-[9px] font-semibold text-amber-300">
                                {t(lang, "ducked")}
                              </span>
                            )}
                          </div>

                          <div className="flex shrink-0 items-center gap-1">
                            {/* Quick Duck 20% button */}
                            <button
                              type="button"
                              onClick={() => handleAppDuck(session)}
                              className={`rounded-md px-1.5 py-0.5 text-[10px] font-semibold transition ${
                                session.isDucked
                                  ? "bg-amber-500/25 text-amber-200 border border-amber-400/30"
                                  : "bg-white/[0.05] text-slate-400 hover:bg-amber-500/15 hover:text-amber-300"
                              }`}
                              title={t(lang, "duckTooltip")}
                            >
                              20%
                            </button>
                          </div>
                        </div>

                        <div className="space-y-1.5">
                          <div className="flex items-center gap-2">
                            <button
                              type="button"
                              onClick={() => handleAppMute(session)}
                              className={`flex h-6 w-6 shrink-0 items-center justify-center rounded-md border transition ${
                                session.muted
                                  ? "border-rose-500/30 bg-rose-500/15 text-rose-400"
                                  : "border-white/10 bg-white/[0.04] text-slate-300 hover:bg-white/[0.08]"
                              }`}
                              title={t(lang, "muteTooltip")}
                            >
                              {session.muted || vol === 0 ? (
                                <VolumeX className="h-3.5 w-3.5" />
                              ) : (
                                <Volume2 className="h-3.5 w-3.5" />
                              )}
                            </button>

                            <input
                              type="range"
                              min={0}
                              max={1}
                              step={0.01}
                              value={vol}
                              onChange={(e) =>
                                handleAppVolume(
                                  session.appKey,
                                  parseFloat(e.target.value)
                                )
                              }
                              onMouseUp={() => endDrag(key)}
                              onTouchEnd={() => endDrag(key)}
                              className={`audio-slider flex-1 ${
                                session.muted
                                  ? "muted"
                                  : session.isDucked
                                  ? "ducked"
                                  : ""
                              }`}
                              style={getSliderBackground(
                                vol,
                                session.muted,
                                session.isDucked
                              )}
                            />

                            <span className="w-8 text-right font-mono text-[11px] font-medium text-slate-300">
                              {Math.round(vol * 100)}%
                            </span>
                          </div>

                          <PeakBar
                            peak={session.peak}
                            muted={session.muted}
                            accent={session.isDucked ? "amber" : "cyan"}
                          />
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </section>
          </>
        )}

        {tab === "devices" && (
          <>
            {/* Favorite A <-> B Selector */}
            <section className="rounded-xl border border-sky-400/20 bg-gradient-to-br from-[#121928] to-[#111622] p-3 space-y-2.5">
              <div className="flex items-start justify-between gap-2">
                <div>
                  <h2 className="text-xs font-semibold text-white">
                    {t(lang, "favoriteSwitchTitle")}
                  </h2>
                  <p className="text-[11px] text-slate-400">
                    {t(lang, "favoriteSwitchSub")}
                  </p>
                </div>
                {config.switchShortcut && (
                  <span className="shrink-0 rounded-md border border-sky-400/25 bg-sky-500/15 px-1.5 py-0.5 font-mono text-[10px] text-sky-200">
                    {config.switchShortcut}
                  </span>
                )}
              </div>

              <div className="grid grid-cols-1 gap-2">
                <label className="space-y-1">
                  <span className="text-[10px] font-medium text-slate-400">
                    {t(lang, "deviceA")}
                  </span>
                  <select
                    value={config.favoriteDeviceA ?? ""}
                    onChange={(e) =>
                      persistConfig(
                        {
                          ...config,
                          favoriteDeviceA: e.target.value || null,
                        },
                        true
                      )
                    }
                    className="w-full rounded-lg border border-white/10 bg-[#0b0e14] px-2.5 py-1.5 text-xs text-slate-100 outline-none focus:border-sky-400"
                  >
                    <option value="">{t(lang, "autoSelect")}</option>
                    {snapshot.outputDevices.map((d) => (
                      <option key={d.id} value={d.id}>
                        {d.name}
                      </option>
                    ))}
                  </select>
                </label>

                <label className="space-y-1">
                  <span className="text-[10px] font-medium text-slate-400">
                    {t(lang, "deviceB")}
                  </span>
                  <select
                    value={config.favoriteDeviceB ?? ""}
                    onChange={(e) =>
                      persistConfig(
                        {
                          ...config,
                          favoriteDeviceB: e.target.value || null,
                        },
                        true
                      )
                    }
                    className="w-full rounded-lg border border-white/10 bg-[#0b0e14] px-2.5 py-1.5 text-xs text-slate-100 outline-none focus:border-sky-400"
                  >
                    <option value="">{t(lang, "autoSelect")}</option>
                    {snapshot.outputDevices.map((d) => (
                      <option key={d.id} value={d.id}>
                        {d.name}
                      </option>
                    ))}
                  </select>
                </label>
              </div>

              <button
                type="button"
                onClick={handleSwitchFavorite}
                disabled={snapshot.outputDevices.length < 2}
                className="flex w-full items-center justify-center gap-1.5 rounded-lg bg-gradient-to-r from-sky-500 to-indigo-600 py-1.5 text-xs font-semibold text-white shadow-md transition hover:brightness-110 disabled:opacity-40"
              >
                <ArrowLeftRight className="h-3.5 w-3.5" />
                <span>{t(lang, "switchNow")}</span>
              </button>
            </section>

            {/* All Output Devices */}
            <section className="space-y-2">
              <div className="px-0.5">
                <h2 className="text-[11px] font-semibold uppercase tracking-wider text-slate-400">
                  {t(lang, "outputDevicesTitle")}
                </h2>
                <p className="text-[10px] text-slate-500">
                  {t(lang, "outputDevicesSub")}
                </p>
              </div>

              <div className="space-y-2">
                {snapshot.outputDevices.map((dev) => {
                  const devKey = `dev:${dev.id}`;
                  const vol = getEffectiveVolume(devKey, dev.volume);
                  return (
                    <div
                      key={dev.id}
                      className={`rounded-xl border p-2.5 transition ${
                        dev.isDefault
                          ? "border-sky-400/35 bg-sky-500/[0.08]"
                          : "border-white/[0.07] bg-[#121722] hover:border-white/15"
                      }`}
                    >
                      <button
                        type="button"
                        onClick={() => handleSetDefaultDevice(dev.id)}
                        className="mb-2 flex w-full items-center justify-between gap-2 text-left"
                      >
                        <div className="flex min-w-0 items-center gap-2">
                          <div
                            className={`flex h-7 w-7 shrink-0 items-center justify-center rounded-lg ${
                              dev.isDefault
                                ? "bg-sky-500/20 text-sky-300"
                                : "bg-white/[0.06] text-slate-400"
                            }`}
                          >
                            <DeviceIcon kind={dev.kind} className="h-4 w-4" />
                          </div>
                          <span className="truncate text-xs font-semibold text-slate-100">
                            {dev.name}
                          </span>
                        </div>

                        {dev.isDefault && (
                          <span className="flex shrink-0 items-center gap-1 rounded-full bg-sky-500/20 px-2 py-0.5 text-[10px] font-semibold text-sky-300">
                            <Check className="h-3 w-3" />
                            {t(lang, "defaultBadge")}
                          </span>
                        )}
                      </button>

                      <div className="space-y-1.5">
                        <div className="flex items-center gap-2">
                          <button
                            type="button"
                            onClick={() => handleMasterMute(dev.id, !dev.muted)}
                            className={`flex h-6 w-6 shrink-0 items-center justify-center rounded-md border transition ${
                              dev.muted
                                ? "border-rose-500/30 bg-rose-500/15 text-rose-400"
                                : "border-white/10 bg-white/[0.04] text-slate-300 hover:bg-white/[0.08]"
                            }`}
                          >
                            {dev.muted || vol === 0 ? (
                              <VolumeX className="h-3.5 w-3.5" />
                            ) : (
                              <Volume2 className="h-3.5 w-3.5" />
                            )}
                          </button>

                          <input
                            type="range"
                            min={0}
                            max={1}
                            step={0.01}
                            value={vol}
                            onChange={(e) =>
                              handleMasterVolume(
                                dev.id,
                                parseFloat(e.target.value)
                              )
                            }
                            onMouseUp={() => endDrag(devKey)}
                            onTouchEnd={() => endDrag(devKey)}
                            className={`audio-slider flex-1 ${
                              dev.muted ? "muted" : ""
                            }`}
                            style={getSliderBackground(vol, dev.muted)}
                          />

                          <span className="w-8 text-right font-mono text-[11px] text-slate-300">
                            {Math.round(vol * 100)}%
                          </span>
                        </div>
                        <PeakBar peak={dev.peak} muted={dev.muted} />
                      </div>
                    </div>
                  );
                })}
              </div>
            </section>
          </>
        )}

        {tab === "shortcuts" && (
          <>
            {/* Global Shortcuts */}
            <section className="rounded-xl border border-white/[0.08] bg-[#121722] p-3 space-y-2.5">
              <h2 className="text-[11px] font-semibold uppercase tracking-wider text-slate-400">
                {t(lang, "globalShortcutsTitle")}
              </h2>

              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 text-xs text-slate-200">
                  <ArrowLeftRight className="h-3.5 w-3.5 text-sky-400" />
                  <span>{t(lang, "deviceSwitchShortcut")}</span>
                </div>
                <ShortcutInput
                  value={config.switchShortcut}
                  onChange={(sc) =>
                    persistConfig({ ...config, switchShortcut: sc }, true)
                  }
                  onRecordingChange={setIsRecordingShortcut}
                  pressKeysLabel={t(lang, "pressKeys")}
                  recordLabel={t(lang, "record")}
                  clearLabel={t(lang, "clear")}
                />
              </div>

              <div className="h-px bg-white/[0.06]" />

              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 text-xs text-slate-200">
                  <MicOff className="h-3.5 w-3.5 text-emerald-400" />
                  <span>{t(lang, "micMuteShortcut")}</span>
                </div>
                <ShortcutInput
                  value={config.micMuteShortcut}
                  onChange={(sc) =>
                    persistConfig({ ...config, micMuteShortcut: sc }, true)
                  }
                  onRecordingChange={setIsRecordingShortcut}
                  placeholder="Ctrl+Alt+M"
                  pressKeysLabel={t(lang, "pressKeys")}
                  recordLabel={t(lang, "record")}
                  clearLabel={t(lang, "clear")}
                />
              </div>
            </section>

            {/* Per-App Shortcuts */}
            <section className="rounded-xl border border-white/[0.08] bg-[#121722] p-3 space-y-3">
              <div>
                <h2 className="text-xs font-semibold text-white">
                  {t(lang, "appShortcutsTitle")}
                </h2>
                <p className="text-[11px] text-slate-400">
                  {t(lang, "appShortcutsSub")}
                </p>
              </div>

              {/* Add rule form */}
              <div className="space-y-2 rounded-lg border border-white/[0.07] bg-[#0b0e14] p-2.5">
                <input
                  list="running-apps-datalist"
                  type="text"
                  value={newRuleApp}
                  onChange={(e) => setNewRuleApp(e.target.value)}
                  placeholder={t(lang, "selectApp")}
                  className="w-full rounded-lg border border-white/10 bg-[#121722] px-2.5 py-1.5 text-xs text-slate-100 placeholder-slate-500 outline-none focus:border-sky-400"
                />
                <datalist id="running-apps-datalist">
                  {snapshot.sessions
                    .filter((s) => !s.isSystemSounds)
                    .map((s) => (
                      <option key={s.appKey} value={s.appKey}>
                        {s.displayName}
                      </option>
                    ))}
                </datalist>

                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div className="flex items-center gap-1 rounded-lg bg-[#121722] p-0.5 border border-white/10">
                    <button
                      type="button"
                      onClick={() => setNewRuleMode("mute")}
                      className={`rounded-md px-2 py-1 text-[11px] font-medium transition ${
                        newRuleMode === "mute"
                          ? "bg-rose-500/20 text-rose-300"
                          : "text-slate-400 hover:text-slate-200"
                      }`}
                    >
                      {t(lang, "modeMute")}
                    </button>
                    <button
                      type="button"
                      onClick={() => setNewRuleMode("duck")}
                      className={`rounded-md px-2 py-1 text-[11px] font-medium transition ${
                        newRuleMode === "duck"
                          ? "bg-amber-500/20 text-amber-300"
                          : "text-slate-400 hover:text-slate-200"
                      }`}
                    >
                      {t(lang, "modeDuck")}
                    </button>
                  </div>

                  {newRuleMode === "duck" && (
                    <div className="flex items-center gap-1 text-[11px] text-slate-400">
                      <span>{t(lang, "duckLevel")}:</span>
                      <input
                        type="number"
                        min={5}
                        max={80}
                        value={newRuleDuckPct}
                        onChange={(e) =>
                          setNewRuleDuckPct(
                            Math.min(80, Math.max(5, Number(e.target.value) || 20))
                          )
                        }
                        className="w-12 rounded border border-white/10 bg-[#121722] px-1.5 py-0.5 text-center font-mono text-xs text-slate-100 outline-none"
                      />
                    </div>
                  )}
                </div>

                <div className="flex items-center justify-between gap-2 pt-0.5">
                  <ShortcutInput
                    value={newRuleShortcut}
                    onChange={setNewRuleShortcut}
                    onRecordingChange={setIsRecordingShortcut}
                    placeholder="Ctrl+Alt+D"
                    pressKeysLabel={t(lang, "pressKeys")}
                    recordLabel={t(lang, "record")}
                    clearLabel={t(lang, "clear")}
                  />

                  <button
                    type="button"
                    onClick={handleAddAppRule}
                    disabled={!newRuleApp.trim() || !newRuleShortcut.trim()}
                    className="flex items-center gap-1 rounded-lg bg-sky-500 px-2.5 py-1.5 text-xs font-semibold text-white transition hover:bg-sky-400 disabled:opacity-40"
                  >
                    <Plus className="h-3.5 w-3.5" />
                    <span>{t(lang, "addRule")}</span>
                  </button>
                </div>
              </div>

              {/* Active Rules List */}
              {config.appShortcuts.length === 0 ? (
                <div className="py-3 text-center text-[11px] text-slate-500">
                  {t(lang, "noRules")}
                </div>
              ) : (
                <div className="space-y-1.5">
                  {config.appShortcuts.map((rule) => (
                    <div
                      key={rule.id}
                      className="flex items-center justify-between gap-2 rounded-lg border border-white/[0.07] bg-[#0b0e14] px-2.5 py-2"
                    >
                      <div className="min-w-0">
                        <div className="flex items-center gap-1.5">
                          <span className="truncate text-xs font-semibold text-slate-100">
                            {rule.appName}
                          </span>
                          <span
                            className={`rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase ${
                              rule.mode === "duck"
                                ? "bg-amber-500/20 text-amber-300"
                                : "bg-rose-500/20 text-rose-300"
                            }`}
                          >
                            {rule.mode === "duck"
                              ? `${rule.duckPercent}%`
                              : t(lang, "modeMute")}
                          </span>
                        </div>
                        <div className="font-mono text-[10px] text-slate-500">
                          {rule.appKey}
                        </div>
                      </div>

                      <div className="flex shrink-0 items-center gap-1.5">
                        <span className="rounded border border-white/10 bg-[#121722] px-1.5 py-0.5 font-mono text-[10px] text-sky-300">
                          {rule.shortcut}
                        </span>
                        <button
                          type="button"
                          onClick={() => handleDeleteAppRule(rule.id)}
                          className="rounded p-1 text-slate-500 transition hover:bg-rose-500/15 hover:text-rose-400"
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </section>
          </>
        )}
      </main>

      {/* Footer */}
      <footer className="flex shrink-0 items-center justify-between border-t border-white/[0.06] bg-[#0d1119] px-3 py-1.5 text-[10px] text-slate-500">
        <span>
          {statusMsg ? (
            <span className="font-medium text-emerald-400">{statusMsg}</span>
          ) : (
            t(lang, "trayHint")
          )}
        </span>
        <button
          type="button"
          onClick={() => invoke("quit_app").catch(() => {})}
          className="flex items-center gap-1 rounded px-1.5 py-0.5 text-slate-400 transition hover:bg-rose-500/15 hover:text-rose-400"
          title={t(lang, "quitTooltip")}
        >
          <Power className="h-3 w-3" />
          <span>{lang === "tr" ? "Çıkış" : "Quit"}</span>
        </button>
      </footer>
    </div>
  );
};
