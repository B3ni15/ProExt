use std::{time::Instant, thread::{self, sleep}, sync::{Arc, Mutex}, collections::HashMap};
use colored::Colorize;
use imgui::{Ui, ColorEditFlags, ImColor32};
use lazy_static::lazy_static;

use glium::{glutin::{event_loop::ControlFlow, event::{Event, WindowEvent, DeviceEvent, ElementState}, dpi::{PhysicalSize, PhysicalPosition}}, Surface};
use imgui_glium_renderer::Renderer;
use mint::{Vector4, Vector2, Vector3};

use crate::{cheat::{features::{radar::render_radar, visuals::{render_fov_circle, render_fov, render_crosshair, render_head_shoot_line}, aimbot::{run_aimbot, aimbot_check}, anti_flashbang::run_anti_flashbang, bunnyhop::run_bunny_hop}, classes::entity::Flags}, ui::windows::hide_window_from_capture};
use crate::{ui::menu::render_menu, utils::{config::{DEBUG, PACKAGE_NAME, PACKAGE_VERSION, PACKAGE_AUTHORS, PROCESS_TITLE, PROCESS_CLASS, TOGGLE_KEY, THREAD_DELAYS, CONFIG}, process_manager::{read_memory, read_memory_auto}}, cheat::classes::{game::{GAME, update_entity_list_entry}, entity::Entity}};
use crate::ui::windows::{create_window, find_window, focus_window, init_imgui, get_window_info, is_window_focused};

