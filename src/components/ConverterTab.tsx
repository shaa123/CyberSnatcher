import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { pickFile, convertFile, checkFfmpeg, openFile, showInFolder } from "../lib/tauri";
import type { ConversionPreset, DownloadProgress } from "../lib/types";

const VIDEO_PRESETS: { label: string; desc: string; preset: ConversionPreset }[] = [
  { label: "MP4", desc: "H.264 universal", preset: { type: "ToMp4" } },
  { label: "MKV", desc: "Matroska container", preset: { type: "ToMkv" } },
  { label: "H.265", desc: "HEVC smaller size", preset: { type: "ToMp4H265" } },
  { label: "720p", desc: "Compress to 720p", preset: { type: "Compress720p" } },
  { label: "480p", desc: "Compress to 480p", preset: { type: "Compress480p" } },
];

const AUDIO_PRESETS: { label: string; desc: string; preset: ConversionPreset }[] = [
  { label: "MP3", desc: "320kbps audio", preset: { type: "ToMp3", bitrate: 320 } },
  { label: "FLAC", desc: "Lossless audio", preset: { type: "ToFlac" } },
  { label: "WAV", desc: "Uncompressed", preset: { type: "ToWav" } },
];

const FILE_FILTERS = [
  {
    name: "Media Files",
    extensions: ["mp4", "mkv", "webm", "avi", "mov", "flv", "m4v", "ts", "mp3", "flac", "wav", "ogg", "m4a", "aac", "wma"],
  },
];

