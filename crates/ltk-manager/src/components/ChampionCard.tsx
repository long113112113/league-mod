import { type Champion } from "@/modules/library/api/useChampions";
import { SkinImage } from "@/components/SkinImage";

interface ChampionCardProps {
    champion: Champion;
    onClick?: (champion: Champion) => void;
}

export function ChampionCard({ champion, onClick }: ChampionCardProps) {
    return (
        <div
            className="relative border border-surface-600 bg-night-500 cursor-pointer transition-colors hover:border-brand-500 hover:shadow-lg hover:shadow-brand-500/20"
            onClick={() => onClick?.(champion)}
        >
            {/* Thumbnail */}
            <div className="aspect-square w-full bg-night-600">
                <SkinImage
                    championId={champion.id}
                    skinId={champion.id * 1000}
                    alt={champion.name}
                    className="h-full w-full object-cover"
                    lazyFetch={true}
                    placeholder={
                        <div className="flex h-full w-full items-center justify-center">
                            <span className="text-4xl font-bold text-night-100">
                                {champion.name.charAt(0).toUpperCase()}
                            </span>
                        </div>
                    }
                />
            </div>

            {/* Simple Label */}
            <div className="bg-surface-800 p-2 text-center border-t border-surface-600">
                <span className="text-sm font-medium text-white block truncate">{champion.name}</span>
            </div>
        </div>
    );
}
