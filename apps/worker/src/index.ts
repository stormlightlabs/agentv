/**
 * Agent V Updater Worker
 *
 * Request path format:
 *   /agentv/{target}-{arch}/{current_version}
 *
 * Data sources:
 *
 *  KV: latest_version + manifest:<version>
 */

export interface Env {
  UPDATE_METADATA?: KVNamespace;
  UPDATER_PATH_PREFIX?: string;
  UPDATER_LATEST_VERSION_KEY?: string;
  UPDATER_MANIFEST_KEY_PREFIX?: string;
}

type UpdateManifest = { version: string; notes: string; pub_date: string; platforms: Record<string, PlatformInfo> };

type PlatformInfo = { url: string; signature: string };

type ParsedRequestPath = { platformKey: string; currentVersion: string };

type ParsedVersion = { major: number; minor: number; patch: number; prerelease: string[] };

const DEFAULT_PATH_PREFIX = "/agentv";
const DEFAULT_LATEST_VERSION_KEY = "latest_version";
const DEFAULT_MANIFEST_KEY_PREFIX = "manifest:";

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    try {
      const url = new URL(request.url);
      const parsedPath = parseRequestPath(url.pathname, env.UPDATER_PATH_PREFIX ?? DEFAULT_PATH_PREFIX);
      if (!parsedPath) {
        return jsonResponse({ error: "Invalid path format" }, 400);
      }

      const manifest = await loadLatestManifest(env);
      if (!manifest) {
        return jsonResponse({ error: "Update metadata not found" }, 404);
      }

      if (!isNewerVersion(parsedPath.currentVersion, manifest.version)) {
        return new Response(null, { status: 204 });
      }

      const platformInfo = manifest.platforms[parsedPath.platformKey];
      if (!platformInfo) {
        return jsonResponse({ error: `Platform ${parsedPath.platformKey} not supported` }, 404);
      }

      const response: UpdateManifest = {
        version: manifest.version,
        notes: manifest.notes,
        pub_date: manifest.pub_date,
        platforms: { [parsedPath.platformKey]: platformInfo },
      };

      return jsonResponse(response, 200, { "Cache-Control": "no-cache" });
    } catch (error) {
      console.error("Updater error:", error);
      return jsonResponse({ error: "Internal server error" }, 500);
    }
  },
};

async function loadLatestManifest(env: Env): Promise<UpdateManifest | null> {
  return loadManifestFromKv(env);
}

async function loadManifestFromKv(env: Env): Promise<UpdateManifest | null> {
  if (!env.UPDATE_METADATA) {
    return null;
  }

  const latestVersionKey = env.UPDATER_LATEST_VERSION_KEY ?? DEFAULT_LATEST_VERSION_KEY;
  const manifestPrefix = env.UPDATER_MANIFEST_KEY_PREFIX ?? DEFAULT_MANIFEST_KEY_PREFIX;

  const latestVersion = (await env.UPDATE_METADATA.get(latestVersionKey))?.trim();
  if (!latestVersion) {
    return null;
  }

  const manifestKey = `${manifestPrefix}${latestVersion}`;
  const manifestValue = await env.UPDATE_METADATA.get(manifestKey, "json");
  if (!isUpdateManifest(manifestValue)) {
    console.error(`Invalid manifest in KV key "${manifestKey}"`);
    return null;
  }

  return manifestValue;
}

function parseRequestPath(pathname: string, configuredPrefix: string): ParsedRequestPath | null {
  const normalizedPrefix = normalizePathPrefix(configuredPrefix);
  const prefixWithSlash = `${normalizedPrefix}/`;
  if (!pathname.startsWith(prefixWithSlash)) {
    return null;
  }

  const remainder = pathname.slice(prefixWithSlash.length);
  const slashIndex = remainder.indexOf("/");
  if (slashIndex <= 0 || slashIndex === remainder.length - 1) {
    return null;
  }

  const platformSegment = remainder.slice(0, slashIndex);
  const currentVersion = decodeURIComponent(remainder.slice(slashIndex + 1));
  if (!currentVersion) {
    return null;
  }

  const dashIndex = platformSegment.indexOf("-");
  if (dashIndex <= 0 || dashIndex === platformSegment.length - 1) {
    return null;
  }

  const target = platformSegment.slice(0, dashIndex);
  const arch = platformSegment.slice(dashIndex + 1);
  return { platformKey: `${target}-${arch}`, currentVersion };
}

