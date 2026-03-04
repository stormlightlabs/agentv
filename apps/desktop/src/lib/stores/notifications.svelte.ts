import { useToast } from "$lib/stores/toast.svelte";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

type NotificationStore = {
  get windowFocused(): boolean;
  init(): Promise<void>;
  notify(title: string, body: string): void;
};

let store: NotificationStore | null = null;

export function useNotifications(): NotificationStore {
  if (store) return store;

  const toast = useToast();
  let windowFocused = $state(true);
  let osPermissionGranted = $state(false);
  const throttleMap = new Map<string, number>();
  const THROTTLE_MS = 10_000;

  function onFocus() {
    windowFocused = true;
  }

  function onBlur() {
    windowFocused = false;
  }

  async function init() {
    window.addEventListener("focus", onFocus);
    window.addEventListener("blur", onBlur);
    windowFocused = document.hasFocus();

    try {
      let granted = await isPermissionGranted();
      if (!granted) {
        const permission = await requestPermission();
        granted = permission === "granted";
      }
      osPermissionGranted = granted;
    } catch {
      osPermissionGranted = false;
    }
  }

  function notify(title: string, body: string) {
    // Throttle: max 1 OS notification per session per 10s
    const now = Date.now();
    const lastTime = throttleMap.get(title) ?? 0;
    const throttled = now - lastTime < THROTTLE_MS;

    if (!windowFocused && osPermissionGranted && !throttled) {
      throttleMap.set(title, now);
      sendNotification({ title, body });
    } else {
      toast.info(body);
    }
  }

  store = {
    get windowFocused() {
      return windowFocused;
    },
    init,
    notify,
  };

  return store;
}
