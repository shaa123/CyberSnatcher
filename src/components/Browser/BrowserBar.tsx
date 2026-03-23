import { useState, useCallback, useEffect, useRef } from "react";
import { useBrowserStore } from "../../stores/browserStore";
import {
  navigateBrowser,
  browserGoBack,
  browserGoForward,
  browserReload,
  addFavorite,
  removeFavorite,
  listFavorites,
  isFavorite as isFavoriteTauri,
} from "../../lib/tauri";
import FavoritesDropdown from "./FavoritesDropdown";

interface BrowserBarProps {
  onFocus?: () => void;
  onBlur?: () => void;
}

export default function BrowserBar({ onFocus, onBlur }: BrowserBarProps) {
  const browserUrl = useBrowserStore((s) => s.browserUrl);
  const setBrowserUrl = useBrowserStore((s) => s.setBrowserUrl);
  const isLoading = useBrowserStore((s) => s.isLoading);
  const favorites = useBrowserStore((s) => s.favorites);
  const setFavorites = useBrowserStore((s) => s.setFavorites);
  const addFav = useBrowserStore((s) => s.addFavorite);
  const removeFav = useBrowserStore((s) => s.removeFavorite);

  const [urlInput, setUrlInput] = useState(browserUrl);
  const [isFav, setIsFav] = useState(false);
  const [showFavorites, setShowFavorites] = useState(false);
  const favRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setUrlInput(browserUrl);
    isFavoriteTauri(browserUrl).then(setIsFav).catch(() => setIsFav(false));
  }, [browserUrl]);

  useEffect(() => {
    listFavorites().then(setFavorites).catch(() => {});
  }, [setFavorites]);

  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (favRef.current && !favRef.current.contains(e.target as Node)) {
        setShowFavorites(false);
      }
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, []);

  const inputRef = useRef<HTMLInputElement>(null);

  const handleNavigate = useCallback(() => {
    let target = urlInput.trim();
    if (!target) return;
    if (!target.startsWith("http://") && !target.startsWith("https://")) {
      if (target.includes(".") && !target.includes(" ")) {
        target = "https://" + target;
      } else {
        target = `https://www.google.com/search?q=${encodeURIComponent(target)}`;
      }
    }
    setBrowserUrl(target);
    navigateBrowser(target).catch(() => {});
    // Blur the input so the webview becomes visible again
    inputRef.current?.blur();
  }, [urlInput, setBrowserUrl]);

  const toggleFavorite = useCallback(async () => {
    if (isFav) {
      const fav = favorites.find((f) => f.url === browserUrl);
      if (fav) {
        await removeFavorite(fav.id).catch(() => {});
        removeFav(fav.id);
      }
      setIsFav(false);
    } else {
      const id = `fav-${Date.now()}`;
      const title = browserUrl.replace(/^https?:\/\//, "").split("/")[0];
      await addFavorite(id, browserUrl, title).catch(() => {});
      addFav({ id, url: browserUrl, title, createdAt: Date.now() });
      setIsFav(true);
    }
  }, [isFav, browserUrl, favorites, removeFav, addFav]);

  const handleFavNavigate = useCallback(
    (url: string) => {
      setBrowserUrl(url);
      setUrlInput(url);
      navigateBrowser(url).catch(() => {});
      setShowFavorites(false);
    },
    [setBrowserUrl]
  );

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: "6px",
        padding: "6px 10px",
        background: "var(--panel)",
        borderBottom: "1px solid var(--border-purple)",
        flexShrink: 0,
      }}
    >
      {/* Nav buttons */}
      <button
        onClick={() => browserGoBack().catch(() => {})}
        title="Back"
        style={{
          background: "transparent",
          border: "none",
          color: "var(--text-dim)",
          cursor: "pointer",
          padding: "4px 6px",
          fontSize: "14px",
        }}
      >
        ◀
      </button>
      <button
        onClick={() => browserGoForward().catch(() => {})}
        title="Forward"
        style={{
          background: "transparent",
          border: "none",
          color: "var(--text-dim)",
          cursor: "pointer",
          padding: "4px 6px",
          fontSize: "14px",
        }}
      >
        ▶
      </button>
      <button
        onClick={() => browserReload().catch(() => {})}
        title="Reload"
        style={{
          background: "transparent",
          border: "none",
          color: "var(--text-dim)",
          cursor: "pointer",
          padding: "4px 6px",
          fontSize: "14px",
        }}
      >
        ↻
      </button>

      {/* URL input */}
      <div style={{ flex: 1, position: "relative" }}>
        {isLoading && (
          <div
            style={{
              position: "absolute",
              bottom: 0,
              left: 0,
              height: "2px",
              background: "var(--cyan)",
              animation: "loading-bar 1.5s ease-in-out infinite",
              width: "60%",
            }}
          />
        )}
        <input
          ref={inputRef}
          value={urlInput}
          onChange={(e) => setUrlInput(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleNavigate()}
          onFocus={onFocus}
          onBlur={onBlur}
          placeholder="Search or enter URL..."
          style={{
            width: "100%",
            background: "var(--input-bg)",
            border: "1px solid var(--border-purple)",
            borderRadius: "3px",
            padding: "7px 12px",
            color: "var(--text)",
            fontFamily: "'Share Tech Mono', monospace",
            fontSize: "12px",
          }}
        />
      </div>

      {/* Go button */}
      <button
        onClick={handleNavigate}
        style={{
          background: "linear-gradient(135deg, #b400ff22, #7700cc22)",
          border: "1px solid #b400ff66",
          borderRadius: "3px",
          color: "#e040fb",
          fontFamily: "'Orbitron', sans-serif",
          fontSize: "10px",
          fontWeight: 700,
          letterSpacing: "1px",
          padding: "7px 14px",
          cursor: "pointer",
        }}
      >
        GO
      </button>

      {/* Favorite star + dropdown */}
      <div ref={favRef} style={{ position: "relative" }}>
        <button
          onClick={toggleFavorite}
          title={isFav ? "Remove from favorites" : "Add to favorites"}
          style={{
            background: "transparent",
            border: "none",
            cursor: "pointer",
            padding: "4px 6px",
            fontSize: "16px",
            color: isFav ? "#fbbf24" : "var(--text-dim)",
            transition: "color 0.2s",
          }}
        >
          {isFav ? "★" : "☆"}
        </button>
        <button
          onClick={() => setShowFavorites(!showFavorites)}
          title="Show favorites"
          style={{
            background: "transparent",
            border: "none",
            cursor: "pointer",
            padding: "4px 4px",
            fontSize: "10px",
            color: "var(--text-dim)",
          }}
        >
          ▼
        </button>
        {showFavorites && (
          <FavoritesDropdown
            favorites={favorites}
            onNavigate={handleFavNavigate}
            onRemove={(id) => {
              removeFavorite(id).catch(() => {});
              removeFav(id);
            }}
          />
        )}
      </div>
    </div>
  );
}
