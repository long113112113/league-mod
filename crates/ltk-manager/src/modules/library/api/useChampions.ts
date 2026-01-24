
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





export function useChampionSkins(championId: number, enabled: boolean = true) {
    return useQuery({
        queryKey: ["champion-skins", championId],
        queryFn: async () => {
            const result = await api.getChampionSkins(championId);
            if (result.ok) {
                return result.value;
            }
            throw result.error;
        },
        enabled: enabled && !!championId,
    });
}

export type Champion = ChampionWithSkins;
