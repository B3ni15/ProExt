use std::{sync::{Arc, Mutex}, time::{Instant, Duration}, mem::size_of};
use lazy_static::lazy_static;
use mint::{Vector2, Vector3};
use rand::{Rng, thread_rng};
use crate::{utils::{config::Config, mouse::move_mouse, process_manager::rpm_auto}, ui::functions::hotkey_index_to_io, cheat::classes::entity::CUtlVector};

lazy_static! {
    pub static ref RCS_TOGGLED: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    pub static ref TOGGLE_CHANGED: Arc<Mutex<Instant>> = Arc::new(Mutex::new(Instant::now()));
    pub static ref LAST_PUNCH: Arc<Mutex<Vector2<f32>>> = Arc::new(Mutex::new(Vector2 { x: 0.0, y: 0.0 }));
}

pub fn get_rcs_toggled(config: Config) -> bool {
    match hotkey_index_to_io(config.rcs.key) {
        Ok(rcs_button) => {
            if config.rcs.mode == 0 {
                return rcs_button.is_pressed();
            } else {
                let rcs_toggled = *RCS_TOGGLED.lock().unwrap();
                let toggle_changed = *TOGGLE_CHANGED.lock().unwrap();

                if rcs_button.is_pressed() && toggle_changed.elapsed() > Duration::from_millis(250) {
                    *RCS_TOGGLED.lock().unwrap() = !rcs_toggled;
                    *TOGGLE_CHANGED.lock().unwrap() = Instant::now();

                    return !rcs_toggled;
                } else {
                    return rcs_toggled;
                }
            }
        },
        Err(rcs_key) => {
            if config.rcs.mode == 0 {
                return rcs_key.is_pressed();
            } else {
                let rcs_toggled = *RCS_TOGGLED.lock().unwrap();
                let toggle_changed = *TOGGLE_CHANGED.lock().unwrap();

                if rcs_key.is_pressed() && toggle_changed.elapsed() > Duration::from_millis(250) {
                    *RCS_TOGGLED.lock().unwrap() = !rcs_toggled;
                    *TOGGLE_CHANGED.lock().unwrap() = Instant::now();
                    
                    return !rcs_toggled;
                } else {
                    return rcs_toggled;
                }
            }
        }
    }
}

pub fn get_rcs_punch(config: Config, shots_fired: u32, aim_punch_cache: CUtlVector) -> Option<(i32, i32)> {
    let yaw_offset = if config.rcs.yaw_offset == 0.0 { 0.0 } else { (thread_rng().gen_range(-config.rcs.yaw_offset .. config.rcs.yaw_offset) * 1000.0).trunc() / 1000.0 };
    let yaw = (config.rcs.yaw + yaw_offset).min(2.0).max(0.0);

    let pitch_offset = if config.rcs.pitch_offset == 0.0 { 0.0 } else { (thread_rng().gen_range(-config.rcs.pitch_offset .. config.rcs.pitch_offset) * 1000.0).trunc() / 1000.0 };
    let pitch = (config.rcs.pitch + pitch_offset).min(2.0).max(0.0);
    
    let mut current_punch = Vector2 { x: 0.0, y: 0.0 };

    if aim_punch_cache.count <= 0 || aim_punch_cache.count > 0xFFFF {
        return None;
    }

    if !rpm_auto(aim_punch_cache.data + (aim_punch_cache.count - 1) * size_of::<Vector3<f32>>() as u64, &mut current_punch) {
        return None;
    }
    
    let mut last_punch = LAST_PUNCH.lock().unwrap();
    let mut new_punch = Vector2 { x: 0.0, y: 0.0 };

    if current_punch.x == 0.0 && current_punch.y == 0.0 {
        return None;
    }

    if shots_fired > config.rcs.start_bullet {
        new_punch.x = ((current_punch.y - last_punch.y) * 2.0 / (pitch * config.rcs.sensitivity)) * 50.0;
        new_punch.y = (-(current_punch.x - last_punch.x) * 2.0 / (yaw * config.rcs.sensitivity)) * 50.0;
    }

    *last_punch = current_punch;
    return Some((new_punch.x as i32, new_punch.y as i32));
}

pub fn run_rcs(config: Config, shots_fired: u32, aim_punch_cache: CUtlVector) {
    if let Some((x, y)) = get_rcs_punch(config, shots_fired, aim_punch_cache) {
        move_mouse(x, y, false);
    }
}