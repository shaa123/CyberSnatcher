import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useDownloadStore } from "../stores/downloadStore";
import type { DownloadProgress } from "../lib/types";

export function useDownloadEvents() {
  const updateItem = useDownloadStore((s) => s.updateItem);

  useEffect(() => {
    const unlisten = listen<DownloadProgress>("download-progress", (event) => {
      const p = event.payload;
      // Browser tab handles its own download progress
      if (p.job_id?.startsWith("browser-")) return;

      if (p.status === "complete") {
        updateItem(p.job_id, { status: "complete", progress: 100, speed: "—", eta: "—" });
      } else if (p.status === "error") {
        updateItem(p.job_id, { status: "error" });
      } else if (p.status === "cancelled") {
        updateItem(p.job_id, { status: "cancelled" });
      } else if (p.percent >= 0) {
        updateItem(p.job_id, {
          progress: p.percent,
          speed: p.speed || "—",
          eta: p.eta || "—",
          status: "downloading",
        });
      }
    });

    return () => { unlisten.then((fn) => fn()); };
  }, [updateItem]);
}

export function useDownloads() {
  const items = useDownloadStore((s) => s.items);
  const selectedId = useDownloadStore((s) => s.selectedId);
  const selectItem = useDownloadStore((s) => s.selectItem);

  const selectedItem = items.find((i) => i.id === selectedId) ?? null;

  const activeCount = items.filter((i) => i.status === "downloading" || i.status === "analyzing").length;
  const completedCount = items.filter((i) => i.status === "complete").length;

  return { items, selectedItem, selectedId, selectItem, activeCount, completedCount };
}