lazy_static! {
    pub static ref WINDOW_INFO: Arc<Mutex<Option<((i32, i32), (i32, i32))>>> = Arc::new(Mutex::new(None));
    pub static ref WINDOW_LAST_MOVED: Arc<Mutex<Instant>> = Arc::new(Mutex::new(Instant::now()));
    pub static ref WINDOW_FOCUSED: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    pub static ref UI_FUNCTIONS: Arc<Mutex<HashMap<String, Box<dyn Fn(&mut Ui) + Send>>>> = Arc::new(Mutex::new(HashMap::new()));

    pub static ref AIMBOT_TOGGLED: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    pub static ref BUNNYHOP_TOGGLED: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    pub static ref TOGGLED: Arc<Mutex<bool>> = Arc::new(Mutex::new(true));
    pub static ref EXIT: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

pub fn color_edit_u32_tuple(ui: &Ui, label: &str, color_tuple: &mut (u32, u32, u32, u32)) {
    let mut color_vector = Vector4 {
        x: color_tuple.0 as f32 / 255.0,
        y: color_tuple.1 as f32 / 255.0,
        z: color_tuple.2 as f32 / 255.0,
        w: color_tuple.3 as f32 / 255.0,
    };

    ui.color_edit4_config(label, &mut color_vector)
        .flags(ColorEditFlags::NO_INPUTS)
        .build();

    color_tuple.0 = (color_vector.x * 255.0) as u32;
    color_tuple.1 = (color_vector.y * 255.0) as u32;
    color_tuple.2 = (color_vector.z * 255.0) as u32;
    color_tuple.3 = (color_vector.w * 255.0) as u32;
}

pub fn color_u32_to_f32(color: (u32, u32, u32, u32)) -> (f32, f32, f32, f32) {
    return (color.0 as f32 / 255.0, color.1 as f32 / 255.0, color.2 as f32 / 255.0, color.3 as f32 / 255.0);
}

pub fn color_with_alpha_mask((red, green, blue, _): (u32, u32, u32, u32), alpha_mask: u32) -> (f32, f32, f32) {
    let red = (red & alpha_mask) >> 24;
    let green = (green & alpha_mask) >> 24;
    let blue = (blue & alpha_mask) >> 24;

    return (red as f32 / 255.0, green as f32 / 255.0, blue as f32 / 255.0);
}

pub fn distance_to_vec(pos1: Vector2<f32>, pos2: Vector2<f32>) -> f32 {
    let x_diff = pos2.x - pos1.x;
    let y_diff = pos2.y - pos1.y;
    let distance = (x_diff.powi(2) + y_diff.powi(2)).sqrt();

    return distance;
}

pub fn rectangle_filled(ui: &mut Ui, pos: Vector2<f32>, size: Vector2<f32>, color: ImColor32) {
    let a = pos;
    let b = Vector2 { x: pos.x + size.x, y: pos.y + size.y };

    ui.get_background_draw_list().add_polyline(vec![a, Vector2 { x: b.x, y: a.y }, b, Vector2 { x: a.x, y: b.y }], color).filled(true).build();
}

pub fn hotkey_index_to_io(hotkey_index: usize) -> Result<rdev::Button, rdev::Key> {
    if hotkey_index == 1 {
        return Ok(rdev::Button::Left);
    }
    else if hotkey_index == 2 {
        return Ok(rdev::Button::Middle);
    }
    else if hotkey_index == 3 {
        return Ok(rdev::Button::Right);
    }
    else if hotkey_index == 4 {
        return Err(rdev::Key::ShiftLeft);
    }
    else if hotkey_index == 5 {
        return Err(rdev::Key::ControlLeft);
    }
    else {
        return Err(rdev::Key::Alt);
    }
}

pub fn init_gui() {
    let title = &format!("{} v{} - {}", (*PACKAGE_NAME).to_uppercase(), (*PACKAGE_VERSION), (*PACKAGE_AUTHORS));
    let window_title = &*PROCESS_TITLE;
    let window_class = &*PROCESS_CLASS;
    let toggle_key = &*TOGGLE_KEY;

    let window_hwnd = match find_window(window_title, Some(window_class)) {
        Some(hwnd) => hwnd,
        None => {
            println!("{} Failed to find {} window", "[ FAIL ]".bold().red(), window_title.bold());
            return;
        }
    };

    if *DEBUG { println!("{} {} Window HWND: {}", "[ INFO ]".blue().bold(), window_title.bold(), format!("{:?}", window_hwnd).bold()); }

    let (event_loop, display) = create_window(title, window_hwnd);
    let (mut winit_platform, mut imgui_context) = init_imgui(&display);

    let self_hwnd = match find_window(title, None) {
        Some(hwnd) => hwnd,
        None => {
            println!("{} Failed to find {} window", "[ FAIL ]".bold().red(), title.bold());
            return;
        }
    };

    if *DEBUG { println!("{} Self Window HWND: {}", "[ INFO ]".blue().bold(), format!("{:?}", self_hwnd).bold()); }

    focus_window(self_hwnd);

    let mut renderer = Renderer::init(&mut imgui_context, &display).unwrap();
    let mut last_frame = Instant::now();

    println!("{} Rendering GUI (toggle: {})", "[ OKAY ]".bold().green(), format!("{:?}", toggle_key).bold());

    let toggled = TOGGLED.clone();
    let exit = EXIT.clone();
    let window_info = WINDOW_INFO.clone();
    let window_last_moved = WINDOW_LAST_MOVED.clone();
    let window_focused = WINDOW_FOCUSED.clone();
    let aimbot_toggled = AIMBOT_TOGGLED.clone();
    let bunnyhop_toggled = BUNNYHOP_TOGGLED.clone();
    
    let key_event_thread = thread::spawn(move || {
        let _ = rdev::listen(move | event | {
            match event.event_type {
                rdev::EventType::KeyRelease(key) => {
                    let window_focused = *window_focused.lock().unwrap();

                    if format!("{:?}", key) == format!("{:?}", toggle_key) && window_focused {
                        let toggled_value = *toggled.lock().unwrap();
                        *toggled.lock().unwrap() = !toggled_value;
                        
                        if toggled_value {
                            focus_window(window_hwnd);
                        } else {
                            focus_window(self_hwnd);
                        }
                    } else {
                        match hotkey_index_to_io((*CONFIG.lock().unwrap()).aim_bot_hot_key) {
                            Ok(_) => {},
                            Err(aimbot_key) => {
                                if (*aimbot_toggled.lock().unwrap()) && key == aimbot_key && is_window_focused(window_hwnd) {
                                    (*aimbot_toggled.lock().unwrap()) = false;
                                }
                            }
                        }

                        if (*bunnyhop_toggled.lock().unwrap()) && key == rdev::Key::Space && is_window_focused(window_hwnd) {
                            (*bunnyhop_toggled.lock().unwrap()) = false;
                        }
                    }
                },
                rdev::EventType::KeyPress(key) => {
                    match hotkey_index_to_io((*CONFIG.lock().unwrap()).aim_bot_hot_key) {
                        Ok(_) => {},
                        Err(aimbot_key) => {
                            if !(*aimbot_toggled.lock().unwrap()) && key == aimbot_key && is_window_focused(window_hwnd) {
                                (*aimbot_toggled.lock().unwrap()) = true;
                            }
                        }
                    }

                    if !(*bunnyhop_toggled.lock().unwrap()) && key == rdev::Key::Space && is_window_focused(window_hwnd) {
                        (*bunnyhop_toggled.lock().unwrap()) = true;
                    }
                },
                rdev::EventType::ButtonPress(button) => {
                    match hotkey_index_to_io((*CONFIG.lock().unwrap()).aim_bot_hot_key) {
                        Err(_) => {},
                        Ok(aimbot_button) => {
                            if !(*aimbot_toggled.lock().unwrap()) && button == aimbot_button && is_window_focused(window_hwnd) {
                                (*aimbot_toggled.lock().unwrap()) = true;
                            }
                        }
                    }
                },
                rdev::EventType::ButtonRelease(button) => {
                    match hotkey_index_to_io((*CONFIG.lock().unwrap()).aim_bot_hot_key) {
                        Err(_) => {},
                        Ok(aimbot_button) => {
                            if (*aimbot_toggled.lock().unwrap()) && button == aimbot_button && is_window_focused(window_hwnd) {
                                (*aimbot_toggled.lock().unwrap()) = false;
                            }
                        }
                    }
                },
                _ => {}
            }
        });
    });

    if *DEBUG { println!("{} KeyEvents Thread ID: {}", "[ INFO ]".blue().bold(), format!("{:?}", key_event_thread.thread().id()).bold()); }

    let window_focused = WINDOW_FOCUSED.clone();
    let window_tasks_thread = thread::spawn(move || {
        let mut stored_window_info: ((i32, i32), (i32, i32)) = ((0, 0), (0, 0));

        loop {
            if let Some(((x, y), (width, height))) = get_window_info(window_hwnd) {
                let window_info_var = ((x + 1, y + 1), (width - 2, height - 2));

                if window_info_var != stored_window_info {
                    stored_window_info = window_info_var;
                    *window_info.lock().unwrap() = Some(window_info_var);
                    *window_last_moved.lock().unwrap() = Instant::now();
                }
            } else {
                *exit.lock().unwrap() = true;
            }

            *window_focused.lock().unwrap() = is_window_focused(window_hwnd) || is_window_focused(self_hwnd);
            sleep(THREAD_DELAYS.window_tasks);
        }
    });

    if *DEBUG { println!("{} WindowTasks Thread ID: {} (delay: {})", "[ INFO ]".blue().bold(), format!("{:?}", window_tasks_thread.thread().id()).bold(), format!("{:?}", THREAD_DELAYS.window_tasks).bold()); }

    let aimbot_toggled = AIMBOT_TOGGLED.clone();
    let bunnyhop_toggled = BUNNYHOP_TOGGLED.clone();
    let ui_functions = UI_FUNCTIONS.clone();
    let window_info = WINDOW_INFO.clone();
    let mut window_hidden_from_capture = false;
    let cheat_tasks_thread = thread::spawn(move || {
        loop {
            if (*aimbot_toggled.lock().unwrap()) && !is_window_focused(window_hwnd) {
                (*aimbot_toggled.lock().unwrap()) = false;
            }

            if (*bunnyhop_toggled.lock().unwrap()) && !is_window_focused(window_hwnd) {
                (*bunnyhop_toggled.lock().unwrap()) = false;
            }

            if !window_hidden_from_capture && (*CONFIG.lock().unwrap()).obs_bypass {
                hide_window_from_capture(self_hwnd, true);
                window_hidden_from_capture = true;
            } else if window_hidden_from_capture && !(*CONFIG.lock().unwrap()).obs_bypass {
                hide_window_from_capture(self_hwnd, false);
                window_hidden_from_capture = false;
            }

            let mut no_pawn = false;
            let matrix_address = GAME.lock().unwrap().address.matrix;
            let controller_address = GAME.lock().unwrap().address.local_controller;
            let pawn_address = GAME.lock().unwrap().address.local_pawn;
            let remove_ui_elements = || {
                (*ui_functions.lock().unwrap()).remove("fov_circle");
                (*ui_functions.lock().unwrap()).remove("fov");
                (*ui_functions.lock().unwrap()).remove("crosshair");
                (*ui_functions.lock().unwrap()).remove("head_shoot_line");
                (*ui_functions.lock().unwrap()).remove("radar");
            };

            if !read_memory(matrix_address, &mut (*GAME.lock().unwrap()).view.matrix, 64) {
                remove_ui_elements();
                continue;
            }

            update_entity_list_entry();

            let mut local_controller_address = 0;
            let mut local_pawn_address = 0;

            if !read_memory_auto(controller_address, &mut local_controller_address) {
                remove_ui_elements();
                continue;
            }

            if !read_memory_auto(pawn_address, &mut local_pawn_address) {
                remove_ui_elements();
                continue;
            }

            let mut local_entity = Entity::default();
            let mut local_player_controller_index = 1;

            if !local_entity.update_controller(local_controller_address) {
                remove_ui_elements();
                continue;
            }

            if !local_entity.update_pawn(local_pawn_address) {
                if !(*CONFIG.lock().unwrap()).show_when_spec {
                    remove_ui_elements();
                    continue;
                };

                no_pawn = true;
            }

            // Aimbot Data
            let mut max_aim_distance: f32 = 100000.0;
            let mut aim_pos = Vector3 { x: 0.0, y: 0.0, z: 0.0 };

            // Radar Data
            let mut radar_points: Vec<(Vector3<f32>, f32)> = Vec::new();

            // Entities
            for i in 0 .. 64 {
                let mut entity = Entity::default();
                let mut entity_address: u64 = 0;

                if !read_memory_auto((*GAME.lock().unwrap()).address.entity_list_entry + (i + 1) * 0x78, &mut entity_address) {
                    continue;
                }

                if entity_address == local_entity.controller.address {
                    local_player_controller_index = i;
                    continue;
                }

                if !entity.update_controller(entity_address) {
                    continue;
                }

                if !entity.update_pawn(entity.pawn.address) {
                    continue;
                }

                if (*CONFIG.lock().unwrap()).team_check && entity.controller.team_id == local_entity.controller.team_id {
                    continue;
                }

                if !entity.is_alive() {
                    continue;
                }

                // Radar Point
                if (*CONFIG.lock().unwrap()).show_radar {
                    radar_points.push((entity.pawn.pos, entity.pawn.view_angle.y));
                }

                if !entity.is_in_screen() {
                    continue;
                }

                // Aimbot Check
                if let Some(((_, _), (width, height))) = *window_info.lock().unwrap() {
                    if let Some(bone) = entity.get_bone() {
                        aimbot_check(bone, width, height, &mut aim_pos, &mut max_aim_distance, local_entity.pawn.b_spotted_by_mask, local_player_controller_index, i, *CONFIG.lock().unwrap());
                    }
                }
            }

            // FOV Circle
            if !no_pawn && (*CONFIG.lock().unwrap()).show_aim_fov_range {
                if let Some(((_, _), (width, height))) = *window_info.lock().unwrap() {
                    (*ui_functions.lock().unwrap()).insert("fov_circle".to_string(), Box::new(move |ui| {
                        render_fov_circle(ui, width, height, local_entity.pawn.fov, *CONFIG.lock().unwrap());
                    }));
                }
            } else {
                (*ui_functions.lock().unwrap()).remove("fov_circle");
            }

            // FOV
            if !no_pawn && (*CONFIG.lock().unwrap()).show_fov_line {
                if let Some(((_, _), (width, height))) = *window_info.lock().unwrap() {
                    (*ui_functions.lock().unwrap()).insert("fov".to_string(), Box::new(move |ui| {
                        render_fov(ui, width, height, local_entity.pawn.fov, *CONFIG.lock().unwrap());
                    }));
                }
            } else {
                (*ui_functions.lock().unwrap()).remove("fov");
            }

            // Crosshair
            if (*CONFIG.lock().unwrap()).show_crosshair {
                if let Some(((_, _), (width, height))) = *window_info.lock().unwrap() {
                    (*ui_functions.lock().unwrap()).insert("crosshair".to_string(), Box::new(move |ui| {
                        render_crosshair(ui, width, height, *CONFIG.lock().unwrap());
                    }));
                }
            } else {
                (*ui_functions.lock().unwrap()).remove("crosshair");
            }

            // Head Shoot Line
            if (*CONFIG.lock().unwrap()).show_head_shoot_line {
                if let Some(((_, _), (width, height))) = *window_info.lock().unwrap() {
                    (*ui_functions.lock().unwrap()).insert("head_shoot_line".to_string(), Box::new(move |ui| {
                        render_head_shoot_line(ui, width, height, local_entity.pawn.fov, local_entity.pawn.view_angle.x, *CONFIG.lock().unwrap());
                    }));
                };
            } else {
                (*ui_functions.lock().unwrap()).remove("head_shoot_line");
            }

            // Radar
            if !no_pawn && (*CONFIG.lock().unwrap()).show_radar {
                (*ui_functions.lock().unwrap()).insert("radar".to_string(), Box::new(move |ui| {
                    render_radar(ui, *CONFIG.lock().unwrap(), local_entity.pawn.pos, local_entity.pawn.view_angle.y, radar_points.clone());
                }));
            } else {
                (*ui_functions.lock().unwrap()).remove("radar");
            }

            // Anti Flashbang
            if !no_pawn && (*CONFIG.lock().unwrap()).anti_flashbang {
                run_anti_flashbang(local_entity.pawn.address);
            }

            // Bunnyhop
            if !no_pawn && (*CONFIG.lock().unwrap()).bunny_hop {
                run_bunny_hop(*bunnyhop_toggled.lock().unwrap(), local_entity.pawn.has_flag(Flags::InAir));
            }

            // Aimbot
            if !no_pawn && *aimbot_toggled.lock().unwrap() {
                run_aimbot(*CONFIG.lock().unwrap(), aim_pos, local_entity.pawn.camera_pos, local_entity.pawn.view_angle, local_entity.pawn.shots_fired, local_entity.pawn.aim_punch_cache);
            }

            sleep(THREAD_DELAYS.cheat_tasks);
        }
    });

    if *DEBUG { println!("{} CheatTasks Thread ID: {} (delay: {})", "[ INFO ]".blue().bold(), format!("{:?}", cheat_tasks_thread.thread().id()).bold(), format!("{:?}", THREAD_DELAYS.cheat_tasks).bold()); }

    let toggled = TOGGLED.clone();
    let exit = EXIT.clone();
    let window_info = WINDOW_INFO.clone();
    let window_last_moved = WINDOW_LAST_MOVED.clone();
    let ui_functions = UI_FUNCTIONS.clone();

    event_loop.run(move | event, _, control_flow | {
        let toggled_value = *toggled.lock().unwrap();
        let window_info_value = *window_info.lock().unwrap();

        let exit = *exit.lock().unwrap();
        let window_last_moved = *window_last_moved.lock().unwrap();
        let gl_window = display.gl_window();

        if window_last_moved.elapsed().as_millis() < 250 {
            gl_window.window().set_cursor_hittest(false).unwrap();
        } else {
            gl_window.window().set_cursor_hittest(toggled_value).unwrap();
        }

        if let Some(((x, y), (width, height))) = window_info_value {
            if window_last_moved.elapsed().as_millis() < 10 {
                gl_window.window().set_inner_size(PhysicalSize::new(width, height));
                gl_window.window().set_outer_position(PhysicalPosition::new(x, y));
            }
        }

        if exit {
            *control_flow = ControlFlow::Exit;
        }

        match event {
            Event::NewEvents(_) => {
                let now = Instant::now();
                imgui_context.io_mut().update_delta_time(now.duration_since(last_frame));
                last_frame = now;
            },
            Event::MainEventsCleared => {
                winit_platform.prepare_frame(imgui_context.io_mut(), gl_window.window()).unwrap();
                gl_window.window().request_redraw();
            },
            Event::RedrawRequested(_) => {
                let ui = imgui_context.frame();

                for (_, function) in (*ui_functions.lock().unwrap()).iter() {
                    function(ui);
                }

                render_menu(ui);

                let mut target = display.draw();
                target.clear_color_srgb(0.0, 0.0, 0.0, 0.0);
                winit_platform.prepare_render(ui, gl_window.window());

                let draw_data = imgui_context.render();
                renderer.render(&mut target, draw_data).unwrap();
                target.finish().unwrap();
            },
            Event::DeviceEvent {
                event: DeviceEvent::Key(key),
                ..
            } => {
                if let Some(keycode) = key.virtual_keycode {
                    if &keycode == toggle_key && key.state == ElementState::Released {
                        *toggled.lock().unwrap() = !toggled_value;

                        if toggled_value {
                            focus_window(window_hwnd);
                        } else {
                            focus_window(self_hwnd);
                        }
                    }
                }
            },
            Event::WindowEvent {
                event: WindowEvent::CloseRequested | WindowEvent::Destroyed,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            },
            event => {
                winit_platform.handle_event(imgui_context.io_mut(), gl_window.window(), &event);
            }
        };
    });
}