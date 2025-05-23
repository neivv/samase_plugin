extern crate samase_plugin;

use std::ffi::c_void;
use samase_plugin::commands;

#[test]
fn cmds() {
    commands::set_default_command_lengths(&vec![1, 2, 3]);
    commands::add_ingame_hook(1, hook1);
    commands::add_ingame_hook(2, hook3);
    commands::add_ingame_hook(1, hook2);
    let globals = commands::IngameHookGlobals {
        is_replay: 0,
        unique_command_user: 0,
        command_user: 0,
        add_to_replay_data: add_to_replay_data,
    };
    unsafe {
        let data = vec![1u8, 2];
        let ptr = data.as_ptr() as *const c_void;
        commands::ingame_hook(ptr, data.len() as u32, 0, &globals, orig);
        let data = vec![2u8, 5, 0];
        let ptr = data.as_ptr() as *const c_void;
        commands::ingame_hook(ptr, data.len() as u32, 0, &globals, orig);
    }
}

unsafe extern "C" fn add_to_replay_data(_: *const u8, _: usize) {
}

unsafe extern "C" fn orig(
    data: *const c_void,
    len: u32,
    replay: u32,
) {
    let data = data as *const u8;
    assert_eq!(len, 2);
    assert_eq!(replay, 0);
    assert_eq!(*data.offset(1), 5);
}

unsafe extern "C" fn hook1(
    data: *const u8,
    len: u32,
    _: u32,
    _: u32,
    orig: unsafe extern "C" fn(*const u8, u32),
) {
    assert_eq!(len, 2);
    let data = vec![1, *data.offset(1) + 1];
    orig(data.as_ptr(), 2);
}

unsafe extern "C" fn hook2(
    data: *const u8,
    len: u32,
    _: u32,
    _: u32,
    orig: unsafe extern "C" fn(*const u8, u32),
) {
    assert_eq!(len, 2);
    let data = vec![1, *data.offset(1) + 2];
    orig(data.as_ptr(), 2);
}

unsafe extern "C" fn hook3(
    data: *const u8,
    len: u32,
    _: u32,
    _: u32,
    orig: unsafe extern "C" fn(*const u8, u32),
) {
    assert_eq!(len, 3);
    let data = vec![1, *data.offset(1) - 3];
    orig(data.as_ptr(), 2);
}
