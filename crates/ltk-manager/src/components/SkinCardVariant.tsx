import { useState } from "react";
import { LuChevronLeft, LuChevronRight } from "react-icons/lu";
import { SkinImage } from "@/components/SkinImage";
import type { SkinData } from "@/lib/tauri";
import { invoke } from "@tauri-apps/api/core";
import { LuPlay } from "react-icons/lu";
import { useSetGlobalProgress } from "@/modules/progress/hooks";
import { useToast } from "@/components/Toast";

interface SkinCardVariantProps {
    championId: number;
    skin: SkinData;
}

export function SkinCardVariant({ championId, skin }: SkinCardVariantProps) {
    const setProgress = useSetGlobalProgress();
    const { toast } = useToast();
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

    const handleDownload = async (e: React.MouseEvent) => {
        e.stopPropagation();

        // Start progress
        setProgress({
            isActive: true,
            message: `Downloading mod skin for ${skin.name}...`,
            processed: 0,
            total: 100
        });
        toast({ title: "Started", description: "Skin mod download started", type: "info" });

        try {
            const res = await invoke("download_skin", {
                championId: championId,
                skinId: currentVariant.id
            });
            console.log(res);

            // Run skin
            toast({ title: "Running", description: "Applying skin mod...", type: "info" });
            setProgress({
                isActive: true,
                message: "Starting Patcher...",
                processed: 100,
                total: 100
            });

            await invoke("run_skin", {
                championId: championId,
                skinId: currentVariant.id
            });

            // Complete progress
            toast({ title: "Success", description: "Skin mod running!", type: "success" });

            // Clear progress after a delay
            setTimeout(() => setProgress(null), 1500);

        } catch (error) {
            console.error("Failed to download/run skin:", error);
            setProgress(null);
            toast({ title: "Error", description: `Failed to download: ${error}`, type: "error" });
        }
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
                <button
                    onClick={handleDownload}
                    className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-brand-500/90 hover:bg-brand-500 text-white p-3 rounded-full opacity-0 group-hover:opacity-100 transition-all transform scale-75 group-hover:scale-100 shadow-lg z-10"
                    title="Run Mod Skin"
                >
                    <LuPlay className="w-6 h-6 fill-current" />
                </button>
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
