<script lang="ts">
  import Dialog from "$lib/components/Dialog.svelte";

  type Props = {
    open: boolean;
    onOpenChange?: (open: boolean) => void;
    onLearnMore?: () => void;
    onDismiss?: () => void;
  };

  let { open, onOpenChange, onLearnMore, onDismiss }: Props = $props();
</script>

<Dialog {open} {onOpenChange} closeOnOutsideClick={false} role="dialog" aria-label="Support Agent V">
  <div class="fixed inset-0 flex items-center justify-center bg-black/50 p-4">
    <div class="bg-surface border-surface-muted w-full max-w-md rounded-lg border shadow-xl">
      <div class="p-6">
        <div class="mb-4 flex items-center gap-3">
          <span class="i-ri-heart-line text-red text-3xl"></span>
          <h2 class="text-fg m-0 text-xl font-semibold">Support Agent V</h2>
        </div>

        <p class="text-fg-dim m-0 mb-4 leading-relaxed">
          Great! You've successfully imported your first sessions. Agent V is <strong>donationware</strong> - free to use
          with no paid tiers.
        </p>

        <p class="text-fg-dim m-0 mb-6 leading-relaxed">
          If Agent V helps your workflow, consider supporting its continued development. Your contribution keeps the
          project independent and privacy-focused.
        </p>

        <div class="mb-6 space-y-3">
          <a
            href="https://github.com/sponsors/desertthunder"
            target="_blank"
            rel="noopener noreferrer"
            class="bg-surface-soft border-surface-muted hover:border-blue flex items-center gap-3 rounded-lg border p-3 no-underline transition-all">
            <span class="i-ri-github-line text-fg-dim text-xl"></span>
            <div class="flex-1">
              <div class="text-fg font-medium">GitHub Sponsors</div>
              <div class="text-fg-dim text-sm">Monthly or one-time support</div>
            </div>
            <span class="i-ri-external-link-line text-fg-dim"></span>
          </a>

          <a
            href="https://ko-fi.com/desertthunder"
            target="_blank"
            rel="noopener noreferrer"
            class="bg-surface-soft border-surface-muted hover:border-blue flex items-center gap-3 rounded-lg border p-3 no-underline transition-all">
            <span class="i-ri-cup-line text-fg-dim text-xl"></span>
            <div class="flex-1">
              <div class="text-fg font-medium">Ko-fi</div>
              <div class="text-fg-dim text-sm">Quick one-time donations</div>
            </div>
            <span class="i-ri-external-link-line text-fg-dim"></span>
          </a>
        </div>

        <div class="flex gap-3">
          <button
            class="bg-blue text-surface hover:bg-blue-bright flex-1 cursor-pointer rounded border-none px-4 py-2 font-medium transition-colors"
            onclick={() => {
              onOpenChange?.(false);
              onLearnMore?.();
            }}
            type="button">
            Learn More
          </button>
          <button
            class="border-surface-muted text-fg hover:border-fg cursor-pointer rounded border bg-transparent px-4 py-2 transition-colors"
            onclick={() => {
              onOpenChange?.(false);
              onDismiss?.();
            }}
            type="button">
            Dismiss
          </button>
        </div>
      </div>
    </div>
  </div>
</Dialog>
