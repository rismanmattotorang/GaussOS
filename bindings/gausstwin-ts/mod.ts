// Re-export everything from the generated WASM bindings
export * from "./pkg/gausstwin";

// Export Deno-specific functionality
import { instantiate } from "./pkg/gausstwin_bg.wasm";

export const Deno = {
    async initialize() {
        await instantiate();
    },
    agentNew: (id: string) => new TSAgent(id),
    modelNew: () => new TSModel(),
    spaceNew: (dimensions: number[]) => new TSSpace(dimensions),
    gridNew: (width: number, height: number) => new TSGrid(width, height),
    gnnNew: () => new TSGNNModel(),
}; 