import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";

import { useGlobalProgressContext, type ProgressState } from "../context";

interface ProgressPayload {
    processed: number;
    total: number;
    message: string;
}

// List of progress event names from backend
const PROGRESS_EVENTS = [
    "image-download-progress",
    "database-update-progress",
    "metadata-download-progress",
] as const;

/**
 * Hook to listen to backend progress events and update global progress state.
 * Should be used once in a component near the root of the app.
 */
export function useProgressListener() {
    const { setProgress } = useGlobalProgressContext();

    useEffect(() => {
        const unlistenPromises = PROGRESS_EVENTS.map((eventName) =>
            listen<ProgressPayload>(eventName, (event) => {
                const { processed, total, message } = event.payload;

                // If completed (processed >= total), clear progress after a short delay
                if (processed >= total && total > 0) {
                    setProgress({
                        isActive: true,
                        message: message || "Completed!",
                        processed,
                        total,
                    });

                    // Clear after 1.5 seconds
                    setTimeout(() => {
                        setProgress(null);
                    }, 1500);
                } else {
                    setProgress({
                        isActive: true,
                        message,
                        processed,
                        total,
                    });
                }
            })
        );

        return () => {
            // Cleanup all listeners
            Promise.all(unlistenPromises).then((unlistenFns) => {
                unlistenFns.forEach((fn) => fn());
            });
        };
    }, [setProgress]);
}

/**
 * Hook to get current global progress state.
 */
export function useGlobalProgress(): ProgressState | null {
    const { progress } = useGlobalProgressContext();
    return progress;
}

/**
 * Hook to manually set progress (for components that trigger operations).
 */
export function useSetGlobalProgress() {
    const { setProgress } = useGlobalProgressContext();
    return setProgress;
}
