import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "media", // tự động theo hệ thống
  theme: {
    extend: {
      borderRadius: { squircle: "28px", super: "40px" },
      fontFamily: {
        sans: ["-apple-system", "SF Pro Display", "Inter", "system-ui", "sans-serif"],
      },
    },
  },
  plugins: [],
} satisfies Config;
