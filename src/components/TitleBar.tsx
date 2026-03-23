import { getCurrentWindow } from "@tauri-apps/api/window";
import { useBrowserStore } from "../stores/browserStore";

export default function TitleBar() {
  const win = getCurrentWindow();
  const activeTab = useBrowserStore((s) => s.activeTab);
  const setActiveTab = useBrowserStore((s) => s.setActiveTab);

  const tabStyle = (tab: "downloads" | "browser") => ({
    background: activeTab === tab ? "#b400ff18" : "transparent",
    border: "none",
    borderBottom: activeTab === tab ? "2px solid #b400ff" : "2px solid transparent",
    color: activeTab === tab ? "#e040fb" : "var(--text-dim)",
    fontFamily: "'Orbitron', sans-serif" as const,
    fontSize: "9px",
    fontWeight: 700 as const,
    letterSpacing: "2px",
    padding: "0 16px",
    height: "100%",
    cursor: "pointer",
    transition: "all 0.2s",
  });

  return (
    <div data-tauri-drag-region className="flex items-center justify-between h-8 select-none shrink-0" style={{ background: "var(--bg)" }}>
      <div data-tauri-drag-region className="pl-3 flex items-center gap-2 h-full">
        <svg width="14" height="14" viewBox="0 0 32 32" fill="none">
          <polygon points="16,2 30,10 30,22 16,30 2,22 2,10" fill="none" stroke="#b400ff" strokeWidth="1.5" />
          <circle cx="16" cy="16" r="3" fill="#00f5ff" />
        </svg>
        <span data-tauri-drag-region style={{ fontFamily: "'Orbitron', sans-serif", fontSize: "10px", fontWeight: 700, letterSpacing: "2px", color: "#b400ff" }}>
          CYBERSNATCHER
        </span>
        <div style={{ width: "1px", height: "14px", background: "var(--border-purple)", margin: "0 8px" }} />
        <button onClick={() => setActiveTab("downloads")} style={tabStyle("downloads")}>
          DOWNLOADS
        </button>
        <button onClick={() => setActiveTab("browser")} style={tabStyle("browser")}>
          BROWSER
        </button>
      </div>
      <div className="flex items-center h-full">
        <button onClick={() => win.minimize()} className="flex items-center justify-center w-10 h-full transition-colors" style={{ color: "var(--text-dim)" }} onMouseEnter={e => e.currentTarget.style.background = "#ffffff08"} onMouseLeave={e => e.currentTarget.style.background = "transparent"}>
          <svg width="10" height="10" viewBox="0 0 12 12"><rect x="1" y="5.5" width="10" height="1" fill="currentColor" /></svg>
        </button>
        <button onClick={() => win.toggleMaximize()} className="flex items-center justify-center w-10 h-full transition-colors" style={{ color: "var(--text-dim)" }} onMouseEnter={e => e.currentTarget.style.background = "#ffffff08"} onMouseLeave={e => e.currentTarget.style.background = "transparent"}>
          <svg width="10" height="10" viewBox="0 0 12 12"><rect x="1.5" y="1.5" width="9" height="9" rx="1" fill="none" stroke="currentColor" strokeWidth="1" /></svg>
        </button>
        <button onClick={() => win.close()} className="flex items-center justify-center w-10 h-full transition-colors group" onMouseEnter={e => e.currentTarget.style.background = "#ff003c66"} onMouseLeave={e => e.currentTarget.style.background = "transparent"}>
          <svg width="10" height="10" viewBox="0 0 12 12"><path d="M1 1l10 10M11 1L1 11" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" style={{ color: "var(--text-dim)" }} /></svg>
        </button>
      </div>
    </div>
  );
}
