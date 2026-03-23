import { useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { useBrowserStore } from "../../stores/browserStore";
import {
  createBrowserWebview,
  destroyBrowserWebview,
  resizeBrowserWebview,
  validateStream,
} from "../../lib/tauri";
import BrowserBar from "./BrowserBar";
import DetectedVideos from "./DetectedVideos";
import type { DetectedStream } from "../../lib/types";

function getOrigin(url: string): string {
  try { return new URL(url).origin; } catch { return url; }
}

interface BrowserViewProps {
  settingsOpen: boolean;
}

export default function BrowserView({ settingsOpen }: BrowserViewProps) {
  const setBrowserUrl = useBrowserStore((s) => s.setBrowserUrl);
  const setIsLoading = useBrowserStore((s) => s.setIsLoading);
  const addDetectedStream = useBrowserStore((s) => s.addDetectedStream);
  const browserSettings = useBrowserStore((s) => s.browserSettings);
  const clearDetectedStreams = useBrowserStore((s) => s.clearDetectedStreams);
  const browserUrl = useBrowserStore((s) => s.browserUrl);
  const streams = useBrowserStore((s) => s.detectedStreams);
  const webviewAreaRef = useRef<HTMLDivElement>(null);
  const webviewCreatedRef = useRef(false);
  const lastOriginRef = useRef(getOrigin(browserUrl));

  // Create webview on mount
  useEffect(() => {
    const timer = setTimeout(() => {
      createBrowserWebview(browserUrl)
        .then(() => {
          webviewCreatedRef.current = true;
        })
        .catch((e) => console.error("Failed to create browser webview:", e));
    }, 100);
    return () => {
      clearTimeout(timer);
      webviewCreatedRef.current = false;
      destroyBrowserWebview().catch(() => {});
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Resize webview to fit container — and HIDE when settings is open
  useEffect(() => {
    const el = webviewAreaRef.current;
    if (!el) return;

    const resize = () => {
      if (settingsOpen) {
        // Move webview off-screen so settings modal is accessible
        resizeBrowserWebview(0, -9999, 1, 1).catch(() => {});
        return;
      }
      const rect = el.getBoundingClientRect();
      resizeBrowserWebview(
        Math.round(rect.x),
        Math.round(rect.y),
        Math.round(rect.width),
        Math.round(rect.height)
      ).catch(() => {});
    };

    // Delay to let webview fully create before first resize
    const timer = setTimeout(resize, 300);
    // Safety net for slow creation
    const timer2 = setTimeout(resize, 600);
    const observer = new ResizeObserver(resize);
    observer.observe(el);
    window.addEventListener("resize", resize);

    return () => {
      clearTimeout(timer);
      clearTimeout(timer2);
      observer.disconnect();
      window.removeEventListener("resize", resize);
    };
  }, [settingsOpen, streams.length]); // re-run when detection panel appears/disappears

  // Listen for browser events from Rust
  useEffect(() => {
    const unlistenNav = listen<{ url: string }>("browser-navigated", (e) => {
      const newUrl = e.payload.url;
      setBrowserUrl(newUrl);

      // Clear detected streams when navigating to a different site
      const newOrigin = getOrigin(newUrl);
      if (newOrigin !== lastOriginRef.current) {
        clearDetectedStreams();
        lastOriginRef.current = newOrigin;
      }
    });
    const unlistenLoading = listen<{ loading: boolean }>(
      "browser-loading",
      (e) => {
        setIsLoading(e.payload.loading);
      }
    );
    return () => {
      unlistenNav.then((fn) => fn());
      unlistenLoading.then((fn) => fn());
    };
  }, [setBrowserUrl, setIsLoading, clearDetectedStreams]);

  // Listen for stream detection events
  const handleStreamDetected = useCallback(
    async (manifestUrl: string, streamType: "hls" | "dash", pageUrl: string, pageTitle: string) => {
      try {
        const result = await validateStream(
          manifestUrl,
          pageUrl,
          streamType,
          browserSettings.minDuration,
          browserSettings.minFileSize,
          pageTitle
        );
        if (result) {
          const stream: DetectedStream = {
            id: `stream-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
            url: manifestUrl,
            type: streamType,
            pageUrl,
            pageTitle: pageTitle || result.title || pageUrl.replace(/^https?:\/\//, "").split("/")[0],
            estimatedDuration: result.duration,
            estimatedSize: result.size,
            qualities: result.qualities || [],
            detectedAt: Date.now(),
          };
          addDetectedStream(stream);
        }
      } catch (e) {
        console.warn("Stream validation failed:", e);
      }
    },
    [addDetectedStream, browserSettings]
  );

  useEffect(() => {
    const unlisten = listen<{
      manifest_url: string;
      stream_type: string;
      page_url: string;
      page_title: string;
    }>("stream-detected-raw", (e) => {
      handleStreamDetected(
        e.payload.manifest_url,
        e.payload.stream_type as "hls" | "dash",
        e.payload.page_url,
        e.payload.page_title || ""
      );
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [handleStreamDetected]);

  // Callback for when address bar gets focus — hide webview so it's not blocking
  const handleBarFocus = useCallback(() => {
    resizeBrowserWebview(0, -9999, 1, 1).catch(() => {});
  }, []);

  const handleBarBlur = useCallback(() => {
    const el = webviewAreaRef.current;
    if (!el || settingsOpen) return;
    const rect = el.getBoundingClientRect();
    resizeBrowserWebview(
      Math.round(rect.x),
      Math.round(rect.y),
      Math.round(rect.width),
      Math.round(rect.height)
    ).catch(() => {});
  }, [settingsOpen]);

  return (
    <div
      style={{
        flex: 1,
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
      }}
    >
      <BrowserBar onFocus={handleBarFocus} onBlur={handleBarBlur} />
      <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>
        {/* Webview area - the actual webview is overlaid on top by Tauri */}
        <div
          ref={webviewAreaRef}
          style={{
            flex: 1,
            position: "relative",
            background: "#0a0614",
            overflow: "hidden",
          }}
        >
          <div
            style={{
              position: "absolute",
              inset: 0,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              color: "var(--text-dimmer)",
              fontSize: "12px",
              letterSpacing: "2px",
              pointerEvents: "none",
            }}
          >
            BROWSER WEBVIEW
          </div>
        </div>
        <DetectedVideos />
      </div>
    </div>
  );
}
