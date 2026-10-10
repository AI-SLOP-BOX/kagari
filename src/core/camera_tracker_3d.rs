//! 3D Camera Tracker & Structure from Motion (SfM) Engine (AE 3D Camera Tracker Parity).
//!
//! Solves 3D camera trajectory (Position, Orientation, Focal Length) and
//! reconstructs a sparse 3D point cloud from 2D tracked feature point trajectories.

#![allow(dead_code)]

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeatureTrack2D {
    pub id: u32,
    pub observations: Vec<(u32, [f32; 2])>, // (frame, [x, y])
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SolvedCameraFrame3D {
    pub frame: u32,
    pub position: [f32; 3],
    pub rotation_matrix: [[f32; 3]; 3],
    pub focal_length_px: f32,
    pub reprojection_error_px: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Point3D {
    pub id: u32,
    pub position: [f32; 3],
    pub error: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CameraTrackerSolution3D {
    pub camera_frames: Vec<SolvedCameraFrame3D>,
    pub point_cloud: Vec<Point3D>,
    pub average_error_px: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CameraTrackerError {
    NotImplemented,
}

impl std::fmt::Display for CameraTrackerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotImplemented => f.write_str("3D camera solving is not implemented"),
        }
    }
}

impl std::error::Error for CameraTrackerError {}

/// Returns an explicit error until a geometric SfM solver replaces the placeholder.
pub fn solve_camera_motion_3d(
    _tracks: &[FeatureTrack2D],
    _image_width: u32,
    _image_height: u32,
    _focal_length_estimate: Option<f32>,
) -> Result<CameraTrackerSolution3D, CameraTrackerError> {
    Err(CameraTrackerError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_tracker_does_not_report_placeholder_as_solution() {
        let mut tracks = Vec::new();
        for i in 0..10 {
            tracks.push(FeatureTrack2D {
                id: i,
                observations: vec![
                    (0, [100.0 + i as f32 * 50.0, 200.0]),
                    (1, [105.0 + i as f32 * 50.0, 202.0]),
                    (2, [110.0 + i as f32 * 50.0, 204.0]),
                ],
            });
        }

        assert!(matches!(
            solve_camera_motion_3d(&tracks, 1920, 1080, None),
            Err(CameraTrackerError::NotImplemented)
        ));
    }
}
