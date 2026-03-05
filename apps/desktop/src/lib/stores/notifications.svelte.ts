import { useToast } from "$lib/stores/toast.svelte";
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { SvelteMap } from "svelte/reactivity";

type NotificationGroup = {
  title: string;
  count: number;
  messages: string[];
};

export class NotificationStore {
  private readonly toast = useToast();
  private readonly throttleMap = new SvelteMap<string, number>();
  private readonly groupedNotifications = new SvelteMap<string, NotificationGroup>();
  private windowFocusedValue = $state(true);
  private osPermissionGranted = $state(false);
  private readonly throttleMs = 10_000;
  private readonly groupWindowMs = 1500;

  get windowFocused(): boolean {
    return this.windowFocusedValue;
  }

  private onFocus = () => {
    this.windowFocusedValue = true;
  };

  private onBlur = () => {
    this.windowFocusedValue = false;
  };

  async init(): Promise<void> {
    window.addEventListener("focus", this.onFocus);
    window.addEventListener("blur", this.onBlur);
    this.windowFocusedValue = document.hasFocus();

    try {
      let granted = await isPermissionGranted();
      if (!granted) {
        const permission = await requestPermission();
        granted = permission === "granted";
      }
      this.osPermissionGranted = granted;
    } catch {
      this.osPermissionGranted = false;
    }
  }

  private flushGroup(title: string): void {
    const group = this.groupedNotifications.get(title);
    if (!group) return;

    this.groupedNotifications.delete(title);

    const body = this.formatGroupedBody(group);
    this.dispatchNotification(group.title, body);
  }

  private formatGroupedBody(group: NotificationGroup): string {
    if (group.count <= 1) {
      return group.messages[0] ?? "New update";
    }

    const previewMessages = group.messages.slice(0, 2);
    const previewText = previewMessages.join(" | ");
    const extraCount = group.count - previewMessages.length;
    const extraText = extraCount > 0 ? ` (+${extraCount} more)` : "";

    return previewText ? `${group.count} updates: ${previewText}${extraText}` : `${group.count} new updates`;
  }

  private dispatchNotification(title: string, body: string): void {
    const now = Date.now();
    const lastTime = this.throttleMap.get(title) ?? 0;
    const throttled = now - lastTime < this.throttleMs;

    if (!this.windowFocusedValue && this.osPermissionGranted && !throttled) {
      this.throttleMap.set(title, now);
      sendNotification({ title, body });
    } else {
      this.toast.info(body);
    }
  }

  notify(title: string, body: string): void {
    const existingGroup = this.groupedNotifications.get(title);

    if (existingGroup) {
      existingGroup.count += 1;
      if (!existingGroup.messages.includes(body) && existingGroup.messages.length < 3) {
        existingGroup.messages.push(body);
      }
      return;
    }

    globalThis.setTimeout(() => {
      this.flushGroup(title);
    }, this.groupWindowMs);

    this.groupedNotifications.set(title, {
      title,
      count: 1,
      messages: [body],
    });
  }
}

let store: NotificationStore | null = null;

export function useNotifications(): NotificationStore {
  if (!store) {
    store = new NotificationStore();
  }
  return store;
}
