<script lang="ts">
  import EventInspector from "$lib/components/EventInspector.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import { useToast } from "$lib/stores/toast.svelte";
  import type { EventData } from "$lib/types";

  type Props = {
    open: boolean;
    selectedEvent: EventData | null;
    onOpenChange?: (open: boolean) => void;
  };

  let { open, selectedEvent, onOpenChange }: Props = $props();

  const toast = useToast();
</script>

{#if selectedEvent}
  <Modal open={open} {onOpenChange} size="lg" aria-label="Event inspector">
    <div class="h-[80vh]">
      <EventInspector
        event={selectedEvent}
        onCopyId={() => toast.success("ID copied")}
        onCopyPayload={() => toast.success("Payload copied")}
        onNavigateParent={(parentId) => {
          if (parentId) {
            toast.info(`Navigate to parent: ${parentId}`);
          }
        }} />
    </div>
  </Modal>
{/if}
