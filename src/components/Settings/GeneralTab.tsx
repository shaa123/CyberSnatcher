interface GeneralTabProps {
  ytdlpInstalled: boolean;
  folderInput: string;
  setFolderInput: (v: string) => void;
  onSaveFolder: () => void;
}

export default function GeneralTab({ ytdlpInstalled, folderInput, setFolderInput, onSaveFolder }: GeneralTabProps) {
  return (
    <div className="space-y-4">
      {/* yt-dlp status */}
      <div className="flex items-center gap-2 p-3 rounded-lg bg-cyber-bg border border-cyber-border">
        <div className={`w-2 h-2 rounded-full ${ytdlpInstalled ? "bg-cyber-success" : "bg-cyber-error"}`} />
        <span className="text-[17px] text-cyber-text-secondary">
          yt-dlp: {ytdlpInstalled ? "Installed" : "Not found — install from https://github.com/yt-dlp/yt-dlp"}
        </span>
      </div>

      {/* Download folder */}
      <div>
        <label className="block text-[17px] font-semibold text-cyber-text-secondary mb-1.5">
          Default Download Folder
        </label>
        <div className="flex gap-2">
          <input
            type="text"
            value={folderInput}
            onChange={(e) => setFolderInput(e.target.value)}
            className="flex-1 bg-cyber-bg border border-cyber-border rounded-lg px-3 py-2 text-[17px] text-cyber-text-primary"
          />
          <button
            onClick={onSaveFolder}
            className="px-3 py-2 bg-cyber-primary/10 border border-cyber-primary/30 rounded-lg text-[17px] text-cyber-primary hover:bg-cyber-primary/20 transition-all"
          >
            Set
          </button>
        </div>
        <p className="text-[17px] text-cyber-text-tertiary mt-1">
          Tip: You can also click folders in the sidebar to set as download target
        </p>
      </div>

      {/* Info */}
      <div className="p-3 rounded-lg bg-cyber-bg border border-cyber-border space-y-1">
        <p className="text-[17px] text-cyber-text-secondary font-semibold">How it works</p>
        <p className="text-[17px] text-cyber-text-tertiary leading-relaxed">
          CyberSnatcher uses yt-dlp under the hood to download videos. Make sure yt-dlp is installed and in your PATH.
          Paste any URL from YouTube, Twitter/X, TikTok, Instagram, Reddit, or direct video links.
        </p>
      </div>
    </div>
  );
}
