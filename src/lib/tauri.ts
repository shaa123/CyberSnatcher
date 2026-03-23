import { invoke } from "@tauri-apps/api/core";
import type { UrlAnalysis, ConversionPreset, MediaInfo } from "./types";

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

export async function pickFile(filters?: { name: string; extensions: string[] }[]): Promise<string | null> {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({ directory: false, multiple: false, filters });
    return selected as string | null;
  } catch {
    return null;
  }
}
