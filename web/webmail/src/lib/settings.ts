export interface UserSettings {
  displayName: string;
  email: string;
  replyTo: string;
  signature: string;
  refreshIntervalMinutes: number;
  blockExternalImages: boolean;
  density: "compact" | "normal";
  customFolders: string[];
}

const SETTINGS_KEY = "fastrmail_settings";

const DEFAULT_SETTINGS: UserSettings = {
  displayName: "System Administrator",
  email: "admin@fastrsoft.com",
  replyTo: "admin@fastrsoft.com",
  signature: "--<br><b>System Administrator</b><br><span style='color: #71717a; font-size: 11px;'>FastrMail High-Performance Mail Engine (Rust 1.81+)</span>",
  refreshIntervalMinutes: 5,
  blockExternalImages: true,
  density: "compact",
  customFolders: ["Projects", "Receipts", "Personal"],
};

export function getSettings(): UserSettings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (raw) {
      return { ...DEFAULT_SETTINGS, ...JSON.parse(raw) };
    }
  } catch {
    // fallback
  }
  return DEFAULT_SETTINGS;
}

export function saveSettings(settings: UserSettings) {
  try {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
  } catch {
    // ignore
  }
}
