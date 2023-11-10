use std::mem;
use std::ffi::{OsString, c_void};
use std::os::windows::prelude::OsStringExt;
use std::sync::{Arc, Mutex};
use colored::Colorize;
use lazy_static::lazy_static;

use windows::Win32::Foundation::{HANDLE, BOOL, CloseHandle, STILL_ACTIVE};
use windows::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};
use windows::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, CREATE_TOOLHELP_SNAPSHOT_FLAGS, PROCESSENTRY32W, Process32NextW, MODULEENTRY32W, TH32CS_SNAPMODULE, Module32NextW};
use windows::Win32::System::Threading::{OpenProcess, GetExitCodeProcess, PROCESS_ALL_ACCESS, PROCESS_CREATE_THREAD};
use windows::Win32::System::Memory::{VirtualQueryEx, MEMORY_BASIC_INFORMATION};

use crate::utils::config::{DEBUG, PROCESS_EXECUTABLE};

lazy_static! {
    pub static ref PROCESS_MANAGER: Arc<Mutex<ProcessManager>> = Arc::new(Mutex::new(ProcessManager {
        attached: false,
        h_process: HANDLE::default(),
        process_id: 0,
        module_address: 0
    }));
}

#[derive(Debug, PartialEq)]
pub enum AttachStatus {
    Success,
    FailedProcessId,
    FailedHProcess,
    FailedModule
}

pub struct ProcessManager {
    pub attached: bool,
    pub h_process: HANDLE,
    pub process_id: u32,
    pub module_address: u64
}

pub fn attach_process_manager() -> AttachStatus {
    let process_name = &*PROCESS_EXECUTABLE;
    let process_manager = PROCESS_MANAGER.clone();
    let mut process_manager = process_manager.lock().unwrap();

    match get_process_id(process_name) {
        0 => { return AttachStatus::FailedProcessId; },
        process_id => { (*process_manager).process_id = process_id; }
    };
    
    if *DEBUG { println!("{} ProcessID: {}", "[ INFO ]".blue().bold(), format!("{}", (*process_manager).process_id).bold()); }

    match unsafe { OpenProcess(PROCESS_ALL_ACCESS | PROCESS_CREATE_THREAD, BOOL::from(true), (*process_manager).process_id) } {
        Ok(handle) => { (*process_manager).h_process = handle; },
        Err(_) => { return AttachStatus::FailedHProcess; }
    }

    if *DEBUG { println!("{} HProcess: {}", "[ INFO ]".blue().bold(), format!("{:?}", (*process_manager).h_process.0).bold()); }

    drop(process_manager);
    let module_address = get_process_module_handle(process_name);
    let mut process_manager = PROCESS_MANAGER.lock().unwrap();

    match module_address {
        0 => { return AttachStatus::FailedModule; },
        module_address => { (*process_manager).module_address = module_address; }
    };

    if *DEBUG { println!("{} ModuleAddress: {}", "[ INFO ]".blue().bold(), format!("{:X}", (*process_manager).module_address).bold()); }

    (*process_manager).attached = true;
    return AttachStatus::Success;
}

pub fn detach_process_manager(process_manager: &mut ProcessManager) {
    if !HANDLE::is_invalid(&process_manager.h_process) { 
        unsafe {
            let _ = CloseHandle((*process_manager).h_process);
        }
    }

    process_manager.h_process = HANDLE::default();
    process_manager.process_id = 0;
    process_manager.module_address = 0;
    process_manager.attached = false;
}

pub fn is_active() -> bool {
    let process_manager = PROCESS_MANAGER.clone();
    let process_manager = process_manager.lock().unwrap();

    if !(*process_manager).attached {
        return false;
    }

    let mut exit_code: u32 = 0;
    
    unsafe { let _ = GetExitCodeProcess((*process_manager).h_process, &mut exit_code); };
    return exit_code == STILL_ACTIVE.0 as u32;
}

pub fn read_memory<ReadType: ?Sized>(address: u64, value: &mut ReadType, size: usize) -> bool {
    let process_manager = PROCESS_MANAGER.clone();
    let process_manager = process_manager.lock().unwrap();

    unsafe {
        match ReadProcessMemory((*process_manager).h_process, address as *mut c_void, value as *mut ReadType as *mut c_void, size, None) {
            Ok(_) => { return true; },
            Err(_) => { return false; }
        }
    }
}

pub fn read_memory_auto<ReadType>(address: u64, value: &mut ReadType) -> bool {
    return read_memory(address, value, mem::size_of::<ReadType>());
}

pub fn write_memory<WriteType: ?Sized>(address: u64, value: &mut WriteType, size: usize) -> bool {
    let process_manager = PROCESS_MANAGER.clone();
    let process_manager = process_manager.lock().unwrap();

    unsafe {
        match WriteProcessMemory((*process_manager).h_process, address as *mut c_void, value as *mut WriteType as *mut c_void, size, None) {
            Ok(_) => { return true; },
            Err(_) => { return false; }
        };
    }
}

pub fn write_memory_auto<WriteType>(address: u64, value: &mut WriteType) -> bool {
    return write_memory(address, value, mem::size_of::<WriteType>());
}

pub fn get_address_with_offset<ReadType>(address: u64, offset: u32, value: &mut ReadType) -> bool {
    return address != 0 && read_memory_auto(address + offset as u64, value);
}

