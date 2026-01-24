import { useState } from "react";
import { LuChevronLeft, LuChevronRight } from "react-icons/lu";
import { SkinImage } from "@/components/SkinImage";
import type { SkinData } from "@/lib/tauri";

interface SkinCardVariantProps {
    championId: number;
    skin: SkinData;
}

export function SkinCardVariant({ championId, skin }: SkinCardVariantProps) {
    const variants = [
        { id: skin.id, name: skin.name, tilePath: skin.tilePath, isChroma: false },
        ...(skin.chromas || []).map(c => ({ ...c, isChroma: true }))
    ];

    const [currentIndex, setCurrentIndex] = useState(0);
    const currentVariant = variants[currentIndex];

    const handlePrev = (e: React.MouseEvent) => {
        e.stopPropagation();
        setCurrentIndex((prev) => (prev === 0 ? variants.length - 1 : prev - 1));
    };

    const handleNext = (e: React.MouseEvent) => {
        e.stopPropagation();
        setCurrentIndex((prev) => (prev === variants.length - 1 ? 0 : prev + 1));
    };

    const hasChromas = variants.length > 1;

    return (
        <div className="bg-night-600 border border-surface-700 p-3 hover:border-brand-500 transition-colors group relative">
            <div className="aspect-square bg-night-800 mb-3 flex items-center justify-center text-surface-600 text-sm overflow-hidden relative">
                <SkinImage
                    championId={championId}
                    skinId={currentVariant.id}
                    alt={currentVariant.name}
                    className="w-full h-full object-cover"
                    lazyFetch={true}
                />
                {hasChromas && (
                    <>
                        <button
                            onClick={handlePrev}
                            className="absolute left-1 top-1/2 -translate-y-1/2 bg-black/50 hover:bg-black/80 text-white p-1 rounded-full opacity-0 group-hover:opacity-100 transition-opacity"
                            title="Previous Chroma"
                        >
                            <LuChevronLeft className="w-5 h-5" />
                        </button>
                        <button
                            onClick={handleNext}
                            className="absolute right-1 top-1/2 -translate-y-1/2 bg-black/50 hover:bg-black/80 text-white p-1 rounded-full opacity-0 group-hover:opacity-100 transition-opacity"
                            title="Next Chroma"
                        >
                            <LuChevronRight className="w-5 h-5" />
                        </button>
                        <div className="absolute bottom-2 right-2 bg-black/60 text-white text-[10px] px-1.5 py-0.5 rounded opacity-0 group-hover:opacity-100 transition-opacity">
                            {currentIndex + 1}/{variants.length}
                        </div>
                    </>
                )}
            </div>

            <div className="text-center">
                <div className="text-sm font-medium text-surface-300 group-hover:text-surface-100 transition-colors truncate" title={currentVariant.name}>
                    {currentVariant.name}
                </div>
                {hasChromas && !currentVariant.isChroma && (
                    <div className="mt-1 text-xs text-surface-500">
                        {skin.chromas.length} chromas
                    </div>
                )}
                {currentVariant.isChroma && (
                    <div className="mt-1 text-xs text-brand-400">
                        Chroma
                    </div>
                )}
            </div>
        </div>
    );
}
