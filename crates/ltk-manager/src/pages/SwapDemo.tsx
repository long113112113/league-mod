import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import { LuPlay, LuCheckCheck, LuLoader, LuFileJson } from "react-icons/lu";

import { Button } from "@/components/Button";

interface ExtractResult {
    success: boolean;
    path?: string;
    error?: string;
    files_count?: number;
}

export function SwapDemo() {
    const [status, setStatus] = useState<"idle" | "extracting" | "success" | "error">("idle");
    const [logs, setLogs] = useState<string[]>([]);
    const [resultPath, setResultPath] = useState<string>("");

    const addLog = (msg: string) => setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] ${msg}`]);

    async function handlePrepSwap() {
        setStatus("extracting");
        setLogs([]);
        addLog("Starting extraction for Ashe (Base Skin)...");

        try {
            // Call the backend command we are about to write
            // Command: extract_base_skin
            // Arg: champion (String)
            const res = await invoke<ExtractResult>("extract_base_skin", { champion: "Ashe" });

            if (res.success) {
                setStatus("success");
                setResultPath(res.path || "");
                addLog(`✅ Success! Extracted ${res.files_count} files.`);
                addLog(`📂 Path: ${res.path}`);
            } else {
                setStatus("error");
                addLog(`❌ Error: ${res.error}`);
            }
        } catch (err) {
            console.error(err);
            setStatus("error");
            addLog(`❌ Invoke Failed: ${String(err)}`);
        }
    }

    return (
        <div className="flex h-full flex-col p-6">
            <header className="mb-8">
                <h1 className="text-2xl font-bold text-surface-100">Swap Logic Demo</h1>
                <p className="text-surface-400">Testing Step 1: Extract Base WAD for remapping.</p>
            </header>

            <div className="grid gap-8 lg:grid-cols-2">
                {/* Left: Control Panel */}
                <div className="space-y-6">
                    <div className="rounded-xl border border-surface-700 bg-surface-800/50 p-6">
                        <h3 className="mb-4 text-lg font-semibold text-surface-200">Target Skin</h3>

                        <div className="flex items-center gap-4 rounded-lg bg-surface-900/50 p-4 ring-1 ring-surface-700">
                            <div className="flex h-16 w-16 items-center justify-center rounded-lg bg-brand-900/20 text-brand-400 ring-1 ring-brand-500/20">
                                <LuFileJson className="h-8 w-8" />
                            </div>
                            <div className="flex-1">
                                <h4 className="font-medium text-surface-100">Ashe Project</h4>
                                <p className="text-sm text-surface-400">Skin ID: 08</p>
                            </div>
                            <div className="text-right text-xs text-surface-500">
                                <p>Base Skin: ID 00</p>
                                <p>Remap: 08 → 00</p>
                            </div>
                        </div>

                        <div className="mt-6 flex justify-end">
                            <Button
                                variant="filled"
                                onClick={handlePrepSwap}
                                disabled={status === "extracting"}
                                left={status === "extracting" ? <LuLoader className="animate-spin" /> : <LuPlay />}
                            >
                                {status === "extracting" ? "Extracting..." : "Prepare Extraction"}
                            </Button>
                        </div>
                    </div>
                </div>

                {/* Right: Output Log */}
                <div className="flex flex-col rounded-xl border border-surface-700 bg-black/40 font-mono text-sm shadow-inner">
                    <div className="border-b border-surface-700 bg-surface-800/80 px-4 py-2 text-xs font-medium text-surface-400">
                        Console Output
                        {resultPath && (
                            <span className="ml-2 select-all text-brand-400">({resultPath})</span>
                        )}
                    </div>
                    <div className="flex-1 space-y-1 overflow-y-auto p-4 text-surface-300 scrollbar-thin scrollbar-thumb-surface-700">
                        {logs.length === 0 ? (
                            <span className="text-surface-600 italic">Ready to start...</span>
                        ) : (
                            logs.map((log, i) => (
                                <div key={i} className="break-all">{log}</div>
                            ))
                        )}
                        {status === "success" && (
                            <div className="mt-2 flex items-center gap-2 text-green-400">
                                <LuCheckCheck className="h-4 w-4" />
                                <span>Operation Completed Successfully</span>
                            </div>
                        )}
                    </div>
                </div>
            </div>
        </div>
    );
}
