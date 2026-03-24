import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { startScrape, previewScrape, exportScrapeData, pickFolder } from "../lib/tauri";
import type { SpiderConfig, FieldRule, CrawlRule, ScrapeProgress, ScrapeResult, ScrapedItem } from "../lib/types";

const DEFAULT_FIELD_RULE: FieldRule = { name: "", css_selector: "", extract: "text", regex_filter: null };
const DEFAULT_CRAWL_RULE: CrawlRule = { link_selector: "a", url_pattern: null };

export default function ScraperTab() {
  // ── Spider config state ──
  const [name, setName] = useState("my-spider");
  const [startUrls, setStartUrls] = useState("");
  const [fieldRules, setFieldRules] = useState<FieldRule[]>([{ ...DEFAULT_FIELD_RULE }]);
  const [crawlRules, setCrawlRules] = useState<CrawlRule[]>([]);
  const [maxPages, setMaxPages] = useState(50);
  const [concurrency, setConcurrency] = useState(4);
  const [requestDelay, setRequestDelay] = useState(1000);
  const [respectRobots, setRespectRobots] = useState(true);
  const [userAgent, setUserAgent] = useState("");
  const [itemSelector, setItemSelector] = useState("");

  // ── Job state ──
  const [phase, setPhase] = useState<"idle" | "running" | "preview" | "done" | "error">("idle");
  const [progress, setProgress] = useState<ScrapeProgress | null>(null);
  const [result, setResult] = useState<ScrapeResult | null>(null);
  const [previewItems, setPreviewItems] = useState<ScrapedItem[] | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [showLogs, setShowLogs] = useState(false);
  const [exportFormat, setExportFormat] = useState<"Json" | "Csv" | "JsonLines">("Json");

  const addLog = useCallback((msg: string) => {
    setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] ${msg}`]);
  }, []);

  // Listen for scrape progress events.
  useEffect(() => {
    const unlisten = listen<ScrapeProgress>("scrape-progress", (event) => {
      const p = event.payload;
      setProgress(p);
      if (p.log_line) addLog(p.log_line);
      if (p.status === "complete" || p.status === "finished") {
        setPhase("done");
      }
      if (p.status === "error" && p.pages_crawled === 0) {
        setPhase("error");
      }
    });
    return () => { unlisten.then((f) => f()); };
  }, [addLog]);

  // Build spider config from form state.
  const buildConfig = (): SpiderConfig => ({
    name,
    start_urls: startUrls.split("\n").map((s) => s.trim()).filter(Boolean),
    field_rules: fieldRules.filter((r) => r.name && r.css_selector),
    crawl_rules: crawlRules.filter((r) => r.link_selector),
    max_pages: maxPages,
    concurrency,
    request_delay_ms: requestDelay,
    respect_robots: respectRobots,
    user_agent: userAgent || null,
    headers: {},
  });

  const handleStartScrape = async () => {
    const config = buildConfig();
    if (config.start_urls.length === 0) return addLog("Error: No start URLs");
    if (config.field_rules.length === 0) return addLog("Error: No field rules");
    setPhase("running");
    setLogs([]);
    setResult(null);
    setPreviewItems(null);
    addLog(`Starting spider "${config.name}" with ${config.start_urls.length} URL(s)...`);
    try {
      const jobId = crypto.randomUUID();
      const res = await startScrape(jobId, config);
      setResult(res);
      setPhase("done");
      addLog(`Complete! ${res.items.length} items scraped from ${res.pages_crawled} pages.`);
    } catch (e: any) {
      setPhase("error");
      addLog(`Error: ${e}`);
    }
  };

  const handlePreview = async () => {
    const urls = startUrls.split("\n").map((s) => s.trim()).filter(Boolean);
    const rules = fieldRules.filter((r) => r.name && r.css_selector);
    if (urls.length === 0 || rules.length === 0) return addLog("Need URL and at least one field rule to preview");
    setPhase("preview");
    setPreviewItems(null);
    addLog(`Previewing: ${urls[0]}`);
    try {
      const items = await previewScrape(urls[0], rules, itemSelector || undefined);
      setPreviewItems(items);
      setPhase("idle");
      addLog(`Preview: ${items.length} item(s) found`);
    } catch (e: any) {
      setPhase("idle");
      addLog(`Preview error: ${e}`);
    }
  };

  const handleExport = async () => {
    if (!result || result.items.length === 0) return;
    const folder = await pickFolder();
    if (!folder) return;
    const ext = exportFormat === "Json" ? "json" : exportFormat === "Csv" ? "csv" : "jsonl";
    const path = `${folder}/${name}_${Date.now()}.${ext}`;
    try {
      const exportPath = await exportScrapeData(result.items, path, exportFormat);
      addLog(`Exported to: ${exportPath}`);
    } catch (e: any) {
      addLog(`Export error: ${e}`);
    }
  };

  // ── Field rule helpers ──
  const updateFieldRule = (idx: number, patch: Partial<FieldRule>) => {
    setFieldRules((prev) => prev.map((r, i) => (i === idx ? { ...r, ...patch } : r)));
  };
  const addFieldRule = () => setFieldRules((prev) => [...prev, { ...DEFAULT_FIELD_RULE }]);
  const removeFieldRule = (idx: number) => setFieldRules((prev) => prev.filter((_, i) => i !== idx));

  // ── Crawl rule helpers ──
  const updateCrawlRule = (idx: number, patch: Partial<CrawlRule>) => {
    setCrawlRules((prev) => prev.map((r, i) => (i === idx ? { ...r, ...patch } : r)));
  };
  const addCrawlRule = () => setCrawlRules((prev) => [...prev, { ...DEFAULT_CRAWL_RULE }]);
  const removeCrawlRule = (idx: number) => setCrawlRules((prev) => prev.filter((_, i) => i !== idx));

  const isRunning = phase === "running" || phase === "preview";

  // ── Render ──
  return (
    <div style={{ width: "100%", maxWidth: "800px", margin: "0 auto" }}>
      {/* Header */}
      <div className="anim-float-in" style={{ textAlign: "center", marginBottom: "24px" }}>
        <div style={{ display: "inline-flex", alignItems: "center", gap: "12px", marginBottom: "6px" }}>
          <svg width="28" height="28" viewBox="0 0 32 32" fill="none">
            <circle cx="16" cy="16" r="13" fill="none" stroke="#39ff14" strokeWidth="1.5" />
            <path d="M10 16h12M16 10v12M12 12l8 8M20 12l-8 8" stroke="#39ff14" strokeWidth="1" opacity="0.5" />
            <circle cx="16" cy="16" r="4" fill="#39ff1433" stroke="#39ff14" strokeWidth="1" />
          </svg>
          <h1 style={{ fontSize: "26px", fontWeight: 800, color: "#39ff14", margin: 0, letterSpacing: "2px" }}>
            CYBER SCRAPER
          </h1>
        </div>
        <div style={{ color: "var(--text-dim)", fontSize: "12px", letterSpacing: "1px" }}>
          SCRAPY-STYLE WEB EXTRACTION ENGINE
        </div>
      </div>

      {/* ── Spider Name ── */}
      <Section title="SPIDER NAME">
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="my-spider"
          style={inputStyle}
          disabled={isRunning}
        />
      </Section>

      {/* ── Start URLs ── */}
      <Section title="START URLs" subtitle="One per line">
        <textarea
          value={startUrls}
          onChange={(e) => setStartUrls(e.target.value)}
          placeholder={"https://example.com/page1\nhttps://example.com/page2"}
          rows={3}
          style={{ ...inputStyle, resize: "vertical", fontFamily: "monospace", fontSize: "12px" }}
          disabled={isRunning}
        />
      </Section>

      {/* ── Field Extraction Rules ── */}
      <Section title="FIELD RULES" subtitle="Define what data to extract">
        {fieldRules.map((rule, idx) => (
          <div key={idx} style={{ display: "flex", gap: "8px", marginBottom: "8px", flexWrap: "wrap" }}>
            <input
              value={rule.name}
              onChange={(e) => updateFieldRule(idx, { name: e.target.value })}
              placeholder="field name"
              style={{ ...inputStyle, flex: "1 1 120px" }}
              disabled={isRunning}
            />
            <input
              value={rule.css_selector}
              onChange={(e) => updateFieldRule(idx, { css_selector: e.target.value })}
              placeholder="CSS selector"
              style={{ ...inputStyle, flex: "2 1 200px", fontFamily: "monospace", fontSize: "12px" }}
              disabled={isRunning}
            />
            <select
              value={rule.extract}
              onChange={(e) => updateFieldRule(idx, { extract: e.target.value })}
              style={{ ...inputStyle, flex: "0 0 100px" }}
              disabled={isRunning}
            >
              <option value="text">text</option>
              <option value="html">html</option>
              <option value="href">href</option>
              <option value="src">src</option>
              <option value="alt">alt</option>
              <option value="title">title</option>
            </select>
            <input
              value={rule.regex_filter || ""}
              onChange={(e) => updateFieldRule(idx, { regex_filter: e.target.value || null })}
              placeholder="regex (optional)"
              style={{ ...inputStyle, flex: "1 1 140px", fontFamily: "monospace", fontSize: "12px" }}
              disabled={isRunning}
            />
            {fieldRules.length > 1 && (
              <button onClick={() => removeFieldRule(idx)} style={removeBtn} disabled={isRunning}>
                ✕
              </button>
            )}
          </div>
        ))}
        <button onClick={addFieldRule} style={addBtn} disabled={isRunning}>
          + Add Field
        </button>
      </Section>

      {/* ── Item Selector (optional) ── */}
      <Section title="ITEM SELECTOR" subtitle="CSS selector for repeating items (optional)">
        <input
          value={itemSelector}
          onChange={(e) => setItemSelector(e.target.value)}
          placeholder=".product-card, .result-item, tr.data-row"
          style={{ ...inputStyle, fontFamily: "monospace", fontSize: "12px" }}
          disabled={isRunning}
        />
      </Section>

      {/* ── Crawl Rules (pagination/link following) ── */}
      <Section title="CRAWL RULES" subtitle="Follow links for multi-page scraping (optional)">
        {crawlRules.map((rule, idx) => (
          <div key={idx} style={{ display: "flex", gap: "8px", marginBottom: "8px" }}>
            <input
              value={rule.link_selector}
              onChange={(e) => updateCrawlRule(idx, { link_selector: e.target.value })}
              placeholder="link CSS selector (e.g. a.next-page)"
              style={{ ...inputStyle, flex: "1", fontFamily: "monospace", fontSize: "12px" }}
              disabled={isRunning}
            />
            <input
              value={rule.url_pattern || ""}
              onChange={(e) => updateCrawlRule(idx, { url_pattern: e.target.value || null })}
              placeholder="URL regex filter (optional)"
              style={{ ...inputStyle, flex: "1", fontFamily: "monospace", fontSize: "12px" }}
              disabled={isRunning}
            />
            <button onClick={() => removeCrawlRule(idx)} style={removeBtn} disabled={isRunning}>
              ✕
            </button>
          </div>
        ))}
        <button onClick={addCrawlRule} style={addBtn} disabled={isRunning}>
          + Add Crawl Rule
        </button>
      </Section>

      {/* ── Settings Row ── */}
      <Section title="SETTINGS">
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: "12px" }}>
          <label style={labelStyle}>
            Max Pages
            <input type="number" value={maxPages} onChange={(e) => setMaxPages(+e.target.value)} min={0} style={inputStyle} disabled={isRunning} />
          </label>
          <label style={labelStyle}>
            Concurrency
            <input type="number" value={concurrency} onChange={(e) => setConcurrency(+e.target.value)} min={1} max={20} style={inputStyle} disabled={isRunning} />
          </label>
          <label style={labelStyle}>
            Delay (ms)
            <input type="number" value={requestDelay} onChange={(e) => setRequestDelay(+e.target.value)} min={0} step={100} style={inputStyle} disabled={isRunning} />
          </label>
        </div>
        <div style={{ display: "flex", gap: "20px", marginTop: "12px", alignItems: "center" }}>
          <label style={{ ...labelStyle, flexDirection: "row", gap: "8px", cursor: "pointer" }}>
            <input type="checkbox" checked={respectRobots} onChange={(e) => setRespectRobots(e.target.checked)} disabled={isRunning} />
            Respect robots.txt
          </label>
        </div>
        <div style={{ marginTop: "12px" }}>
          <label style={labelStyle}>
            User-Agent (optional)
            <input
              value={userAgent}
              onChange={(e) => setUserAgent(e.target.value)}
              placeholder="Custom User-Agent string"
              style={{ ...inputStyle, fontFamily: "monospace", fontSize: "12px" }}
              disabled={isRunning}
            />
          </label>
        </div>
      </Section>

      {/* ── Action Buttons ── */}
      <div style={{ display: "flex", gap: "12px", marginBottom: "24px" }}>
        <button onClick={handlePreview} disabled={isRunning} style={previewBtnStyle}>
          {phase === "preview" ? "PREVIEWING..." : "PREVIEW"}
        </button>
        <button onClick={handleStartScrape} disabled={isRunning} style={scrapeBtnStyle}>
          {phase === "running" ? "SCRAPING..." : "START SCRAPE"}
        </button>
      </div>

      {/* ── Progress Bar ── */}
      {phase === "running" && progress && (
        <div style={{ marginBottom: "20px" }}>
          <div style={{ display: "flex", justifyContent: "space-between", fontSize: "12px", color: "#39ff14", marginBottom: "6px" }}>
            <span>{progress.current_url ? `Crawling: ${progress.current_url.slice(0, 60)}...` : "Starting..."}</span>
            <span>{progress.pages_crawled}/{progress.pages_total} pages | {progress.items_scraped} items</span>
          </div>
          <div style={{ height: "6px", background: "#1a1a2e", borderRadius: "3px", overflow: "hidden" }}>
            <div
              style={{
                height: "100%",
                width: `${progress.pages_total > 0 ? (progress.pages_crawled / progress.pages_total) * 100 : 0}%`,
                background: "linear-gradient(90deg, #39ff14, #00f5ff)",
                borderRadius: "3px",
                transition: "width 0.3s ease",
              }}
            />
          </div>
        </div>
      )}

      {/* ── Preview Results ── */}
      {previewItems && previewItems.length > 0 && (
        <Section title={`PREVIEW RESULTS (${previewItems.length} items)`}>
          <div style={{ overflowX: "auto" }}>
            <table style={tableStyle}>
              <thead>
                <tr>
                  {Object.keys(previewItems[0].fields).map((key) => (
                    <th key={key} style={thStyle}>{key}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {previewItems.slice(0, 20).map((item, idx) => (
                  <tr key={idx}>
                    {Object.keys(previewItems[0].fields).map((key) => (
                      <td key={key} style={tdStyle}>{item.fields[key]?.slice(0, 120) || ""}</td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
            {previewItems.length > 20 && (
              <div style={{ color: "var(--text-dim)", fontSize: "11px", marginTop: "4px" }}>
                Showing 20 of {previewItems.length} items
              </div>
            )}
          </div>
        </Section>
      )}

      {/* ── Full Results ── */}
      {result && result.items.length > 0 && (
        <Section title={`RESULTS (${result.items.length} items from ${result.pages_crawled} pages)`}>
          <div style={{ overflowX: "auto" }}>
            <table style={tableStyle}>
              <thead>
                <tr>
                  {Object.keys(result.items[0].fields).map((key) => (
                    <th key={key} style={thStyle}>{key}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {result.items.slice(0, 50).map((item, idx) => (
                  <tr key={idx}>
                    {Object.keys(result.items[0].fields).map((key) => (
                      <td key={key} style={tdStyle}>{item.fields[key]?.slice(0, 120) || ""}</td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
            {result.items.length > 50 && (
              <div style={{ color: "var(--text-dim)", fontSize: "11px", marginTop: "4px" }}>
                Showing 50 of {result.items.length} items
              </div>
            )}
          </div>

          {/* Export controls */}
          <div style={{ display: "flex", gap: "12px", marginTop: "16px", alignItems: "center" }}>
            <select
              value={exportFormat}
              onChange={(e) => setExportFormat(e.target.value as any)}
              style={{ ...inputStyle, width: "auto" }}
            >
              <option value="Json">JSON</option>
              <option value="Csv">CSV</option>
              <option value="JsonLines">JSON Lines</option>
            </select>
            <button onClick={handleExport} style={exportBtnStyle}>
              EXPORT DATA
            </button>
          </div>

          {result.errors.length > 0 && (
            <div style={{ marginTop: "12px", color: "#ff003c", fontSize: "12px" }}>
              {result.errors.length} error(s) during crawl
            </div>
          )}
        </Section>
      )}

      {/* ── Log Panel ── */}
      <div style={{ marginBottom: "24px" }}>
        <button
          onClick={() => setShowLogs(!showLogs)}
          style={{
            background: "transparent", border: "1px solid #39ff1440",
            color: "#39ff14", cursor: "pointer", padding: "6px 14px",
            borderRadius: "4px", fontSize: "11px", letterSpacing: "1px",
          }}
        >
          {showLogs ? "HIDE" : "SHOW"} LOGS ({logs.length})
        </button>
        {showLogs && (
          <div style={{
            marginTop: "8px", background: "#0a0a14", border: "1px solid #39ff1420",
            borderRadius: "6px", padding: "12px", maxHeight: "200px",
            overflowY: "auto", fontFamily: "monospace", fontSize: "11px",
            color: "#39ff14aa", lineHeight: "1.6",
          }}>
            {logs.length === 0 ? (
              <span style={{ color: "var(--text-dim)" }}>No logs yet...</span>
            ) : (
              logs.map((line, i) => <div key={i}>{line}</div>)
            )}
          </div>
        )}
      </div>
    </div>
  );
}

// ── Reusable section wrapper ──
function Section({ title, subtitle, children }: { title: string; subtitle?: string; children: React.ReactNode }) {
  return (
    <div style={{ marginBottom: "20px" }}>
      <div style={{ marginBottom: "8px" }}>
        <span style={{ color: "#39ff14", fontSize: "11px", fontWeight: 700, letterSpacing: "2px" }}>{title}</span>
        {subtitle && <span style={{ color: "var(--text-dim)", fontSize: "10px", marginLeft: "8px" }}>{subtitle}</span>}
      </div>
      {children}
    </div>
  );
}

// ── Inline styles ──
const inputStyle: React.CSSProperties = {
  background: "#0d0d1a",
  border: "1px solid #39ff1430",
  borderRadius: "4px",
  color: "#e0e0e0",
  padding: "8px 12px",
  fontSize: "13px",
  width: "100%",
  outline: "none",
};

const labelStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: "4px",
  color: "var(--text-dim)",
  fontSize: "11px",
  letterSpacing: "0.5px",
};

const removeBtn: React.CSSProperties = {
  background: "#ff003c20",
  border: "1px solid #ff003c40",
  color: "#ff003c",
  borderRadius: "4px",
  cursor: "pointer",
  padding: "4px 10px",
  fontSize: "14px",
  flexShrink: 0,
};

const addBtn: React.CSSProperties = {
  background: "transparent",
  border: "1px dashed #39ff1440",
  color: "#39ff14",
  borderRadius: "4px",
  cursor: "pointer",
  padding: "6px 14px",
  fontSize: "11px",
  letterSpacing: "1px",
};

const previewBtnStyle: React.CSSProperties = {
  flex: 1,
  background: "#00f5ff15",
  border: "1px solid #00f5ff50",
  color: "#00f5ff",
  borderRadius: "6px",
  cursor: "pointer",
  padding: "12px",
  fontSize: "14px",
  fontWeight: 700,
  letterSpacing: "2px",
  transition: "all 0.2s",
};

const scrapeBtnStyle: React.CSSProperties = {
  flex: 2,
  background: "linear-gradient(135deg, #39ff1420, #00f5ff10)",
  border: "1px solid #39ff14",
  color: "#39ff14",
  borderRadius: "6px",
  cursor: "pointer",
  padding: "12px",
  fontSize: "14px",
  fontWeight: 700,
  letterSpacing: "2px",
  transition: "all 0.2s",
};

const exportBtnStyle: React.CSSProperties = {
  background: "#b400ff15",
  border: "1px solid #b400ff50",
  color: "#b400ff",
  borderRadius: "6px",
  cursor: "pointer",
  padding: "8px 20px",
  fontSize: "12px",
  fontWeight: 700,
  letterSpacing: "1px",
};

const tableStyle: React.CSSProperties = {
  width: "100%",
  borderCollapse: "collapse",
  fontSize: "12px",
};

const thStyle: React.CSSProperties = {
  background: "#39ff1410",
  border: "1px solid #39ff1420",
  color: "#39ff14",
  padding: "6px 10px",
  textAlign: "left",
  fontWeight: 700,
  fontSize: "11px",
  letterSpacing: "1px",
  whiteSpace: "nowrap",
};

const tdStyle: React.CSSProperties = {
  border: "1px solid #ffffff08",
  padding: "6px 10px",
  color: "#c0c0c0",
  maxWidth: "250px",
  overflow: "hidden",
  textOverflow: "ellipsis",
  whiteSpace: "nowrap",
};
