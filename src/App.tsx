import { useState, useEffect, useRef, useCallback } from "react";
import { downloadDir } from "@tauri-apps/api/path";
import { listen } from "@tauri-apps/api/event";
import TitleBar from "./components/TitleBar";
import BrowserTab from "./components/BrowserTab";
import SettingsModal from "./components/Settings/SettingsModal";
import { analyzeUrl, startDownload, cancelDownload, checkYtdlp, checkFfmpeg, showInFolder, openFile, convertFile, nativeDownload, updateYtdlp, hideBrowser, showBrowser } from "./lib/tauri";
import type { DownloadProgress, DownloadItem, ConversionPreset } from "./lib/types";

const FORMATS = ["Default", "MP4", "MP3 Audio", "WEBM", "MKV"];
const QUALITIES = ["2160p", "1440p", "1080p", "720p", "480p", "360p"];

const SMART_PRESETS = [
  { label: "BEST QUALITY", desc: "Max resolution", format: "Default", quality: "best", icon: "◆" },
  { label: "1080p MP4", desc: "Balanced", format: "MP4", quality: "1080p", icon: "▣" },
  { label: "720p SAVE", desc: "Smaller file", format: "MP4", quality: "720p", icon: "▤" },
  { label: "AUDIO ONLY", desc: "MP3 extract", format: "MP3 Audio", quality: "audio", icon: "♫" },
] as const;

