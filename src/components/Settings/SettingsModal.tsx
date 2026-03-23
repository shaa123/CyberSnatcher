import { useState } from "react";
import { useDownloadStore } from "../../stores/downloadStore";
import GeneralTab from "./GeneralTab";
import SupportedSitesTab from "./SupportedSitesTab";
import LogsTab from "./LogsTab";

interface SettingsModalProps {
  onClose: () => void;
  ytdlpInstalled: boolean;
  logs: string[];
}

export default function SettingsModal({ onClose, ytdlpInstalled, logs }: SettingsModalProps) {
  const downloadFolder = useDownloadStore((s) => s.downloadFolder);
  const setDownloadFolder = useDownloadStore((s) => s.setDownloadFolder);

  const [folderInput, setFolderInput] = useState(downloadFolder);
  const [activeTab, setActiveTab] = useState<"general" | "sites" | "logs">("general");

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={onClose} />
      <div className="relative bg-cyber-surface border border-cyber-border rounded-xl w-[560px] max-h-[80vh] overflow-hidden shadow-2xl flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-cyber-border shrink-0">
          <h2 className="text-[17px] font-bold text-cyber-text-primary">Settings</h2>
          <button onClick={onClose} className="text-cyber-text-tertiary hover:text-cyber-text-primary transition-colors">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
              <path d="M18 6L6 18M6 6l12 12" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
            </svg>
          </button>
        </div>

        {/* Tabs */}
        <div className="flex border-b border-cyber-border shrink-0">
          <button
            onClick={() => setActiveTab("general")}
            className={`px-4 py-2.5 text-[15px] font-medium transition-colors ${
              activeTab === "general"
                ? "text-cyber-primary border-b-2 border-cyber-primary"
                : "text-cyber-text-tertiary hover:text-cyber-text-secondary"
            }`}
          >
            General
          </button>
          <button
            onClick={() => setActiveTab("sites")}
            className={`px-4 py-2.5 text-[15px] font-medium transition-colors ${
              activeTab === "sites"
                ? "text-cyber-primary border-b-2 border-cyber-primary"
                : "text-cyber-text-tertiary hover:text-cyber-text-secondary"
            }`}
          >
            Supported Sites
          </button>
          <button
            onClick={() => setActiveTab("logs")}
            className={`px-4 py-2.5 text-[15px] font-medium transition-colors ${
              activeTab === "logs"
                ? "text-cyber-primary border-b-2 border-cyber-primary"
                : "text-cyber-text-tertiary hover:text-cyber-text-secondary"
            }`}
          >
            Logs
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-4">
          {activeTab === "general" ? (
            <GeneralTab
              ytdlpInstalled={ytdlpInstalled}
              folderInput={folderInput}
              setFolderInput={setFolderInput}
              onSaveFolder={() => setDownloadFolder(folderInput)}
            />
          ) : activeTab === "sites" ? (
            <SupportedSitesTab />
          ) : (
            <LogsTab logs={logs} />
          )}
        </div>

        {/* Footer */}
        <div className="p-4 border-t border-cyber-border shrink-0">
          <button onClick={onClose} className="btn-shimmer px-4 py-2 text-[17px] font-medium bg-cyber-primary rounded-lg text-white hover:bg-cyber-primary-hover transition-all">
            Done
          </button>
        </div>
      </div>
    </div>
  );
}
