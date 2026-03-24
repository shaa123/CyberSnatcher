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
  status: "queued" | "analyzing" | "downloading" | "complete" | "error" | "cancelled";
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

export interface LicenseStatus {
  activated: boolean;
  license_key: string | null;
  email: string | null;
  product_name: string | null;
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

// ── Scraper types ───────────────────────────────────────────────────────────

export interface FieldRule {
  name: string;
  css_selector: string;
  extract: string;
  regex_filter: string | null;
}

export interface CrawlRule {
  link_selector: string;
  url_pattern: string | null;
}

export interface SpiderConfig {
  name: string;
  start_urls: string[];
  field_rules: FieldRule[];
  crawl_rules: CrawlRule[];
  max_pages: number;
  concurrency: number;
  request_delay_ms: number;
  respect_robots: boolean;
  user_agent: string | null;
  headers: Record<string, string>;
}

export interface ScrapedItem {
  fields: Record<string, string>;
}

export interface ScrapeProgress {
  job_id: string;
  pages_crawled: number;
  pages_total: number;
  items_scraped: number;
  current_url: string;
  status: string;
  log_line: string;
}

export interface ScrapeResult {
  job_id: string;
  items: ScrapedItem[];
  pages_crawled: number;
  errors: string[];
  export_path: string | null;
}
