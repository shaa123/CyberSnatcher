import { useCallback, useState, useEffect } from "react";
import { useBrowserStore } from "../../stores/browserStore";
import { useDownloadStore } from "../../stores/downloadStore";
import { startBrowserDownload } from "../../lib/tauri";
import { listen } from "@tauri-apps/api/event";
import type { DetectedStream, DownloadProgress } from "../../lib/types";
import { downloadDir } from "@tauri-apps/api/path";

function formatDuration(seconds: number | null): string {
  if (seconds == null) return "—";
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

function formatSize(bytes: number | null): string {
  if (bytes == null) return "—";
  if (bytes > 1073741824) return `${(bytes / 1073741824).toFixed(1)} GB`;
  return `${(bytes / 1048576).toFixed(1)} MB`;
}

interface DownloadState {
  jobId: string;
  percent: number;
  speed: string;
  status: "downloading" | "converting" | "complete" | "error";
  filePath?: string;
}

export default function DetectedVideos() {
  const streams = useBrowserStore((s) => s.detectedStreams);
  const clearStreams = useBrowserStore((s) => s.clearDetectedStreams);
  const removeStream = useBrowserStore((s) => s.removeDetectedStream);
  const downloadFolder = useDownloadStore((s) => s.downloadFolder);

  // Track download progress per stream (by stream id)
  const [downloads, setDownloads] = useState<Record<string, DownloadState>>({});
  // Map jobId -> streamId for progress routing
  const [jobToStream, setJobToStream] = useState<Record<string, string>>({});

  // Listen for browser download progress events
  useEffect(() => {
    const unlisten = listen<DownloadProgress>("download-progress", (event) => {
      const p = event.payload;
      if (!p.job_id.startsWith("browser-")) return;

      // Find which stream this job belongs to
      setJobToStream((mapping) => {
        const streamId = mapping[p.job_id];
        if (!streamId) return mapping;

        setDownloads((prev) => {
          const current = prev[streamId];
          if (!current) return prev;

          if (p.status === "complete") {
            return {
              ...prev,
              [streamId]: { ...current, percent: 100, status: "complete", speed: "", filePath: p.file_path || undefined },
            };
          } else if (p.status === "error" || p.status === "cancelled") {
            return {
              ...prev,
              [streamId]: { ...current, status: "error", speed: p.log_line || "Error" },
            };
          } else if (p.status === "converting") {
            return {
              ...prev,
              [streamId]: { ...current, percent: 95, status: "converting", speed: "Muxing..." },
            };
          } else if (p.percent >= 0) {
            return {
              ...prev,
              [streamId]: {
                ...current,
                percent: p.percent,
                speed: p.speed || current.speed,
                status: "downloading",
              },
            };
          }
          return prev;
        });

        return mapping;
      });
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  const handleDownload = useCallback(
    async (stream: DetectedStream) => {
      const jobId = `browser-${Date.now()}`;
      const safeName =
        stream.pageTitle.replace(/[<>:"/\\|?*]/g, "_").slice(0, 80) ||
        "browser_download";

      // Register job -> stream mapping
      setJobToStream((prev) => ({ ...prev, [jobId]: stream.id }));
      setDownloads((prev) => ({
        ...prev,
        [stream.id]: { jobId, percent: 0, speed: "Starting...", status: "downloading" },
      }));

      // Ensure we have a valid download folder
      let folder = downloadFolder;
      if (!folder) {
        try {
          folder = await downloadDir();
        } catch {
          folder = "";
        }
      }
      if (!folder) {
        setDownloads((prev) => ({
          ...prev,
          [stream.id]: { jobId, percent: 0, speed: "No download folder", status: "error" },
        }));
        return;
      }

      try {
        await startBrowserDownload(
          jobId,
          stream.url,
          stream.type,
          stream.pageUrl,
          "best",
          folder,
          safeName
        );
      } catch (e) {
        setDownloads((prev) => ({
          ...prev,
          [stream.id]: { jobId, percent: 0, speed: String(e), status: "error" },
        }));
      }
    },
    [downloadFolder]
  );

  const handleDismissCompleted = useCallback((streamId: string) => {
    removeStream(streamId);
    setDownloads((prev) => {
      const next = { ...prev };
      delete next[streamId];
      return next;
    });
  }, [removeStream]);

  // Only render when there are detected streams
  if (streams.length === 0) return null;

  return (
    <div
      style={{
        width: "280px",
        flexShrink: 0,
        background: "var(--panel-alt)",
        borderLeft: "1px solid var(--border-purple)",
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
        animation: "slide-in-right 0.3s ease-out",
      }}
    >
      {/* Header */}
      <div
        style={{
          padding: "10px 12px",
          borderBottom: "1px solid var(--border-purple)",
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
          <span
            style={{
              width: "6px",
              height: "6px",
              borderRadius: "50%",
              background: "#00f5ff",
              boxShadow: "0 0 8px #00f5ff",
              animation: "blink 1.5s infinite",
            }}
          />
          <span
            style={{
              fontSize: "10px",
              letterSpacing: "2px",
              color: "var(--cyan)",
              fontFamily: "'Orbitron', sans-serif",
              fontWeight: 700,
            }}
          >
            {streams.length} STREAM{streams.length > 1 ? "S" : ""} FOUND
          </span>
        </div>
        <button
          onClick={clearStreams}
          style={{
            background: "transparent",
            border: "none",
            color: "var(--text-dimmer)",
            cursor: "pointer",
            fontSize: "9px",
            letterSpacing: "1px",
          }}
        >
          CLEAR
        </button>
      </div>

      {/* Stream list */}
      <div style={{ flex: 1, overflowY: "auto", padding: "8px" }}>
        {streams.map((stream) => {
          const dl = downloads[stream.id];
          const isDownloading = dl && dl.status === "downloading";
          const isConverting = dl && dl.status === "converting";
          const isComplete = dl && dl.status === "complete";
          const isError = dl && dl.status === "error";
          const isActive = isDownloading || isConverting;

          return (
            <div
              key={stream.id}
              style={{
                background: "var(--input-bg)",
                border: `1px solid ${isComplete ? "#00ff8844" : isError ? "#ff003c44" : isActive ? "#00f5ff44" : "var(--border-purple)"}`,
                borderRadius: "3px",
                padding: "10px",
                marginBottom: "8px",
                animation: "float-in 0.3s ease-out",
              }}
            >
              {/* Type badge + status */}
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  marginBottom: "6px",
                }}
              >
                <span
                  style={{
                    fontSize: "9px",
                    fontWeight: 700,
                    letterSpacing: "1px",
                    padding: "2px 6px",
                    borderRadius: "2px",
                    background: stream.type === "hls" ? "#00f5ff18" : "#b400ff18",
                    color: stream.type === "hls" ? "#00f5ff" : "#e040fb",
                    border: `1px solid ${stream.type === "hls" ? "#00f5ff44" : "#b400ff44"}`,
                    fontFamily: "'Orbitron', sans-serif",
                  }}
                >
                  {stream.type.toUpperCase()}
                </span>
                {!isActive && (
                  <button
                    onClick={() => isComplete ? handleDismissCompleted(stream.id) : removeStream(stream.id)}
                    style={{
                      background: "transparent",
                      border: "none",
                      color: "var(--text-dimmer)",
                      cursor: "pointer",
                      fontSize: "11px",
                      padding: "0 2px",
                    }}
                  >
                    ✕
                  </button>
                )}
              </div>

              {/* Title */}
              <div
                style={{
                  fontSize: "11px",
                  color: "var(--text)",
                  whiteSpace: "nowrap",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  marginBottom: "4px",
                }}
                title={stream.pageTitle}
              >
                {stream.pageTitle || "Unknown"}
              </div>

              {/* Stats */}
              <div
                style={{
                  display: "flex",
                  gap: "10px",
                  marginBottom: "8px",
                  fontSize: "10px",
                  color: "var(--text-dim)",
                }}
              >
                <span>⏱ {formatDuration(stream.estimatedDuration)}</span>
                <span>◈ {formatSize(stream.estimatedSize)}</span>
              </div>

              {/* Progress bar (when downloading) */}
              {isActive && dl && (
                <div style={{ marginBottom: "6px" }}>
                  <div style={{
                    display: "flex",
                    justifyContent: "space-between",
                    marginBottom: "3px",
                    fontSize: "9px",
                  }}>
                    <span style={{ color: isConverting ? "#b400ff" : "#00f5ff", letterSpacing: "1px" }}>
                      {isConverting ? "MUXING" : "DOWNLOADING"}
                      {dl.speed ? ` · ${dl.speed}` : ""}
                    </span>
                    <span style={{ color: "#00f5ff", fontWeight: 700 }}>
                      {Math.round(dl.percent)}%
                    </span>
                  </div>
                  <div style={{
                    height: "3px",
                    background: "var(--border-dim)",
                    borderRadius: "2px",
                    overflow: "hidden",
                  }}>
                    <div style={{
                      height: "100%",
                      width: `${dl.percent}%`,
                      background: isConverting
                        ? "linear-gradient(90deg, #b400ff, #e040fb)"
                        : "linear-gradient(90deg, #00f5ff, #00f5ff)",
                      borderRadius: "2px",
                      transition: "width 0.3s ease",
                      boxShadow: `0 0 6px ${isConverting ? "#b400ff" : "#00f5ff"}`,
                    }} />
                  </div>
                </div>
              )}

              {/* Complete state */}
              {isComplete && (
                <div style={{
                  padding: "4px 8px",
                  background: "#00ff8812",
                  border: "1px solid #00ff8833",
                  borderRadius: "2px",
                  fontSize: "9px",
                  color: "#00ff88",
                  fontFamily: "'Orbitron', sans-serif",
                  fontWeight: 700,
                  letterSpacing: "2px",
                  textAlign: "center",
                }}>
                  COMPLETE
                </div>
              )}

              {/* Error state */}
              {isError && dl && (
                <div style={{ marginBottom: "6px" }}>
                  <div style={{
                    padding: "4px 8px",
                    background: "#ff003c12",
                    border: "1px solid #ff003c33",
                    borderRadius: "2px",
                    fontSize: "9px",
                    color: "#ff003c",
                    letterSpacing: "1px",
                    marginBottom: "4px",
                    whiteSpace: "nowrap",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                  }}
                    title={dl.speed}
                  >
                    ERROR: {dl.speed}
                  </div>
                  <button
                    onClick={() => {
                      setDownloads((prev) => {
                        const next = { ...prev };
                        delete next[stream.id];
                        return next;
                      });
                    }}
                    style={{
                      width: "100%",
                      padding: "4px",
                      background: "transparent",
                      border: "1px solid #ff003c33",
                      borderRadius: "2px",
                      color: "#ff003c",
                      fontFamily: "'Orbitron', sans-serif",
                      fontSize: "8px",
                      fontWeight: 700,
                      letterSpacing: "1px",
                      cursor: "pointer",
                    }}
                  >
                    RETRY
                  </button>
                </div>
              )}

              {/* Download button (only when not downloading) */}
              {!dl && (
                <button
                  onClick={() => handleDownload(stream)}
                  style={{
                    width: "100%",
                    padding: "6px",
                    background: "linear-gradient(135deg, #00f5ff15, #00f5ff08)",
                    border: "1px solid #00f5ff44",
                    borderRadius: "2px",
                    color: "#00f5ff",
                    fontFamily: "'Orbitron', sans-serif",
                    fontSize: "9px",
                    fontWeight: 700,
                    letterSpacing: "2px",
                    cursor: "pointer",
                    transition: "all 0.2s",
                  }}
                  onMouseEnter={(e) =>
                    (e.currentTarget.style.background = "#00f5ff22")
                  }
                  onMouseLeave={(e) =>
                    (e.currentTarget.style.background =
                      "linear-gradient(135deg, #00f5ff15, #00f5ff08)")
                  }
                >
                  DOWNLOAD
                </button>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