function normalizePathPrefix(prefix: string): string {
  const trimmed = prefix.trim();
  if (!trimmed) {
    return DEFAULT_PATH_PREFIX;
  }
  return `/${trimmed.replace(/^\/+|\/+$/g, "")}`;
}

function isUpdateManifest(value: unknown): value is UpdateManifest {
  if (!isRecord(value)) {
    return false;
  }

  if (
    typeof value.version !== "string" ||
    typeof value.notes !== "string" ||
    typeof value.pub_date !== "string" ||
    !isRecord(value.platforms)
  ) {
    return false;
  }

  for (const platformValue of Object.values(value.platforms)) {
    if (!isPlatformInfo(platformValue)) {
      return false;
    }
  }

  return true;
}

function isPlatformInfo(value: unknown): value is PlatformInfo {
  return isRecord(value) && typeof value.url === "string" && typeof value.signature === "string";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

/**
 * Returns true when latestVersion is newer than currentVersion.
 * Supports normal semver and Agent V dev versions like v1.4.1.dev2+gabc123.
 */
function isNewerVersion(currentVersion: string, latestVersion: string): boolean {
  const current = parseVersion(currentVersion);
  const latest = parseVersion(latestVersion);

  if (!current || !latest) {
    return currentVersion !== latestVersion;
  }

  return compareVersions(current, latest) < 0;
}

function parseVersion(version: string): ParsedVersion | null {
  let normalized = version.trim().replace(/^v/i, "");
  normalized = normalized.replace(/\.dev(\d+)(?=$|[+-])/, "-dev.$1");

  const match = normalized.match(
    /^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/,
  );
  if (!match) {
    return null;
  }

  const [, major, minor, patch, prereleaseRaw] = match;
  return {
    major: Number(major),
    minor: Number(minor),
    patch: Number(patch),
    prerelease: prereleaseRaw ? prereleaseRaw.split(".") : [],
  };
}

function compareVersions(a: ParsedVersion, b: ParsedVersion): number {
  if (a.major !== b.major) {
    return a.major < b.major ? -1 : 1;
  }
  if (a.minor !== b.minor) {
    return a.minor < b.minor ? -1 : 1;
  }
  if (a.patch !== b.patch) {
    return a.patch < b.patch ? -1 : 1;
  }
  return comparePrerelease(a.prerelease, b.prerelease);
}

function comparePrerelease(a: string[], b: string[]): number {
  if (a.length === 0 && b.length === 0) {
    return 0;
  }
  if (a.length === 0) {
    return 1;
  }
  if (b.length === 0) {
    return -1;
  }

  const length = Math.max(a.length, b.length);
  for (let i = 0; i < length; i += 1) {
    const left = a[i];
    const right = b[i];

    if (left === undefined) {
      return -1;
    }
    if (right === undefined) {
      return 1;
    }
    if (left === right) {
      continue;
    }

    const leftIsNumber = /^\d+$/.test(left);
    const rightIsNumber = /^\d+$/.test(right);

    if (leftIsNumber && rightIsNumber) {
      const leftValue = Number(left);
      const rightValue = Number(right);
      if (leftValue !== rightValue) {
        return leftValue < rightValue ? -1 : 1;
      }
      continue;
    }
    if (leftIsNumber !== rightIsNumber) {
      return leftIsNumber ? -1 : 1;
    }

    return left < right ? -1 : 1;
  }

  return 0;
}

function jsonResponse(payload: unknown, status: number, headers: HeadersInit = {}): Response {
  return new Response(JSON.stringify(payload), { status, headers: { "Content-Type": "application/json", ...headers } });
}
