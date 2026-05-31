import { 
    TSAgent, 
    TSModel, 
    TSSpace, 
    TSGrid, 
    TSGNNModel,
    AgentState,
    Deno as GaussTwin
} from "../mod.ts";

async function runSimulation() {
    // Create a 2D space
    const space = GaussTwin.spaceNew([100, 100]);
    console.log("Space dimensions:", space.dimensions());

    // Create a grid
    const grid = GaussTwin.gridNew(10, 10);

    // Create some agents
    const agents = Array.from({ length: 5 }, (_, i) => 
        GaussTwin.agentNew(`agent_${i}`));

    // Create a model
    const model = GaussTwin.modelNew();

    // Update agent states
    for (const agent of agents) {
        const state: AgentState = {
            id: agent.id,
            position: [Math.random() * 100, Math.random() * 100],
            data: { type: "walker" }
        };
        await agent.updateState(state);
        console.log(`Agent ${agent.id} initialized`);
    }

    // Create and train a GNN model
    const gnn = GaussTwin.gnnNew();
    const trainingData = {
        nodes: [[1.0, 0.0], [0.0, 1.0]],
        edges: [[0, 1]],
        labels: [1, 0]
    };
    await gnn.train(trainingData);
    console.log("GNN model trained");

    // Run simulation
    console.log("Running simulation...");
    await model.run(100);
    console.log("Simulation completed");
}

if (import.meta.main) {
    runSimulation().catch(console.error);
} 