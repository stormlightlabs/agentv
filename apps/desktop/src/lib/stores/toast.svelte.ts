import type { ToastNotification } from "$lib/types";

type ToastStore = {
  get notifications(): ToastNotification[];
  addToast(message: string, type: ToastNotification["type"], duration?: number): string;
  removeToast(id: string): void;
};

let toastStore: ToastStore | null = null;

function getToastStore(): ToastStore {
  if (toastStore) return toastStore;

  let notifications = $state<ToastNotification[]>([]);

  function addToast(message: string, type: ToastNotification["type"] = "info", duration = 5000) {
    const id = Math.random().toString(36).substring(2, 9);
    const notification: ToastNotification = { id, type, message, duration };
    notifications = [...notifications, notification];

    if (notifications.length > 3) {
      notifications = notifications.slice(-3);
    }

    return id;
  }

  function removeToast(id: string) {
    notifications = notifications.filter((n) => n.id !== id);
  }

  toastStore = {
    get notifications() {
      return notifications;
    },
    addToast,
    removeToast,
  };

  return toastStore;
}

export function useToast() {
  const store = getToastStore();

  function success(message: string, duration?: number) {
    return store.addToast(message, "success", duration);
  }

  function error(message: string, duration?: number) {
    return store.addToast(message, "error", duration);
  }

  function info(message: string, duration?: number) {
    return store.addToast(message, "info", duration);
  }

  return {
    get notifications() {
      return store.notifications;
    },
    addToast: store.addToast,
    removeToast: store.removeToast,
    success,
    error,
    info,
  };
}
