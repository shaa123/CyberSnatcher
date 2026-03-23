import { useState } from "react";
import { useDownloadStore } from "../../stores/downloadStore";
import { useBrowserStore } from "../../stores/browserStore";
import { saveBrowserSettings } from "../../lib/tauri";

interface SettingsModalProps {
  onClose: () => void;
  ytdlpInstalled: boolean;
}

export default function SettingsModal({ onClose, ytdlpInstalled }: SettingsModalProps) {
  const downloadFolder = useDownloadStore((s) => s.downloadFolder);
  const setDownloadFolder = useDownloadStore((s) => s.setDownloadFolder);
  const browserSettings = useBrowserStore((s) => s.browserSettings);
  const setBrowserSettings = useBrowserStore((s) => s.setBrowserSettings);

  const [folderInput, setFolderInput] = useState(downloadFolder);
  const [minDuration, setMinDuration] = useState(browserSettings.minDuration);
  const [minSizeMB, setMinSizeMB] = useState(browserSettings.minFileSize / 1048576);

  const handleSaveBrowserSettings = () => {
    const sizeBytes = Math.round(minSizeMB * 1048576);
    setBrowserSettings({ minDuration, minFileSize: sizeBytes });
    saveBrowserSettings(minDuration, sizeBytes).catch(() => {});
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

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-4">
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

            {/* Browser Detection Settings */}
            <div>
              <label className="block text-xs font-semibold text-cyber-text-secondary mb-1.5" style={{ letterSpacing: "1px", color: "var(--cyan)" }}>
                Browser Stream Detection
              </label>
              <div className="p-3 rounded-lg bg-cyber-bg border border-cyber-border space-y-3">
                <div>
                  <div className="flex justify-between items-center mb-1">
                    <span className="text-[11px] text-cyber-text-secondary">Min Duration</span>
                    <span className="text-[11px] font-mono" style={{ color: "var(--cyan)" }}>{minDuration}s</span>
                  </div>
                  <input
                    type="range"
                    min={5}
                    max={300}
                    step={5}
                    value={minDuration}
                    onChange={(e) => setMinDuration(Number(e.target.value))}
                    style={{
                      width: "100%", height: "4px", appearance: "none", WebkitAppearance: "none",
                      background: `linear-gradient(90deg, #00f5ff ${(minDuration / 300) * 100}%, var(--border-dim) ${(minDuration / 300) * 100}%)`,
                      borderRadius: "2px", outline: "none", cursor: "pointer",
                    }}
                  />
                  <div className="flex justify-between text-[9px] text-cyber-text-tertiary mt-0.5">
                    <span>5s</span>
                    <span>300s</span>
                  </div>
                </div>
                <div>
                  <div className="flex justify-between items-center mb-1">
                    <span className="text-[11px] text-cyber-text-secondary">Min File Size</span>
                    <span className="text-[11px] font-mono" style={{ color: "var(--cyan)" }}>{minSizeMB} MB</span>
                  </div>
                  <input
                    type="range"
                    min={0.5}
                    max={50}
                    step={0.5}
                    value={minSizeMB}
                    onChange={(e) => setMinSizeMB(Number(e.target.value))}
                    style={{
                      width: "100%", height: "4px", appearance: "none", WebkitAppearance: "none",
                      background: `linear-gradient(90deg, #00f5ff ${(minSizeMB / 50) * 100}%, var(--border-dim) ${(minSizeMB / 50) * 100}%)`,
                      borderRadius: "2px", outline: "none", cursor: "pointer",
                    }}
                  />
                  <div className="flex justify-between text-[9px] text-cyber-text-tertiary mt-0.5">
                    <span>0.5 MB</span>
                    <span>50 MB</span>
                  </div>
                </div>
                <button
                  onClick={handleSaveBrowserSettings}
                  className="w-full px-3 py-1.5 bg-cyber-primary/10 border border-cyber-primary/30 rounded text-[11px] text-cyber-primary hover:bg-cyber-primary/20 transition-all"
                  style={{ letterSpacing: "1px" }}
                >
                  Apply Detection Settings
                </button>
              </div>
            </div>

            {/* Info */}
            <div className="p-3 rounded-lg bg-cyber-bg border border-cyber-border space-y-1">
              <p className="text-xs text-cyber-text-secondary font-semibold">How it works</p>
              <p className="text-[10px] text-cyber-text-tertiary leading-relaxed">
                CyberSnatcher uses yt-dlp under the hood to download videos. Make sure yt-dlp is installed and in your PATH.
                Paste any URL from YouTube, Twitter/X, TikTok, Instagram, Reddit, or direct video links.
                The Browser tab auto-detects HLS/DASH streams and downloads them natively (no yt-dlp).
              </p>
            </div>
          </div>
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
