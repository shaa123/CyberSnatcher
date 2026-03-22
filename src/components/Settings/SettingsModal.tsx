import { useState, useEffect } from "react";
import { useDownloadStore } from "../../stores/downloadStore";
import { getBrowserSettings, setBrowserSettings } from "../../lib/tauri";

interface SettingsModalProps {
  onClose: () => void;
  ytdlpInstalled: boolean;
}

export default function SettingsModal({ onClose, ytdlpInstalled }: SettingsModalProps) {
  const downloadFolder = useDownloadStore((s) => s.downloadFolder);
  const setDownloadFolder = useDownloadStore((s) => s.setDownloadFolder);
  const [tab, setTab] = useState<"general" | "browser">("general");
  const [folderInput, setFolderInput] = useState(downloadFolder);

  // Browser settings state
  const [adblockEnabled, setAdblockEnabled] = useState(true);
  const [popupBlockerEnabled, setPopupBlockerEnabled] = useState(true);
  const [settingsLoaded, setSettingsLoaded] = useState(false);

  // Load browser settings from backend on mount
  useEffect(() => {
    getBrowserSettings()
      .then((settings) => {
        setAdblockEnabled(settings.adblock_enabled);
        setPopupBlockerEnabled(settings.popup_blocker_enabled);
        setSettingsLoaded(true);
      })
      .catch(() => setSettingsLoaded(true));
  }, []);

  const handleToggleAdblock = async (enabled: boolean) => {
    setAdblockEnabled(enabled);
    try {
      await setBrowserSettings(enabled, popupBlockerEnabled);
    } catch (e) {
      console.error("Failed to update adblock setting:", e);
    }
  };

  const handleTogglePopupBlocker = async (enabled: boolean) => {
    setPopupBlockerEnabled(enabled);
    try {
      await setBrowserSettings(adblockEnabled, enabled);
    } catch (e) {
      console.error("Failed to update popup blocker setting:", e);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={onClose} />
      <div className="relative bg-cyber-surface border border-cyber-border rounded-xl w-[520px] max-h-[80vh] overflow-hidden shadow-2xl flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-cyber-border shrink-0">
          <h2 className="text-base font-bold text-cyber-text-primary">Settings</h2>
          <button onClick={onClose} className="text-cyber-text-tertiary hover:text-cyber-text-primary transition-colors">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
              <path d="M18 6L6 18M6 6l12 12" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
            </svg>
          </button>
        </div>

        {/* Tabs */}
        <div className="flex border-b border-cyber-border shrink-0">
          {(["general", "browser"] as const).map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              className={`flex-1 py-2 text-xs font-semibold transition-colors ${
                tab === t ? "text-cyber-primary border-b-2 border-cyber-primary" : "text-cyber-text-tertiary hover:text-cyber-text-secondary"
              }`}
            >
              {t === "general" ? "General" : t === "browser" ? "Browser" : "Output Log"}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-4">
          {tab === "general" && (
            <div className="space-y-4">
              {/* yt-dlp status */}
              <div className="flex items-center gap-2 p-3 rounded-lg bg-cyber-bg border border-cyber-border">
                <div className={`w-2 h-2 rounded-full ${ytdlpInstalled ? "bg-cyber-success" : "bg-cyber-error"}`} />
                <span className="text-xs text-cyber-text-secondary">
                  yt-dlp: {ytdlpInstalled ? "Installed" : "Not found — install from https://github.com/yt-dlp/yt-dlp"}
                </span>
              </div>

              {/* Download folder */}
              <div>
                <label className="block text-xs font-semibold text-cyber-text-secondary mb-1.5">
                  Default Download Folder
                </label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={folderInput}
                    onChange={(e) => setFolderInput(e.target.value)}
                    className="flex-1 bg-cyber-bg border border-cyber-border rounded-lg px-3 py-2 text-sm text-cyber-text-primary font-mono"
                  />
                  <button
                    onClick={() => setDownloadFolder(folderInput)}
                    className="px-3 py-2 bg-cyber-primary/10 border border-cyber-primary/30 rounded-lg text-sm text-cyber-primary hover:bg-cyber-primary/20 transition-all"
                  >
                    Set
                  </button>
                </div>
                <p className="text-[10px] text-cyber-text-tertiary mt-1">
                  Tip: You can also click folders in the sidebar to set as download target
                </p>
              </div>

              {/* Info */}
              <div className="p-3 rounded-lg bg-cyber-bg border border-cyber-border space-y-1">
                <p className="text-xs text-cyber-text-secondary font-semibold">How it works</p>
                <p className="text-[10px] text-cyber-text-tertiary leading-relaxed">
                  CyberSnatcher uses yt-dlp under the hood to download videos. Make sure yt-dlp is installed and in your PATH.
                  Paste any URL from YouTube, Twitter/X, TikTok, Instagram, Reddit, or direct video links.
                </p>
              </div>
            </div>
          )}

          {tab === "browser" && (
            <div className="space-y-4">
              {/* Adblock toggle */}
              <div className="flex items-center justify-between p-3 rounded-lg bg-cyber-bg border border-cyber-border">
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#e040fb" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
                    </svg>
                    <span className="text-xs font-semibold text-cyber-text-primary">Ad Blocker</span>
                  </div>
                  <p className="text-[10px] text-cyber-text-tertiary mt-1 ml-6">
                    Blocks ads, trackers, and analytics scripts in the built-in browser
                  </p>
                </div>
                <button
                  onClick={() => handleToggleAdblock(!adblockEnabled)}
                  disabled={!settingsLoaded}
                  style={{
                    width: "44px",
                    height: "24px",
                    borderRadius: "12px",
                    border: "none",
                    cursor: settingsLoaded ? "pointer" : "default",
                    background: adblockEnabled ? "#b400ff" : "#2a1e3a",
                    position: "relative",
                    transition: "background 0.2s",
                    flexShrink: 0,
                  }}
                >
                  <div
                    style={{
                      width: "18px",
                      height: "18px",
                      borderRadius: "50%",
                      background: "#fff",
                      position: "absolute",
                      top: "3px",
                      left: adblockEnabled ? "23px" : "3px",
                      transition: "left 0.2s",
                    }}
                  />
                </button>
              </div>

              {/* Popup blocker toggle */}
              <div className="flex items-center justify-between p-3 rounded-lg bg-cyber-bg border border-cyber-border">
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#00f5ff" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
                      <line x1="9" y1="9" x2="15" y2="15"/>
                      <line x1="15" y1="9" x2="9" y2="15"/>
                    </svg>
                    <span className="text-xs font-semibold text-cyber-text-primary">Popup Blocker</span>
                  </div>
                  <p className="text-[10px] text-cyber-text-tertiary mt-1 ml-6">
                    Prevents websites from opening unwanted popup windows
                  </p>
                </div>
                <button
                  onClick={() => handleTogglePopupBlocker(!popupBlockerEnabled)}
                  disabled={!settingsLoaded}
                  style={{
                    width: "44px",
                    height: "24px",
                    borderRadius: "12px",
                    border: "none",
                    cursor: settingsLoaded ? "pointer" : "default",
                    background: popupBlockerEnabled ? "#b400ff" : "#2a1e3a",
                    position: "relative",
                    transition: "background 0.2s",
                    flexShrink: 0,
                  }}
                >
                  <div
                    style={{
                      width: "18px",
                      height: "18px",
                      borderRadius: "50%",
                      background: "#fff",
                      position: "absolute",
                      top: "3px",
                      left: popupBlockerEnabled ? "23px" : "3px",
                      transition: "left 0.2s",
                    }}
                  />
                </button>
              </div>

              {/* Info */}
              <div className="p-3 rounded-lg bg-cyber-bg border border-cyber-border space-y-1">
                <p className="text-xs text-cyber-text-secondary font-semibold">How it works</p>
                <p className="text-[10px] text-cyber-text-tertiary leading-relaxed">
                  The ad blocker injects content-blocking scripts into the built-in browser. It blocks known ad
                  domains (DoubleClick, Google Ads, etc.), hides ad containers via CSS, and prevents ad-related
                  network requests. The popup blocker prevents sites from opening new windows via window.open().
                  Changes take effect immediately on the current page and all future pages.
                </p>
              </div>
            </div>
          )}

        </div>

        {/* Footer */}
        <div className="p-4 border-t border-cyber-border shrink-0">
          <button onClick={onClose} className="btn-shimmer px-4 py-2 text-sm font-medium bg-cyber-primary rounded-lg text-white hover:bg-cyber-primary-hover transition-all">
            Done
          </button>
        </div>
      </div>
    </div>
  );
}
