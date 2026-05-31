import asyncio
import numpy as np
from gausstwin import PyAgent, PyModel, PySpace, PyGrid, PyGNNModel

async def run_simulation():
    # Create a 2D space
    space = PySpace([100.0, 100.0])
    print(f"Space dimensions: {space.dimensions()}")

    # Create a grid
    grid = PyGrid((10, 10))

    # Create some agents
    agents = [PyAgent(f"agent_{i}") for i in range(5)]
    
    # Create a model
    model = PyModel()

    # Update agent states
    for i, agent in enumerate(agents):
        state = {
            "position": [np.random.random() * 100, np.random.random() * 100],
            "velocity": [0.0, 0.0],
            "data": {"type": "walker"}
        }
        await agent.update_state(state)
        print(f"Agent {agent.id} initialized")

    # Create and train a GNN model
    gnn = PyGNNModel()
    training_data = {
        "nodes": [[1.0, 0.0], [0.0, 1.0]],
        "edges": [[0, 1]],
        "labels": [1, 0]
    }
    await gnn.train(training_data)
    print("GNN model trained")

    # Run simulation
    print("Running simulation...")
    await model.run(100)
    print("Simulation completed")

if __name__ == "__main__":
    asyncio.run(run_simulation()) 