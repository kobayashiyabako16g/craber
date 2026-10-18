import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  // Every asset is referenced relatively so that `craber build` output can be
  // dropped under any prefix — GitHub Pages serves it from /<repo>/ — without
  // rewriting the bundle. `craber build --base-path` overrides this at build
  // time when an absolute prefix is actually wanted.
  base: "./",
  build: {
    outDir: "dist",
    // The static site has no backend, so the whole app has to be cacheable by
    // content hash; keep the default hashed asset names.
    assetsDir: "assets",
  },
});
