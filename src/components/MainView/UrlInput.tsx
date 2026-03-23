import { useState, useCallback } from "react";
import { analyzeUrl, startDownload } from "../../lib/tauri";
import { useDownloadStore } from "../../stores/downloadStore";

function detectBadge(url: string): string | null {
  if (!url.trim()) return null;
  if (url.includes("youtube.com") || url.includes("youtu.be")) return "YouTube";
  if (url.includes("tiktok.com")) return "TikTok";
  if (url.includes("twitter.com") || url.includes("x.com")) return "Twitter/X";
  if (url.includes("instagram.com")) return "Instagram";
  if (url.includes(".m3u8")) return "HLS Stream";
  if (url.includes(".mpd")) return "DASH Stream";
  if (url.startsWith("http")) return "Link";
  return null;
}

export default function UrlInput() {
  const [url, setUrl] = useState("");
  const [loading, setLoading] = useState(false);
  const addItem = useDownloadStore((s) => s.addItem);
  const updateItem = useDownloadStore((s) => s.updateItem);
  const downloadFolder = useDownloadStore((s) => s.downloadFolder);
  const badge = detectBadge(url);

  const handleSnatch = useCallback(async () => {
    const trimmed = url.trim();
    if (!trimmed || loading) return;

    const jobId = `dl-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;

    // Add to queue immediately
    addItem({
      id: jobId,
      url: trimmed,
      title: "Analyzing...",
      site_name: badge || "Unknown",
      status: "analyzing",
      progress: 0,
      speed: "—",
      eta: "—",
      outputDir: downloadFolder,
      quality: "best",
      logs: [],
      created_at: Date.now(),
    });

    setUrl("");
    setLoading(true);

    try {
      // Analyze URL to get title
      const analysis = await analyzeUrl(trimmed);
      const title = analysis.title || trimmed.split("/").pop() || "Download";
      updateItem(jobId, {
        title,
        site_name: analysis.site_name,
        status: "downloading",
      });

      // Start the actual download
      const outDir = downloadFolder || "";
      await startDownload(jobId, trimmed, outDir, "best");
    } catch (e) {
      updateItem(jobId, {
        title: url,
        status: "error",
        logs: [`Error: ${e}`],
      });
    }

    setLoading(false);
  }, [url, loading, addItem, updateItem, downloadFolder, badge]);

  return (
    <div className="px-5 pt-4 pb-2">
      <div className="relative">
        <input
          type="text"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleSnatch()}
          placeholder="Paste any URL..."
          className="w-full bg-cyber-surface border border-cyber-border rounded-lg px-4 py-3 text-[17px] text-cyber-text-primary placeholder:text-cyber-text-tertiary font-mono transition-all hover:border-cyber-primary/30 focus:border-cyber-primary/50"
        />
        {badge && (
          <span className="absolute right-3 top-1/2 -translate-y-1/2 text-[17px] font-semibold px-2 py-0.5 rounded bg-cyber-primary/15 text-cyber-primary border border-cyber-primary/20">
            {badge}
          </span>
        )}
      </div>
      <button
        onClick={handleSnatch}
        disabled={!url.trim() || loading}
        className="btn-shimmer w-full mt-2 bg-cyber-primary hover:bg-cyber-primary-hover disabled:bg-cyber-card disabled:text-cyber-text-tertiary text-white font-semibold text-[17px] py-2.5 rounded-lg transition-all disabled:cursor-not-allowed"
      >
        <span className="flex items-center justify-center gap-2">
          {loading ? (
            <span className="animate-pulse">Analyzing...</span>
          ) : (
            <>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
                <path d="M12 3v12m0 0l-4-4m4 4l4-4M5 17v2a2 2 0 002 2h10a2 2 0 002-2v-2" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
              Snatch
            </>
          )}
        </span>
      </button>
    </div>
  );
}
