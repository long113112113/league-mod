
import { useQuery } from "@tanstack/react-query";
import { type ChampionWithSkins, api } from "@/lib/tauri";

export function useChampions() {
    return useQuery({
        queryKey: ["champions"],
        queryFn: async () => {
            const response = await api.getChampionsWithSkins();

            if (!response.ok) {
                throw response.error;
            }

            const champions = response.value;
            return champions;
        },
    });
}

export function useChampionIcon(championId: number) {
    return useQuery({
        queryKey: ["champion-icon", championId],
        queryFn: async () => {
            const result = await api.getChampionIconData(championId);
            if (result.ok) {
                return result.value;
            }
            return null;
        },
        staleTime: Infinity, // Icons don't change often
        enabled: !!championId,
    });
}

export type Champion = ChampionWithSkins;
