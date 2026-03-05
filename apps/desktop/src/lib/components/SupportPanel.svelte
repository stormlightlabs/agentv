<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { fade } from "svelte/transition";

  const fundingLinks = [
    {
      name: "GitHub Sponsors",
      url: "https://github.com/sponsors/desertthunder",
      icon: "i-ri-github-line",
      description: "Sponsor my open source work",
    },
    { name: "Ko-fi", url: "https://ko-fi.com/desertthunder", icon: "i-ri-cup-line", description: "Buy me a coffee" },
  ];

  async function openFundingLink(url: string) {
    await openUrl(url);
  }
</script>

<div class="bg-surface-soft flex h-full flex-col overflow-hidden" in:fade={{ duration: 200 }}>
  <div class="border-surface-muted bg-surface border-b p-4">
    <h2 class="text-fg m-0 text-lg font-semibold">Support Agent V</h2>
    <p class="text-fg-dim m-0 mt-1 text-sm">Free to use. Funded by community support.</p>
  </div>

  <div class="flex-1 overflow-y-auto p-4">
    <div class="mx-auto max-w-2xl">
      <div class="bg-surface border-surface-muted mb-6 rounded-lg border p-6">
        <div class="mb-4 flex items-center gap-3">
          <span class="i-ri-heart-line text-red text-2xl"></span>
          <h3 class="text-fg m-0 text-lg font-semibold">Why Support Matters</h3>
        </div>
        <p class="text-fg-dim m-0 mb-4 leading-relaxed">
          Agent V is <strong>donationware</strong> - free to use with no paid tiers or feature restrictions. Your support
          helps keep the project independent, privacy-focused, and continuously improved.
        </p>
        <div class="grid grid-cols-1 gap-4 md:grid-cols-2">
          <div class="bg-surface-soft flex items-start gap-3 rounded p-3">
            <span class="i-ri-shield-check-line text-green mt-0.5"></span>
            <div>
              <div class="text-fg text-sm font-medium">Independent</div>
              <div class="text-fg-dim text-xs">No corporate interests or data harvesting</div>
            </div>
          </div>
          <div class="bg-surface-soft flex items-start gap-3 rounded p-3">
            <span class="i-ri-lock-line text-blue mt-0.5"></span>
            <div>
              <div class="text-fg text-sm font-medium">Privacy-First</div>
              <div class="text-fg-dim text-xs">Your data stays local, always</div>
            </div>
          </div>
          <div class="bg-surface-soft flex items-start gap-3 rounded p-3">
            <span class="i-ri-code-box-line text-purple mt-0.5"></span>
            <div>
              <div class="text-fg text-sm font-medium">Open Source</div>
              <div class="text-fg-dim text-xs">Transparent development for the community</div>
            </div>
          </div>
          <div class="bg-surface-soft flex items-start gap-3 rounded p-3">
            <span class="i-ri-rocket-line text-yellow mt-0.5"></span>
            <div>
              <div class="text-fg text-sm font-medium">Continuous Improvement</div>
              <div class="text-fg-dim text-xs">New features and better performance</div>
            </div>
          </div>
        </div>
      </div>

      <div class="bg-surface border-surface-muted rounded-lg border p-6">
        <div class="mb-4 flex items-center gap-3">
          <span class="i-ri-hand-heart-line text-green text-2xl"></span>
          <h3 class="text-fg m-0 text-lg font-semibold">Ways to Support</h3>
        </div>
        <p class="text-fg-dim m-0 mb-6 leading-relaxed">
          Every contribution helps, no matter the size. Choose the option that works best for you.
        </p>
        <div class="space-y-3">
          {#each fundingLinks as link (link.name)}
            <button
              class="bg-surface-soft border-surface-muted hover:border-blue hover:bg-blue/5 group flex w-full cursor-pointer items-center gap-4 rounded-lg border p-4 text-left transition-all"
              onclick={() => openFundingLink(link.url)}>
              <span class="{link.icon} text-fg-dim group-hover:text-blue text-2xl transition-colors"></span>
              <div class="flex-1">
                <div class="text-fg font-medium">{link.name}</div>
                <div class="text-fg-dim text-sm">{link.description}</div>
              </div>
              <span class="i-ri-external-link-line text-fg-dim group-hover:text-blue transition-colors"></span>
            </button>
          {/each}
        </div>
      </div>

      <div class="bg-surface-soft border-surface-muted mt-6 rounded-lg border p-4">
        <div class="mb-2 flex items-center gap-2">
          <span class="i-ri-information-line text-fg-dim"></span>
          <span class="text-fg text-sm font-medium">Other Ways to Help</span>
        </div>
        <ul class="text-fg-dim m-0 space-y-1 pl-5 text-sm">
          <li>Star the project on GitHub</li>
          <li>Share Agent V with your team</li>
          <li>Report bugs and suggest features</li>
          <li>Contribute code or documentation</li>
        </ul>
      </div>

      <div class="mt-6 text-center">
        <p class="text-fg-dim m-0 text-sm">Thank you for using Agent V! Your support keeps this project alive.</p>
      </div>
    </div>
  </div>
</div>
