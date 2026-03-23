import { invoke } from "@tauri-apps/api/core";
import type { UrlAnalysis, ConversionPreset, MediaInfo, FavoriteItem } from "./types";

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

export const createBrowserWebview = (url: string) =>
  invoke<void>("create_browser_webview", { url });

export const destroyBrowserWebview = () =>
  invoke<void>("destroy_browser_webview");

export const navigateBrowser = (url: string) =>
  invoke<void>("navigate_browser", { url });

export const browserGoBack = () => invoke<void>("browser_go_back");
export const browserGoForward = () => invoke<void>("browser_go_forward");
export const browserReload = () => invoke<void>("browser_reload");

export const resizeBrowserWebview = (x: number, y: number, width: number, height: number) =>
  invoke<void>("resize_browser_webview", { x, y, width, height });

// ── Favorites commands ───────────────────────────────────────────────────────

export const addFavorite = (id: string, url: string, title: string, favicon?: string) =>
  invoke<void>("add_favorite", { id, url, title, favicon: favicon || null });

export const removeFavorite = (id: string) =>
  invoke<void>("remove_favorite", { id });

export const listFavorites = () =>
  invoke<FavoriteItem[]>("list_favorites");

export const isFavorite = (url: string) =>
  invoke<boolean>("is_favorite", { url });

// ── Stream detection commands ────────────────────────────────────────────────

export const validateStream = (
  manifestUrl: string,
  pageUrl: string,
  streamType: string,
  minDuration: number,
  minFileSize: number
) =>
  invoke<{ title: string; duration: number | null; size: number | null; qualities: string[] } | null>(
    "validate_stream",
    { manifestUrl, pageUrl, streamType, minDuration, minFileSize }
  );

// ── Browser download (NO yt-dlp) ────────────────────────────────────────────

export const startBrowserDownload = (
  jobId: string,
  manifestUrl: string,
  streamType: string,
  pageUrl: string,
  quality: string,
  outputDir: string,
  filename: string
) =>
  invoke<string>("start_browser_download", {
    jobId,
    manifestUrl,
    streamType,
    pageUrl,
    quality,
    outputDir,
    filename,
  });

// ── Browser settings ─────────────────────────────────────────────────────────

export const saveBrowserSettings = (minDuration: number, minFileSize: number) =>
  invoke<void>("save_browser_settings", { minDuration, minFileSize });

export const loadBrowserSettings = () =>
  invoke<{ minDuration: number; minFileSize: number }>("load_browser_settings");
