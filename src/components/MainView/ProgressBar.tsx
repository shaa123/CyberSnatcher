export default function ProgressBar({ progress, large = false }: { progress: number; large?: boolean }) {
  const h = large ? "h-2.5" : "h-1.5";
  return (
    <div className={`w-full bg-cyber-bg rounded-full overflow-hidden ${h}`}>
      <div
        className={`${h} progress-gradient rounded-full transition-all duration-500 ease-out relative`}
        style={{ width: `${Math.min(100, Math.max(0, progress))}%` }}
      >
        {progress > 0 && progress < 100 && (
          <div className="absolute inset-0 bg-gradient-to-r from-transparent via-white/10 to-transparent animate-shimmer bg-[length:200%_100%]" />
        )}
      </div>
    </div>
  );
}
