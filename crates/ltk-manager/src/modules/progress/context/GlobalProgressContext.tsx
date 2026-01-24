import { createContext, useContext, useState, type ReactNode } from "react";

export interface ProgressState {
    isActive: boolean;
    message: string;
    processed: number;
    total: number;
}

interface GlobalProgressContextType {
    progress: ProgressState | null;
    setProgress: (progress: ProgressState | null) => void;
}

const GlobalProgressContext = createContext<GlobalProgressContextType | null>(null);

export function GlobalProgressProvider({ children }: { children: ReactNode }) {
    const [progress, setProgress] = useState<ProgressState | null>(null);

    return (
        <GlobalProgressContext.Provider value={{ progress, setProgress }}>
            {children}
        </GlobalProgressContext.Provider>
    );
}

export function useGlobalProgressContext() {
    const context = useContext(GlobalProgressContext);
    if (!context) {
        throw new Error("useGlobalProgressContext must be used within a GlobalProgressProvider");
    }
    return context;
}
