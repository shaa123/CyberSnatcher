export interface DownloadProgress {
  job_id: string;
  percent: number;
  speed: string;
  eta: string;
  status: string;
  log_line: string;
  file_path?: string;
  file_size?: number;
}

export interface DownloadItem {
  id: string;
  url: string;
  title: string;
  site_name: string;
  status: "queued" | "downloading" | "complete" | "error" | "cancelled";
  progress: number;
  speed: string;
  eta: string;
  outputDir: string;
  quality: string;
  formatType: string;
  logs: string[];
  filePath?: string;
  fileSize?: number;
  created_at: number;
}

export interface ConversionPreset {
  type: "ToMp4" | "ToMkv" | "ToMp4H265" | "Compress720p" | "Compress480p" | "ToMp3" | "ToFlac" | "ToWav";
  bitrate?: number;
}

export interface UrlAnalysis {
  title: string;
  thumbnail: string;
  duration: string;
  site_name: string;
  media_type: string;
  qualities: string[];
}

export interface MediaInfo {
  width: number;
  height: number;
  duration: string;
  codec_name: string;
  codec_type: string;
  bit_rate: string;
  nb_frames: string;
}

// ── Browser Types ──

export interface DetectedStream {
  id: string;
  url: string;
  type: "hls" | "dash";
  pageUrl: string;
  pageTitle: string;
  estimatedDuration: number | null;
  estimatedSize: number | null;
  qualities: string[];
  detectedAt: number;
}

export interface FavoriteItem {
  id: string;
  url: string;
  title: string;
  favicon?: string;
  createdAt: number;
}

export interface BrowserSettings {
  minDuration: number;    // seconds, default 40
  minFileSize: number;    // bytes, default 2097152 (2MB)
}
