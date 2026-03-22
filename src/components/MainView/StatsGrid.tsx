interface StatsGridProps {
  speed: string;
  downloaded: string;
  eta: string;
  fileSize: string;
  format: string;
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-cyber-bg border border-cyber-border rounded-lg p-3">
      <p className="text-[10px] uppercase tracking-widest text-cyber-text-tertiary mb-1">
        {label}
      </p>
      <p className="text-sm font-semibold text-cyber-text-primary font-mono">
        {value}
      </p>
    </div>
  );
}

export default function StatsGrid({
  speed,
  downloaded,
  eta,
  fileSize,
  format,
}: StatsGridProps) {
  return (
    <div className="grid grid-cols-3 gap-2">
      <StatCard label="Speed" value={speed} />
      <StatCard label="Downloaded" value={downloaded} />
      <StatCard label="ETA" value={eta} />
      <StatCard label="File Size" value={fileSize} />
      <StatCard label="Format" value={format} />
      <StatCard label="Status" value="Active" />
    </div>
  );
}
