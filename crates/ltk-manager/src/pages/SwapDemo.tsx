import { useMutation } from "@tanstack/react-query";
import { useState } from "react";
import { LuFlaskConical, LuPlay } from "react-icons/lu";

import { Button } from "@/components/Button";
import { api } from "@/lib/tauri";
import { unwrapForQuery } from "@/utils/query";

export function SwapDemo() {
    const [logs, setLogs] = useState<string[]>([]);
    const [isProcessing, setIsProcessing] = useState(false);

    const extractMutation = useMutation({
        mutationFn: async () => {
            setLogs((prev) => [...prev, "Starting extraction for Ashe..."]);
            const result = await api.extractBaseSkin("Ashe");
            return unwrapForQuery(result);
        },
        onSuccess: (data) => {
            setLogs((prev) => [
                ...prev,
                `Extraction successful!`,
                `Files exported to: ${data.path}`,
                `Total files: ${data.filesCount}`,
            ]);
        },
        onError: (error) => {
            setLogs((prev) => [...prev, `Error: ${error.message}`]);
        },
        onSettled: () => {
            setIsProcessing(false);
        },
    });

    const handleExtract = () => {
        setIsProcessing(true);
        extractMutation.mutate();
    };

    return (
        <div className="h-full overflow-auto p-6">
            <header className="mb-8 flex items-center justify-between">
                <div>
                    <h2 className="text-2xl font-bold text-surface-100">Swap Engine Demo</h2>
                    <p className="mt-1 text-surface-400">
                        Test the skin extraction and swapping logic directly.
                    </p>
                </div>
                <LuFlaskConical className="h-8 w-8 text-brand-400" />
            </header>

            <div className="grid gap-8 lg:grid-cols-2">
                {/* Controls */}
                <section className="space-y-6">
                    <div className="rounded-lg border border-surface-700 bg-surface-800 p-6">
                        <h3 className="mb-4 text-lg font-medium text-surface-100">1. Target Selection</h3>
                        <div className="space-y-4">
                            <div>
                                <label className="mb-2 block text-sm font-medium text-surface-400">
                                    Champion
                                </label>
                                <select
                                    className="w-full rounded-md border border-surface-600 bg-surface-700 px-3 py-2 text-surface-100 placeholder:text-surface-500 focus:border-brand-500 focus:outline-none"
                                    value="Ashe"
                                    disabled
                                >
                                    <option>Ashe</option>
                                </select>
                            </div>

                            <div>
                                <label className="mb-2 block text-sm font-medium text-surface-400">
                                    Target Skin
                                </label>
                                <select
                                    className="w-full rounded-md border border-surface-600 bg-surface-700 px-3 py-2 text-surface-100 placeholder:text-surface-500 focus:border-brand-500 focus:outline-none"
                                    value="project"
                                    disabled
                                >
                                    <option value="project">Project: Ashe (Skin08)</option>
                                </select>
                            </div>
                        </div>
                    </div>

                    <div className="rounded-lg border border-surface-700 bg-surface-800 p-6">
                        <h3 className="mb-4 text-lg font-medium text-surface-100">2. Actions</h3>
                        <div className="flex flex-col gap-3">
                            <Button
                                variant="filled"
                                size="lg"
                                onClick={handleExtract}
                                loading={isProcessing}
                                left={!isProcessing && <LuPlay className="h-5 w-5" />}
                                className="w-full justify-center"
                            >
                                Prepare Extraction
                            </Button>
                            <p className="text-xs text-surface-500">
                                This will extract Ashe's base skin WAD to the output directory.
                            </p>
                        </div>
                    </div>
                </section>

                {/* Logs */}
                <section className="flex h-[500px] flex-col rounded-lg border border-surface-700 bg-black/50 font-mono text-sm">
                    <div className="border-b border-surface-700 px-4 py-2 text-xs font-medium uppercase tracking-wider text-surface-400">
                        Execution Log
                    </div>
                    <div className="flex-1 overflow-auto p-4">
                        {logs.length === 0 ? (
                            <span className="text-surface-600 italic">Ready to process...</span>
                        ) : (
                            <ul className="space-y-1">
                                {logs.map((log, i) => (
                                    <li key={i} className="text-surface-300">
                                        <span className="mr-2 text-surface-600">[{i + 1}]</span>
                                        {log}
                                    </li>
                                ))}
                            </ul>
                        )}
                    </div>
                </section>
            </div>
        </div>
    );
}
