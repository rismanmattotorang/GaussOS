import {
    TSAgent,
    TSModel,
    TSSpace,
    TSGrid,
    AgentState,
    SimulationConfig,
    Deno as GaussTwin
} from "../mod.ts";

class SpatialQueryDemo {
    private space: TSSpace;
    private grid: TSGrid;
    private model: TSModel;
    private agents: TSAgent[];
    private canvas: HTMLCanvasElement;
    private ctx: CanvasRenderingContext2D;

    constructor(numAgents = 100, spaceSize = 100) {
        // Initialize GaussTwin components
        this.space = GaussTwin.spaceNew([spaceSize, spaceSize]);
        this.grid = GaussTwin.gridNew(10, 10);
        this.model = GaussTwin.modelNew();
        this.agents = [];

        // Create canvas
        this.canvas = document.createElement("canvas");
        this.canvas.width = 1200;
        this.canvas.height = 600;
        document.body.appendChild(this.canvas);
        this.ctx = this.canvas.getContext("2d")!;

        // Create agents with random positions
        for (let i = 0; i < numAgents; i++) {
            const agent = GaussTwin.agentNew(`agent_${i}`);
            const state: AgentState = {
                id: agent.id,
                position: [Math.random() * spaceSize, Math.random() * spaceSize],
                data: { type: "point" }
            };
            agent.updateState(state);
            this.agents.push(agent);
            this.model.addAgent(agent);
        }
    }

    private drawAgents(ctx: CanvasRenderingContext2D, agents: TSAgent[], 
                      color = "blue", size = 5, alpha = 0.6): void {
        ctx.fillStyle = `rgba(${color === "blue" ? "0,0,255" : 
                               color === "red" ? "255,0,0" : 
                               "0,255,0"},${alpha})`;
        
        for (const agent of agents) {
            const pos = agent.getPosition();
            const x = (pos[0] / 100) * (this.canvas.width / 2 - 40);
            const y = (pos[1] / 100) * (this.canvas.height - 40) + 20;
            
            ctx.beginPath();
            ctx.arc(x, y, size, 0, Math.PI * 2);
            ctx.fill();
        }
    }

    private drawQueryRegion(ctx: CanvasRenderingContext2D, 
                          minBound: number[], maxBound: number[]): void {
        const x1 = (minBound[0] / 100) * (this.canvas.width / 2 - 40);
        const y1 = (minBound[1] / 100) * (this.canvas.height - 40) + 20;
        const width = ((maxBound[0] - minBound[0]) / 100) * (this.canvas.width / 2 - 40);
        const height = ((maxBound[1] - minBound[1]) / 100) * (this.canvas.height - 40);

        ctx.strokeStyle = "rgba(0,0,0,0.5)";
        ctx.setLineDash([5, 5]);
        ctx.strokeRect(x1, y1, width, height);
        ctx.setLineDash([]);
    }

    private drawLine(ctx: CanvasRenderingContext2D, 
                    start: number[], end: number[]): void {
        const x1 = (start[0] / 100) * (this.canvas.width / 2 - 40) + this.canvas.width / 2;
        const y1 = (start[1] / 100) * (this.canvas.height - 40) + 20;
        const x2 = (end[0] / 100) * (this.canvas.width / 2 - 40) + this.canvas.width / 2;
        const y2 = (end[1] / 100) * (this.canvas.height - 40) + 20;

        ctx.strokeStyle = "rgba(0,0,0,0.3)";
        ctx.beginPath();
        ctx.moveTo(x1, y1);
        ctx.lineTo(x2, y2);
        ctx.stroke();
    }

    private drawGridOccupancy(): void {
        const cellWidth = (this.canvas.width / 2 - 40) / 10;
        const cellHeight = (this.canvas.height - 40) / 10;
        const maxOccupancy = Math.max(...Array.from({ length: 10 }, (_, i) =>
            Math.max(...Array.from({ length: 10 }, (_, j) =>
                this.grid.getCellOccupancy(i, j)))));

        for (let i = 0; i < 10; i++) {
            for (let j = 0; j < 10; j++) {
                const occupancy = this.grid.getCellOccupancy(i, j);
                const intensity = occupancy / maxOccupancy;
                
                this.ctx.fillStyle = `rgba(255,0,0,${intensity * 0.7})`;
                this.ctx.fillRect(
                    i * cellWidth + this.canvas.width / 2,
                    j * cellHeight + 20,
                    cellWidth,
                    cellHeight
                );
                
                this.ctx.strokeStyle = "rgba(0,0,0,0.2)";
                this.ctx.strokeRect(
                    i * cellWidth + this.canvas.width / 2,
                    j * cellHeight + 20,
                    cellWidth,
                    cellHeight
                );
            }
        }
    }

    visualize(): void {
        // Clear canvas
        this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);

        // Draw titles
        this.ctx.fillStyle = "black";
        this.ctx.font = "16px Arial";
        this.ctx.fillText("Spatial Queries", 20, 20);
        this.ctx.fillText("Grid Occupancy", this.canvas.width / 2 + 20, 20);

        // Draw all agents (left side)
        this.drawAgents(this.ctx, this.agents);

        // Define and visualize query region
        const minBound = [20, 20];
        const maxBound = [60, 60];
        this.drawQueryRegion(this.ctx, minBound, maxBound);

        // Query and draw agents in region
        const regionAgents = this.space.queryRegion(minBound, maxBound);
        this.drawAgents(this.ctx, regionAgents, "red", 7);

        // Define query point and find nearest neighbors
        const queryPoint = [70, 70];
        const k = 5;
        const neighbors = this.space.getNearestNeighbors(queryPoint, k);

        // Draw query point
        this.ctx.fillStyle = "rgba(0,255,0,1)";
        const qx = (queryPoint[0] / 100) * (this.canvas.width / 2 - 40);
        const qy = (queryPoint[1] / 100) * (this.canvas.height - 40) + 20;
        this.ctx.beginPath();
        this.ctx.arc(qx, qy, 8, 0, Math.PI * 2);
        this.ctx.fill();

        // Draw nearest neighbors and connection lines
        for (const neighbor of neighbors) {
            const pos = neighbor.getPosition();
            this.drawLine(this.ctx, queryPoint, Array.from(pos));
        }
        this.drawAgents(this.ctx, neighbors, "red", 7);

        // Draw grid occupancy (right side)
        this.drawGridOccupancy();
    }
}

// Initialize and run demo
const demo = new SpatialQueryDemo(100);
demo.visualize(); 