import { useState, useEffect, useRef, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  openBrowserView,
  navigateBrowser,
  resizeBrowser,
  browserGoBack,
  browserGoForward,
  browserRefresh,
  showBrowser,
  hideBrowser,
  openFile,
  showInFolder,
  parseHls,
  downloadHlsStream,
  downloadHlsLiveStream,
  parseDash,
  downloadDashStream,
  nativeDownload,
  getBrowserCookies,
  removeDetectedVideo,
  startRecording,
  stopRecording,


} from "../lib/tauri";
import type { DetectedVideo } from "../lib/types";
import type { HlsQuality } from "../lib/tauri";

interface DownloadJob {
  jobId: string;
  url: string;
  title: string;
  percent: number;
  status: string;
  logLine: string;
  speed: string;
  eta: string;
  error: string | null;
  filePath: string | null;
}

interface Props {
  visible: boolean;
  downloadFolder: string;
}

export default function BrowserTab({ visible, downloadFolder }: Props) {
  const [urlInput, setUrlInput] = useState("https://www.google.com");
  const [currentUrl, setCurrentUrl] = useState("");
  const [loading, setLoading] = useState(false);
  const [browserOpen, setBrowserOpen] = useState(false);
  const [detected, setDetected] = useState<DetectedVideo[]>([]);
  const [hlsQualities, setHlsQualities] = useState<HlsQuality[] | null>(null);
  const [hlsPendingUrl, setHlsPendingUrl] = useState<string | null>(null);
  const [hlsPendingTitle, setHlsPendingTitle] = useState<string>("");
  const [hlsPendingCookies, setHlsPendingCookies] = useState<string | null>(null);
  const [hlsPendingPageUrl, setHlsPendingPageUrl] = useState<string | null>(null);
  const [downloading, setDownloading] = useState<string | null>(null);
  const [activeJob, setActiveJob] = useState<DownloadJob | null>(null);
  const [recording, setRecording] = useState(false);
  const [recordingResult, setRecordingResult] = useState<string | null>(null);
  const viewportRef = useRef<HTMLDivElement>(null);

  // ── Show/hide browser webview when tab switches ──
  useEffect(() => {
    if (visible && browserOpen) {
      showBrowser().catch(() => {});
      updatePosition();
    } else if (!visible && browserOpen) {
      hideBrowser().catch(() => {});
    }
  }, [visible, browserOpen]);

  // ── Listen for events from Rust ──
  useEffect(() => {
    const unsubs: Promise<() => void>[] = [];

    unsubs.push(
      listen<DetectedVideo>("browser-video-detected", (e) => {
        setDetected((prev) => {
          // Dedup: check full URL and also base path (without query params)
          const newUrl = e.payload.url;
          const newBase = newUrl.split("?")[0];
          if (prev.some((v) => v.url === newUrl || v.url.split("?")[0] === newBase)) return prev;
          return [...prev, e.payload];
        });
      })
    );

    unsubs.push(listen<string>("browser-url-changed", (e) => {
      setCurrentUrl(e.payload);
      setUrlInput(e.payload);
      setLoading(true);
    }));

    unsubs.push(listen<string>("browser-page-loaded", () => { setLoading(false); }));
    unsubs.push(listen("browser-videos-cleared", () => { setDetected([]); }));

    // Listen for download progress
    unsubs.push(
      listen<any>("download-progress", (e) => {
        const p = e.payload;
        setActiveJob((prev) => {
          if (!prev || prev.jobId !== p.job_id) return prev;
          return {
            ...prev,
            percent: p.percent >= 0 ? p.percent : prev.percent,
            status: p.status,
            logLine: p.log_line,
            speed: p.speed || prev.speed,
            eta: p.eta || prev.eta,
            filePath: p.file_path || prev.filePath,
            error: p.status === "error" ? p.log_line : null,
          };
        });
        if (p.status === "complete") {
          setTimeout(() => { setActiveJob(null); setDownloading(null); }, 3000);
        }
      })
    );

    return () => { unsubs.forEach((p) => p.then((fn) => fn())); };
  }, []);

  // ── Resize observer ──
  const updatePosition = useCallback(() => {
    if (!viewportRef.current || !browserOpen) return;
    const rect = viewportRef.current.getBoundingClientRect();
    if (rect.width > 0 && rect.height > 0) {
      resizeBrowser(rect.x, rect.y, rect.width, rect.height).catch(() => {});
    }
  }, [browserOpen]);

  useEffect(() => {
    if (!viewportRef.current || !browserOpen || !visible) return;
    const observer = new ResizeObserver(() => updatePosition());
    observer.observe(viewportRef.current);
    window.addEventListener("resize", updatePosition);
    updatePosition();
    return () => { observer.disconnect(); window.removeEventListener("resize", updatePosition); };
  }, [browserOpen, visible, updatePosition]);

  // ── Recording: capture the browser viewport area ──
  const getViewportScreenCoords = useCallback(async () => {
    const el = viewportRef.current;
    if (!el) return { x: 0, y: 0, w: 400, h: 300 };
    const bounds = el.getBoundingClientRect();
    const win = getCurrentWindow();
    const scaleFactor = await win.scaleFactor();
    const winPos = await win.outerPosition();
    return {
      x: winPos.x + bounds.left * scaleFactor,
      y: winPos.y + bounds.top * scaleFactor,
      w: bounds.width * scaleFactor,
      h: bounds.height * scaleFactor,
    };
  }, []);

  const handleStartRecording = useCallback(async () => {
    setRecordingResult(null);
    const coords = await getViewportScreenCoords();
    setRecording(true);
    try {
      await startRecording(coords.x, coords.y, coords.w, coords.h);
    } catch (e) {
      console.error("Failed to start recording:", e);
      setRecording(false);
    }
  }, [getViewportScreenCoords]);

  const handleStopRecording = useCallback(async () => {
    setRecording(false);
    try {
      const path = await stopRecording();
      setRecordingResult(path);
    } catch (e) {
      console.error("Failed to stop recording:", e);
    }
  }, []);

  // ── Navigation ──
  const handleGo = useCallback(async () => {
    let target = urlInput.trim();
    if (!target) return;
    if (!target.startsWith("http://") && !target.startsWith("https://")) {
      if (target.includes(".") && !target.includes(" ")) {
        target = "https://" + target;
      } else {
        target = `https://www.google.com/search?q=${encodeURIComponent(target)}`;
      }
    }
    setUrlInput(target);
    setLoading(true);
    setDetected([]);
    setHlsQualities(null);

    if (!browserOpen) {
      if (viewportRef.current) {
        const rect = viewportRef.current.getBoundingClientRect();
        try {
          await openBrowserView(target, rect.x, rect.y, rect.width, rect.height);
          setBrowserOpen(true);
        } catch (e) {
          console.error("Failed to open browser:", e);
          setLoading(false);
        }
      }
    } else {
      try { await navigateBrowser(target); }
      catch (e) { console.error("Navigation failed:", e); setLoading(false); }
    }
  }, [urlInput, browserOpen]);

  // ── Get cookies ──
  const getCookies = useCallback(async (url: string): Promise<string | null> => {
    try {
      const cookies = await getBrowserCookies(url);
      return cookies || null;
    } catch { return null; }
  }, []);

  // ── Handle GRAB click ──
  const handleDownload = useCallback(async (video: DetectedVideo) => {
    const title = video.page_title || video.url.split("/").pop() || "download";
    const safeName = title.replace(/[<>:"/\\|?*]/g, "_").slice(0, 100) || "video";
    const jobId = `browser-${Date.now()}`;

    setDownloading(video.url);
    const cookies = await getCookies(video.url) || video.cookies || null;

    setActiveJob({
      jobId, url: video.url, title: safeName, percent: 0,
      status: "downloading", logLine: "Starting...", speed: "", eta: "",
      error: null, filePath: null,
    });

    try {
      if (video.video_type === "hls") {
        const result = await parseHls(video.url);
        if (result.is_master && result.qualities.length > 1) {
          setHlsQualities(result.qualities);
          setHlsPendingUrl(video.url);
          setHlsPendingTitle(safeName);
          setHlsPendingCookies(cookies);
          setHlsPendingPageUrl(video.page_url || null);
          setActiveJob((prev) => prev ? { ...prev, logLine: "Pick a quality..." } : null);
          return;
        }
        await downloadHlsStream(jobId, video.url, downloadFolder, safeName, undefined, cookies ?? undefined, video.page_url || undefined);
      } else if (video.video_type === "dash") {
        await downloadDashStream(jobId, video.url, downloadFolder, safeName, undefined, cookies ?? undefined, video.page_url || undefined);
      } else {
        await nativeDownload(jobId, video.url, downloadFolder, safeName, video.page_url, cookies ?? undefined);
      }

      setDetected((prev) => prev.filter((v) => v.url !== video.url));
      removeDetectedVideo(video.url).catch(() => {});
    } catch (e: any) {
      const errMsg = typeof e === "string" ? e : e?.message || "Download failed";
      setActiveJob((prev) => prev ? { ...prev, status: "error", error: errMsg, logLine: errMsg } : null);
      setTimeout(() => { setActiveJob(null); setDownloading(null); }, 5000);
      return;
    }
    setDownloading(null);
  }, [getCookies, downloadFolder]);

  // ── Pick HLS quality ──
  const handlePickQuality = useCallback(async (idx: number) => {
    if (!hlsPendingUrl) return;
    const url = hlsPendingUrl;
    const title = hlsPendingTitle;
    const cookies = hlsPendingCookies;
    const pageUrl = hlsPendingPageUrl;
    const jobId = activeJob?.jobId || `browser-${Date.now()}`;

    setHlsQualities(null);
    setHlsPendingUrl(null);
    setHlsPendingPageUrl(null);
    setActiveJob((prev) => prev ? { ...prev, logLine: "Downloading...", percent: 0 } : {
      jobId, url, title, percent: 0, status: "downloading",
      logLine: "Downloading...", speed: "", eta: "", error: null, filePath: null,
    });

    try {
      await downloadHlsStream(jobId, url, downloadFolder, title, idx, cookies ?? undefined, pageUrl ?? undefined);
      setDetected((prev) => prev.filter((v) => v.url !== url));
      removeDetectedVideo(url).catch(() => {});
    } catch (e: any) {
      const errMsg = typeof e === "string" ? e : e?.message || "Download failed";
      setActiveJob((prev) => prev ? { ...prev, status: "error", error: errMsg } : null);
      setTimeout(() => { setActiveJob(null); setDownloading(null); }, 5000);
      return;
    }
    setDownloading(null);
  }, [hlsPendingUrl, hlsPendingTitle, hlsPendingCookies, hlsPendingPageUrl, activeJob, downloadFolder]);

  const formatSize = (bytes?: number | null) => {
    if (!bytes) return "";
    if (bytes > 1073741824) return `${(bytes / 1073741824).toFixed(2)} GB`;
    if (bytes > 1048576) return `${(bytes / 1048576).toFixed(1)} MB`;
    if (bytes > 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${bytes} B`;
  };

  if (!visible) return null;

  const hasDetected = detected.length > 0;
  const hasProgress = activeJob !== null;
  const showSidePanel = hasDetected || hasProgress;

  return (
    <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden", position: "relative" }}>

      {/* ── Browser toolbar ── */}
      <div style={{
        display: "flex", alignItems: "center", gap: "6px",
        padding: "8px 12px",
        background: "var(--panel)", borderBottom: "1px solid var(--border-purple)",
        flexShrink: 0,
      }}>
        <button onClick={() => browserGoBack()} title="Back" style={navBtnStyle}>◂</button>
        <button onClick={() => browserGoForward()} title="Forward" style={navBtnStyle}>▸</button>
        <button onClick={() => browserRefresh()} title="Refresh" style={{
          ...navBtnStyle,
          animation: loading ? "spin 1s linear infinite" : "none",
        }}>↻</button>

        <div style={{ flex: 1, position: "relative" }}>
          {loading && (
            <div style={{
              position: "absolute", left: 0, bottom: 0, height: "2px",
              background: "linear-gradient(90deg, #b400ff, #00f5ff)",
              animation: "shimmer 1.5s infinite",
              width: "60%", borderRadius: "1px",
            }} />
          )}
          <input
            value={urlInput}
            onChange={(e) => setUrlInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleGo()}
            onFocus={(e) => e.target.select()}
            placeholder="Enter URL or search..."
            style={{
              width: "100%", background: "var(--input-bg)",
              border: "1px solid var(--border-purple)", borderRadius: "3px",
              padding: "8px 12px", color: "var(--text)",
              fontFamily: "'Share Tech Mono', monospace", fontSize: "13px",
            }}
          />
        </div>

        <button onClick={handleGo} style={{
          background: "linear-gradient(135deg, #b400ff22, #7700cc22)",
          border: "1px solid #b400ff", borderRadius: "3px",
          color: "#e040fb", fontFamily: "'Orbitron', sans-serif",
          fontSize: "11px", fontWeight: 700, letterSpacing: "2px",
          padding: "8px 16px", cursor: "pointer", whiteSpace: "nowrap",
        }}>GO</button>

        <button onClick={recording ? handleStopRecording : handleStartRecording} style={{
          background: recording ? "linear-gradient(135deg, #ff444433, #cc000022)" : "linear-gradient(135deg, #ff222211, #88000011)",
          border: `1px solid ${recording ? "#ff4444" : "#ff444466"}`, borderRadius: "3px",
          color: recording ? "#ff6666" : "#ff444499", fontFamily: "'Orbitron', sans-serif",
          fontSize: "11px", fontWeight: 700, letterSpacing: "2px",
          padding: "8px 12px", cursor: "pointer", whiteSpace: "nowrap",
        }}>{recording ? "⏹ STOP" : "⏺ REC"}</button>
      </div>

      {/* ── Main area: browser viewport + side panel ── */}
      <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>

        {/* ── Browser viewport ── */}
        <div
          ref={viewportRef}
          style={{
            flex: 1, minHeight: "200px", position: "relative",
            background: browserOpen ? "transparent" : "var(--panel)",
          }}
        >
          {!browserOpen && (
            <div style={{
              display: "flex", flexDirection: "column", alignItems: "center",
              justifyContent: "center", height: "100%", gap: "12px",
            }}>
              <div style={{ fontSize: "48px", opacity: 0.15 }}>🌐</div>
              <p style={{ color: "var(--text-dim)", fontSize: "14px", letterSpacing: "2px" }}>
                ENTER A URL AND HIT GO
              </p>
              <p style={{ color: "var(--text-dimmer)", fontSize: "11px", letterSpacing: "1px" }}>
                Videos will be auto-detected as you browse
              </p>
            </div>
          )}
        </div>

        {/* ── Right side panel: detected videos + progress ── */}
        {showSidePanel && (
          <div style={{
            width: "320px", flexShrink: 0,
            background: "#050310", borderLeft: "1px solid #00f5ff33",
            display: "flex", flexDirection: "column",
            overflow: "hidden",
          }}>

            {/* ── Detected Videos ── */}
            {hasDetected && (
              <div style={{
                flex: hasProgress ? "none" : 1,
                padding: "10px",
                overflowY: "auto",
                borderBottom: hasProgress ? "1px solid #b400ff33" : "none",
              }}>
                <div style={{
                  display: "flex", alignItems: "center", justifyContent: "space-between",
                  marginBottom: "8px",
                }}>
                  <span style={{ fontSize: "10px", letterSpacing: "2px", color: "#00f5ff" }}>
                    ▸ DETECTED ({detected.length})
                  </span>
                </div>
                <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
                  {detected.map((v, i) => {
                    const isDownloading = downloading === v.url;
                    return (
                      <div key={i} style={{
                        padding: "6px 8px", background: "var(--input-bg)",
                        border: `1px solid #3a2a5533`,
                        borderRadius: "2px",
                      }}>
                        {/* Top row: badge + size */}
                        <div style={{ display: "flex", alignItems: "center", gap: "4px", marginBottom: "4px" }}>
                          <span style={{
                            fontSize: "8px", padding: "1px 5px", borderRadius: "2px",
                            fontWeight: 700, letterSpacing: "0.5px",
                            background: v.video_type === "hls" ? "#ff880018"
                              : v.video_type === "dash" ? "#0040ff18"
                              : "#00ff4018",
                            color: v.video_type === "hls" ? "#ffaa44"
                              : v.video_type === "dash" ? "#6688ee"
                              : "#66ee88",
                            border: `1px solid ${v.video_type === "hls" ? "#ff880028"
                              : v.video_type === "dash" ? "#0040ff28"
                              : "#00ff4028"}`,
                          }}>
                            {v.label}
                          </span>
                          {v.file_size && (
                            <span style={{
                              fontSize: "8px", color: "#00ff88", fontWeight: 700,
                            }}>
                              {formatSize(v.file_size)}
                            </span>
                          )}
                        </div>

                        {/* Title or URL */}
                        <div style={{
                          fontSize: "10px", color: "var(--text)",
                          overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
                          marginBottom: "2px", fontWeight: 600,
                        }}>
                          {v.page_title || "Untitled"}
                        </div>
                        <div style={{
                          fontSize: "8px", color: "#555",
                          overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
                          marginBottom: "4px",
                        }}>
                          {v.url.length > 50 ? v.url.slice(0, 47) + "..." : v.url}
                        </div>

                        {/* Button */}
                        <button
                          onClick={() => !isDownloading && handleDownload(v)}
                          disabled={isDownloading}
                          style={{
                            width: "100%", padding: "3px 8px",
                            background: isDownloading ? "#3a2a5533" : "linear-gradient(135deg, #b400ff33, #7700cc22)",
                            border: `1px solid ${isDownloading ? "var(--border-dim)" : "#b400ff"}`,
                            borderRadius: "2px",
                            color: isDownloading ? "var(--text-dim)" : "#e040fb",
                            fontFamily: "'Orbitron', sans-serif",
                            fontSize: "8px", fontWeight: 700, letterSpacing: "1px",
                            cursor: isDownloading ? "not-allowed" : "pointer",
                          }}
                        >
                          {isDownloading ? "..." : "⬇ GRAB"}
                        </button>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}

            {/* ── Download Progress ── */}
            {hasProgress && (
              <div style={{
                padding: "10px",
                background: "#0a0614",
                flexShrink: 0,
              }}>
                <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "6px" }}>
                  <span style={{ fontSize: "10px", letterSpacing: "2px", color: "#b400ff" }}>
                    {activeJob.status === "complete" ? "✓ DONE"
                      : activeJob.status === "error" ? "✗ ERROR"
                      : "▸ DOWNLOADING"}
                  </span>
                  {(activeJob.status === "error" || activeJob.status === "complete") && (
                    <button onClick={() => { setActiveJob(null); setDownloading(null); }} style={{
                      background: "none", border: "none", color: "#666", cursor: "pointer", fontSize: "11px",
                    }}>✕</button>
                  )}
                </div>

                <div style={{
                  fontSize: "10px", color: "var(--text-muted)",
                  overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
                  marginBottom: "6px",
                }}>
                  {activeJob.title}
                </div>

                {activeJob.status !== "error" && (
                  <div style={{ width: "100%", height: "5px", background: "#1a1a28", borderRadius: "3px", overflow: "hidden", marginBottom: "4px" }}>
                    <div style={{
                      width: `${Math.max(0, Math.min(100, activeJob.percent))}%`,
                      height: "100%", borderRadius: "3px",
                      transition: "width 0.2s",
                      background: activeJob.status === "complete"
                        ? "linear-gradient(90deg, #00ff88, #00cc66)"
                        : "linear-gradient(90deg, #b400ff, #e040fb)",
                    }} />
                  </div>
                )}

                <div style={{ display: "flex", justifyContent: "space-between", fontSize: "9px" }}>
                  <span style={{ color: "#888", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: 1 }}>
                    {activeJob.logLine}
                  </span>
                  <span style={{
                    color: activeJob.status === "complete" ? "#00ff88"
                      : activeJob.status === "error" ? "#ff4444"
                      : "#e040fb",
                    fontWeight: 700, flexShrink: 0, marginLeft: "6px",
                  }}>
                    {activeJob.status === "complete" ? "100%"
                      : activeJob.status === "error" ? "FAIL"
                      : activeJob.percent >= 0 ? `${Math.round(activeJob.percent)}%` : ""}
                  </span>
                </div>

                {activeJob.error && (
                  <div style={{
                    fontSize: "9px", color: "#ff4444", background: "#ff000010",
                    border: "1px solid #ff000030", borderRadius: "2px",
                    padding: "4px 6px", marginTop: "4px", lineHeight: 1.3,
                  }}>
                    {activeJob.error}
                  </div>
                )}

                {activeJob.status === "complete" && activeJob.filePath && (
                  <div style={{ display: "flex", gap: "4px", marginTop: "6px" }}>
                    <button onClick={() => activeJob.filePath && openFile(activeJob.filePath)} style={sideBtnStyle("#00ff88")}>▶ OPEN</button>
                    <button onClick={() => activeJob.filePath && showInFolder(activeJob.filePath)} style={sideBtnStyle("#e040fb")}>◈ FOLDER</button>
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </div>

      {/* ── Recording result toast ── */}
      {recordingResult && (
        <div style={{
          position: "absolute", bottom: 16, left: "50%", transform: "translateX(-50%)",
          zIndex: 100, background: "#0a0a1a", border: "1px solid #00f5ff66",
          borderRadius: "6px", padding: "10px 16px", display: "flex",
          alignItems: "center", gap: "10px", maxWidth: "90%",
        }}>
          <span style={{ color: "#00ff88", fontSize: "12px", fontFamily: "'Orbitron', sans-serif" }}>
            RECORDING SAVED
          </span>
          <span style={{ color: "#aaa", fontSize: "11px", fontFamily: "'Share Tech Mono', monospace", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {recordingResult}
          </span>
          <button onClick={() => { import("../lib/tauri").then(m => m.showInFolder(recordingResult!)); }} style={{
            background: "none", border: "1px solid #00f5ff44", borderRadius: "3px",
            color: "#00f5ff", fontSize: "10px", padding: "3px 8px", cursor: "pointer",
            fontFamily: "'Orbitron', sans-serif",
          }}>SHOW</button>
          <button onClick={() => setRecordingResult(null)} style={{
            background: "none", border: "none", color: "#666", fontSize: "14px",
            cursor: "pointer", padding: "0 4px",
          }}>×</button>
        </div>
      )}

      {/* ── HLS Quality picker modal ── */}
      {hlsQualities && (
        <div style={{
          position: "absolute", inset: 0, background: "rgba(0,0,0,0.7)",
          display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100,
        }}>
          <div style={{
            background: "var(--panel)", border: "1px solid var(--border-purple)",
            borderRadius: "4px", padding: "20px", minWidth: "300px", maxWidth: "400px",
          }}>
            <div style={{ fontSize: "13px", letterSpacing: "3px", color: "#b400ff", marginBottom: "12px" }}>
              ▸ SELECT QUALITY
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
              {hlsQualities.map((q, i) => (
                <button key={i} onClick={() => handlePickQuality(i)} style={{
                  display: "flex", justifyContent: "space-between", alignItems: "center",
                  padding: "10px 14px", background: "var(--input-bg)",
                  border: "1px solid var(--border-dim)", borderRadius: "3px",
                  color: "var(--text)", cursor: "pointer", transition: "all 0.2s",
                  fontFamily: "'Share Tech Mono', monospace", fontSize: "13px",
                }}>
                  <span>{q.label}</span>
                  <span style={{ fontSize: "11px", color: "var(--text-dim)" }}>
                    {q.resolution || `${Math.round(q.bandwidth / 1000)}kbps`}
                  </span>
                </button>
              ))}
            </div>
            <button onClick={() => { setHlsQualities(null); setHlsPendingUrl(null); setActiveJob(null); setDownloading(null); }} style={{
              marginTop: "10px", width: "100%", padding: "8px",
              background: "transparent", border: "1px solid var(--border-dim)",
              borderRadius: "3px", color: "var(--text-dim)", cursor: "pointer",
              fontFamily: "'Share Tech Mono', monospace", fontSize: "11px",
            }}>CANCEL</button>
          </div>
        </div>
      )}
    </div>
  );
}

// ── Resizable crop overlay ─────────────────────────────────────────────────

const navBtnStyle: React.CSSProperties = {
  width: "30px", height: "30px", display: "flex",
  alignItems: "center", justifyContent: "center",
  background: "transparent", border: "1px solid var(--border-dim)",
  borderRadius: "3px", color: "var(--text-dim)",
  fontSize: "14px", cursor: "pointer", flexShrink: 0,
};

function sideBtnStyle(color: string): React.CSSProperties {
  return {
    padding: "3px 8px", background: `${color}11`,
    border: `1px solid ${color}44`, borderRadius: "2px",
    color, fontFamily: "'Orbitron', sans-serif",
    fontSize: "8px", fontWeight: 700, letterSpacing: "1px",
    cursor: "pointer",
  };
}