const URL_REGEX = /https?:\/\/[^\s<>"']+/;

const PLATFORMS = [
  { name: "YouTube", color: "#ff003c" },
  { name: "Twitter/X", color: "#00f5ff" },
  { name: "Instagram", color: "#b400ff" },
  { name: "TikTok", color: "#00f5ff" },
  { name: "Reddit", color: "#e040fb" },
];

function detectPlatform(u: string) {
  if (u.includes("youtube") || u.includes("youtu.be")) return PLATFORMS[0];
  if (u.includes("twitter") || u.includes("x.com")) return PLATFORMS[1];
  if (u.includes("instagram")) return PLATFORMS[2];
  if (u.includes("tiktok")) return PLATFORMS[3];
  if (u.includes("reddit")) return PLATFORMS[4];
  return null;
}

function formatToQuality(f: string, qualityIdx: number): string {
  if (f.includes("Audio") || f.includes("MP3")) return "audio";
  if (f === "Default") return "best";
  const q = QUALITIES[qualityIdx];
  return q || "best";
}

export default function App() {
  // ── Tab state ──
  const [activeTab, setActiveTab] = useState<"download" | "browser">("download");
  const [showSettings, setShowSettings] = useState(false);

  // ── All existing state (unchanged) ──
  const [url, setUrl] = useState("");
  const [format, setFormat] = useState("Default");
  const [qualityIdx, setQualityIdx] = useState(2);
  const [phase, setPhase] = useState<"idle" | "fetching" | "ready" | "downloading" | "done" | "error">("idle");
  const [progress, setProgress] = useState(0);
  const [speed, setSpeed] = useState("");
  const [eta, setEta] = useState("");
  const [videoTitle, setVideoTitle] = useState("");
  const [videoDuration, setVideoDuration] = useState("");
  const [videoPlatform, setVideoPlatform] = useState("");
  const [videoPlatformColor, setVideoPlatformColor] = useState("#b400ff");
  const [history, setHistory] = useState<DownloadItem[]>([]);
  const [log, setLog] = useState<{ msg: string; time: string }[]>([]);
  const [glitching, setGlitching] = useState(false);
  const [downloadFolder, setDownloadFolder] = useState("");
  const [currentJobId, setCurrentJobId] = useState("");
  const [ytdlpOk, setYtdlpOk] = useState(true);
  const [ffmpegOk, setFfmpegOk] = useState(true);
  const [converting, setConverting] = useState(false);
  const [filePath, setFilePath] = useState<string | null>(null);
  const [fileSize, setFileSize] = useState<number | null>(null);
  const [clipboardWatch, setClipboardWatch] = useState(true);
  const [lastClipboard, setLastClipboard] = useState("");
  const [writeSubs, setWriteSubs] = useState(false);
  const [duplicateUrl, setDuplicateUrl] = useState<string | null>(null);
  const [smartMode, setSmartMode] = useState(true);
  const logRef = useRef<HTMLDivElement>(null);

  const addLog = useCallback((msg: string) => {
    const time = new Date().toLocaleTimeString("en", { hour12: false });
    setLog((prev) => [...prev, { msg, time }]);
  }, []);

  useEffect(() => {
    if (logRef.current) logRef.current.scrollTop = logRef.current.scrollHeight;
  }, [log]);

  useEffect(() => {
    const iv = setInterval(() => {
      setGlitching(true);
      setTimeout(() => setGlitching(false), 300);
    }, 7000);
    return () => clearInterval(iv);
  }, []);

  useEffect(() => {
    downloadDir().then(setDownloadFolder).catch(() => {});
    checkYtdlp().then((ok) => {
      setYtdlpOk(ok);
      if (ok) {
        updateYtdlp().then((msg) => {
          if (msg && !msg.includes("up to date")) {
            console.log("[yt-dlp update]", msg);
          }
        }).catch(() => {});
      }
    }).catch(() => setYtdlpOk(false));
    checkFfmpeg().then(setFfmpegOk).catch(() => setFfmpegOk(false));
  }, []);

  // ── Clipboard monitoring ──
  useEffect(() => {
    if (!clipboardWatch) return;
    const iv = setInterval(async () => {
      try {
        const text = await navigator.clipboard.readText();
        if (text && text !== lastClipboard && URL_REGEX.test(text.trim())) {
          const trimmed = text.trim().split(/\s/)[0];
          if (trimmed !== url && trimmed !== lastClipboard) {
            setLastClipboard(trimmed);
            if (phase === "idle" || phase === "done" || phase === "error") {
              setUrl(trimmed);
              const p = detectPlatform(trimmed);
              if (p) addLog(`Clipboard detected: ${p.name} URL`);
              else addLog("Clipboard detected: URL auto-filled");
            }
          }
        }
      } catch {}
    }, 1500);
    return () => clearInterval(iv);
  }, [clipboardWatch, lastClipboard, url, phase, addLog]);

  useEffect(() => {
    const unlisten = listen<DownloadProgress>("download-progress", (event) => {
      const p = event.payload;
      if (p.job_id !== currentJobId && currentJobId) return;

      if (p.log_line && !p.log_line.startsWith("CYBERPROG")) {
        addLog(p.log_line);
      }

      if (p.status === "complete") {
        setProgress(100);
        setPhase("done");
        if (p.file_path) setFilePath(p.file_path);
        if (p.file_size) setFileSize(p.file_size);
        addLog("EXTRACTION COMPLETE ✓");
        setHistory((h) => [
          { id: p.job_id, url, title: videoTitle || url, site_name: videoPlatform, status: "complete", progress: 100, speed: "", eta: "", outputDir: downloadFolder, quality: format, formatType: format, logs: [], filePath: p.file_path, fileSize: p.file_size, created_at: Date.now() },
          ...h,
        ]);
      } else if (p.status === "error" || p.status === "cancelled") {
        setPhase("error");
        addLog(p.status === "cancelled" ? "CANCELLED BY USER" : `ERROR: ${p.log_line}`);
        setTimeout(() => setPhase("idle"), 2500);
      } else if (p.status === "converting") {
        setProgress(99);
        addLog("Merging audio/video tracks...");
      } else if (p.percent >= 0) {
        setProgress(p.percent);
        if (p.speed) setSpeed(p.speed);
        if (p.eta) setEta(p.eta);
      }
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [currentJobId, addLog, url, videoTitle, videoPlatform, downloadFolder, format]);

  // ── Tab switching: show/hide browser webview ──
  useEffect(() => {
    if (activeTab === "browser") {
      showBrowser().catch(() => {});
    } else {
      hideBrowser().catch(() => {});
    }
  }, [activeTab]);

  // ── Browser download callbacks ──
  const handleBrowserDownloadDirect = useCallback(async (videoUrl: string, title: string) => {
    setActiveTab("download");
    setUrl(videoUrl);
    setVideoTitle(title);
    setPhase("downloading");
    setLog([]);
    setProgress(0);
    setFilePath(null);
    setFileSize(null);
    const jobId = `dl-${Date.now()}`;
    setCurrentJobId(jobId);
    const safeName = title.replace(/[<>:"/\\|?*]/g, "_").slice(0, 100) || "download";
    addLog(`Browser capture → downloading ${safeName}...`);
    try {
      const result = await nativeDownload(jobId, videoUrl, downloadFolder, safeName);
      setFilePath(result);
      setProgress(100);
      setPhase("done");
      addLog("EXTRACTION COMPLETE ✓");
    } catch (e) {
      addLog(`ERR: ${e}`);
      setPhase("error");
      setTimeout(() => setPhase("idle"), 2500);
    }
  }, [downloadFolder, addLog]);

  const handleBrowserDownloadNative = useCallback(async (videoUrl: string, title: string) => {
    setActiveTab("download");
    setUrl(videoUrl);
    setVideoTitle(title);
    setPhase("downloading");
    setLog([]);
    setProgress(0);
    setFilePath(null);
    setFileSize(null);
    const jobId = `dl-${Date.now()}`;
    setCurrentJobId(jobId);
    const safeName = title.replace(/[<>:"/\\|?*]/g, "_").slice(0, 100) || "download";
    addLog(`Browser capture → downloading stream ${safeName}...`);
    try {
      const result = await nativeDownload(jobId, videoUrl, downloadFolder, safeName);
      setFilePath(result);
      setProgress(100);
      setPhase("done");
      addLog("EXTRACTION COMPLETE ✓");
    } catch (e) {
      const errStr = String(e);
      if (errStr === "USE_YTDLP") {
        addLog("Falling back to yt-dlp...");
        try {
          await startDownload(jobId, videoUrl, safeName, downloadFolder, "best", "Default");
        } catch (e2) {
          addLog(`ERR: ${e2}`);
          setPhase("error");
          setTimeout(() => setPhase("idle"), 2500);
        }
      } else {
        addLog(`ERR: ${e}`);
        setPhase("error");
        setTimeout(() => setPhase("idle"), 2500);
      }
    }
  }, [downloadFolder, addLog]);

  // ── Duplicate detection ──
  const checkDuplicate = useCallback((targetUrl: string): boolean => {
    const normalizeUrl = (u: string) => u.replace(/^https?:\/\//, "").replace(/\/$/, "").replace(/[?#].*$/, "");
    const norm = normalizeUrl(targetUrl);
    return history.some((h) => normalizeUrl(h.url) === norm);
  }, [history]);

  const handleProbe = useCallback(async () => {
    if (!url.trim()) {
      setPhase("error");
      addLog("ERR: No target URL provided.");
      setTimeout(() => setPhase("idle"), 1500);
      return;
    }

    // Duplicate detection
    if (checkDuplicate(url)) {
      setDuplicateUrl(url);
      return;
    }

    const isHls = url.includes(".m3u8");
    const isDirect = [".mp4", ".webm", ".mkv", ".avi", ".mov", ".ts"].some(ext => url.toLowerCase().includes(ext));

    if (isHls || isDirect) {
      setPhase("downloading");
      setLog([]);
      setProgress(0);
      setFilePath(null);
      setFileSize(null);
      const jobId = `dl-${Date.now()}`;
      setCurrentJobId(jobId);
      const safeName = url.split("/").pop()?.split("?")[0]?.replace(/[<>:"/\\|?*]/g, "_") || "download";
      setVideoTitle(safeName);
      addLog(isHls ? "HLS stream detected — routing to native engine..." : "Direct file detected — downloading...");

      try {
        const result = await nativeDownload(jobId, url, downloadFolder, safeName);
        setFilePath(result);
        setFileSize(null);
        setProgress(100);
        setPhase("done");
        addLog("EXTRACTION COMPLETE ✓");
      } catch (e) {
        const errStr = String(e);
        if (errStr === "USE_YTDLP") {
          addLog("Native engine deferred — falling back to yt-dlp...");
          await probeWithYtdlp();
        } else {
          addLog(`ERR: ${e}`);
          setPhase("error");
          setTimeout(() => setPhase("idle"), 2500);
        }
      }
      return;
    }

    await probeWithYtdlp();
  }, [url, addLog, downloadFolder, checkDuplicate]);

  const probeWithYtdlp = useCallback(async () => {
    setPhase("fetching");
    setLog([]);
    addLog("Initializing extraction sequence...");

    try {
      const analysis = await analyzeUrl(url);
      const platform = detectPlatform(url);
      setVideoTitle(analysis.title || "Unknown Target");
      setVideoDuration(analysis.duration || "—");
      setVideoPlatform(platform?.name || analysis.site_name || "Unknown");
      setVideoPlatformColor(platform?.color || "#b400ff");
      addLog("Metadata acquired. Target locked.");
      setPhase("ready");
    } catch (e) {
      addLog(`ERR: ${e}`);
      setPhase("error");
      setTimeout(() => setPhase("idle"), 2000);
    }
  }, [url, addLog]);

  const handleExtract = useCallback(async () => {
    setPhase("downloading");
    setProgress(0);
    setSpeed("");
    setEta("");
    setFilePath(null);
    setFileSize(null);
    const jobId = `dl-${Date.now()}`;
    setCurrentJobId(jobId);
    addLog(`Initiating ${format} extraction...`);

    try {
      await startDownload(jobId, url, videoTitle || url, downloadFolder, formatToQuality(format, qualityIdx), format, writeSubs);
    } catch (e) {
      addLog(`ERR: ${e}`);
      setPhase("error");
    }
  }, [url, format, qualityIdx, downloadFolder, addLog, videoTitle, writeSubs]);

  const handleCancel = useCallback(() => {
    if (currentJobId) cancelDownload(currentJobId);
  }, [currentJobId]);

  const handleConvert = useCallback(async (preset: ConversionPreset) => {
    if (!filePath) return;
    const convJobId = `conv-${Date.now()}`;
    setConverting(true);
    setCurrentJobId(convJobId);
    addLog(`Starting conversion: ${preset.type}...`);
    try {
      const result = await convertFile(convJobId, filePath, preset);
      setFilePath(result);
      setConverting(false);
      addLog("Conversion complete ✓");
    } catch (e) {
      addLog(`Conversion error: ${e}`);
      setConverting(false);
    }
  }, [filePath, addLog]);

  const reset = () => {
    setPhase("idle");
    setUrl("");
    setVideoTitle("");
    setProgress(0);
    setSpeed("");
    setEta("");
    setLog([]);
    setCurrentJobId("");
    setFormat("Default");
    setQualityIdx(2);
    setFilePath(null);
    setFileSize(null);
  };

  const applyPreset = useCallback((preset: typeof SMART_PRESETS[number]) => {
    setFormat(preset.format);
    if (preset.quality !== "best" && preset.quality !== "audio") {
      const idx = QUALITIES.indexOf(preset.quality);
      if (idx >= 0) setQualityIdx(idx);
    }
  }, []);

  const forceProbeDuplicate = useCallback(async () => {
    setDuplicateUrl(null);
    const isHls = url.includes(".m3u8");
    const isDirect = [".mp4", ".webm", ".mkv", ".avi", ".mov", ".ts"].some(ext => url.toLowerCase().includes(ext));
    if (isHls || isDirect) {
      setPhase("downloading");
      setLog([]);
      setProgress(0);
      setFilePath(null);
      setFileSize(null);
      const jobId = `dl-${Date.now()}`;
      setCurrentJobId(jobId);
      const safeName = url.split("/").pop()?.split("?")[0]?.replace(/[<>:"/\\|?*]/g, "_") || "download";
      setVideoTitle(safeName);
      addLog(isHls ? "HLS stream detected — routing to native engine..." : "Direct file detected — downloading...");
      try {
        const result = await nativeDownload(jobId, url, downloadFolder, safeName);
        setFilePath(result);
        setFileSize(null);
        setProgress(100);
        setPhase("done");
        addLog("EXTRACTION COMPLETE ✓");
      } catch (e) {
        const errStr = String(e);
        if (errStr === "USE_YTDLP") {
          addLog("Native engine deferred — falling back to yt-dlp...");
          await probeWithYtdlp();
        } else {
          addLog(`ERR: ${e}`);
          setPhase("error");
          setTimeout(() => setPhase("idle"), 2500);
        }
      }
      return;
    }
    await probeWithYtdlp();
  }, [url, addLog, downloadFolder, probeWithYtdlp]);

  const platform = detectPlatform(url);

  return (
    <div style={{ height: "100vh", display: "flex", flexDirection: "column", background: "var(--bg)", position: "relative", overflow: "hidden" }}>
      <TitleBar />

      {/* Background effects */}
      <div className="noise-overlay" />
      <div className="grid-bg" />
      <div className="scanline" />
      <div style={{ position: "fixed", top: "-100px", right: "-100px", width: "400px", height: "400px", borderRadius: "50%", background: "radial-gradient(circle, #b400ff18 0%, transparent 70%)", pointerEvents: "none", zIndex: 0 }} />
      <div style={{ position: "fixed", bottom: "-150px", left: "-150px", width: "500px", height: "500px", borderRadius: "50%", background: "radial-gradient(circle, #00f5ff12 0%, transparent 70%)", pointerEvents: "none", zIndex: 0 }} />

      {/* ── TAB BAR ── */}
      <div style={{
        display: "flex", alignItems: "center", gap: "0",
        borderBottom: "1px solid var(--border-purple)",
        background: "var(--panel)", flexShrink: 0,
        position: "relative", zIndex: 2,
        paddingLeft: "12px",
      }}>
        {(["download", "browser"] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            style={{
              padding: "10px 24px",
              background: activeTab === tab ? "#b400ff15" : "transparent",
              border: "none",
              borderBottom: activeTab === tab ? "2px solid #b400ff" : "2px solid transparent",
              color: activeTab === tab ? "#e040fb" : "var(--text-dim)",
              fontFamily: "'Orbitron', sans-serif",
              fontSize: "11px",
              fontWeight: 700,
              letterSpacing: "3px",
              cursor: "pointer",
              transition: "all 0.2s",
            }}
          >
            {tab === "download" ? "◈ DOWNLOAD" : "◈ BROWSER"}
          </button>
        ))}
        <div style={{ flex: 1 }} />
        {activeTab === "browser" && (
          <span style={{
            fontSize: "9px", color: "var(--text-dimmer)", letterSpacing: "1px",
            marginRight: "8px",
          }}>
            Videos auto-detected while browsing
          </span>
        )}
        <button
          onClick={() => setShowSettings(true)}
          title="Settings"
          style={{
            background: "transparent",
            border: "none",
            cursor: "pointer",
            padding: "8px 12px",
            color: "var(--text-dim)",
            transition: "color 0.2s",
            display: "flex",
            alignItems: "center",
          }}
          onMouseEnter={(e) => (e.currentTarget.style.color = "#e040fb")}
          onMouseLeave={(e) => (e.currentTarget.style.color = "var(--text-dim)")}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="12" cy="12" r="3"/>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
          </svg>
        </button>
      </div>

      {/* ── BROWSER TAB ── */}
      <BrowserTab
        visible={activeTab === "browser"}
        downloadFolder={downloadFolder}
        onDownloadDirect={handleBrowserDownloadDirect}
        onDownloadNative={handleBrowserDownloadNative}
      />

      {/* ── DOWNLOAD TAB (existing UI, unchanged) ── */}
      {activeTab === "download" && (
        <div style={{ flex: 1, overflowY: "auto", display: "flex", flexDirection: "column", alignItems: "center", padding: "20px 20px 30px", position: "relative", zIndex: 2 }}>
          <div style={{ width: "100%", maxWidth: "700px" }}>

            {/* Header */}
            <div className="anim-float-in" style={{ textAlign: "center", marginBottom: "28px" }}>
              <div style={{ display: "inline-flex", alignItems: "center", gap: "12px", marginBottom: "6px" }}>
                <svg width="28" height="28" viewBox="0 0 32 32" fill="none">
                  <polygon points="16,2 30,10 30,22 16,30 2,22 2,10" fill="none" stroke="#b400ff" strokeWidth="1.5" />
                  <polygon points="16,8 24,12 24,20 16,24 8,20 8,12" fill="#b400ff22" stroke="#00f5ff" strokeWidth="1" />
                  <circle cx="16" cy="16" r="3" fill="#00f5ff" />
                </svg>
                <h1 style={{
                  fontFamily: "'Orbitron', sans-serif", fontSize: "32px", fontWeight: 900, margin: 0,
                  background: "linear-gradient(135deg, #b400ff 0%, #e040fb 40%, #00f5ff 100%)",
                  WebkitBackgroundClip: "text", WebkitTextFillColor: "transparent",
                  letterSpacing: "4px",
                  animation: glitching ? "glitch 0.3s steps(2) both" : "flicker 8s infinite",
                }}>CYBERSNATCHER</h1>
              </div>
              <p style={{ color: "var(--text-dim)", fontSize: "17px", letterSpacing: "3px" }}>
                NEURAL VIDEO EXTRACTION SYSTEM v2.7
              </p>
            </div>

            {/* Main Panel */}
            <div className="corner-accents anim-pulse-border" style={{
              background: "linear-gradient(145deg, var(--panel) 0%, var(--panel-alt) 100%)",
              border: "1px solid var(--border-purple)", borderRadius: "4px",
              padding: "30px", marginBottom: "16px",
              animation: "pulse-border 4s ease-in-out infinite, float-in 0.6s ease both",
            }}>
              <div style={{ position: "absolute", bottom: "-1px", left: "-1px", width: "14px", height: "14px", borderBottom: "2px solid var(--cyan)", borderLeft: "2px solid var(--cyan)" }} />
              <div style={{ position: "absolute", bottom: "-1px", right: "-1px", width: "14px", height: "14px", borderBottom: "2px solid var(--cyan)", borderRight: "2px solid var(--cyan)" }} />

              {/* URL Input */}
              <label style={{ display: "block", fontSize: "17px", letterSpacing: "3px", color: "var(--purple)", marginBottom: "8px" }}>
                ▸ TARGET URL
              </label>
              <div style={{ display: "flex", gap: "10px", marginBottom: "16px" }}>
                <div style={{ position: "relative", flex: 1 }}>
                  {platform && (
                    <span style={{
                      position: "absolute", left: "12px", top: "50%", transform: "translateY(-50%)",
                      width: "8px", height: "8px", borderRadius: "50%",
                      background: platform.color, boxShadow: `0 0 8px ${platform.color}`,
                      animation: "badge-pop 0.3s ease both",
                    }} />
                  )}
                  <input
                    value={url}
                    onChange={(e) => setUrl(e.target.value)}
                    onKeyDown={(e) => e.key === "Enter" && handleProbe()}
                    placeholder="https://..."
                    disabled={phase === "downloading" || phase === "fetching"}
                    style={{
                      width: "100%", background: "var(--input-bg)",
                      border: `1px solid ${phase === "error" ? "var(--red)" : "var(--border-purple)"}`,
                      borderRadius: "3px", padding: `14px ${platform ? "14px" : "14px"} 10px ${platform ? "30px" : "14px"}`,
                      color: "var(--text)", fontFamily: "'Share Tech Mono', monospace", fontSize: "17px",
                      transition: "border-color 0.2s, box-shadow 0.2s",
                    }}
                  />
                </div>
                <button
                  onClick={handleProbe}
                  disabled={phase === "fetching" || phase === "downloading"}
                  style={{
                    background: "linear-gradient(135deg, #b400ff22, #7700cc22)",
                    border: "1px solid #b400ff", borderRadius: "3px", color: "#e040fb",
                    fontFamily: "'Orbitron', sans-serif", fontSize: "17px", fontWeight: 700,
                    letterSpacing: "2px", padding: "0 24px", cursor: "pointer",
                    whiteSpace: "nowrap", opacity: (phase === "fetching" || phase === "downloading") ? 0.5 : 1,
                  }}
                >
                  {phase === "fetching" ? "PROBING..." : "PROBE"}
                </button>
              </div>

              {/* Options row: clipboard + subs */}
              <div style={{ display: "flex", gap: "16px", marginBottom: "14px", alignItems: "center" }}>
                <label style={{ display: "flex", alignItems: "center", gap: "6px", cursor: "pointer", fontSize: "12px", color: clipboardWatch ? "#00f5ff" : "var(--text-dim)", letterSpacing: "1px", userSelect: "none" }}
                  onClick={() => setClipboardWatch(!clipboardWatch)}>
                  <span style={{
                    width: "14px", height: "14px", border: `1px solid ${clipboardWatch ? "#00f5ff" : "var(--border-dim)"}`,
                    borderRadius: "2px", display: "inline-flex", alignItems: "center", justifyContent: "center",
                    background: clipboardWatch ? "#00f5ff15" : "transparent", transition: "all 0.2s",
                  }}>{clipboardWatch ? "✓" : ""}</span>
                  CLIPBOARD MONITOR
                </label>
                <label style={{ display: "flex", alignItems: "center", gap: "6px", cursor: "pointer", fontSize: "12px", color: writeSubs ? "#e040fb" : "var(--text-dim)", letterSpacing: "1px", userSelect: "none" }}
                  onClick={() => setWriteSubs(!writeSubs)}>
                  <span style={{
                    width: "14px", height: "14px", border: `1px solid ${writeSubs ? "#e040fb" : "var(--border-dim)"}`,
                    borderRadius: "2px", display: "inline-flex", alignItems: "center", justifyContent: "center",
                    background: writeSubs ? "#b400ff15" : "transparent", transition: "all 0.2s",
                  }}>{writeSubs ? "✓" : ""}</span>
                  DOWNLOAD SUBTITLES
                </label>
              </div>

              {/* Smart Mode Presets */}
              {smartMode && phase !== "downloading" && (
                <div style={{ marginBottom: "14px" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "8px" }}>
                    <label style={{ fontSize: "12px", letterSpacing: "3px", color: "var(--purple)" }}>▸ SMART MODE</label>
                    <span style={{ fontSize: "10px", color: "var(--text-dimmer)", letterSpacing: "1px", cursor: "pointer" }}
                      onClick={() => setSmartMode(false)}>HIDE</span>
                  </div>
                  <div style={{ display: "flex", gap: "6px", flexWrap: "wrap" }}>
                    {SMART_PRESETS.map((p) => (
                      <button key={p.label} onClick={() => applyPreset(p)} style={{
                        flex: "1 1 0", minWidth: "120px", padding: "10px 8px", textAlign: "center",
                        background: format === p.format && (p.quality === "best" || p.quality === "audio" || QUALITIES[qualityIdx] === p.quality)
                          ? "#b400ff18" : "var(--input-bg)",
                        border: `1px solid ${format === p.format && (p.quality === "best" || p.quality === "audio" || QUALITIES[qualityIdx] === p.quality)
                          ? "#b400ff" : "var(--border-dim)"}`,
                        borderRadius: "3px", cursor: "pointer", transition: "all 0.2s",
                      }}>
                        <div style={{ fontSize: "14px", marginBottom: "2px" }}>{p.icon}</div>
                        <div style={{ fontSize: "10px", color: "#e040fb", letterSpacing: "1px", fontFamily: "'Orbitron', sans-serif", fontWeight: 700 }}>{p.label}</div>
                        <div style={{ fontSize: "9px", color: "var(--text-dimmer)", marginTop: "2px" }}>{p.desc}</div>
                      </button>
                    ))}
                  </div>
                </div>
              )}
              {!smartMode && phase !== "downloading" && (
                <div style={{ marginBottom: "8px", textAlign: "right" }}>
                  <span style={{ fontSize: "10px", color: "var(--text-dimmer)", letterSpacing: "1px", cursor: "pointer" }}
                    onClick={() => setSmartMode(true)}>SHOW SMART MODE</span>
                </div>
              )}

              {/* Format selector */}
              <label style={{ display: "block", fontSize: "17px", letterSpacing: "3px", color: "var(--purple)", marginBottom: "8px" }}>
                ▸ OUTPUT FORMAT
              </label>
              <div style={{ display: "flex", flexWrap: "wrap", gap: "6px", marginBottom: "12px" }}>
                {FORMATS.map((f) => (
                  <button key={f} onClick={() => setFormat(f)} style={{
                    background: format === f ? "#b400ff22" : "transparent",
                    border: `1px solid ${format === f ? "#b400ff" : "var(--border-dim)"}`,
                    borderRadius: "3px", color: format === f ? "#e040fb" : "var(--text-dim)",
                    fontFamily: "'Share Tech Mono', monospace", fontSize: "17px",
                    padding: "8px 18px", cursor: "pointer", transition: "all 0.2s",
                    boxShadow: format === f ? "0 0 10px #b400ff30" : "none", letterSpacing: "1px",
                  }}>{f}</button>
                ))}
              </div>

              {/* Quality slider */}
              {format !== "Default" && format !== "MP3 Audio" && (
                <div style={{ marginBottom: "18px" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "6px" }}>
                    <label style={{ fontSize: "17px", letterSpacing: "3px", color: "var(--purple)" }}>
                      ▸ QUALITY
                    </label>
                    <span style={{ fontSize: "17px", color: "#00f5ff", fontWeight: 700, letterSpacing: "1px", fontFamily: "'Share Tech Mono', monospace" }}>
                      {QUALITIES[qualityIdx]}
                    </span>
                  </div>
                  <div style={{ position: "relative", padding: "4px 0" }}>
                    <input
                      type="range"
                      min={0}
                      max={QUALITIES.length - 1}
                      value={qualityIdx}
                      onChange={(e) => setQualityIdx(Number(e.target.value))}
                      style={{
                        width: "100%", height: "4px", appearance: "none", WebkitAppearance: "none",
                        background: `linear-gradient(90deg, #00f5ff ${((qualityIdx) / (QUALITIES.length - 1)) * 100}%, var(--border-dim) ${((qualityIdx) / (QUALITIES.length - 1)) * 100}%)`,
                        borderRadius: "2px", outline: "none", cursor: "pointer",
                      }}
                    />
                    <div style={{ display: "flex", justifyContent: "space-between", marginTop: "4px" }}>
                      {QUALITIES.map((q, i) => (
                        <span key={q} style={{
                          fontSize: "14px", color: i === qualityIdx ? "#00f5ff" : "var(--text-dimmer)",
                          letterSpacing: "0.5px", transition: "color 0.2s",
                          cursor: "pointer", userSelect: "none",
                        }} onClick={() => setQualityIdx(i)}>{q}</span>
                      ))}
                    </div>
                  </div>
                </div>
              )}

              {/* Video meta card */}
              {videoTitle && phase !== "idle" && phase !== "fetching" && (
                <div className="anim-float-in" style={{
                  background: "var(--input-bg)", border: "1px solid #00f5ff33", borderRadius: "3px",
                  padding: "12px 14px", marginBottom: "16px", display: "flex", alignItems: "center", gap: "12px",
                  boxShadow: "0 0 20px #00f5ff15",
                }}>
                  <div style={{
                    width: "48px", height: "34px", borderRadius: "2px", flexShrink: 0,
                    background: "linear-gradient(135deg, #b400ff44, #00f5ff22)",
                    border: "1px solid #00f5ff33", display: "flex", alignItems: "center", justifyContent: "center",
                  }}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="#00f5ff88"><path d="M8 5v14l11-7z" /></svg>
                  </div>
                  <div style={{ flex: 1, overflow: "hidden" }}>
                    <div style={{ fontSize: "17px", color: "var(--text)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{videoTitle}</div>
                    <div style={{ display: "flex", gap: "12px", marginTop: "3px" }}>
                      <span style={{ fontSize: "17px", color: "var(--text-dim)" }}>⏱ {videoDuration}</span>
                      <span style={{ fontSize: "17px", color: videoPlatformColor, letterSpacing: "1px" }}>◈ {videoPlatform}</span>
                    </div>
                  </div>
                  <span style={{ fontSize: "17px", color: "#00f5ff", letterSpacing: "2px", border: "1px solid #00f5ff44", padding: "4px 12px", borderRadius: "3px" }}>LOCKED</span>
                </div>
              )}

              {/* Progress bar */}
              {(phase === "downloading" || phase === "done") && (
                <div className="anim-float-in" style={{ marginBottom: "16px" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "5px" }}>
                    <span style={{ fontSize: "17px", color: "#00f5ff", letterSpacing: "2px" }}>
                      {phase === "done" ? "COMPLETE" : "EXTRACTING"}
                      {speed && phase === "downloading" ? ` · ${speed}` : ""}
                      {eta && phase === "downloading" ? ` · ETA ${eta}` : ""}
                    </span>
                    <span style={{ fontSize: "17px", color: "#00f5ff", fontWeight: 700 }}>{Math.round(progress)}%</span>
                  </div>
                  <div className="cyber-progress-track">
                    <div className="cyber-progress-bar" style={{ width: `${progress}%` }} />
                  </div>
                </div>
              )}

              {/* Action buttons */}
              <div style={{ display: "flex", gap: "8px", flexWrap: "wrap" }}>
                <button
                  onClick={phase === "done" ? reset : phase === "ready" ? handleExtract : undefined}
                  disabled={phase === "idle" || phase === "fetching" || phase === "downloading" || phase === "error"}
                  style={{
                    flex: 1, padding: "16px",
                    background: phase === "done" ? "linear-gradient(135deg, #00f5ff22, #00f5ff11)" : "linear-gradient(135deg, #b400ff33, #7700cc22)",
                    border: `1px solid ${phase === "done" ? "#00f5ff" : "#b400ff"}`,
                    borderRadius: "3px", color: phase === "done" ? "#00f5ff" : "#e040fb",
                    fontFamily: "'Orbitron', sans-serif", fontWeight: 700, fontSize: "17px", letterSpacing: "3px",
                    cursor: (phase === "ready" || phase === "done") ? "pointer" : "not-allowed",
                    opacity: (phase === "idle" || phase === "fetching" || phase === "downloading") ? 0.5 : 1,
                    animation: phase === "ready" ? "pulse-cyan 2s infinite" : "none",
                    transition: "all 0.2s",
                  }}
                >
                  {phase === "idle" && "◈ AWAIT TARGET"}
                  {phase === "fetching" && "◈ PROBING TARGET..."}
                  {phase === "ready" && "▶ EXTRACT NOW"}
                  {phase === "downloading" && `◈ EXTRACTING ${Math.round(progress)}%`}
                  {phase === "done" && "✓ COMPLETE — RESET"}
                  {phase === "error" && "✕ ERROR"}
                </button>
                {phase === "downloading" && (
                  <button onClick={handleCancel} style={{
                    padding: "16px 24px", background: "#ff003c22", border: "1px solid #ff003c66",
                    borderRadius: "3px", color: "#ff003c", fontFamily: "'Orbitron', sans-serif",
                    fontWeight: 700, fontSize: "17px", letterSpacing: "2px", cursor: "pointer",
                  }}>ABORT</button>
                )}
              </div>

              {/* Completion actions */}
              {phase === "done" && filePath && !converting && (
                <div className="anim-float-in" style={{ marginTop: "10px" }}>
                  <div style={{ display: "flex", gap: "8px", alignItems: "center", flexWrap: "wrap" }}>
                    <button onClick={() => openFile(filePath)} style={{
                      padding: "10px 18px", background: "#00f5ff11", border: "1px solid #00f5ff44",
                      borderRadius: "3px", color: "#00f5ff", fontFamily: "'Share Tech Mono', monospace",
                      fontSize: "14px", cursor: "pointer", letterSpacing: "1px", transition: "all 0.2s",
                    }}>▶ OPEN FILE</button>
                    <button onClick={() => showInFolder(filePath)} style={{
                      padding: "10px 18px", background: "#b400ff11", border: "1px solid #b400ff44",
                      borderRadius: "3px", color: "#e040fb", fontFamily: "'Share Tech Mono', monospace",
                      fontSize: "14px", cursor: "pointer", letterSpacing: "1px", transition: "all 0.2s",
                    }}>◈ SHOW IN FOLDER</button>
                    {fileSize && (
                      <span style={{ fontSize: "14px", color: "var(--text-dim)", marginLeft: "auto", letterSpacing: "1px" }}>
                        {fileSize > 1073741824 ? `${(fileSize / 1073741824).toFixed(2)} GB` : `${(fileSize / 1048576).toFixed(1)} MB`}
                      </span>
                    )}
                  </div>
                  {ffmpegOk && (
                    <div style={{ display: "flex", gap: "6px", marginTop: "8px", flexWrap: "wrap" }}>
                      <span style={{ fontSize: "12px", color: "var(--text-dim)", letterSpacing: "2px", alignSelf: "center", marginRight: "4px" }}>CONVERT:</span>
                      {([
                        { label: "MP4", preset: { type: "ToMp4" } as ConversionPreset },
                        { label: "MKV", preset: { type: "ToMkv" } as ConversionPreset },
                        { label: "H.265", preset: { type: "ToMp4H265" } as ConversionPreset },
                        { label: "720p", preset: { type: "Compress720p" } as ConversionPreset },
                        { label: "480p", preset: { type: "Compress480p" } as ConversionPreset },
                      ]).map(({ label, preset }) => (
                        <button key={label} onClick={() => handleConvert(preset)} style={{
                          padding: "5px 10px", background: "transparent", border: "1px solid var(--border-dim)",
                          borderRadius: "2px", color: "var(--text-dim)", fontFamily: "'Share Tech Mono', monospace",
                          fontSize: "11px", cursor: "pointer", transition: "all 0.2s", letterSpacing: "1px",
                        }}>{label}</button>
                      ))}
                      <span style={{ fontSize: "12px", color: "var(--text-dim)", letterSpacing: "2px", alignSelf: "center", margin: "0 4px" }}>AUDIO:</span>
                      {([
                        { label: "MP3", preset: { type: "ToMp3", bitrate: 320 } as ConversionPreset },
                        { label: "FLAC", preset: { type: "ToFlac" } as ConversionPreset },
                        { label: "WAV", preset: { type: "ToWav" } as ConversionPreset },
                      ]).map(({ label, preset }) => (
                        <button key={label} onClick={() => handleConvert(preset)} style={{
                          padding: "5px 10px", background: "transparent", border: "1px solid #00f5ff33",
                          borderRadius: "2px", color: "#00f5ff88", fontFamily: "'Share Tech Mono', monospace",
                          fontSize: "11px", cursor: "pointer", transition: "all 0.2s", letterSpacing: "1px",
                        }}>{label}</button>
                      ))}
                    </div>
                  )}
                </div>
              )}
              {converting && (
                <div className="anim-float-in" style={{ marginTop: "10px", padding: "12px", background: "var(--input-bg)", border: "1px solid #b400ff44", borderRadius: "3px" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "6px" }}>
                    <span style={{ fontSize: "14px", color: "#b400ff", letterSpacing: "2px" }}>CONVERTING...</span>
                    <span style={{ fontSize: "14px", color: "#b400ff", fontWeight: 700 }}>{Math.round(progress)}%</span>
                  </div>
                  <div className="cyber-progress-track">
                    <div className="cyber-progress-bar" style={{ width: `${progress}%`, background: "linear-gradient(90deg, #b400ff, #e040fb)" }} />
                  </div>
                </div>
              )}
            </div>

            {/* System Log */}
            {log.length > 0 && (
              <div className="anim-float-in" style={{
                background: "#050310", border: "1px solid #b400ff22", borderRadius: "3px",
                padding: "14px 16px", marginBottom: "16px",
              }}>
                <div style={{ fontSize: "17px", letterSpacing: "3px", color: "var(--purple)", marginBottom: "8px" }}>▸ SYSTEM LOG</div>
                <div ref={logRef} style={{ maxHeight: "110px", overflowY: "auto", display: "flex", flexDirection: "column", gap: "3px" }}>
                  {log.map((l, i) => (
                    <div key={i} style={{ display: "flex", gap: "12px", fontSize: "17px" }}>
                      <span style={{ color: "var(--text-dimmer)", flexShrink: 0 }}>{l.time}</span>
                      <span style={{ color: l.msg.includes("ERR") || l.msg.includes("ERROR") ? "var(--red)" : l.msg.includes("✓") || l.msg.includes("COMPLETE") ? "#00ff88" : "var(--text-muted)" }}>{l.msg}</span>
                    </div>
                  ))}
                  {(phase === "fetching" || phase === "downloading") && (
                    <span className="anim-blink" style={{ display: "inline-block", width: "8px", height: "12px", background: "#b400ff", borderRadius: "1px", marginLeft: "2px" }} />
                  )}
                </div>
              </div>
            )}

            {/* Extraction History */}
            {history.length > 0 && (
              <div className="anim-float-in" style={{
                background: "linear-gradient(145deg, var(--panel), var(--panel-alt))",
                border: "1px solid #3a2a5555", borderRadius: "4px", padding: "18px 20px",
              }}>
                <div style={{ fontSize: "17px", letterSpacing: "3px", color: "var(--text-dim)", marginBottom: "12px" }}>▸ EXTRACTION HISTORY</div>
                <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
                  {history.map((h, i) => (
                    <div key={i} style={{
                      display: "flex", alignItems: "center", gap: "10px",
                      padding: "8px 10px", background: "#0a061422", border: "1px solid #3a2a5533", borderRadius: "2px",
                      animation: i === 0 ? "float-in 0.4s ease both" : "none",
                    }}>
                      <div style={{ width: "6px", height: "8px", borderRadius: "50%", background: "#00f5ff", boxShadow: "0 0 8px #00f5ff", flexShrink: 0 }} />
                      <div style={{ flex: 1, overflow: "hidden" }}>
                        <div style={{ fontSize: "17px", color: "#c084fc", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{h.title}</div>
                        <div style={{ fontSize: "17px", color: "var(--text-dimmer)", marginTop: "2px" }}>{h.quality} · {h.site_name}</div>
                      </div>
                      <span style={{ fontSize: "17px", color: "#00f5ff", letterSpacing: "1px", flexShrink: 0 }}>✓ DONE</span>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Footer */}
            <div style={{ textAlign: "center", marginTop: "20px" }}>
              <p style={{ fontSize: "17px", color: "#2a1e3a", letterSpacing: "2px" }}>
                CYBERSNATCHER · POWERED BY yt-dlp + ffmpeg · TAURI 2.x
              </p>
            </div>
          </div>
        </div>
      )}

      {/* Duplicate Detection Modal */}
      {duplicateUrl && (
        <div style={{
          position: "fixed", inset: 0, background: "rgba(0,0,0,0.7)", zIndex: 1000,
          display: "flex", alignItems: "center", justifyContent: "center",
        }} onClick={() => setDuplicateUrl(null)}>
          <div onClick={(e) => e.stopPropagation()} style={{
            background: "var(--panel)", border: "1px solid #fbbf24", borderRadius: "4px",
            padding: "24px 28px", maxWidth: "420px", width: "90%",
          }}>
            <div style={{ fontSize: "14px", color: "#fbbf24", letterSpacing: "2px", fontFamily: "'Orbitron', sans-serif", fontWeight: 700, marginBottom: "12px" }}>
              DUPLICATE DETECTED
            </div>
            <p style={{ fontSize: "13px", color: "var(--text-dim)", marginBottom: "6px", lineHeight: 1.5 }}>
              This URL has already been downloaded:
            </p>
            <p style={{ fontSize: "11px", color: "var(--text-muted)", wordBreak: "break-all", marginBottom: "16px",
              padding: "8px", background: "var(--input-bg)", border: "1px solid var(--border-dim)", borderRadius: "2px" }}>
              {duplicateUrl}
            </p>
            <div style={{ display: "flex", gap: "8px" }}>
              <button onClick={() => setDuplicateUrl(null)} style={{
                flex: 1, padding: "10px", background: "transparent", border: "1px solid var(--border-dim)",
                borderRadius: "3px", color: "var(--text-dim)", fontFamily: "'Orbitron', sans-serif",
                fontSize: "11px", fontWeight: 700, letterSpacing: "2px", cursor: "pointer",
              }}>CANCEL</button>
              <button onClick={() => forceProbeDuplicate()} style={{
                flex: 1, padding: "10px", background: "#fbbf2422", border: "1px solid #fbbf24",
                borderRadius: "3px", color: "#fbbf24", fontFamily: "'Orbitron', sans-serif",
                fontSize: "11px", fontWeight: 700, letterSpacing: "2px", cursor: "pointer",
              }}>DOWNLOAD ANYWAY</button>
            </div>
          </div>
        </div>
      )}

      {/* Settings Modal */}
      {showSettings && (
        <SettingsModal
          onClose={() => setShowSettings(false)}
          ytdlpInstalled={ytdlpOk}
        />
      )}
    </div>
  );
}