export default function ConverterTab() {
  const [ffmpegOk, setFfmpegOk] = useState(true);
  const [filePath, setFilePath] = useState<string | null>(null);
  const [fileName, setFileName] = useState("");
  const [, setFileSize] = useState<number | null>(null);
  const [selectedPreset, setSelectedPreset] = useState<{ label: string; preset: ConversionPreset } | null>(null);
  const [phase, setPhase] = useState<"idle" | "converting" | "done" | "error">("idle");
  const [progress, setProgress] = useState(0);
  const [outputPath, setOutputPath] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState("");
  const [jobId, setJobId] = useState("");

  useEffect(() => {
    checkFfmpeg().then(setFfmpegOk).catch(() => setFfmpegOk(false));
  }, []);

  // Listen for conversion progress
  useEffect(() => {
    if (!jobId) return;
    const unlisten = listen<DownloadProgress>("download-progress", (event) => {
      const p = event.payload;
      if (p.job_id !== jobId) return;

      if (p.status === "complete") {
        setProgress(100);
        setPhase("done");
        if (p.file_path) setOutputPath(p.file_path);
        if (p.file_size) setFileSize(p.file_size);
      } else if (p.status === "error") {
        setPhase("error");
        setErrorMsg(p.log_line || "Conversion failed");
      } else if (p.percent >= 0) {
        setProgress(p.percent);
      }
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [jobId]);

  const handlePickFile = useCallback(async () => {
    const path = await pickFile(FILE_FILTERS);
    if (path) {
      setFilePath(path);
      const name = path.split(/[/\\]/).pop() || path;
      setFileName(name);
      setPhase("idle");
      setOutputPath(null);
      setSelectedPreset(null);
      setProgress(0);
      setErrorMsg("");
    }
  }, []);

  const handleConvert = useCallback(async () => {
    if (!filePath || !selectedPreset) return;
    const id = `conv-${Date.now()}`;
    setJobId(id);
    setPhase("converting");
    setProgress(0);
    setOutputPath(null);
    setErrorMsg("");
    try {
      const result = await convertFile(id, filePath, selectedPreset.preset);
      setOutputPath(result);
      setPhase("done");
      setProgress(100);
    } catch (e) {
      setPhase("error");
      setErrorMsg(String(e));
    }
  }, [filePath, selectedPreset]);

  const reset = () => {
    setFilePath(null);
    setFileName("");
    setFileSize(null);
    setSelectedPreset(null);
    setPhase("idle");
    setProgress(0);
    setOutputPath(null);
    setErrorMsg("");
    setJobId("");
  };

  if (!ffmpegOk) {
    return (
      <div style={{ width: "100%", maxWidth: "700px" }}>
        <div className="corner-accents" style={{
          background: "linear-gradient(145deg, var(--panel) 0%, var(--panel-alt) 100%)",
          border: "1px solid var(--border-purple)", borderRadius: "4px",
          padding: "30px", textAlign: "center",
        }}>
          <div style={{ fontSize: "17px", color: "#ff003c", marginBottom: "8px", fontWeight: 700 }}>
            FFmpeg NOT FOUND
          </div>
          <p style={{ fontSize: "17px", color: "var(--text-dim)", lineHeight: 1.6 }}>
            FFmpeg is required for media conversion. Make sure ffmpeg is installed and accessible.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div style={{ width: "100%", maxWidth: "700px" }}>
      <div className="corner-accents anim-pulse-border" style={{
        background: "linear-gradient(145deg, var(--panel) 0%, var(--panel-alt) 100%)",
        border: "1px solid var(--border-purple)", borderRadius: "4px",
        padding: "30px", position: "relative",
        animation: "pulse-border 4s ease-in-out infinite",
      }}>
        <div style={{ position: "absolute", bottom: "-1px", left: "-1px", width: "14px", height: "14px", borderBottom: "2px solid var(--cyan)", borderLeft: "2px solid var(--cyan)" }} />
        <div style={{ position: "absolute", bottom: "-1px", right: "-1px", width: "14px", height: "14px", borderBottom: "2px solid var(--cyan)", borderRight: "2px solid var(--cyan)" }} />

        {/* File picker */}
        <label style={{ display: "block", fontSize: "17px", color: "var(--purple)", marginBottom: "8px" }}>
          ▸ SOURCE FILE
        </label>

        <div style={{ display: "flex", gap: "10px", marginBottom: "20px" }}>
          <div
            onClick={phase !== "converting" ? handlePickFile : undefined}
            style={{
              flex: 1, padding: "14px",
              background: "var(--input-bg)",
              border: `1px solid ${filePath ? "#00f5ff44" : "var(--border-purple)"}`,
              borderRadius: "3px",
              color: filePath ? "var(--text)" : "var(--text-dimmer)",
              fontSize: "17px",
              cursor: phase !== "converting" ? "pointer" : "not-allowed",
              overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
              transition: "all 0.2s",
            }}
          >
            {fileName || "Click to select a media file..."}
          </div>
          <button
            onClick={handlePickFile}
            disabled={phase === "converting"}
            style={{
              background: "linear-gradient(135deg, #b400ff22, #7700cc22)",
              border: "1px solid #b400ff",
              borderRadius: "3px", color: "#e040fb",
              fontFamily: "system-ui, -apple-system, sans-serif", fontSize: "17px", fontWeight: 700,
              padding: "0 20px", cursor: phase !== "converting" ? "pointer" : "not-allowed",
              opacity: phase === "converting" ? 0.5 : 1,
            }}
          >
            BROWSE
          </button>
        </div>

        {/* Preset selection — only show when file is selected */}
        {filePath && phase !== "done" && (
          <>
            <label style={{ display: "block", fontSize: "17px", color: "var(--purple)", marginBottom: "8px" }}>
              ▸ VIDEO FORMATS
            </label>
            <div style={{ display: "flex", gap: "6px", flexWrap: "wrap", marginBottom: "16px" }}>
              {VIDEO_PRESETS.map((p) => (
                <button
                  key={p.label}
                  onClick={() => phase !== "converting" && setSelectedPreset(p)}
                  disabled={phase === "converting"}
                  style={{
                    flex: "1 1 0", minWidth: "100px", padding: "10px 8px", textAlign: "center",
                    background: selectedPreset?.label === p.label ? "#b400ff18" : "var(--input-bg)",
                    border: `1px solid ${selectedPreset?.label === p.label ? "#b400ff" : "var(--border-dim)"}`,
                    borderRadius: "3px", cursor: phase !== "converting" ? "pointer" : "not-allowed",
                    transition: "all 0.2s",
                    boxShadow: selectedPreset?.label === p.label ? "0 0 10px #b400ff30" : "none",
                  }}
                >
                  <div style={{ fontSize: "17px", color: "#e040fb", fontFamily: "system-ui, -apple-system, sans-serif", fontWeight: 700 }}>{p.label}</div>
                  <div style={{ fontSize: "17px", color: "var(--text-dimmer)", marginTop: "2px" }}>{p.desc}</div>
                </button>
              ))}
            </div>

            <label style={{ display: "block", fontSize: "17px", color: "var(--purple)", marginBottom: "8px" }}>
              ▸ AUDIO FORMATS
            </label>
            <div style={{ display: "flex", gap: "6px", flexWrap: "wrap", marginBottom: "20px" }}>
              {AUDIO_PRESETS.map((p) => (
                <button
                  key={p.label}
                  onClick={() => phase !== "converting" && setSelectedPreset(p)}
                  disabled={phase === "converting"}
                  style={{
                    flex: "1 1 0", minWidth: "100px", padding: "10px 8px", textAlign: "center",
                    background: selectedPreset?.label === p.label ? "#00f5ff18" : "var(--input-bg)",
                    border: `1px solid ${selectedPreset?.label === p.label ? "#00f5ff" : "var(--border-dim)"}`,
                    borderRadius: "3px", cursor: phase !== "converting" ? "pointer" : "not-allowed",
                    transition: "all 0.2s",
                    boxShadow: selectedPreset?.label === p.label ? "0 0 10px #00f5ff30" : "none",
                  }}
                >
                  <div style={{ fontSize: "17px", color: "#00f5ff", fontFamily: "system-ui, -apple-system, sans-serif", fontWeight: 700 }}>{p.label}</div>
                  <div style={{ fontSize: "17px", color: "var(--text-dimmer)", marginTop: "2px" }}>{p.desc}</div>
                </button>
              ))}
            </div>
          </>
        )}

        {/* Progress */}
        {phase === "converting" && (
          <div className="anim-float-in" style={{ marginBottom: "16px" }}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "5px" }}>
              <span style={{ fontSize: "17px", color: "#b400ff" }}>CONVERTING...</span>
              <span style={{ fontSize: "17px", color: "#b400ff", fontWeight: 700 }}>{Math.round(progress)}%</span>
            </div>
            <div className="cyber-progress-track">
              <div className="cyber-progress-bar" style={{ width: `${progress}%`, background: "linear-gradient(90deg, #b400ff, #e040fb)" }} />
            </div>
          </div>
        )}

        {/* Error */}
        {phase === "error" && (
          <div className="anim-float-in" style={{
            padding: "12px", background: "#ff003c11", border: "1px solid #ff003c44",
            borderRadius: "3px", marginBottom: "16px",
          }}>
            <span style={{ fontSize: "17px", color: "#ff003c" }}>ERROR: {errorMsg}</span>
          </div>
        )}

        {/* Convert button */}
        {filePath && phase !== "done" && (
          <button
            onClick={handleConvert}
            disabled={!selectedPreset || phase === "converting"}
            style={{
              width: "100%", padding: "16px",
              background: selectedPreset ? "linear-gradient(135deg, #b400ff33, #7700cc22)" : "transparent",
              border: `1px solid ${selectedPreset ? "#b400ff" : "var(--border-dim)"}`,
              borderRadius: "3px",
              color: selectedPreset ? "#e040fb" : "var(--text-dimmer)",
              fontFamily: "system-ui, -apple-system, sans-serif", fontWeight: 700, fontSize: "17px",
              cursor: selectedPreset && phase !== "converting" ? "pointer" : "not-allowed",
              opacity: !selectedPreset || phase === "converting" ? 0.5 : 1,
              animation: selectedPreset && phase === "idle" ? "pulse-cyan 2s infinite" : "none",
              transition: "all 0.2s",
            }}
          >
            {phase === "converting" ? `◈ CONVERTING ${Math.round(progress)}%` : selectedPreset ? `▶ CONVERT TO ${selectedPreset.label}` : "◈ SELECT A FORMAT"}
          </button>
        )}

        {/* Completion */}
        {phase === "done" && outputPath && (
          <div className="anim-float-in">
            <div style={{
              padding: "14px", background: "#00f5ff11", border: "1px solid #00f5ff44",
              borderRadius: "3px", marginBottom: "12px",
            }}>
              <div style={{ fontSize: "17px", color: "#00f5ff", fontWeight: 700, marginBottom: "6px" }}>
                ✓ CONVERSION COMPLETE
              </div>
              <div style={{ fontSize: "17px", color: "var(--text-dim)", wordBreak: "break-all" }}>
                {outputPath}
              </div>
            </div>
            <div style={{ display: "flex", gap: "8px" }}>
              <button onClick={() => openFile(outputPath)} style={{
                flex: 1, padding: "12px", background: "#00f5ff11", border: "1px solid #00f5ff44",
                borderRadius: "3px", color: "#00f5ff", fontFamily: "system-ui, -apple-system, sans-serif",
                fontSize: "17px", fontWeight: 700, cursor: "pointer", transition: "all 0.2s",
              }}>▶ OPEN FILE</button>
              <button onClick={() => showInFolder(outputPath)} style={{
                flex: 1, padding: "12px", background: "#b400ff11", border: "1px solid #b400ff44",
                borderRadius: "3px", color: "#e040fb", fontFamily: "system-ui, -apple-system, sans-serif",
                fontSize: "17px", fontWeight: 700, cursor: "pointer", transition: "all 0.2s",
              }}>◈ SHOW IN FOLDER</button>
              <button onClick={reset} style={{
                flex: 1, padding: "12px", background: "transparent", border: "1px solid var(--border-dim)",
                borderRadius: "3px", color: "var(--text-dim)", fontFamily: "system-ui, -apple-system, sans-serif",
                fontSize: "17px", fontWeight: 700, cursor: "pointer", transition: "all 0.2s",
              }}>↻ CONVERT ANOTHER</button>
            </div>
          </div>
        )}

        {/* Empty state hint */}
        {!filePath && (
          <div style={{ textAlign: "center", padding: "20px 0" }}>
            <div style={{ fontSize: "17px", color: "var(--text-dimmer)", marginBottom: "8px" }}>
              Select a media file to convert between formats
            </div>
            <div style={{ fontSize: "17px", color: "var(--text-dimmer)" }}>
              Supports MP4, MKV, WEBM, AVI, MOV, MP3, FLAC, WAV, and more
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
