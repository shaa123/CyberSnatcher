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

export default function BrowserView() {
  const setBrowserUrl = useBrowserStore((s) => s.setBrowserUrl);
  const setIsLoading = useBrowserStore((s) => s.setIsLoading);
  const addDetectedStream = useBrowserStore((s) => s.addDetectedStream);
  const browserSettings = useBrowserStore((s) => s.browserSettings);
  const browserUrl = useBrowserStore((s) => s.browserUrl);
  const webviewAreaRef = useRef<HTMLDivElement>(null);

  // Create webview on mount
  useEffect(() => {
    const timer = setTimeout(() => {
      createBrowserWebview(browserUrl).catch((e) =>
        console.error("Failed to create browser webview:", e)
      );
    }, 100);
    return () => {
      clearTimeout(timer);
      destroyBrowserWebview().catch(() => {});
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Resize webview to fit container
  useEffect(() => {
    const el = webviewAreaRef.current;
    if (!el) return;

    const resize = () => {
      const rect = el.getBoundingClientRect();
      resizeBrowserWebview(
        Math.round(rect.x),
        Math.round(rect.y),
        Math.round(rect.width),
        Math.round(rect.height)
      ).catch(() => {});
    };

    resize();
    const observer = new ResizeObserver(resize);
    observer.observe(el);
    window.addEventListener("resize", resize);

    return () => {
      observer.disconnect();
      window.removeEventListener("resize", resize);
    };
  }, []);

  // Listen for browser events from Rust
  useEffect(() => {
    const unlistenNav = listen<{ url: string }>("browser-navigated", (e) => {
      setBrowserUrl(e.payload.url);
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
  }, [setBrowserUrl, setIsLoading]);

  // Listen for stream detection events
  const handleStreamDetected = useCallback(
    async (manifestUrl: string, streamType: "hls" | "dash", pageUrl: string) => {
      try {
        const result = await validateStream(
          manifestUrl,
          pageUrl,
          streamType,
          browserSettings.minDuration,
          browserSettings.minFileSize
        );
        if (result) {
          const stream: DetectedStream = {
            id: `stream-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
            url: manifestUrl,
            type: streamType,
            pageUrl,
            pageTitle: result.title || pageUrl.replace(/^https?:\/\//, "").split("/")[0],
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
    }>("stream-detected-raw", (e) => {
      handleStreamDetected(
        e.payload.manifest_url,
        e.payload.stream_type as "hls" | "dash",
        e.payload.page_url
      );
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [handleStreamDetected]);

  return (
    <div
      style={{
        flex: 1,
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
      }}
    >
      <BrowserBar />
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
