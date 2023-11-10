use colored::Colorize;
use lazy_static::lazy_static;
use std::sync::{Mutex, Arc};

use crate::utils::{process_manager::{search_memory, read_memory_auto, get_process_module_handle}, config::DEBUG};

lazy_static! {
    pub static ref ENTITY_LIST: Arc<Mutex<u32>> = Arc::new(Mutex::new(0x0));
    pub static ref MATRIX: Arc<Mutex<u32>> = Arc::new(Mutex::new(0x0));
    pub static ref VIEW_ANGLE: Arc<Mutex<u32>> = Arc::new(Mutex::new(0x0));
    pub static ref LOCAL_PLAYER_CONTROLLER: Arc<Mutex<u32>> = Arc::new(Mutex::new(0x0));
    pub static ref LOCAL_PLAYER_PAWN: Arc<Mutex<u32>> = Arc::new(Mutex::new(0x0));
    pub static ref FORCE_JUMP: Arc<Mutex<u32>> = Arc::new(Mutex::new(0x0));
    pub static ref GLOBAL_VARS: Arc<Mutex<u32>> = Arc::new(Mutex::new(0x0));

    pub static ref ENTITY_OFFSETS: Arc<Mutex<EntityOffsets>> = Arc::new(Mutex::new(EntityOffsets {
        health: 0x32C,
        team_id: 0x3BF,
        is_alive: 0x7C4,
        player_pawn: 0x7BC,
        is_player_name: 0x610
    }));
    
    pub static ref PAWN_OFFSETS: Arc<Mutex<PawnOffsets>> = Arc::new(Mutex::new(PawnOffsets {
        pos: 0x1224,
        max_health: 0x328,
        current_health: 0x32C,
        game_scene_node: 0x310,
        bone_array: 0x1E0,
        ang_eye_angles: 0x1510,
        vec_last_clip_camera_pos: 0x128C,
        p_clipping_weapon: 0x12A8,
        i_shots_fired: 0x1418,
        fl_flash_duration: 0x1468,
        aim_punch_angle: 0x1714,
        aim_punch_cache: 0x1738,
        i_id_ent_index: 0x153C,
        i_team_num: 0x3BF,
        camera_services: 0x10E0,
        i_fov_start: 0x214,
        f_flags: 0x3C8,
        b_spotted_by_mask: 0x1630 + 0xC
    }));
    
    pub static ref GLOBAL_VAR_OFFSETS: Arc<Mutex<GlobalVarOffsets>> = Arc::new(Mutex::new(GlobalVarOffsets {
        real_time: 0x00,
        frame_count: 0x04,
        max_clients: 0x10,
        interval_per_tick: 0x14,
        current_time: 0x2C,
        current_time2: 0x30,
        tick_count: 0x40,
        interval_per_tick2: 0x44,
        current_netchan: 0x0048,
        current_map: 0x0180,
        current_map_name: 0x0188
    }));
    
    pub static ref SIGNATURES: Arc<Mutex<Signatures>> = Arc::new(Mutex::new(Signatures {
        global_vars: "48 89 0D ?? ?? ?? ?? 48 89 41".to_string(),
        view_matrix: "48 8D 0D ?? ?? ?? ?? 48 C1 E0 06".to_string(),
        view_angles: "48 8B 0D ?? ?? ?? ?? 48 8B 01 48 FF 60 30".to_string(),
        entity_list: "48 8B 0D ?? ?? ?? ?? 48 89 7C 24 ?? 8B FA C1".to_string(),
        local_player_controller: "48 8B 05 ?? ?? ?? ?? 48 85 C0 74 4F".to_string(),
        force_jump: "48 8B 05 ?? ?? ?? ?? 48 8D 1D ?? ?? ?? ?? 48 89 45".to_string(),
        local_player_pawn: "48 8D 05 ?? ?? ?? ?? C3 CC CC CC CC CC CC CC CC 48 83 EC ?? 8B 0D".to_string()
    }));
}

pub struct EntityOffsets {
    pub health: u32,
    pub team_id: u32,
    pub is_alive: u32,
    pub player_pawn: u32,
    pub is_player_name: u32,
}

pub struct PawnOffsets {
    pub pos: u32,
    pub max_health: u32,
    pub current_health: u32,
    pub game_scene_node: u32,
    pub bone_array: u32,
    pub ang_eye_angles: u32,
    pub vec_last_clip_camera_pos: u32,
    pub p_clipping_weapon: u32,
    pub i_shots_fired: u32,
    pub fl_flash_duration: u32,
    pub aim_punch_angle: u32,
    pub aim_punch_cache: u32,
    pub i_id_ent_index: u32,
    pub i_team_num: u32,
    pub camera_services: u32,
    pub i_fov_start: u32,
    pub f_flags: u32,
    pub b_spotted_by_mask: u32,
}

