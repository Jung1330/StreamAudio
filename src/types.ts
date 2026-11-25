export type Language = "tr" | "en";

export interface AudioDeviceInfo {
  id: string;
  name: string;
  isDefault: boolean;
  volume: number;
  muted: boolean;
  peak: number;
  kind: "headphones" | "speaker" | "monitor" | "generic";
}

export interface MicInfo {
  id: string;
  name: string;
  volume: number;
  muted: boolean;
  peak: number;
}

export interface AudioSessionInfo {
  appKey: string;
  displayName: string;
  exeName: string;
  pid: number;
  volume: number;
  muted: boolean;
  peak: number;
  isActive: boolean;
  isSystemSounds: boolean;
  isDucked: boolean;
}

export interface AudioSnapshot {
  outputDevices: AudioDeviceInfo[];
  defaultOutputId: string | null;
  mic: MicInfo | null;
  sessions: AudioSessionInfo[];
}

export interface AppShortcutRule {
  id: string;
  appKey: string;
  appName: string;
  shortcut: string;
  mode: "mute" | "duck";
  duckPercent: number;
}

export interface AppConfig {
  language: Language;
  pinned: boolean;
  switchShortcut: string;
  micMuteShortcut: string;
  favoriteDeviceA: string | null;
  favoriteDeviceB: string | null;
  appShortcuts: AppShortcutRule[];
}

export interface OsdToastPayload {
  title: string;
  subtitle: string;
  icon: "headphones" | "speaker" | "mic-off" | "mic-on" | "mute" | "duck" | "volume";
}
