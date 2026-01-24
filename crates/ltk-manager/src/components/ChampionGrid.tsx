

import { useState } from "react";
import { LuSearch } from "react-icons/lu";

import { ChampionCard } from "@/components/ChampionCard";
import { useChampions } from "@/modules/library/api/useChampions";
import type { AppError } from "@/lib/tauri";

export function ChampionGrid({ onSelectChampion }: { onSelectChampion?: (champion: any) => void }) {
    const [searchQuery, setSearchQuery] = useState("");
    const { data: champions = [], isLoading, error } = useChampions();

    const filteredChampions = champions.filter((champion) =>
        champion.name.toLowerCase().includes(searchQuery.toLowerCase())
    );

    if (isLoading) {
        return <LoadingState />;
    }

    if (error) {
        return <ErrorState error={error as unknown as AppError} />;
    }

    return (
        <div className="flex h-full flex-col">
            {/* Search Toolbar */}
            <div className="flex items-center gap-4 border-b border-surface-600/50 px-6 py-4">
                <div className="relative max-w-md flex-1">
                    <LuSearch className="absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2 text-surface-500" />
                    <input
                        type="text"
                        placeholder="Search champions..."
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        className="w-full rounded-lg border border-surface-600 bg-night-500 py-2 pr-4 pl-10 text-surface-100 placeholder:text-surface-500 focus:border-transparent focus:ring-2 focus:ring-brand-500 focus:outline-none"
                    />
                </div>
                <div className="text-sm text-surface-400">
                    {filteredChampions.length} champions
                </div>
            </div>

            {/* Grid Content */}
            <div className="flex-1 overflow-auto p-6">
                <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
                    {filteredChampions.map((champion) => (
                        <ChampionCard
                            key={champion.id}
                            champion={champion}
                            onClick={onSelectChampion}
                        />
                    ))}
                </div>

                {filteredChampions.length === 0 && (
                    <div className="flex h-64 flex-col items-center justify-center text-center">
                        <LuSearch className="mb-4 h-12 w-12 text-surface-600" />
                        <h3 className="mb-1 text-lg font-medium text-surface-300">No champions found</h3>
                        <p className="text-surface-500">Try adjusting your search query</p>
                    </div>
                )}
            </div>
        </div>
    );
}

function LoadingState() {
    return (
        <div className="flex h-64 items-center justify-center">
            <div className="h-8 w-8 animate-spin rounded-full border-2 border-brand-500 border-t-transparent" />
        </div>
    );
}

function ErrorState({ error }: { error: AppError }) {
    return (
        <div className="flex h-64 flex-col items-center justify-center text-center">
            <div className="mb-4 rounded-full bg-red-500/10 p-4">
                <span className="text-2xl">⚠️</span>
            </div>
            <h3 className="mb-1 text-lg font-medium text-surface-300">Failed to load champions</h3>
            <p className="mb-2 text-surface-500">{error.message || "Unknown error"}</p>
        </div>
    );
}
