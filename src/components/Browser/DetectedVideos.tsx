import { useCallback } from "react";
import { useBrowserStore } from "../../stores/browserStore";
import { useDownloadStore } from "../../stores/downloadStore";
import { startBrowserDownload } from "../../lib/tauri";
import type { DetectedStream } from "../../lib/types";

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

export default function DetectedVideos() {
  const streams = useBrowserStore((s) => s.detectedStreams);
  const clearStreams = useBrowserStore((s) => s.clearDetectedStreams);
  const removeStream = useBrowserStore((s) => s.removeDetectedStream);
  const downloadFolder = useDownloadStore((s) => s.downloadFolder);
  const addItem = useDownloadStore((s) => s.addItem);

  const handleDownload = useCallback(
    async (stream: DetectedStream) => {
      const jobId = `browser-${Date.now()}`;
      const safeName =
        stream.pageTitle.replace(/[<>:"/\\|?*]/g, "_").slice(0, 80) ||
        "browser_download";

      addItem({
        id: jobId,
        url: stream.url,
        title: safeName,
        site_name: `Browser ${stream.type.toUpperCase()}`,
        status: "downloading",
        progress: 0,
        speed: "",
        eta: "",
        outputDir: downloadFolder,
        quality: "best",
        formatType: stream.type.toUpperCase(),
        logs: [],
        created_at: Date.now(),
      });

      try {
        await startBrowserDownload(
          jobId,
          stream.url,
          stream.type,
          stream.pageUrl,
          "best",
          downloadFolder,
          safeName
        );
      } catch (e) {
        console.error("Browser download failed:", e);
      }

      removeStream(stream.id);
    },
    [downloadFolder, addItem, removeStream]
  );

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
        {streams.map((stream) => (
          <div
            key={stream.id}
            style={{
              background: "var(--input-bg)",
              border: "1px solid var(--border-purple)",
              borderRadius: "3px",
              padding: "10px",
              marginBottom: "8px",
              animation: "float-in 0.3s ease-out",
            }}
          >
            {/* Type badge */}
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
                  background:
                    stream.type === "hls" ? "#00f5ff18" : "#b400ff18",
                  color: stream.type === "hls" ? "#00f5ff" : "#e040fb",
                  border: `1px solid ${stream.type === "hls" ? "#00f5ff44" : "#b400ff44"}`,
                  fontFamily: "'Orbitron', sans-serif",
                }}
              >
                {stream.type.toUpperCase()}
              </span>
              <button
                onClick={() => removeStream(stream.id)}
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

            {/* Download button */}
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
          </div>
        ))}
      </div>
    </div>
  );
}
