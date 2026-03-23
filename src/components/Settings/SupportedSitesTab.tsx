import { useState, useMemo, useRef, useEffect } from "react";
import { SUPPORTED_SITES } from "../../data/supportedSites";

const TOTAL_SITES = SUPPORTED_SITES.reduce((sum, g) => sum + 1 + g.variants.length, 0);

export default function SupportedSitesTab() {
  const [search, setSearch] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const filtered = useMemo(() => {
    if (!search.trim()) return SUPPORTED_SITES;
    const q = search.toLowerCase();
    return SUPPORTED_SITES.filter(
      (g) =>
        g.name.toLowerCase().includes(q) ||
        g.variants.some((v) => v.toLowerCase().includes(q))
    );
  }, [search]);

  const filteredCount = filtered.reduce((sum, g) => sum + 1 + g.variants.length, 0);

  return (
    <div className="flex flex-col h-full">
      {/* Search */}
      <div className="relative mb-3">
        <input
          ref={inputRef}
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder={`Search ${TOTAL_SITES.toLocaleString()}+ supported sites...`}
          className="w-full bg-cyber-bg border border-cyber-border rounded-lg px-3 py-2 text-[17px] text-cyber-text-primary pr-8"
        />
        {search && (
          <button
            onClick={() => setSearch("")}
            className="absolute right-2 top-1/2 -translate-y-1/2 text-cyber-text-tertiary hover:text-cyber-text-primary transition-colors"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
              <path d="M18 6L6 18M6 6l12 12" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
            </svg>
          </button>
        )}
      </div>

      {/* Count */}
      <p className="text-[13px] text-cyber-text-tertiary mb-2">
        {search
          ? `Showing ${filtered.length} of ${SUPPORTED_SITES.length} sites (${filteredCount} extractors)`
          : `${SUPPORTED_SITES.length} sites · ${TOTAL_SITES.toLocaleString()} extractors`}
      </p>

      {/* List */}
      <div className="flex-1 overflow-y-auto -mr-2 pr-2">
        {filtered.length === 0 ? (
          <div className="flex items-center justify-center py-12 text-cyber-text-tertiary text-[15px]">
            No sites match "{search}"
          </div>
        ) : (
          <div className="space-y-0.5">
            {filtered.map((group) => (
              <div
                key={group.name}
                className="flex items-baseline gap-2 px-2 py-1 rounded hover:bg-cyber-primary/5 transition-colors"
              >
                <span className="text-[15px] text-cyber-text-primary font-medium shrink-0">
                  {group.name}
                </span>
                {group.variants.length > 0 && (
                  <span className="text-[13px] text-cyber-text-tertiary truncate">
                    :{group.variants.join(", :")}
                  </span>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
