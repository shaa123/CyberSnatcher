// ADD this field to your DetectedVideo interface in src/lib/types.ts:
// cookies?: string | null;

// Full interface should look like:
export interface DetectedVideo {
  url: string;
  video_type: "direct" | "hls" | "dash" | "capture";
  label: string;
  page_url: string;
  page_title: string;
  file_path?: string | null;
  file_size?: number | null;
  cookies?: string | null;
}
