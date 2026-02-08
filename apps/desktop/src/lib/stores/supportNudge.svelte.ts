import { logError } from "./logger.svelte";

const STORAGE_KEY = "agent-v-support-nudge";

type SupportNudgeState = { onboardingComplete: boolean; nudgeDismissed: boolean; firstIngestCompleted: boolean };

function createSupportNudgeStore() {
  let state = $state<SupportNudgeState>({
    onboardingComplete: false,
    nudgeDismissed: false,
    firstIngestCompleted: false,
  });

  function loadFromStorage(): SupportNudgeState {
    if (typeof window === "undefined") {
      return { onboardingComplete: false, nudgeDismissed: false, firstIngestCompleted: false };
    }
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        return JSON.parse(stored);
      }
    } catch {
      logError("Failed to load support nudge state from storage");
    }
    return { onboardingComplete: false, nudgeDismissed: false, firstIngestCompleted: false };
  }

  function saveToStorage(newState: SupportNudgeState): void {
    if (typeof window === "undefined") return;
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(newState));
    } catch {
      logError("Failed to save support nudge state to storage");
    }
  }

  function init(): void {
    state = loadFromStorage();
  }

  function markOnboardingComplete(): void {
    state = { ...state, onboardingComplete: true };
    saveToStorage(state);
  }

  function markFirstIngestCompleted(): void {
    state = { ...state, firstIngestCompleted: true };
    saveToStorage(state);
  }

  function dismissNudge(): void {
    state = { ...state, nudgeDismissed: true };
    saveToStorage(state);
  }

  function shouldShowNudge(): boolean {
    return state.onboardingComplete && state.firstIngestCompleted && !state.nudgeDismissed;
  }

  function reset(): void {
    state = { onboardingComplete: false, nudgeDismissed: false, firstIngestCompleted: false };
    saveToStorage(state);
  }

  init();

  return {
    get state() {
      return state;
    },
    init,
    markOnboardingComplete,
    markFirstIngestCompleted,
    dismissNudge,
    shouldShowNudge,
    reset,
  };
}

export const supportNudgeStore = createSupportNudgeStore();
