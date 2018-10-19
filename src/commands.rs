use std::cell::RefCell;
use std::mem;
use std::slice;

use libc::c_void;
use thread_local::CachedThreadLocal;
use parking_lot::{Mutex, RwLock};

// data, len, game player, unique player, orig
pub type IngameCommandHook =
    unsafe extern fn(*const u8, u32, u32, u32, unsafe extern fn(*const u8, u32));
pub type CommandLength = unsafe extern fn(*const u8, u32) -> u32;

lazy_static! {
    static ref INGAME_HOOKS: RwLock<Vec<(u8, IngameCommandHook)>> = RwLock::new(Vec::new());
    static ref COMMAND_LENGTHS: RwLock<Vec<(u8, CommandLength)>> = RwLock::new(Vec::new());
    static ref DEFAULT_COMMAND_LENGTHS: Mutex<Vec<u32>> = Mutex::new(Vec::new());
    static ref HOOK_CALL_STATE: CachedThreadLocal<RefCell<Vec<HookCallState<'static>>>> =
        CachedThreadLocal::new();
}

#[derive(Clone, Copy)]
struct HookCallState<'a> {
    orig_hooks: &'a [(u8, IngameCommandHook)],
    remaining_hooks: &'a [(u8, IngameCommandHook)],
    globals: &'a IngameHookGlobals,
    orig: unsafe extern fn(*const c_void, u32, u32),
    id: u8,
    replayed_command: u32,
}

pub struct IngameHookGlobals {
    pub is_replay: u32,
    pub unique_command_user: u32,
    pub command_user: u32,
}

pub unsafe extern fn ingame_hook(
    data: *const c_void,
    len: u32,
    replayed_command: u32,
    globals: &IngameHookGlobals,
    orig: unsafe extern fn(*const c_void, u32, u32),
) {
    let data = slice::from_raw_parts(data as *const u8, len as usize);
    let hooks = INGAME_HOOKS.read();
    let cmd_lengths = COMMAND_LENGTHS.read();
    let mut pos = data;
    while pos.len() > 0 {
        let skip = globals.is_replay != 0 && replayed_command == 0 && !is_replay_command(pos[0]);
        let len = command_len(&cmd_lengths, pos);
        if len > pos.len() {
            error!("Command {:x} too short for its length {:x}", pos[0], len);
            return;
        }
        if !skip {
            handle_ingame_hooks(&hooks, &pos[..len], replayed_command, globals, orig);
        }
        pos = &pos[len..];
    }
}

fn is_replay_command(byte: u8) -> bool {
    match byte {
        0x5 | 0x8 | 0x10 | 0x11 | 0x55 | 0x56 | 0x5d => true,
        _ => false,
    }
}

unsafe fn handle_ingame_hooks(
    hooks: &[(u8, IngameCommandHook)],
    cmd: &[u8],
    replayed_command: u32,
    globals: &IngameHookGlobals,
    orig: unsafe extern fn(*const c_void, u32, u32),
) {
    unsafe extern fn call_orig(data: *const u8, len: u32) {
        if len == 0 {
            return;
        }
        let id = *data;
        let state: HookCallState;
        let mut hook = None;
        {
            let states = HOOK_CALL_STATE.get().unwrap();
            let mut state_guard = states.borrow_mut();
            let state_ref: &mut HookCallState = state_guard.last_mut().unwrap();
            state = *state_ref;
            if state.id == id {
                for i in 0..state.remaining_hooks.len() {
                    let (other_id, h) = state.remaining_hooks[i];
                    if id == other_id {
                        state_ref.remaining_hooks = &state.remaining_hooks[i + 1..];
                        hook = Some(h);
                        break;
                    }
                }
            }
        }
        // If the command id gets changed, reset the hook position
        if state.id != id {
            let cmd = slice::from_raw_parts(data, len as usize);
            let HookCallState {
                orig_hooks,
                replayed_command,
                globals,
                orig,
                ..
            } = state;
            handle_ingame_hooks(orig_hooks, cmd, replayed_command, globals, orig);
        } else {
            if let Some(hook) = hook {
                let player = state.globals.command_user;
                let uniq = state.globals.unique_command_user;
                hook(data, len, player, uniq, call_orig);
            } else {
                trace!("Calling orig for {:x}:{:x}", id, len);
                (state.orig)(data as *const c_void, len, state.replayed_command);
            }
        }
    }

    let id = cmd[0];
    for i in 0..hooks.len() {
        let (other_id, hook) = hooks[i];
        if id == other_id {
            let state = HookCallState {
                orig_hooks: hooks,
                remaining_hooks: &hooks[i + 1..],
                globals,
                replayed_command,
                orig,
                id,
            };
            let state: HookCallState<'static> = mem::transmute(state);
            let player = globals.command_user;
            let uniq = globals.unique_command_user;
            let states = HOOK_CALL_STATE.get_or(|| RefCell::new(Vec::new()));
            states.borrow_mut().push(state);
            hook(cmd.as_ptr(), cmd.len() as u32, player, uniq, call_orig);
            states.borrow_mut().pop().unwrap();
            return;
        }
    }
    // TODO 0
    orig(cmd.as_ptr() as *const c_void, cmd.len() as u32, replayed_command);
}

unsafe fn command_len(overrides: &[(u8, CommandLength)], cmd: &[u8]) -> usize {
    let id = cmd[0];
    for &(other_id, len) in overrides {
        if other_id == id {
            return len(cmd.as_ptr(), cmd.len() as u32) as usize;
        }
    }
    match id {
        0x6 | 0x7 => {
            let mut pos = 5;
            while pos < cmd.len() {
                if cmd[pos] == 0 {
                    return pos + 1;
                }
                pos += 1;
            }
            !0
        }
        0x9 | 0xa | 0xb => {
            cmd.get(1).cloned().unwrap_or(12) as usize * 2 + 2
        }
        0x63 | 0x64 | 0x65 => {
            cmd.get(1).cloned().unwrap_or(12) as usize * 4 + 2
        }
        _ => {
            DEFAULT_COMMAND_LENGTHS.lock().get(id as usize).cloned()
                .unwrap_or(!0) as usize
        }
    }
}

pub fn set_default_command_lengths(mut lengths: Vec<u32>) {
    if let Some(sync) = lengths.get_mut(0x37) {
        // This isn't set correctly for whatever reason, and it works out for bw since
        // it's not replay skipped
        *sync = 7;
    }
    *DEFAULT_COMMAND_LENGTHS.lock() = lengths;
}

pub fn add_ingame_hook(cmd: u8, hook: IngameCommandHook) {
    INGAME_HOOKS.write().push((cmd, hook));
}

pub fn add_length_override(cmd: u8, fun: CommandLength) {
    COMMAND_LENGTHS.write().push((cmd, fun));
}
