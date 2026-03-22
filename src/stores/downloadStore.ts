import { create } from "zustand";
import type { DownloadItem } from "../lib/types";

interface DownloadStore {
  items: DownloadItem[];
  selectedId: string | null;
  downloadFolder: string;
  customFolders: string[];
  selectItem: (id: string | null) => void;
  setDownloadFolder: (path: string) => void;
  setCustomFolders: (folders: string[]) => void;
  addItem: (item: DownloadItem) => void;
  updateItem: (id: string, updates: Partial<DownloadItem>) => void;
  appendLog: (id: string, line: string) => void;
  removeItem: (id: string) => void;
}

export const useDownloadStore = create<DownloadStore>((set) => ({
  items: [],
  selectedId: null,
  downloadFolder: "",
  customFolders: [],
  selectItem: (id) => set({ selectedId: id }),
  setDownloadFolder: (path) => set({ downloadFolder: path }),
  setCustomFolders: (folders) => set({ customFolders: folders }),
  addItem: (item) =>
    set((s) => ({ items: [item, ...s.items], selectedId: item.id })),
  updateItem: (id, updates) =>
    set((s) => ({
      items: s.items.map((i) => (i.id === id ? { ...i, ...updates } : i)),
    })),
  appendLog: (id, line) =>
    set((s) => ({
      items: s.items.map((i) =>
        i.id === id ? { ...i, logs: [...i.logs, line] } : i
      ),
    })),
  removeItem: (id) =>
    set((s) => ({
      items: s.items.filter((i) => i.id !== id),
      selectedId: s.selectedId === id ? null : s.selectedId,
    })),
}));
