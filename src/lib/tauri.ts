import { invoke } from "@tauri-apps/api/core";
import type { UrlAnalysis, ConversionPreset, MediaInfo, DetectedVideo } from "./types";

// ── Existing commands ────────────────────────────────────────────────────────

export const checkYtdlp = () => invoke<boolean>("check_ytdlp");
export const checkFfmpeg = () => invoke<boolean>("check_ffmpeg");
export const getYtdlpVersion = () => invoke<string>("get_ytdlp_version");
export const updateYtdlp = () => invoke<string>("update_ytdlp");
export const analyzeUrl = (url: string) => invoke<UrlAnalysis>("analyze_url", { url });

export const startDownload = (
  jobId: string, url: string, title: string,
  outputDir: string, formatQuality: string, formatType: string,
  writeSubs?: boolean
) => invoke<void>("start_download", { jobId, url, title, outputDir, formatQuality, formatType, writeSubs: writeSubs || false });

export const cancelDownload = (jobId: string) => invoke<void>("cancel_download", { jobId });

export const nativeDownload = (
  jobId: string, url: string, outputDir: string, filename: string,
  pageUrl?: string, cookies?: string
) => invoke<string>("native_download", { jobId, url, pageUrl: pageUrl || null, cookies: cookies || null, outputDir, filename });

export const convertFile = (jobId: string, inputPath: string, preset: ConversionPreset) =>
  invoke<string>("convert_file", { jobId, inputPath, preset });

export const getMediaInfo = (filePath: string) => invoke<MediaInfo>("get_media_info", { filePath });

export const createFolder = (path: string) => invoke<void>("create_folder", { path });
export const deleteFolder = (path: string) => invoke<void>("delete_folder", { path });
export const listFolderContents = (path: string) => invoke<string[]>("list_folder_contents", { path });
export const openInExplorer = (path: string) => invoke<void>("open_in_explorer", { path });
export const openFile = (path: string) => invoke<void>("open_file", { path });
export const showInFolder = (path: string) => invoke<void>("show_in_folder", { path });

export async function pickFolder(): Promise<string | null> {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({ directory: true, multiple: false });
    return selected as string | null;
  } catch {
    return null;
  }
}

// ── Browser commands ─────────────────────────────────────────────────────────

export const openBrowserView = (url: string, x: number, y: number, width: number, height: number) =>
  invoke<void>("open_browser_view", { url, x, y, width, height });

export const navigateBrowser = (url: string) => invoke<void>("navigate_browser", { url });

export const resizeBrowser = (x: number, y: number, width: number, height: number) =>
  invoke<void>("resize_browser", { x, y, width, height });

export const closeBrowser = () => invoke<void>("close_browser");
export const browserGoBack = () => invoke<void>("browser_go_back");
export const browserGoForward = () => invoke<void>("browser_go_forward");
export const browserRefresh = () => invoke<void>("browser_refresh");
export const getDetectedVideos = () => invoke<DetectedVideo[]>("get_detected_videos");
export const showBrowser = () => invoke<void>("show_browser");
export const hideBrowser = () => invoke<void>("hide_browser");

export const getBrowserCookies = (url: string) =>
  invoke<string>("get_browser_cookies", { url });

export const removeDetectedVideo = (url: string) =>
  invoke<void>("remove_detected_video", { url });

export interface BrowserSettings {
  adblock_enabled: boolean;
  popup_blocker_enabled: boolean;
}

export const getBrowserSettings = () =>
  invoke<BrowserSettings>("get_browser_settings");

export const setBrowserSettings = (adblockEnabled: boolean, popupBlockerEnabled: boolean) =>
  invoke<void>("set_browser_settings", { adblockEnabled, popupBlockerEnabled });

// ── HLS stream commands ──────────────────────────────────────────────────────

export interface HlsQuality {
  url: string;
  bandwidth: number;
  label: string;
  resolution: string | null;
}

export interface HlsParseResult {
  is_master: boolean;
  qualities: HlsQuality[];
  media_info: {
    segments: number;
    duration: number;
    is_live: boolean;
    encrypted: boolean;
  } | null;
}

export const parseHls = (url: string) =>
  invoke<HlsParseResult>("parse_hls", { url });

export const downloadHlsStream = (
  jobId: string, url: string, outputDir: string, filename: string,
  qualityIdx?: number, cookies?: string, pageUrl?: string
) => invoke<string>("download_hls_stream", {
  jobId, url, outputDir, filename,
  qualityIdx: qualityIdx ?? null,
  cookies: cookies ?? null,
  pageUrl: pageUrl ?? null,
});

export const downloadHlsLiveStream = (
  jobId: string, url: string, outputDir: string, filename: string,
  qualityIdx?: number, cookies?: string, pageUrl?: string
) => invoke<string>("download_hls_live_stream", {
  jobId, url, outputDir, filename,
  qualityIdx: qualityIdx ?? null,
  cookies: cookies ?? null,
  pageUrl: pageUrl ?? null,
});

// ── DASH stream commands ─────────────────────────────────────────────────────

export interface DashQuality {
  rep_id: string;
  bandwidth: number;
  height: number | null;
  label: string;
  codecs: string | null;
}

export interface DashParseResult {
  qualities: DashQuality[];
  duration: number | null;
  has_audio: boolean;
}

export const parseDash = (url: string) =>
  invoke<DashParseResult>("parse_dash", { url });

export const downloadDashStream = (
  jobId: string, url: string, outputDir: string, filename: string,
  repId?: string, cookies?: string, pageUrl?: string
) => invoke<string>("download_dash_stream", {
  jobId, url, outputDir, filename,
  repId: repId ?? null,
  cookies: cookies ?? null,
  pageUrl: pageUrl ?? null,
});
