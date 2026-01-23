
import { type Champion, useChampionIcon } from "@/modules/library/api/useChampions";

interface ChampionCardProps {
    champion: Champion;
}

export function ChampionCard({ champion }: ChampionCardProps) {
    const { data: iconUrl } = useChampionIcon(champion.id);

    return (
        <div className="relative border border-surface-600 bg-night-500">
            {/* Thumbnail */}
            <div className="aspect-square w-full bg-night-600">
                {iconUrl ? (
                    <img
                        src={iconUrl}
                        alt={champion.name}
                        className="h-full w-full object-cover"
                        loading="lazy"
                    />
                ) : (
                    <div className="flex h-full w-full items-center justify-center">
                        <span className="text-4xl font-bold text-night-100">
                            {champion.name.charAt(0).toUpperCase()}
                        </span>
                    </div>
                )}
            </div>

            {/* Simple Label */}
            <div className="bg-surface-800 p-2 text-center border-t border-surface-600">
                <span className="text-sm font-medium text-white block truncate">{champion.name}</span>
            </div>
        </div>
    );
}
