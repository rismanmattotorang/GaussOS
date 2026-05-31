declare module "gausstwin" {
    export interface AgentState {
        id: string;
        position: number[];
        data: any;
    }

    export interface SimulationConfig {
        timeStep: number;
        maxAgents: number;
        spaceBounds: number[];
    }

    export interface PerformanceStats {
        fps: number;
        memoryUsage: number;
        agentUpdateTime: number;
    }

    export class TSAgent {
        constructor(id: string);
        readonly id: string;
        updateState(state: AgentState): Promise<void>;
        getPosition(): Float64Array;
        setBehavior(behavior: TSBehavior): void;
        getNeighbors(radius: number): TSAgent[];
    }

    export class TSModel {
        constructor();
        run(steps: number, config?: SimulationConfig): Promise<void>;
        addAgent(agent: TSAgent): void;
        getMetrics(): TSMetrics;
    }

    export class TSSpace {
        constructor(dimensions: number[]);
        dimensions(): number[];
        queryRegion(minBound: number[], maxBound: number[]): TSAgent[];
        getNearestNeighbors(point: number[], k: number): TSAgent[];
    }

    export class TSGrid {
        constructor(width: number, height: number);
        getCellOccupancy(x: number, y: number): number;
        getAgentsInCell(x: number, y: number): TSAgent[];
    }

    export class TSGNNModel {
        constructor();
        train(data: any): Promise<void>;
        predict(input: any): Promise<any>;
    }

    export class TSMetrics {
        agentCount(): number;
        averageDensity(): number;
        getPerformanceStats(): PerformanceStats;
    }

    export interface BehaviorParams {
        [key: string]: any;
    }

    export class TSBehavior {
        constructor(behaviorType: "random_walk" | "flocking", params?: BehaviorParams);
    }

    // Deno-specific types
    export namespace Deno {
        export function agentNew(id: string): TSAgent;
        export function modelNew(): TSModel;
        export function spaceNew(dimensions: number[]): TSSpace;
        export function gridNew(width: number, height: number): TSGrid;
        export function gnnNew(): TSGNNModel;
        export function behaviorNew(behaviorType: string, params?: BehaviorParams): TSBehavior;
    }
} 