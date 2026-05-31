use crate::{AgentId, Space};
use kiss3d::window::Window;
use kiss3d::light::Light;
use kiss3d::nalgebra::{Point3, Translation3};
use plotters::prelude::*;
use std::collections::HashMap;
use std::path::Path;

/// Color scheme for visualization
#[derive(Debug, Clone, Copy)]
pub struct ColorScheme {
    pub background: (u8, u8, u8),
    pub agent: (u8, u8, u8),
    pub highlight: (u8, u8, u8),
    pub grid: (u8, u8, u8),
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            background: (255, 255, 255),
            agent: (0, 0, 255),
            highlight: (255, 0, 0),
            grid: (200, 200, 200),
        }
    }
}

/// Visualization options
#[derive(Debug, Clone)]
pub struct VisualizationOptions {
    pub width: u32,
    pub height: u32,
    pub colors: ColorScheme,
    pub show_grid: bool,
    pub show_labels: bool,
    pub point_size: f32,
}

impl Default for VisualizationOptions {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            colors: ColorScheme::default(),
            show_grid: true,
            show_labels: false,
            point_size: 5.0,
        }
    }
}

/// Trait for visualizable spaces
pub trait Visualizable: Space {
    /// Get the bounds of the space
    fn bounds(&self) -> (Point3<f64>, Point3<f64>);
    
    /// Get agent colors (optional)
    fn agent_colors(&self) -> Option<HashMap<AgentId, (u8, u8, u8)>> {
        None
    }
}

/// Save a 2D visualization of the space to an image file
pub fn save_2d_visualization<S: Visualizable>(
    space: &S,
    path: impl AsRef<Path>,
    options: &VisualizationOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(
        path.as_ref(),
        (options.width, options.height),
    ).into_drawing_area();
    
    root.fill(&RGBColor(
        options.colors.background.0,
        options.colors.background.1,
        options.colors.background.2,
    ))?;
    
    let bounds = space.bounds();
    let mut chart = ChartBuilder::on(&root)
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(
            bounds.0.x..bounds.1.x,
            bounds.0.y..bounds.1.y,
        )?;
    
    if options.show_grid {
        chart.configure_mesh()
            .draw()?;
    }
    
    let agent_colors = space.agent_colors().unwrap_or_default();
    
    // Draw agents
    for id in (0..space.agent_count()).map(|_| AgentId::new()) {
        if let Some(pos) = space.get_position(id) {
            let color = agent_colors.get(&id).copied().unwrap_or(options.colors.agent);
            chart.draw_series(std::iter::once(Circle::new(
                (pos.x, pos.y),
                options.point_size as i32,
                &RGBColor(color.0, color.1, color.2),
            )))?;
            
            if options.show_labels {
                chart.draw_series(std::iter::once(Text::new(
                    format!("{}", id.raw()),
                    (pos.x, pos.y),
                    ("sans-serif", 15.0).into_font(),
                )))?;
            }
        }
    }
    
    root.present()?;
    Ok(())
}

/// Interactive 3D visualization window
pub struct Visualization3D {
    window: Window,
    options: VisualizationOptions,
}

impl Visualization3D {
    /// Create a new 3D visualization window
    pub fn new(options: VisualizationOptions) -> Self {
        let mut window = Window::new("Space Visualization");
        window.set_background_color(
            options.colors.background.0 as f32 / 255.0,
            options.colors.background.1 as f32 / 255.0,
            options.colors.background.2 as f32 / 255.0,
        );
        window.set_light(Light::StickToCamera);
        
        Self { window, options }
    }
    
    /// Update the visualization with current space state
    pub fn update<S: Visualizable>(&mut self, space: &S) {
        let agent_colors = space.agent_colors().unwrap_or_default();
        
        for id in (0..space.agent_count()).map(|_| AgentId::new()) {
            if let Some(pos) = space.get_position(id) {
                let mut sphere = self.window.add_sphere(self.options.point_size);
                sphere.set_local_translation(Translation3::new(
                    pos.x as f32,
                    pos.y as f32,
                    pos.z as f32,
                ));
                
                let color = agent_colors.get(&id).copied().unwrap_or(self.options.colors.agent);
                sphere.set_color(
                    color.0 as f32 / 255.0,
                    color.1 as f32 / 255.0,
                    color.2 as f32 / 255.0,
                );
            }
        }
    }
    
    /// Render one frame
    pub fn render(&mut self) -> bool {
        self.window.render()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    
    struct TestSpace {
        agents: HashMap<AgentId, Point3<f64>>,
    }
    
    impl TestSpace {
        fn new() -> Self {
            Self {
                agents: HashMap::new(),
            }
        }
    }
    
    impl Space for TestSpace {
        type Position = Point3<f64>;
        
        fn add_agent(&self, id: AgentId, pos: Self::Position) {
            self.agents.insert(id, pos);
        }
        
        fn remove_agent(&self, id: AgentId) {
            self.agents.remove(&id);
        }
        
        fn move_agent(&self, id: AgentId, new_pos: Self::Position) {
            self.agents.insert(id, new_pos);
        }
        
        fn get_position(&self, id: AgentId) -> Option<Self::Position> {
            self.agents.get(&id).cloned()
        }
        
        fn query_radius(&self, _center: Self::Position, _radius: f64) -> Vec<AgentId> {
            vec![]
        }
        
        fn query_k_nearest(&self, _center: Self::Position, _k: usize) -> Vec<AgentId> {
            vec![]
        }
        
        fn agent_count(&self) -> usize {
            self.agents.len()
        }
        
        fn clear(&self) {
            self.agents.clear();
        }
    }
    
    impl Visualizable for TestSpace {
        fn bounds(&self) -> (Point3<f64>, Point3<f64>) {
            (
                Point3::new(-10.0, -10.0, -10.0),
                Point3::new(10.0, 10.0, 10.0),
            )
        }
    }
    
    #[test]
    fn test_2d_visualization() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.png");
        
        let space = TestSpace::new();
        let options = VisualizationOptions::default();
        
        save_2d_visualization(&space, &path, &options).unwrap();
        assert!(path.exists());
    }
} 