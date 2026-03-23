import type { DownloadItem } from "../../lib/types";
import { getStatusLabel, getStatusColor } from "../../hooks/useDownloads";

interface QueueItemProps {
  item: DownloadItem;
  isSelected: boolean;
  onSelect: (id: string) => void;
}

export default function QueueItem({ item, isSelected, onSelect }: QueueItemProps) {
  const statusLabel = getStatusLabel(item.status);
  const statusColors = getStatusColor(item);
  const isDownloading = statusLabel === "Downloading";

  return (
    <button
      onClick={() => onSelect(item.id)}
      className={`w-full text-left p-2.5 rounded-lg border transition-all ${
        isSelected
          ? "bg-cyber-card-hover border-cyber-primary/30 shadow-cyber-card"
          : "bg-transparent border-transparent hover:bg-cyber-card hover:border-cyber-border"
      }`}
    >
      <div className="flex gap-2.5">
        {/* Thumbnail placeholder */}
        <div className="w-10 h-10 rounded bg-cyber-card border border-cyber-border shrink-0 flex items-center justify-center">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
            <rect
              x="2"
              y="4"
              width="20"
              height="16"
              rx="2"
              stroke="currentColor"
              strokeWidth="1.5"
              className="text-cyber-text-tertiary"
            />
            <polygon
              points="10,9 16,12 10,15"
              fill="currentColor"
              className="text-cyber-text-tertiary"
            />
          </svg>
        </div>

        {/* Info */}
        <div className="flex-1 min-w-0">
          <p className="text-[17px] font-medium text-cyber-text-primary truncate">
            {item.title}
          </p>
          <div className="flex items-center gap-1.5 mt-1">
            <span
              className={`text-[17px] font-semibold px-1.5 py-0.5 rounded ${statusColors} ${
                isDownloading ? "status-downloading" : ""
              }`}
            >
              {statusLabel}
            </span>
            {isDownloading && (
              <span className="text-[17px] text-cyber-text-secondary">
                {item.speed}
              </span>
            )}
          </div>

          {/* Progress bar (only for downloading/converting) */}
          {(statusLabel === "Downloading" || statusLabel === "Converting") && (
            <div className="mt-1.5 h-1 bg-cyber-bg rounded-full overflow-hidden">
              <div
                className="h-full progress-gradient rounded-full transition-all duration-300"
                style={{ width: `${item.progress}%` }}
              />
            </div>
          )}
        </div>
      </div>
    </button>
  );
}
