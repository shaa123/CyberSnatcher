import { useDownloads } from "../../hooks/useDownloads";
import { useDownloadStore } from "../../stores/downloadStore";
import { cancelDownload } from "../../lib/tauri";
import ProgressBar from "./ProgressBar";
import type { DownloadItem } from "../../lib/types";

function statusBadge(status: DownloadItem["status"]) {
  switch (status) {
    case "analyzing": return { text: "Analyzing", cls: "text-cyber-info bg-cyber-info/10" };
    case "downloading": return { text: "Downloading", cls: "text-cyber-primary bg-cyber-primary/10 status-downloading" };
    case "complete": return { text: "Done", cls: "text-cyber-success bg-cyber-success/10" };
    case "error": return { text: "Failed", cls: "text-cyber-error bg-cyber-error/10" };
    case "cancelled": return { text: "Cancelled", cls: "text-cyber-text-tertiary bg-cyber-card" };
  }
}

export default function DownloadQueue() {
  const { items, selectedId, selectItem } = useDownloads();
  const removeItem = useDownloadStore((s) => s.removeItem);

  if (items.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <p className="text-xs text-cyber-text-tertiary text-center px-8">
          Paste a URL above and hit Snatch to start downloading
        </p>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto">
      {items.map((item) => {
        const badge = statusBadge(item.status);
        const isSelected = selectedId === item.id;
        const isActive = item.status === "downloading" || item.status === "analyzing";

        return (
          <div
            key={item.id}
            onClick={() => selectItem(item.id)}
            className={`px-5 py-3 border-b border-cyber-border cursor-pointer transition-all ${
              isSelected ? "bg-cyber-card" : "hover:bg-cyber-surface"
            }`}
          >
            <div className="flex items-start justify-between gap-2">
              <div className="flex-1 min-w-0">
                <p className="text-sm text-cyber-text-primary truncate font-medium">
                  {item.title}
                </p>
                <div className="flex items-center gap-2 mt-1">
                  <span className={`text-[10px] font-semibold px-1.5 py-0.5 rounded ${badge.cls}`}>
                    {badge.text}
                  </span>
                  <span className="text-[10px] text-cyber-text-tertiary">{item.site_name}</span>
                  {isActive && item.speed !== "—" && (
                    <span className="text-[10px] text-cyber-text-secondary font-mono">{item.speed}</span>
                  )}
                </div>
              </div>
              <div className="flex items-center gap-1 shrink-0">
                {isActive && (
                  <button
                    onClick={(e) => { e.stopPropagation(); cancelDownload(item.id); }}
                    className="text-cyber-text-tertiary hover:text-cyber-error text-xs transition-colors p-1"
                    title="Cancel"
                  >
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                      <path d="M18 6L6 18M6 6l12 12" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
                    </svg>
                  </button>
                )}
                {!isActive && (
                  <button
                    onClick={(e) => { e.stopPropagation(); removeItem(item.id); }}
                    className="text-cyber-text-tertiary hover:text-cyber-error text-xs transition-colors p-1"
                    title="Remove"
                  >
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                      <path d="M18 6L6 18M6 6l12 12" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
                    </svg>
                  </button>
                )}
              </div>
            </div>
            {(item.status === "downloading") && (
              <div className="mt-2 flex items-center gap-3">
                <div className="flex-1">
                  <ProgressBar progress={item.progress} />
                </div>
                <span className="text-[10px] font-mono text-cyber-primary shrink-0">
                  {Math.round(item.progress)}%
                </span>
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
