const { execSync } = require("child_process");
const net = require("net");

const PORT = 1420;

const server = net.createServer();
server.once("error", () => {
  // Port is in use — kill the process holding it
  try {
    if (process.platform === "win32") {
      const out = execSync(`netstat -ano | findstr :${PORT} | findstr LISTENING`, { encoding: "utf8" });
      const pids = [...new Set(out.trim().split("\n").map(l => l.trim().split(/\s+/).pop()))];
      for (const pid of pids) {
        try { execSync(`taskkill /F /PID ${pid}`, { stdio: "ignore" }); } catch {}
      }
    } else {
      execSync(`lsof -ti:${PORT} | xargs kill -9`, { stdio: "ignore" });
    }
  } catch {}
});
server.once("listening", () => {
  // Port is free — nothing to do
  server.close();
});
server.listen(PORT);
