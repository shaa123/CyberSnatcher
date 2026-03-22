/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        cyber: {
          bg: "#0a0a0f",
          surface: "#12111a",
          card: "#1a1726",
          "card-hover": "#221e30",
          border: "#2a2540",
          "border-hover": "rgba(108, 63, 255, 0.25)",
          primary: "#8b5cf6",
          "primary-hover": "#a78bfa",
          "primary-glow": "rgba(139, 92, 246, 0.125)",
          accent: "#c084fc",
          "accent-secondary": "#6c3fff",
          "text-primary": "#e8e6f0",
          "text-secondary": "#8b86a0",
          "text-tertiary": "#5a5670",
          success: "#4ade80",
          warning: "#fbbf24",
          error: "#f87171",
          info: "#60a5fa",
        },
      },
      fontFamily: {
        sans: ["'Segoe UI'", "'Inter'", "system-ui", "sans-serif"],
        mono: [
          "'Cascadia Code'",
          "'JetBrains Mono'",
          "'Fira Code'",
          "monospace",
        ],
      },
      boxShadow: {
        "cyber-glow":
          "0 0 0 2px rgba(139, 92, 246, 0.25), 0 0 20px rgba(139, 92, 246, 0.0625)",
        "cyber-card": "0 0 20px rgba(139, 92, 246, 0.05)",
      },
      keyframes: {
        pulse_purple: {
          "0%, 100%": { opacity: "1" },
          "50%": { opacity: "0.6" },
        },
        shimmer: {
          "0%": { backgroundPosition: "-200% 0" },
          "100%": { backgroundPosition: "200% 0" },
        },
      },
      animation: {
        "pulse-purple": "pulse_purple 2s ease-in-out infinite",
        shimmer: "shimmer 2s linear infinite",
      },
    },
  },
  plugins: [],
};
