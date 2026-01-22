import { createFileRoute } from "@tanstack/react-router";

import { SwapDemo } from "@/pages/SwapDemo";

export const Route = createFileRoute("/demo")({
    component: SwapDemo,
});
