# Agent V Updater Worker

Cloudflare Worker that serves Tauri v2 updater manifests from KV metadata.

## Setup

1. Install dependencies:

   ```bash
   pnpm install
   ```

2. Configure bindings in `apps/worker/wrangler.toml`:
   - Set `kv_namespaces.id` for `UPDATE_METADATA`

3. Create local env file:

   ```bash
   cp apps/worker/.env.sample apps/worker/.dev.vars
   ```

4. Configure deploy auth in your shell or CI:

   ```bash
   export CLOUDFLARE_ACCOUNT_ID=...
   export CLOUDFLARE_API_TOKEN=...
   ```

5. Deploy:

   ```bash
   pnpm --filter agent-v-updater deploy
   ```

## API

### Check for Updates

```text
GET https://apps.stormlightlabs.org/agentv/{target}-{arch}/{current_version}
```

**Parameters:**

- `target`: `darwin`, `windows`, or `linux`
- `arch`: `x86_64`, `aarch64`, or `universal` (macOS)
- `current_version`: Currently installed version (e.g., `v0.8.0`)

**Response:**

- `204 No Content`: No update available
- `200 OK`: JSON manifest with update details

## Metadata Resolution

The worker resolves update metadata in this order:

1. KV source:
   - `latest_version` (or `UPDATER_LATEST_VERSION_KEY`)
   - `manifest:<version>` (or `UPDATER_MANIFEST_KEY_PREFIX + version`)

**Example Response:**

```json
{
  "version": "v0.9.0",
  "notes": "Release notes...",
  "pub_date": "2025-02-06T12:00:00Z",
  "platforms": {
    "darwin-universal": {
      "url": "https://.../Agent-V_0.9.0_macos.dmg",
      "signature": "base64-signature..."
    }
  }
}
```

## Publishing Updates

1. Build and sign artifacts via GitHub Actions
2. Upload signed artifacts to your public download host (for example GitHub Releases URLs in the manifest)
3. Update KV metadata:

   ```bash
   wrangler kv:key put --binding=UPDATE_METADATA "latest_version" "v0.9.0"
   wrangler kv:key put --binding=UPDATE_METADATA "manifest:v0.9.0" '{"version":"v0.9.0",...}'
   ```
