export interface DetectedVideo {
  url: string;
  video_type: "direct" | "hls" | "dash";
  label: string;
  page_url: string;
  page_title: string;
  file_size?: number | null;
  cookies?: string | null;
}

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
