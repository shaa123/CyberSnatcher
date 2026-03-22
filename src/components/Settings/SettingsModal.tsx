import { useState } from "react";
import { useDownloadStore } from "../../stores/downloadStore";
import { useDownloads } from "../../hooks/useDownloads";

interface SettingsModalProps {
  onClose: () => void;
  ytdlpInstalled: boolean;
}

export default function SettingsModal({ onClose, ytdlpInstalled }: SettingsModalProps) {
  const downloadFolder = useDownloadStore((s) => s.downloadFolder);
  const setDownloadFolder = useDownloadStore((s) => s.setDownloadFolder);
  const { items, selectedId } = useDownloads();
  const [tab, setTab] = useState<"general" | "logs">("general");
  const [folderInput, setFolderInput] = useState(downloadFolder);

  const selectedItem = items.find((i) => i.id === selectedId);

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
          <button
            onClick={() => setTab("general")}
            className={`flex-1 py-2 text-xs font-semibold transition-colors ${
              tab === "general" ? "text-cyber-primary border-b-2 border-cyber-primary" : "text-cyber-text-tertiary hover:text-cyber-text-secondary"
            }`}
          >
            General
          </button>
          <button
            onClick={() => setTab("logs")}
            className={`flex-1 py-2 text-xs font-semibold transition-colors ${
              tab === "logs" ? "text-cyber-primary border-b-2 border-cyber-primary" : "text-cyber-text-tertiary hover:text-cyber-text-secondary"
            }`}
          >
            Output Log
          </button>
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

          {tab === "logs" && (
            <div>
              {selectedItem ? (
                <>
                  <p className="text-xs text-cyber-text-secondary mb-2 font-semibold">
                    Logs for: {selectedItem.title}
                  </p>
                  <div className="bg-cyber-bg border border-cyber-border rounded-lg p-3 max-h-[300px] overflow-y-auto terminal-log">
                    {selectedItem.logs.length === 0 ? (
                      <span className="text-cyber-text-tertiary">No log output yet.</span>
                    ) : (
                      selectedItem.logs.map((line, i) => (
                        <div key={i} className={
                          line.includes("[download]") ? "log-success" :
                          line.includes("[info]") ? "log-info" :
                          line.includes("Error") || line.includes("error") ? "log-error" :
                          ""
                        }>{line}</div>
                      ))
                    )}
                    {selectedItem.status === "downloading" && (
                      <div className="text-cyber-text-tertiary mt-1"><span className="animate-pulse">▊</span></div>
                    )}
                  </div>
                </>
              ) : (
                <div className="text-xs text-cyber-text-tertiary text-center py-8">
                  Select a download to view its output log
                </div>
              )}
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
