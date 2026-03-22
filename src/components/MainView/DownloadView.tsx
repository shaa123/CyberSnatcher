import type { DownloadItem } from "../../lib/types";
import { getStatusLabel } from "../../hooks/useDownloads";
import ProgressBar from "./ProgressBar";
import StatsGrid from "./StatsGrid";
import { useState } from "react";

interface DownloadViewProps {
  item: DownloadItem;
}

const mockLogs = [
  { type: "info" as const, msg: "[info] Extracting URL: https://youtube.com/watch?v=example2" },
  { type: "info" as const, msg: "[info] Detected format: MP4 1080p (h264+aac)" },
  { type: "success" as const, msg: "[download] Downloading video stream..." },
  { type: "info" as const, msg: "[download] 67.0% of 386.5MB at 2.4MB/s ETA 01:23" },
];

export default function DownloadView({ item }: DownloadViewProps) {
  const [showLogs, setShowLogs] = useState(true);
  const statusLabel = getStatusLabel(item.status);
  const isActive = statusLabel === "Downloading" || statusLabel === "Converting";
  const isDone = statusLabel === "Done";
  const isFailed = statusLabel === "Failed";
  const isPaused = statusLabel === "Paused";

  const failedError =
    typeof item.status === "object" && "Failed" in item.status
      ? item.status.Failed.error
      : null;

  return (
    <div className="flex-1 flex flex-col p-5 overflow-y-auto">
      {/* Header */}
      <div className="mb-5">
        <div className="flex items-center gap-2 mb-1">
          <span className="text-[10px] font-semibold px-1.5 py-0.5 rounded bg-cyber-primary/15 text-cyber-primary border border-cyber-primary/20">
            {item.site_name}
          </span>
          <span className="text-[10px] text-cyber-text-tertiary font-mono truncate">
            {item.url}
          </span>
        </div>
        <h1 className="text-xl font-bold text-cyber-text-primary">{item.title}</h1>
      </div>

      {/* Large progress bar */}
      <div className="mb-2">
        <div className="flex items-center justify-between mb-2">
          <span className="text-sm text-cyber-text-secondary">Progress</span>
          <span className="text-sm font-semibold font-mono text-cyber-primary">
            {item.progress}%
          </span>
        </div>
        <ProgressBar progress={item.progress} large />
      </div>

      {/* Stats */}
      <div className="mt-4 mb-4">
        <StatsGrid
          speed={item.speed}
          downloaded={`${Math.round((parseFloat(item.file_size) || 0) * item.progress / 100)} MB`}
          eta={item.eta}
          fileSize={item.file_size}
          format={item.format}
        />
      </div>

      {/* Action buttons */}
      <div className="flex gap-2 mb-4">
        {isActive && (
          <>
            <button className="btn-shimmer px-4 py-2 text-sm font-medium bg-cyber-card border border-cyber-border rounded-lg text-cyber-text-primary hover:bg-cyber-card-hover hover:border-cyber-primary/30 transition-all">
              Pause
            </button>
            <button className="px-4 py-2 text-sm font-medium bg-cyber-error/10 border border-cyber-error/20 rounded-lg text-cyber-error hover:bg-cyber-error/20 transition-all">
              Cancel
            </button>
          </>
        )}
        {isPaused && (
          <button className="btn-shimmer px-4 py-2 text-sm font-medium bg-cyber-primary rounded-lg text-white hover:bg-cyber-primary-hover transition-all">
            Resume
          </button>
        )}
        {isDone && (
          <button className="btn-shimmer px-4 py-2 text-sm font-medium bg-cyber-success/10 border border-cyber-success/20 rounded-lg text-cyber-success hover:bg-cyber-success/20 transition-all">
            Open File
          </button>
        )}
        {isFailed && (
          <>
            <button className="btn-shimmer px-4 py-2 text-sm font-medium bg-cyber-primary rounded-lg text-white hover:bg-cyber-primary-hover transition-all">
              Retry
            </button>
            {failedError && (
              <span className="flex items-center text-xs text-cyber-error font-mono">
                {failedError}
              </span>
            )}
          </>
        )}
      </div>

      {/* Log output */}
      <div className="flex-1 min-h-0">
        <button
          onClick={() => setShowLogs(!showLogs)}
          className="flex items-center gap-1.5 text-xs text-cyber-text-tertiary hover:text-cyber-text-secondary transition-colors mb-2"
        >
          <svg
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            className={`transition-transform ${showLogs ? "rotate-90" : ""}`}
          >
            <path
              d="M9 18l6-6-6-6"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
          Output Log
        </button>

        {showLogs && (
          <div className="bg-cyber-bg border border-cyber-border rounded-lg p-3 max-h-48 overflow-y-auto terminal-log">
            {mockLogs.map((log, i) => (
              <div key={i} className={`log-${log.type}`}>
                {log.msg}
              </div>
            ))}
            <div className="text-cyber-text-tertiary mt-1">
              <span className="animate-pulse">▊</span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
