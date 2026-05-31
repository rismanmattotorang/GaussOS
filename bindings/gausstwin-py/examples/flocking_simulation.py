import asyncio
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.animation import FuncAnimation
from gausstwin import PyAgent, PyModel, PySpace, PyGrid, PyBehavior, PySimulationConfig

class FlockingSimulation:
    def __init__(self, num_agents=50, space_size=100.0):
        self.space = PySpace([space_size, space_size])
        self.grid = PyGrid((10, 10))
        self.model = PyModel()
        
        # Create agents with flocking behavior
        self.agents = []
        for i in range(num_agents):
            agent = PyAgent(f"boid_{i}")
            # Initialize with random position and velocity
            state = {
                "position": [np.random.random() * space_size, np.random.random() * space_size],
                "velocity": [np.random.random() * 2 - 1, np.random.random() * 2 - 1],
                "data": {"type": "boid"}
            }
            asyncio.run(agent.update_state(state))
            
            # Set flocking behavior
            flocking_params = {
                "separation_radius": 5.0,
                "alignment_radius": 15.0,
                "cohesion_radius": 25.0,
                "separation_weight": 1.5,
                "alignment_weight": 1.0,
                "cohesion_weight": 1.0,
                "max_speed": 2.0
            }
            behavior = PyBehavior("flocking", flocking_params)
            agent.set_behavior(behavior)
            
            self.agents.append(agent)
            self.model.add_agent(agent)

    async def run_step(self):
        # Configure simulation parameters
        config = PySimulationConfig(
            time_step=0.1,
            max_agents=len(self.agents),
            space_bounds=[100.0, 100.0]
        )
        await self.model.run(1, config)
        
        # Get metrics
        metrics = self.model.get_metrics()
        print(f"Agents: {metrics.agent_count()}, Density: {metrics.average_density():.2f}")
        
        # Return current positions for visualization
        positions = np.array([agent.get_position() for agent in self.agents])
        return positions

    def visualize(self):
        fig, ax = plt.subplots(figsize=(8, 8))
        scatter = ax.scatter([], [], c='b', alpha=0.6)
        ax.set_xlim(0, 100)
        ax.set_ylim(0, 100)
        ax.set_title("Flocking Simulation")

        async def update(frame):
            positions = await self.run_step()
            scatter.set_offsets(positions)
            return scatter,

        anim = FuncAnimation(fig, update, frames=200, interval=50, blit=True)
        plt.show()

if __name__ == "__main__":
    simulation = FlockingSimulation(num_agents=50)
    simulation.visualize() 