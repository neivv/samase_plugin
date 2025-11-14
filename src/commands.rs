use std::cell::RefCell;
use std::ffi::c_void;
use std::mem;
use std::slice;

use thread_local::ThreadLocal;
use once_cell::sync::{Lazy, OnceCell};
use parking_lot::{Mutex, MutexGuard, RwLock, const_mutex, const_rwlock};

pub use super::{IngameCommandHook, CommandLength};

static INGAME_HOOKS: RwLock<Vec<(u8, IngameCommandHook)>> = const_rwlock(Vec::new());
static COMMAND_LENGTHS: OnceCell<&CommandLengths> = OnceCell::new();
static COMMAND_LENGTHS_MUTABLE: Mutex<CommandLengths> = const_mutex(CommandLengths::new());

pub struct CommandLengths {
    /// Either < 0x400, or usize::MAX in which case it is taken as integer, otherwise
    /// a function pointer.
    data: [usize; 0x100],
}

impl CommandLengths {
    const fn new() -> CommandLengths {
        CommandLengths {
            data: [usize::MAX; 0x100],
        }
    }

    /// Returns value larger than cmd.len() on error (Usually usize::MAX)
    pub fn command_len(&self, cmd: &[u8]) -> usize {
        let id = cmd.get(0).copied().unwrap_or(0);
        let value = self.data[id as usize];
        if value < 0x400 {
            return value;
        } else {
            unsafe {
                let func: CommandLength = mem::transmute(value);
                // sign extend just to get u32::MAX to usize::MAX for bit more consistency.
                func(cmd.as_ptr(), cmd.len() as u32) as i32 as isize as usize
            }
        }
    }
}

unsafe extern "C" fn save_command_length(cmd: *const u8, len: u32) -> u32 {
    let len = len as usize;
    let mut pos = 5;
    while pos < len {
        if *cmd.add(pos) == 0 {
            return (pos + 1) as u32;
        }
        pos += 1;
    }
    u32::MAX
}

unsafe extern "C" fn select_command_legacy_length(cmd: *const u8, len: u32) -> u32 {
    if len < 2 {
        u32::MAX
    } else {
        (*cmd.add(1) as u32) * 2 + 2
    }
}

unsafe extern "C" fn select_command_extended_length(cmd: *const u8, len: u32) -> u32 {
    if len < 2 {
        u32::MAX
    } else {
        (*cmd.add(1) as u32) * 4 + 2
    }
}

/// Will permamently lock the mutex on first access, preventing further mutation.
pub fn get_command_lengths() -> &'static CommandLengths {
    COMMAND_LENGTHS.get_or_init(|| {
        MutexGuard::leak(COMMAND_LENGTHS_MUTABLE.lock())
    })
}

static HOOK_CALL_STATE: Lazy<ThreadLocal<RefCell<Vec<HookCallState<'static>>>>> =
    Lazy::new(|| ThreadLocal::new());

#[derive(Clone, Copy)]
struct HookCallState<'a> {
    orig_hooks: &'a [(u8, IngameCommandHook)],
    remaining_hooks: &'a [(u8, IngameCommandHook)],
    globals: &'a IngameHookGlobals,
    orig: unsafe extern "C" fn(*const c_void, u32, u32),
    id: u8,
    replayed_command: u32,
}

pub struct IngameHookGlobals {
    pub is_replay: u32,
    pub unique_command_user: u32,
    pub command_user: u32,
    pub add_to_replay_data: unsafe extern "C" fn(*const u8, usize),
}

pub unsafe extern "C" fn ingame_hook(
    data: *const c_void,
    len: u32,
    replayed_command: u32,
    globals: &IngameHookGlobals,
    orig: unsafe extern "C" fn(*const c_void, u32, u32),
) {
    let data = slice::from_raw_parts(data as *const u8, len as usize);
    let hooks = INGAME_HOOKS.read();
    let cmd_lengths = get_command_lengths();
    let mut pos = data;
    while pos.len() > 0 {
        let skip = globals.is_replay != 0 && replayed_command == 0 && !is_replay_command(pos[0]);
        let len = cmd_lengths.command_len(pos);
        if len > pos.len() {
            error!("Command {:x} too short for its length {:x}", pos[0], len);
            return;
        }
        if !skip {
            let bytes = &pos[..len];
            handle_ingame_hooks(&hooks, bytes, replayed_command, globals, orig);
            if globals.is_replay == 0 && !is_replay_command(pos[0]) {
                (globals.add_to_replay_data)(bytes.as_ptr(), len);
            }
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
    orig: unsafe extern "C" fn(*const c_void, u32, u32),
) {
    unsafe extern "C" fn call_orig(data: *const u8, len: u32) {
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
    orig(cmd.as_ptr() as *const c_void, cmd.len() as u32, replayed_command);
}

/// Should be called before any function overrides are added for ones included in here.
pub fn set_default_command_lengths(lengths: &[u32]) {
    let mut out = COMMAND_LENGTHS_MUTABLE.lock();
    for (i, &value) in lengths.iter().enumerate() {
        if i >= 0x100 {
            break;
        }
        if (value as usize) < 0x400 {
            out.data[i] = value as usize;
        }
    }
    out.data[0x6] = save_command_length as *const() as usize;
    out.data[0x7] = save_command_length as *const() as usize;
    out.data[0x9] = select_command_legacy_length as *const() as usize;
    out.data[0xa] = select_command_legacy_length as *const() as usize;
    out.data[0xb] = select_command_legacy_length as *const() as usize;
    out.data[0x63] = select_command_extended_length as *const() as usize;
    out.data[0x64] = select_command_extended_length as *const() as usize;
    out.data[0x65] = select_command_extended_length as *const() as usize;
    // This isn't set correctly for whatever reason, and it works out for bw since
    // it's not replay skipped
    out.data[0x37] = 7;
}

pub fn add_ingame_hook(cmd: u8, hook: IngameCommandHook) {
    INGAME_HOOKS.write().push((cmd, hook));
}

pub fn add_length_override(cmd: u8, fun: CommandLength) {
    let mut out = COMMAND_LENGTHS_MUTABLE.lock();
    out.data[cmd as usize] = fun as usize;
}
