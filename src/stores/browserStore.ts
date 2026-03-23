import { create } from "zustand";
import type { DetectedStream, FavoriteItem, BrowserSettings } from "../lib/types";

interface BrowserStore {
  activeTab: "downloads" | "browser";
  setActiveTab: (tab: "downloads" | "browser") => void;

  browserUrl: string;
  setBrowserUrl: (url: string) => void;
  isLoading: boolean;
  setIsLoading: (loading: boolean) => void;

  favorites: FavoriteItem[];
  setFavorites: (favs: FavoriteItem[]) => void;
  addFavorite: (fav: FavoriteItem) => void;
  removeFavorite: (id: string) => void;

  detectedStreams: DetectedStream[];
  addDetectedStream: (stream: DetectedStream) => void;
  clearDetectedStreams: () => void;
  removeDetectedStream: (id: string) => void;

  browserSettings: BrowserSettings;
  setBrowserSettings: (settings: BrowserSettings) => void;
}

export const useBrowserStore = create<BrowserStore>((set) => ({
  activeTab: "downloads",
  setActiveTab: (tab) => set({ activeTab: tab }),

  browserUrl: "https://www.google.com",
  setBrowserUrl: (url) => set({ browserUrl: url }),
  isLoading: false,
  setIsLoading: (loading) => set({ isLoading: loading }),

  favorites: [],
  setFavorites: (favs) => set({ favorites: favs }),
  addFavorite: (fav) => set((s) => ({ favorites: [fav, ...s.favorites] })),
  removeFavorite: (id) => set((s) => ({ favorites: s.favorites.filter((f) => f.id !== id) })),

  detectedStreams: [],
  addDetectedStream: (stream) =>
    set((s) => {
      if (s.detectedStreams.some((d) => d.url === stream.url)) return s;
      return { detectedStreams: [stream, ...s.detectedStreams] };
    }),
  clearDetectedStreams: () => set({ detectedStreams: [] }),
  removeDetectedStream: (id) =>
    set((s) => ({ detectedStreams: s.detectedStreams.filter((d) => d.id !== id) })),

  browserSettings: { minDuration: 40, minFileSize: 2097152 },
  setBrowserSettings: (settings) => set({ browserSettings: settings }),
}));
