import { useDownloads } from "../hooks/useDownloads";

export default function StatusBar({ ytdlpOk }: { ytdlpOk: boolean }) {
  const { activeCount, completedCount } = useDownloads();

  return (
    <div className="h-7 bg-cyber-surface border-t border-cyber-border flex items-center justify-between px-3 text-[10px] shrink-0">
      <span className="text-cyber-text-tertiary flex items-center gap-1.5">
        {!ytdlpOk ? (
          <><span className="w-1.5 h-1.5 rounded-full bg-cyber-error" /> yt-dlp not found</>
        ) : activeCount > 0 ? (
          <><span className="w-1.5 h-1.5 rounded-full bg-cyber-primary animate-pulse" /> Downloading...</>
        ) : (
          "Ready"
        )}
      </span>
      <span className="text-cyber-text-tertiary">
        {completedCount} completed · {activeCount} active
      </span>
    </div>
  );
}
