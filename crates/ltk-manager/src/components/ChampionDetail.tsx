import { useChampionSkins, type Champion } from "@/modules/library/api/useChampions";
import { SkinImage } from "@/components/SkinImage";
import { SkinCardVariant } from "@/components/SkinCardVariant";

interface ChampionDetailProps {
    champion: Champion;
}

export function ChampionDetail({ champion }: ChampionDetailProps) {
    const { data: skins = [], isLoading } = useChampionSkins(champion.id);

    const defaultSkin = skins.find(s => s.id % 1000 === 0);
    const displaySkins = skins.filter(s => s.id % 1000 !== 0);

    return (
        <div className="flex h-full flex-col p-6 overflow-auto">
            <div className="flex items-start gap-6 mb-8">
                {/* Hero Section / Header */}
                <div className="h-32 w-32 shrink-0 overflow-hidden border-2 border-surface-600 bg-night-700 shadow-xl">
                    {defaultSkin ? (
                        <SkinImage
                            championId={champion.id}
                            skinId={defaultSkin.id}
                            alt={champion.name}
                            className="w-full h-full object-cover"
                            placeholder={
                                <div className="flex h-full w-full items-center justify-center bg-night-800">
                                    <span className="text-4xl font-bold text-night-200">
                                        {champion.name.charAt(0).toUpperCase()}
                                    </span>
                                </div>
                            }
                        />
                    ) : (
                        <div className="flex h-full w-full items-center justify-center bg-night-800">
                            <span className="text-4xl font-bold text-night-200">
                                {champion.name.charAt(0).toUpperCase()}
                            </span>
                        </div>
                    )}
                </div>

                <div className="flex-1 pt-2">
                    <h1 className="text-3xl font-bold text-surface-100 mb-2">{champion.name}</h1>
                    <p className="text-surface-400 text-lg leading-relaxed max-w-3xl">
                        {champion.description}
                    </p>
                </div>
            </div>

            <div className="space-y-4">
                <h2 className="text-xl font-semibold text-surface-200 flex items-center gap-2">
                    Available Skins
                    <span className="text-sm font-normal text-surface-500 bg-surface-800 px-2 py-0.5">
                        {isLoading ? "..." : displaySkins.length}
                    </span>
                </h2>

                {isLoading ? (
                    <div className="py-12 text-center text-surface-500">Loading skins...</div>
                ) : (
                    <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
                        {displaySkins.map((skin) => (
                            <SkinCardVariant
                                key={skin.id}
                                championId={champion.id}
                                skin={skin}
                            />
                        ))}

                        {displaySkins.length === 0 && (
                            <div className="col-span-full py-12 text-center text-surface-500">
                                No skins available for this champion.
                            </div>
                        )}
                    </div>
                )}
            </div>
        </div>
    );
}