pub fn search_memory(signature: &str, start_address: u64, end_address: u64, search_num: i32) -> Vec<u64> {
    let process_manager = PROCESS_MANAGER.clone();

    fn get_signature_array(signature: &str) -> Vec<u16> {
        let mut signature_array: Vec<u16> = Vec::new();
        let mut sig = signature.to_string();
        let _ = sig.retain(|c| c != ' ');
        let size = sig.len();

        if size % 2 != 0 {
            return signature_array;
        }

        for i in (0..size).step_by(2) {
            let byte_str = &sig[i..(i + 2)];
            let byte: u16 = if byte_str == "??" {
                256
            } else {
                u16::from_str_radix(byte_str, 16).unwrap()
            };

            signature_array.push(byte);
        }

        return signature_array;
    }

    fn get_next_array(signature_array: &[u16]) -> Vec<i16> {
        let mut next_array: Vec<i16> = vec![-1; 260];

        for (i, &byte) in signature_array.iter().enumerate() {
            next_array[byte as usize] = i as i16;
        }

        return next_array;
    }

    fn search_memory_block(memory_buffer: &mut [u8], next_array: &[i16], signature_array: &[u16], start_address: u64, size: u32, result_array: &mut Vec<u64>) {
        if !read_memory(start_address, memory_buffer, size as usize) {
            return;
        }

        let signature_length = signature_array.len();

        let mut i = 0;
        let mut j;
        let mut k;

        while i < size {
            j = i;
            k = 0;

            while k < signature_length && j < size && (signature_array[k] == memory_buffer[j as usize] as u16 || signature_array[k] == 256) {
                k += 1;
                j += 1;
            }

            if k == signature_length {
                result_array.push(start_address + i as u64);
            }

            if (i + signature_length as u32) >= size {
                return;
            }

            let num = next_array[memory_buffer[i as usize + signature_length as usize] as usize];

            if num == -1 {
                i += signature_length as u32 - next_array[256] as u32;
            } else {
                i += signature_length as u32 - num as u32;
            }
        }
    }

    const BLOCK_MAX_SIZE: u32 = 409600;

    let signature_array = get_signature_array(signature);
    let next_array = get_next_array(&signature_array);
    
    let mut result_array: Vec<u64> = Vec::new();
    let mut memory_buffer: Vec<u8> = vec![0; BLOCK_MAX_SIZE as usize];

    let mut start_address = start_address;
    let end_address = end_address;

    unsafe {
        let mut mbi: MEMORY_BASIC_INFORMATION = MEMORY_BASIC_INFORMATION::default();

        while VirtualQueryEx((*process_manager.lock().unwrap()).h_process, Some(start_address as *mut c_void), &mut mbi, mem::size_of::<MEMORY_BASIC_INFORMATION>()) != 0 {
            let mut count = 0;
            let mut block_size = mbi.RegionSize;

            while block_size >= BLOCK_MAX_SIZE as usize {
                if result_array.len() >= search_num as usize {
                    break;
                }

                search_memory_block(&mut memory_buffer, &next_array, &signature_array, start_address + (BLOCK_MAX_SIZE as u64 * count), BLOCK_MAX_SIZE, &mut result_array);

                block_size -= BLOCK_MAX_SIZE as usize;
                count += 1;
            }

            if result_array.len() >= search_num as usize {
                break;
            }

            search_memory_block(&mut memory_buffer, &next_array, &signature_array, start_address + (BLOCK_MAX_SIZE as u64 * count), block_size as u32, &mut result_array);
            start_address += mbi.RegionSize as u64;

            if result_array.len() >= search_num as usize || (end_address != 0 && start_address > end_address) {
                break;
            }
        }
    }

    return result_array;
}

pub fn trace_address(base_address: u64, offsets: &[u32]) -> u64 {
    let mut address: u64 = 0;

    if offsets.is_empty() {
        return base_address;
    }

    if !read_memory_auto(base_address, &mut address) {
        return 0;
    }

    for i in 0..offsets.len() - 1 {
        if !read_memory_auto(address + offsets[i] as u64, &mut address) {
            return 0;
        }
    }

    return if address == 0 {
        0
    } else {
        address + offsets[offsets.len() - 1] as u64
    };
}

pub fn get_process_id(process_name: &str) -> u32 {
    let mut process_info: PROCESSENTRY32W = PROCESSENTRY32W::default();
    let h_snapshot = match unsafe { CreateToolhelp32Snapshot(CREATE_TOOLHELP_SNAPSHOT_FLAGS(15), 0) } {
        Ok(snapshot) => snapshot,
        Err(_) => { return 0; }
    };

    process_info.dwSize = mem::size_of::<PROCESSENTRY32W>() as u32;

    unsafe {
        while Process32NextW(h_snapshot, &mut process_info).is_ok() {
            let current_name = OsString::from_wide(&process_info.szExeFile[..]).into_string().unwrap().replace("\u{0}", "");

            if current_name == process_name {
                let _ = CloseHandle(h_snapshot);
                return process_info.th32ProcessID;
            }
        }

        let _ = CloseHandle(h_snapshot);
        return 0;
    }
}

pub fn get_process_module_handle(module_name: &str) -> u64 {
    let process_manager = PROCESS_MANAGER.clone();
    let process_manager = process_manager.lock().unwrap();

    let mut module_info: MODULEENTRY32W = MODULEENTRY32W::default();
    let h_snapshot = match unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPMODULE, (*process_manager).process_id) } {
        Ok(snapshot) => snapshot,
        Err(_) => { return 0; }
    };

    module_info.dwSize = mem::size_of::<MODULEENTRY32W>() as u32;

    unsafe {
        while Module32NextW(h_snapshot, &mut module_info).is_ok() {
            let current_name = OsString::from_wide(&module_info.szModule[..]).into_string().unwrap().replace("\u{0}", "");

            if current_name == module_name {
                let _ = CloseHandle(h_snapshot);
                return module_info.hModule.0 as u64;
            }
        }

        let _ = CloseHandle(h_snapshot);
        return 0;
    }
}

impl Drop for ProcessManager {
    fn drop(&mut self) {
        detach_process_manager(self);
    }
}