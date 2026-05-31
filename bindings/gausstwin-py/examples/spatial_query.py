import asyncio
import numpy as np
import matplotlib.pyplot as plt
from gausstwin import PyAgent, PyModel, PySpace, PyGrid, PySimulationConfig

class SpatialQueryDemo:
    def __init__(self, num_agents=100, space_size=100.0):
        self.space = PySpace([space_size, space_size])
        self.grid = PyGrid((10, 10))
        self.model = PyModel()
        
        # Create agents with random positions
        self.agents = []
        for i in range(num_agents):
            agent = PyAgent(f"agent_{i}")
            state = {
                "position": [np.random.random() * space_size, np.random.random() * space_size],
                "data": {"type": "point"}
            }
            asyncio.run(agent.update_state(state))
            self.agents.append(agent)
            self.model.add_agent(agent)

    def visualize_spatial_queries(self):
        # Get all agent positions
        positions = np.array([agent.get_position() for agent in self.agents])
        
        # Define a query region
        min_bound = [20.0, 20.0]
        max_bound = [60.0, 60.0]
        
        # Query agents in region
        region_agents = self.space.query_region(min_bound, max_bound)
        region_positions = np.array([agent.get_position() for agent in region_agents])
        
        # Define a point for nearest neighbor search
        query_point = np.array([70.0, 70.0])
        k = 5  # Number of nearest neighbors
        
        # Find k nearest neighbors
        neighbors = self.space.get_nearest_neighbors(query_point.tolist(), k)
        neighbor_positions = np.array([agent.get_position() for agent in neighbors])
        
        # Visualization
        plt.figure(figsize=(12, 6))
        
        # Plot region query results
        plt.subplot(121)
        plt.scatter(positions[:, 0], positions[:, 1], c='blue', alpha=0.6, label='All Agents')
        plt.scatter(region_positions[:, 0], region_positions[:, 1], 
                   c='red', s=100, alpha=0.6, label='In Region')
        plt.plot([min_bound[0], max_bound[0], max_bound[0], min_bound[0], min_bound[0]],
                 [min_bound[1], min_bound[1], max_bound[1], max_bound[1], min_bound[1]],
                 'k--', label='Query Region')
        plt.title('Region Query')
        plt.legend()
        plt.grid(True)
        
        # Plot nearest neighbor results
        plt.subplot(122)
        plt.scatter(positions[:, 0], positions[:, 1], c='blue', alpha=0.6, label='All Agents')
        plt.scatter(query_point[0], query_point[1], c='green', s=200, 
                   marker='*', label='Query Point')
        plt.scatter(neighbor_positions[:, 0], neighbor_positions[:, 1],
                   c='red', s=100, alpha=0.6, label=f'{k} Nearest')
        
        # Draw lines to nearest neighbors
        for pos in neighbor_positions:
            plt.plot([query_point[0], pos[0]], [query_point[1], pos[1]], 'k--', alpha=0.3)
            
        plt.title('K-Nearest Neighbors')
        plt.legend()
        plt.grid(True)
        
        plt.tight_layout()
        plt.show()

    def analyze_grid_occupancy(self):
        # Analyze grid cell occupancy
        occupancy_matrix = np.zeros((10, 10))
        for i in range(10):
            for j in range(10):
                occupancy_matrix[i, j] = self.grid.get_cell_occupancy(i, j)
        
        # Visualize grid occupancy
        plt.figure(figsize=(8, 6))
        plt.imshow(occupancy_matrix, cmap='YlOrRd', interpolation='nearest')
        plt.colorbar(label='Number of Agents')
        plt.title('Grid Cell Occupancy')
        plt.xlabel('Grid X')
        plt.ylabel('Grid Y')
        plt.show()

        # Print statistics
        print(f"Maximum cell occupancy: {np.max(occupancy_matrix)}")
        print(f"Average cell occupancy: {np.mean(occupancy_matrix):.2f}")
        print(f"Empty cells: {np.sum(occupancy_matrix == 0)}")

if __name__ == "__main__":
    demo = SpatialQueryDemo(num_agents=100)
    demo.visualize_spatial_queries()
    demo.analyze_grid_occupancy() 