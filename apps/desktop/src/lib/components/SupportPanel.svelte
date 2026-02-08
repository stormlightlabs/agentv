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

<div class="flex flex-col h-full overflow-hidden bg-surface-soft" in:fade={{ duration: 200 }}>
  <div class="p-4 border-b border-surface-muted bg-surface">
    <h2 class="m-0 text-lg font-semibold text-fg">Support Agent V</h2>
    <p class="m-0 mt-1 text-sm text-fg-dim">Free to use. Funded by community support.</p>
  </div>

  <div class="flex-1 overflow-y-auto p-4">
    <div class="max-w-2xl mx-auto">
      <div class="p-6 bg-surface border border-surface-muted rounded-lg mb-6">
        <div class="flex items-center gap-3 mb-4">
          <span class="i-ri-heart-line text-2xl text-red"></span>
          <h3 class="m-0 text-lg font-semibold text-fg">Why Support Matters</h3>
        </div>
        <p class="m-0 mb-4 text-fg-dim leading-relaxed">
          Agent V is <strong>donationware</strong> - free to use with no paid tiers or feature restrictions. Your support
          helps keep the project independent, privacy-focused, and continuously improved.
        </p>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="flex items-start gap-3 p-3 bg-surface-soft rounded">
            <span class="i-ri-shield-check-line text-green mt-0.5"></span>
            <div>
              <div class="font-medium text-fg text-sm">Independent</div>
              <div class="text-xs text-fg-dim">No corporate interests or data harvesting</div>
            </div>
          </div>
          <div class="flex items-start gap-3 p-3 bg-surface-soft rounded">
            <span class="i-ri-lock-line text-blue mt-0.5"></span>
            <div>
              <div class="font-medium text-fg text-sm">Privacy-First</div>
              <div class="text-xs text-fg-dim">Your data stays local, always</div>
            </div>
          </div>
          <div class="flex items-start gap-3 p-3 bg-surface-soft rounded">
            <span class="i-ri-code-box-line text-purple mt-0.5"></span>
            <div>
              <div class="font-medium text-fg text-sm">Open Source</div>
              <div class="text-xs text-fg-dim">Transparent development for the community</div>
            </div>
          </div>
          <div class="flex items-start gap-3 p-3 bg-surface-soft rounded">
            <span class="i-ri-rocket-line text-yellow mt-0.5"></span>
            <div>
              <div class="font-medium text-fg text-sm">Continuous Improvement</div>
              <div class="text-xs text-fg-dim">New features and better performance</div>
            </div>
          </div>
        </div>
      </div>

      <div class="p-6 bg-surface border border-surface-muted rounded-lg">
        <div class="flex items-center gap-3 mb-4">
          <span class="i-ri-hand-heart-line text-2xl text-green"></span>
          <h3 class="m-0 text-lg font-semibold text-fg">Ways to Support</h3>
        </div>
        <p class="m-0 mb-6 text-fg-dim leading-relaxed">
          Every contribution helps, no matter the size. Choose the option that works best for you.
        </p>
        <div class="space-y-3">
          {#each fundingLinks as link}
            <button
              class="w-full p-4 bg-surface-soft border border-surface-muted rounded-lg flex items-center gap-4 hover:border-blue hover:bg-blue/5 transition-all text-left cursor-pointer group"
              onclick={() => openFundingLink(link.url)}>
              <span class="{link.icon} text-2xl text-fg-dim group-hover:text-blue transition-colors"></span>
              <div class="flex-1">
                <div class="font-medium text-fg">{link.name}</div>
                <div class="text-sm text-fg-dim">{link.description}</div>
              </div>
              <span class="i-ri-external-link-line text-fg-dim group-hover:text-blue transition-colors"></span>
            </button>
          {/each}
        </div>
      </div>

      <div class="mt-6 p-4 bg-surface-soft border border-surface-muted rounded-lg">
        <div class="flex items-center gap-2 mb-2">
          <span class="i-ri-information-line text-fg-dim"></span>
          <span class="text-sm font-medium text-fg">Other Ways to Help</span>
        </div>
        <ul class="m-0 pl-5 text-sm text-fg-dim space-y-1">
          <li>Star the project on GitHub</li>
          <li>Share Agent V with your team</li>
          <li>Report bugs and suggest features</li>
          <li>Contribute code or documentation</li>
        </ul>
      </div>

      <div class="mt-6 text-center">
        <p class="m-0 text-sm text-fg-dim">Thank you for using Agent V! Your support keeps this project alive.</p>
      </div>
    </div>
  </div>
</div>
