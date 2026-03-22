const { execSync } = require("child_process");
const PORT = 1420;

try {
  if (process.platform === "win32") {
    const out = execSync("netstat -ano", { encoding: "utf8" });
    const lines = out.split("\n").filter(
      (l) => l.includes(":" + PORT + " ") && /LISTENING/i.test(l)
    );
    const pids = [...new Set(lines.map((l) => l.trim().split(/\s+/).pop()))];
    for (const pid of pids) {
      if (pid && pid !== "0") {
        try {
          execSync("taskkill /F /PID " + pid, { stdio: "ignore" });
        } catch {}
      }
    }
  } else {
    try {
      execSync("lsof -ti:" + PORT + " | xargs kill -9", { stdio: "ignore" });
    } catch {}
  }
} catch {}
