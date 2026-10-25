pub use crate::core::flow_graph_model::{FlowGraphModel, FlowGraphNode};
use eframe::egui;

impl FlowGraphModel {
    pub fn positions(
        &self,
        origin: egui::Pos2,
        column_spacing: f32,
        row_spacing: f32,
    ) -> Vec<egui::Pos2> {
        self.positions_f64(
            origin.x as f64,
            origin.y as f64,
            column_spacing as f64,
            row_spacing as f64,
        )
        .into_iter()
        .map(|(x, y)| egui::pos2(x as f32, y as f32))
        .collect()
    }
}
