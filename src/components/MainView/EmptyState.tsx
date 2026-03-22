export default function EmptyState() {
  return (
    <div className="flex-1 flex flex-col items-center justify-center gap-4">
      {/* Big download icon */}
      <div className="relative">
        <div className="absolute inset-0 bg-cyber-primary/10 rounded-full blur-3xl scale-150" />
        <svg
          width="80"
          height="80"
          viewBox="0 0 24 24"
          fill="none"
          className="relative"
        >
          <path
            d="M12 3v12m0 0l-4-4m4 4l4-4"
            stroke="#8b5cf6"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
          <path
            d="M5 17v2a2 2 0 002 2h10a2 2 0 002-2v-2"
            stroke="#c084fc"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </div>

      <div className="text-center space-y-1.5">
        <h2 className="text-lg font-semibold text-cyber-text-primary">
          Paste a URL to start snatching
        </h2>
        <p className="text-sm text-cyber-text-tertiary max-w-xs">
          Drop a YouTube link, HLS stream, or any video URL into the sidebar
        </p>
      </div>

      {/* Keyboard hint */}
      <div className="flex items-center gap-1.5 mt-2">
        <kbd className="px-1.5 py-0.5 text-[10px] font-mono bg-cyber-card border border-cyber-border rounded text-cyber-text-tertiary">
          Ctrl+V
        </kbd>
        <span className="text-[10px] text-cyber-text-tertiary">
          in the URL field
        </span>
      </div>
    </div>
  );
}
