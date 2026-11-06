import { defineLoader } from "vitepress";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

export default defineLoader({
  load() {
    const path = resolve(__dirname, "../../package.json");
    const file = readFileSync(path, "utf-8");
    
    return {
      packageJson: JSON.parse(file),
    }
  }
});
