#!/usr/bin/env node
/**
 * Generate Tauri updater manifest from release artifacts
 */

import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const version = process.argv[2];
if (!version) {
  console.error("Usage: node generate-update-manifest.js <version>");
  process.exit(1);
}

const platforms = {};
const artifactsDir = "./artifacts";

// Map file extensions to platform keys
const platformMap = {
  "_x64-setup.exe": ["windows-x86_64"],
  "_x64_en-US.msi": ["windows-x86_64"],
  "_aarch64.dmg": ["darwin-aarch64"],
  "_x86_64.dmg": ["darwin-x86_64"],
  ".app": ["darwin-universal"],
  "_amd64.AppImage": ["linux-x86_64"],
};

// Find signature files and match them to artifacts
try {
  const files = readdirSync(artifactsDir, { recursive: true });
  const sigFiles = files.filter((f) => f.endsWith(".sig"));

  for (const sigFile of sigFiles) {
    const baseName = sigFile.replace(".sig", "");
    const sigPath = join(artifactsDir, sigFile);
    const signature = readFileSync(sigPath, "utf8").trim();

    // Find the corresponding artifact
    for (const [suffix, platformKeys] of Object.entries(platformMap)) {
      if (baseName.endsWith(suffix)) {
        for (const platformKey of platformKeys) {
          platforms[platformKey] = {
            url: `https://github.com/stormlightlabs/agent-v/releases/download/${version}/${baseName}`,
            signature,
          };
        }
        break;
      }
    }
  }

  const manifest = { version, notes: `Agent V ${version}`, pub_date: new Date().toISOString(), platforms };

  writeFileSync("./manifest.json", JSON.stringify(manifest, null, 2));
  console.log("Generated manifest.json:");
  console.log(JSON.stringify(manifest, null, 2));
} catch (error) {
  console.error("Failed to generate manifest:", error.message);
  process.exit(1);
}
