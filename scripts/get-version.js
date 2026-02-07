#!/usr/bin/env node
/**
 * Generate version string from git describe
 * Format: v1.4.1.dev2+gd67aed3
 *
 * Inspired by https://pdsls.dev
 */

import { execSync } from "node:child_process";
import { writeFileSync } from "node:fs";

try {
  /** raw: v1.4.1-2-gd67aed3 */
  const raw = execSync("git describe --tags --long --always --dirty", { encoding: "utf8" }).trim();

  /** want: v1.4.1.dev2+gd67aed3 */
  const m = raw.match(/^v?(\d+\.\d+\.\d+)-(\d+)-g([0-9a-f]+)(-dirty)?$/);

  let build = raw;
  if (m) {
    const [, ver, n, sha, dirty] = m;
    build = `v${ver}.dev${n}+g${sha}${dirty ? ".dirty" : ""}`;
  }

  console.log(build);
  writeFileSync("./VERSION", build);
} catch (error) {
  console.error("Failed to generate version:", error.message);
  process.exit(1);
}
