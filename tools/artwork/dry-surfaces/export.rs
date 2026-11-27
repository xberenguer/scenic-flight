#![allow(dead_code)]

mod math;
mod ground_cover;

mod world {
    pub use scenic_flight::world::Surface;
}

mod render {
    pub fn floor_i32(value: f32) -> i32 {
        value.floor() as i32
    }
}

fn main() -> std::io::Result<()> {
    ground_cover::export_dry_surfaces()
}
