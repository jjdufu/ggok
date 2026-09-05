import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const dir = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [react(), tailwindcss()],
  base: "/",
  publicDir: resolve(dir, "public"),
  build: {
    outDir: resolve(dir, "dist"),
    emptyOutDir: true,
    cssCodeSplit: false,
    assetsInlineLimit: 0,
    chunkSizeWarningLimit: 800,
    rollupOptions: {
      input: resolve(dir, "index.html"),
      output: {
        format: "es",
        entryFileNames: "app.js",
        chunkFileNames: "app-[name].js",
        assetFileNames: (info) => {
          const n = info.name || "";
          if (n.endsWith(".css")) return "app.css";
          return "assets/[name][extname]";
        }
      }
    }
  }
});
