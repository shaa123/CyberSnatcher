import { useEffect, useRef } from "react";

interface LogsTabProps {
  logs: string[];
}

export default function LogsTab({ logs }: LogsTabProps) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <p className="text-[15px] font-semibold text-cyber-text-secondary">Output Log</p>
        <span className="text-[13px] text-cyber-text-tertiary">{logs.length} entries</span>
      </div>

      <div className="bg-cyber-bg border border-cyber-border rounded-lg p-3 max-h-[400px] overflow-y-auto font-mono text-[13px]">
        {logs.length === 0 ? (
          <p className="text-cyber-text-tertiary">No logs yet. Paste a URL and start a download to see output here.</p>
        ) : (
          logs.map((line, i) => (
            <div
              key={i}
              className={`py-0.5 ${
                line.includes("ERR") || line.includes("error")
                  ? "text-cyber-error"
                  : line.includes("COMPLETE") || line.includes("complete")
                  ? "text-cyber-success"
                  : "text-cyber-text-tertiary"
              }`}
            >
              {line}
            </div>
          ))
        )}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}
