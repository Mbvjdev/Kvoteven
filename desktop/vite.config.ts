import { defineConfig } from "vitest/config";
export default defineConfig({ clearScreen: false, server: { host: "127.0.0.1", port: 1420, strictPort: true }, build: { target: "es2022", sourcemap: false }, test: { include: ["src/**/*.test.ts", "tests/**/*.test.ts"] } });
