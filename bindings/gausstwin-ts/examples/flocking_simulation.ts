import {
    TSAgent,
    TSModel,
    TSSpace,
    TSGrid,
    TSBehavior,
    AgentState,
    SimulationConfig,
    Deno as GaussTwin
} from "../mod.ts";

class FlockingSimulation {
    private space: TSSpace;
    private grid: TSGrid;
    private model: TSModel;
    private agents: TSAgent[];
    private canvas: HTMLCanvasElement;
    private ctx: CanvasRenderingContext2D;

    constructor(numAgents = 50, spaceSize = 100) {
        // Initialize GaussTwin components
        this.space = GaussTwin.spaceNew([spaceSize, spaceSize]);
        this.grid = GaussTwin.gridNew(10, 10);
        this.model = GaussTwin.modelNew();
        this.agents = [];

        // Create canvas
        this.canvas = document.createElement("canvas");
        this.canvas.width = 800;
        this.canvas.height = 800;
        document.body.appendChild(this.canvas);
        this.ctx = this.canvas.getContext("2d")!;

        // Create agents with flocking behavior
        for (let i = 0; i < numAgents; i++) {
            const agent = GaussTwin.agentNew(`boid_${i}`);
            
            // Initialize with random position and velocity
            const state: AgentState = {
                id: agent.id,
                position: [Math.random() * spaceSize, Math.random() * spaceSize],
                data: {
                    type: "boid",
                    velocity: [Math.random() * 2 - 1, Math.random() * 2 - 1]
                }
            };
            agent.updateState(state);

            // Set flocking behavior
            const flockingParams = {
                separation_radius: 5.0,
                alignment_radius: 15.0,
                cohesion_radius: 25.0,
                separation_weight: 1.5,
                alignment_weight: 1.0,
                cohesion_weight: 1.0,
                max_speed: 2.0
            };
            const behavior = GaussTwin.behaviorNew("flocking", flockingParams);
            agent.setBehavior(behavior);

            this.agents.push(agent);
            this.model.addAgent(agent);
        }
    }

    async runStep(): Promise<void> {
        // Configure simulation parameters
        const config: SimulationConfig = {
            timeStep: 0.1,
            maxAgents: this.agents.length,
            spaceBounds: [100, 100]
        };
        await this.model.run(1, config);

        // Get metrics
        const metrics = this.model.getMetrics();
        console.log(
            `Agents: ${metrics.agentCount()}, ` +
            `Density: ${metrics.averageDensity().toFixed(2)}`
        );
    }

    private drawAgents(): void {
        this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
        this.ctx.fillStyle = "rgba(0, 0, 255, 0.6)";

        for (const agent of this.agents) {
            const pos = agent.getPosition();
            const x = (pos[0] / 100) * this.canvas.width;
            const y = (pos[1] / 100) * this.canvas.height;
            
            this.ctx.beginPath();
            this.ctx.arc(x, y, 5, 0, Math.PI * 2);
            this.ctx.fill();
        }
    }

    async animate(): Promise<void> {
        const frame = async () => {
            await this.runStep();
            this.drawAgents();
            requestAnimationFrame(() => frame());
        };

        await frame();
    }
}

// Initialize and run simulation
const simulation = new FlockingSimulation(50);
simulation.animate().catch(console.error); 