pub struct GlobalVarOffsets {
    pub real_time: u32,
    pub frame_count: u32,
    pub max_clients: u32,
    pub interval_per_tick: u32,
    pub current_time: u32,
    pub current_time2: u32,
    pub tick_count: u32,
    pub interval_per_tick2: u32,
    pub current_netchan: u32,
    pub current_map: u32,
    pub current_map_name: u32,
}

pub struct Signatures {
    pub global_vars: String,
    pub view_matrix: String,
    pub view_angles: String,
    pub entity_list: String,
    pub local_player_controller: String,
    pub force_jump: String,
    pub local_player_pawn: String
}

pub fn search_offsets(signature: String, module_address: u64) -> Option<u64> {
    let address_list: Vec<u64> = search_memory(&signature, module_address, module_address + 0x4000000, 1);
    let mut offsets: u32 = 0;

    if address_list.is_empty() {
        return None;
    }

    if !read_memory_auto(address_list[0] + 3, &mut offsets) {
        return None;
    }

    let return_item = address_list[0] + offsets as u64 + 7;

    if return_item != 0 {
        return Some(return_item);
    }

    return None;
}

pub fn update_offsets() -> Option<String> {
    let signatures = SIGNATURES.lock().unwrap();
    let mut entity_list = ENTITY_LIST.lock().unwrap();
    let mut local_player_controller = LOCAL_PLAYER_CONTROLLER.lock().unwrap();
    let mut matrix = MATRIX.lock().unwrap();
    let mut global_vars = GLOBAL_VARS.lock().unwrap();
    let mut view_angle = VIEW_ANGLE.lock().unwrap();
    let mut local_player_pawn = LOCAL_PLAYER_PAWN.lock().unwrap();
    let mut force_jump = FORCE_JUMP.lock().unwrap();

    let client_dll = get_process_module_handle("client.dll") as u64;
    if client_dll == 0 { return Some("ClientDLL".to_string()); }

    if *DEBUG { println!("{} ClientDLL Handle: {}", "[ INFO ]".blue().bold(), format!("{:X}", client_dll as u32).bold()); }

    match search_offsets(signatures.entity_list.clone(), client_dll) {
        Some(address) => *entity_list = (address - client_dll) as u32,
        _ => { return Some("EntityList".to_string()) }
    };

    if *DEBUG { println!("{} EntityList Offset: {}", "[ INFO ]".blue().bold(), format!("{:X}", *entity_list).bold()); }

    match search_offsets(signatures.local_player_controller.clone(), client_dll) {
        Some(address) => *local_player_controller = (address - client_dll) as u32,
        _ => { return Some("LocalPlayerController".to_string()) }
    };

    if *DEBUG { println!("{} LocalPlayerController Offset: {}", "[ INFO ]".blue().bold(), format!("{:X}", *local_player_controller).bold()); }

    match search_offsets(signatures.view_matrix.clone(), client_dll) {
        Some(address) => *matrix = (address - client_dll) as u32,
        _ => { return Some("ViewMatrix".to_string()) }
    };

    if *DEBUG { println!("{} ViewMatrix Offset: {}", "[ INFO ]".blue().bold(), format!("{:X}", *matrix).bold()); }

    match search_offsets(signatures.global_vars.clone(), client_dll) {
        Some(address) => *global_vars = (address - client_dll) as u32,
        _ => { return Some("GlobalVars".to_string()) }
    };

    if *DEBUG { println!("{} GlobalVars Offset: {}", "[ INFO ]".blue().bold(), format!("{:X}", *global_vars).bold()); }

    match search_offsets(signatures.view_angles.clone(), client_dll) {
        Some(mut address) => {
            if !read_memory_auto(address, &mut address) { return Some("ViewAnglesMemory".to_string()); };
            *view_angle = (address + 0x4518 - client_dll) as u32;
        },
        _ => { return Some("ViewAngles".to_string()) }
    };

    if *DEBUG { println!("{} ViewAngles Offset: {}", "[ INFO ]".blue().bold(), format!("{:X}", *view_angle).bold()); }

    match search_offsets(signatures.local_player_pawn.clone(), client_dll) {
        Some(address) => *local_player_pawn = (address + 0x138 - client_dll) as u32,
        _ => { return Some("LocalPlayerPawn".to_string()) }
    };

    if *DEBUG { println!("{} LocalPlayerPawn Offset: {}", "[ INFO ]".blue().bold(), format!("{:X}", *local_player_pawn).bold()); }

    match search_offsets(signatures.force_jump.clone(), client_dll) {
        Some(address) => *force_jump = (address + 0x30 - client_dll) as u32,
        _ => { return Some("ForceJump".to_string()) }
    };

    if *DEBUG { println!("{} ForceJump Offset: {}", "[ INFO ]".blue().bold(), format!("{:X}", *force_jump).bold()); }

    return None;
}