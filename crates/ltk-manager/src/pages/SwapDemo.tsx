import { useState } from "react";
import { LuPlay, LuLoader, LuFileJson, LuRefreshCw, LuArrowRightLeft } from "react-icons/lu";

import { Button } from "@/components/Button";
import { api, type SkinInfo } from "@/lib/tauri";

export function SwapDemo() {
    const [status, setStatus] = useState<"idle" | "extracting" | "loading_skins" | "swapping" | "success" | "error">("idle");
    const [logs, setLogs] = useState<string[]>([]);
    const [skins, setSkins] = useState<SkinInfo[]>([]);
    const [lastSwappedPath, setLastSwappedPath] = useState<string>("");

    // Champion Config
    const CHAMPION = "Ashe";

    const addLog = (msg: string) => setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] ${msg}`]);

    async function loadSkins() {
        setStatus("loading_skins");
        try {
            const list = await api.getExtractedSkins(CHAMPION);
            if (list.ok) {
                setSkins(list.value);
                addLog(`Found ${list.value.length} extracted skins.`);
            } else {
                addLog(`Failed to list skins: ${list.error}`);
            }
            setStatus("idle");
        } catch (e) {
            addLog(`Error listing skins: ${e}`);
            setStatus("idle");
        }
    }

    async function handleExtract() {
        setStatus("extracting");
        setLogs([]);
        addLog(`Starting extraction for ${CHAMPION}...`);

        try {
            const res = await api.extractBaseSkin(CHAMPION);

            if (res.ok && res.value.success) {
                addLog(`✅ Extracted successfully!`);
                addLog(`📂 Path: ${res.value.path}`);
                await loadSkins(); // Refresh list after extract
            } else {
                setStatus("error");
                addLog(`❌ Detailed Error: ${res.ok ? res.value.error : res.error}`);
            }
        } catch (err) {
            console.error(err);
            setStatus("error");
            addLog(`❌ Invoke Failed: ${String(err)}`);
        }
    }

    async function handleSwap(skinId: number, skinName: string) {
        if (status === "swapping") return;
        setStatus("swapping");
        addLog(`🔄 Swapping ${skinName} (ID: ${skinId}) to Base...`);

        try {
            const res = await api.prepareSwap(CHAMPION, skinId, 0); // 0 is Base

            if (res.ok && res.value.success) {
                setStatus("success");
                addLog(`✅ Swap Complete!`);
                addLog(`📂 Mod Created: ${res.value.mod_path}`);
                addLog(`📄 Files Remapped: ${res.value.file_count}`);
                setLastSwappedPath(res.value.mod_path || "");
            } else {
                setStatus("error");
                addLog(`❌ Swap Error: ${res.ok ? res.value.error : res.error}`);
            }
            // Reset status after a delay so user can swap again
            setTimeout(() => setStatus("idle"), 2000);
        } catch (e) {
            setStatus("error");
            addLog(`❌ Swap Exception: ${e}`);
        }
    }

    return (
        <div className="flex h-full flex-col p-6">
            <header className="mb-8">
                <h1 className="text-2xl font-bold text-surface-100">Swap Logic Demo</h1>
                <p className="text-surface-400">Step 1: Extract Base WAD. Step 2: Swap a skin to Base.</p>
            </header>

            <div className="grid gap-8 lg:grid-cols-2">
                {/* Left: Control Panel */}
                <div className="space-y-6">
                    {/* Extraction Section */}
                    <div className="rounded-xl border border-surface-700 bg-surface-800/50 p-6">
                        <h3 className="mb-4 text-lg font-semibold text-surface-200">1. Extraction</h3>
                        <p className="mb-4 text-sm text-surface-400">
                            Extracts `{CHAMPION}.wad.client` from League install. Required before swapping.
                        </p>
                        <div className="flex justify-end">
                            <Button
                                variant="filled"
                                onClick={handleExtract}
                                disabled={status === "extracting"}
                                left={status === "extracting" ? <LuLoader className="animate-spin" /> : <LuPlay />}
                            >
                                {status === "extracting" ? "Extracting..." : "Extract Base Skin"}
                            </Button>
                        </div>
                    </div>

                    {/* Skins List Section */}
                    {skins.length > 0 && (
                        <div className="rounded-xl border border-surface-700 bg-surface-800/50 p-6">
                            <div className="flex items-center justify-between mb-4">
                                <h3 className="text-lg font-semibold text-surface-200">2. Available Skins</h3>
                                <Button size="sm" variant="ghost" onClick={loadSkins}>
                                    <LuRefreshCw className="h-4 w-4" />
                                </Button>
                            </div>

                            <div className="space-y-3 max-h-[400px] overflow-y-auto pr-2 scrollbar-thin scrollbar-thumb-surface-700">
                                {skins.map((skin) => (
                                    <div key={skin.id} className="flex items-center justify-between rounded-lg bg-surface-900/50 p-3 ring-1 ring-surface-700">
                                        <div className="flex items-center gap-3">
                                            <div className="flex h-10 w-10 items-center justify-center rounded bg-surface-800 text-surface-400">
                                                <LuFileJson />
                                            </div>
                                            <div>
                                                <p className="font-medium text-surface-200">{skin.name}</p>
                                                <p className="text-xs text-surface-500">ID: {skin.id}</p>
                                            </div>
                                        </div>
                                        <Button
                                            size="sm"
                                            variant="outline"
                                            disabled={status === "swapping"}
                                            onClick={() => handleSwap(skin.id, skin.name)}
                                            left={<LuArrowRightLeft />}
                                        >
                                            Swap to Base
                                        </Button>
                                    </div>
                                ))}
                            </div>
                        </div>
                    )}
                </div>

                {/* Right: Output Log */}
                <div className="flex flex-col rounded-xl border border-surface-700 bg-black/40 font-mono text-sm shadow-inner h-[600px]">
                    <div className="border-b border-surface-700 bg-surface-800/80 px-4 py-2 text-xs font-medium text-surface-400">
                        Console Output
                        {lastSwappedPath && (
                            <span className="ml-2 select-all text-brand-400 block truncate">Last Mod: {lastSwappedPath}</span>
                        )}
                    </div>
                    <div className="flex-1 space-y-1 overflow-y-auto p-4 text-surface-300 scrollbar-thin scrollbar-thumb-surface-700">
                        {logs.length === 0 ? (
                            <span className="text-surface-600 italic">Ready to start...</span>
                        ) : (
                            logs.map((log, i) => (
                                <div key={i} className="break-all border-l-2 border-transparent pl-2 hover:border-surface-600">
                                    {log}
                                </div>
                            ))
                        )}
                    </div>
                </div>
            </div>
        </div>
    );
}
