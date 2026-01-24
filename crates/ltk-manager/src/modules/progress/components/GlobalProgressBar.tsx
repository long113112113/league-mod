import { LuLoader } from "react-icons/lu";

import { useGlobalProgress } from "../hooks";

/**
 * Global progress bar that displays at the top of the app.
 * Shows download/update progress for background operations.
 */
export function GlobalProgressBar() {
    const progress = useGlobalProgress();

    if (!progress || !progress.isActive) {
        return null;
    }

    const percentage = progress.total > 0
        ? Math.round((progress.processed / progress.total) * 100)
        : 0;

    return (
        <div className="relative z-50 w-full shrink-0">
            {/* Progress info bar */}
            <div className="flex items-center gap-2 bg-gradient-to-r from-accent-600/30 to-accent-700/20 px-4 py-1.5 backdrop-blur-sm">
                <LuLoader className="h-4 w-4 shrink-0 animate-spin text-accent-400" />
                <span className="flex-1 truncate text-xs font-medium text-accent-100">
                    {progress.message}
                </span>
                <span className="text-xs font-semibold text-accent-300">
                    {percentage}%
                </span>
            </div>

            {/* Progress bar */}
            <div className="h-0.5 w-full bg-surface-800">
                <div
                    className="h-full bg-gradient-to-r from-accent-400 to-accent-500 transition-all duration-300 ease-out"
                    style={{ width: `${percentage}%` }}
                />
            </div>
        </div>
    );
}
