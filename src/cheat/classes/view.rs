use mint::{Vector2, Vector3};
use crate::ui::main::WINDOW_INFO;

pub struct View {
    pub matrix: [[f32; 4]; 4]
}

impl View {
    pub fn world_to_screen(&self, pos: Vector3<f32>, to_pos: &mut Vector2<f32>) -> bool {
        let window_info = WINDOW_INFO.lock().unwrap();
        let view = self.matrix[3][0] * pos.x + self.matrix[3][2] * pos.z + self.matrix[3][3]; 
        
        // [TODO] Figure out why this returns false even if entity on screen at certain positions.
        // if view <= 0.01 {
        //     return false;
        // }

        if let Some(((_, _), (x, y))) = *window_info {
            let sight_x = x as f32 / 2.0;
            let sight_y = y as f32 / 2.0;

            to_pos.x = sight_x + (self.matrix[0][0] * pos.x + self.matrix[0][1] * pos.y + self.matrix[0][2] * pos.z + self.matrix[0][3]) / view * sight_x;
            to_pos.y = sight_y - (self.matrix[1][0] * pos.x + self.matrix[1][1] * pos.y + self.matrix[1][2] * pos.z + self.matrix[1][3]) / view * sight_y;

            return true;
        } else {
            return false;
        }
    }
}