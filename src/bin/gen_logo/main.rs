mod geometry;
mod reveal;
mod still;

use kagari_vfx::core::project_migration::save_project_atomic;
use kagari_vfx::core::timeline::Project;
use reveal::build_reveal;
use still::build_still;

fn main() {
    let project = Project {
        compositions: vec![build_still(), build_reveal()],
        ..Default::default()
    };
    save_project_atomic(&project, "logo_project.json").unwrap();
    println!("Wrote logo_project.json");
}
