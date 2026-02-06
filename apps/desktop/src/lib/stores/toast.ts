import type { ToastNotification } from "$lib/types";

let notifications = $state<ToastNotification[]>([]);

export function useToast() {
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

  function success(message: string, duration?: number) {
    return addToast(message, "success", duration);
  }

  function error(message: string, duration?: number) {
    return addToast(message, "error", duration);
  }

  function info(message: string, duration?: number) {
    return addToast(message, "info", duration);
  }

  return {
    get notifications() {
      return notifications;
    },
    addToast,
    removeToast,
    success,
    error,
    info,
  };
}
