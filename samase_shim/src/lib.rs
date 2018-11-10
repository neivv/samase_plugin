extern crate byteorder;
#[macro_use] extern crate lazy_static;
extern crate libc;
extern crate parking_lot;
extern crate thread_local;
#[macro_use] extern crate whack;
extern crate winapi;

extern crate samase_plugin;

use std::cell::{Cell, RefCell, RefMut};
use std::ffi::{CStr, CString};
use std::io::{self, Write};
use std::mem;
use std::ptr::{null, null_mut};
use std::slice;
use std::sync::{Once, ONCE_INIT};

use byteorder::{WriteBytesExt, LE};
use libc::c_void;
use thread_local::CachedThreadLocal;
use parking_lot::Mutex;
use winapi::um::heapapi::{GetProcessHeap, HeapAlloc, HeapCreate, HeapFree};
use winapi::um::winnt::{HANDLE, HEAP_CREATE_ENABLE_EXECUTE};

use samase_plugin::commands::{CommandLength, IngameCommandHook};
use samase_plugin::save::{SaveHook, LoadHook};

mod bw;
mod windows;

pub use samase_plugin::PluginApi;

lazy_static! {
    static ref EXEC_HEAP: usize = unsafe {
        HeapCreate(HEAP_CREATE_ENABLE_EXECUTE, 0, 0) as usize
    };
    static ref PATCHER: whack::Patcher = whack::Patcher::new();
    static ref CONTEXT:
        CachedThreadLocal<RefCell<Option<InternalContext>>> = CachedThreadLocal::new();
    static ref FIRST_FILE_ACCESS_HOOKS: Mutex<Vec<unsafe extern fn()>> = Mutex::new(Vec::new());
    static ref LAST_FILE_POINTER: CachedThreadLocal<Cell<u64>> = CachedThreadLocal::new();
}

unsafe fn exec_alloc(size: usize) -> *mut u8 {
    HeapAlloc(*EXEC_HEAP as HANDLE, 0, size) as *mut u8
}

pub struct Context {
    api: PluginApi,
}

#[derive(Default)]
struct InternalContext {
    replace_patches: Vec<(usize, Vec<u8>)>,
    unsupported_features: Vec<String>,
    step_objects: Vec<(unsafe extern fn(), u32)>,
    step_order: Vec<unsafe extern fn(*mut c_void, unsafe extern fn(*mut c_void))>,
    step_order_hidden: Vec<unsafe extern fn(*mut c_void, unsafe extern fn(*mut c_void))>,
    process_commands: Vec<
        unsafe extern fn(*const c_void, u32, u32, unsafe extern fn(*const c_void, u32, u32))
    >,
    process_lobby_commands: Vec<
        unsafe extern fn(*const c_void, u32, u32, unsafe extern fn(*const c_void, u32, u32))
    >,
    send_command: Vec<unsafe extern fn(*mut c_void, u32, unsafe extern fn(*mut c_void, u32))>,
    step_secondary_order: Vec<unsafe extern fn(*mut c_void, unsafe extern fn(*mut c_void))>,
    game_screen_rclick: Vec<unsafe extern fn(*mut c_void, unsafe extern fn(*mut c_void))>,
    aiscript_hooks: Vec<(u8, unsafe extern fn(*mut c_void))>,
    save_extensions_used: bool,
}

enum FnTraitGlobal<T> {
    Set(T),
    NotSet,
}

unsafe impl<T> Sync for FnTraitGlobal<T> { }
impl<T: Copy + Clone> FnTraitGlobal<T> {
    fn set(&mut self, val: T) {
        *self = FnTraitGlobal::Set(val);
    }

    unsafe fn get(&mut self) -> T {
        match *self {
            FnTraitGlobal::Set(val) => val,
            FnTraitGlobal::NotSet => panic!("Accessing FnTraitGlobal without setting it"),
        }
    }
}

impl Context {
    pub fn api(&self) -> *const PluginApi {
        &self.api
    }
}

struct BwFile(*mut c_void);

impl io::Read for BwFile {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        unsafe {
            let read =
                bw::fread(out.as_ptr() as *mut c_void, 1, out.len() as u32, self.0) as usize;
            if read > out.len() {
                // Maybe ok?
                Err(io::Error::last_os_error())
            } else {
                Ok(read)
            }
        }
    }
}

impl io::Write for BwFile {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        unsafe {
            let written =
                bw::fwrite(data.as_ptr() as *const c_void, 1, data.len() as u32, self.0) as usize;
            if written > data.len() {
                Err(io::Error::last_os_error())
            } else {
                Ok(written as usize)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl io::Seek for BwFile {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let (method, low) = match pos {
            io::SeekFrom::Start(pos) => (0, pos as u32),
            io::SeekFrom::Current(pos) => (1, pos as u32),
            io::SeekFrom::End(pos) => (2, 0u32.wrapping_sub(pos as u32)),
        };
        unsafe {
            let result = bw::fseek(self.0, low, method);
            if result == 0 {
                // Ugly hack since I don't think there is ftell linked to bw..
                Ok(LAST_FILE_POINTER.get().unwrap().get())
            } else {
                Err(io::Error::last_os_error())
            }
        }
    }
}

impl samase_plugin::save::File for BwFile {
    fn warn(&mut self, msg: &str) {
        if let Ok(msg) = CString::new(msg) {
            unsafe {
                bw::print_text(msg.as_ptr() as *const u8, 8 ,0);
            }
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        let ctx = CONTEXT.get().unwrap().borrow_mut().take()
            .expect("Missing context for samase shim???");
        if !ctx.unsupported_features.is_empty() {
            windows::message_box(
                "Warning",
                &format!(
                    "The following features won't work with your version of StarCraft:\n\n{}",
                    ctx.unsupported_features.join("\n"),
                ),
            );
        }
        let mut patcher = PATCHER.lock().unwrap();
        let mut exe = patcher.patch_exe(0x00400000);
        unsafe {
            for (hook, after) in ctx.step_objects {
                exe.hook_closure(bw::StepObjects, move |orig: &Fn()| {
                    if after == 0 {
                        hook();
                        orig();
                    } else {
                        orig();
                        hook();
                    }
                });
            }
            for hook in ctx.step_order {
                exe.hook_closure(bw::StepOrder, move |unit, orig: &Fn(_)| {
                    // Sketchy, whack should just give fnptrs as the fn traits it currently gives
                    // are stateless anyways.
                    static mut ORIG: FnTraitGlobal<*const Fn(*mut c_void)> =
                        FnTraitGlobal::NotSet;
                    ORIG.set(mem::transmute(orig));
                    unsafe extern fn call_orig(unit: *mut c_void) {
                        let orig = ORIG.get();
                        (*orig)(unit);
                    }
                    hook(unit, call_orig);
                });
            }
            for hook in ctx.step_order_hidden {
                exe.hook_closure(bw::StepOrder_Hidden, move |unit, orig: &Fn(_)| {
                    static mut ORIG: FnTraitGlobal<*const Fn(*mut c_void)> =
                        FnTraitGlobal::NotSet;
                    ORIG.set(mem::transmute(orig));
                    unsafe extern fn call_orig(unit: *mut c_void) {
                        let orig = ORIG.get();
                        (*orig)(unit);
                    }
                    hook(unit, call_orig);
                });
            }
            for hook in ctx.step_secondary_order {
                exe.hook_closure(bw::StepSecondaryOrder, move |unit, orig: &Fn(_)| {
                    static mut ORIG: FnTraitGlobal<*const Fn(*mut c_void)> =
                        FnTraitGlobal::NotSet;
                    ORIG.set(mem::transmute(orig));
                    unsafe extern fn call_orig(unit: *mut c_void) {
                        let orig = ORIG.get();
                        (*orig)(unit);
                    }
                    hook(unit, call_orig);
                });
            }
            for hook in ctx.send_command {
                exe.hook_closure(bw::SendCommand, move |data, len, orig: &Fn(_, _)| {
                    static mut ORIG: FnTraitGlobal<*const Fn(*mut c_void, u32)> =
                        FnTraitGlobal::NotSet;
                    ORIG.set(mem::transmute(orig));
                    unsafe extern fn call_orig(data: *mut c_void, len: u32) {
                        let orig = ORIG.get();
                        (*orig)(data, len);
                    }
                    hook(data, len, call_orig);
                });
            }
            for hook in ctx.process_commands {
                exe.hook_closure(
                    bw::ProcessCommands,
                    move |data, len, replay, orig: &Fn(_, _, _)| {
                        static mut ORIG: FnTraitGlobal<*const Fn(*const c_void, u32, u32)> =
                            FnTraitGlobal::NotSet;
                        ORIG.set(mem::transmute(orig));
                        unsafe extern fn call_orig(data: *const c_void, len: u32, replay: u32) {
                            let orig = ORIG.get();
                            (*orig)(data, len, replay);
                        }
                        hook(data, len, replay, call_orig);
                    },
                );
            }
            for hook in ctx.process_lobby_commands {
                exe.hook_closure(
                    bw::ProcessLobbyCommands,
                    move |data, len, replay, orig: &Fn(_, _, _)| {
                        static mut ORIG: FnTraitGlobal<*const Fn(*const c_void, u32, u32)> =
                            FnTraitGlobal::NotSet;
                        ORIG.set(mem::transmute(orig));
                        unsafe extern fn call_orig(data: *const c_void, len: u32, replay: u32) {
                            let orig = ORIG.get();
                            (*orig)(data, len, replay);
                        }
                        hook(data, len, replay, call_orig);
                    },
                );
            }
            for hook in ctx.game_screen_rclick {
                exe.hook_closure(bw::GameScreenRClick, move |event, orig: &Fn(_)| {
                    static mut ORIG: FnTraitGlobal<*const Fn(*mut c_void)> =
                        FnTraitGlobal::NotSet;
                    ORIG.set(mem::transmute(orig));
                    unsafe extern fn call_orig(event: *mut c_void) {
                        let event = event as *const bw::scr::Event;
                        let mut converted = bw::event_to_1161(&*event);
                        let orig = ORIG.get();
                        (*orig)(&mut converted as *mut bw::Event as *mut c_void);
                    }
                    let event = event as *const bw::Event;
                    let mut converted = bw::event_to_scr(&*event);
                    hook(&mut converted as *mut bw::scr::Event as *mut c_void, call_orig);
                });
            }
            apply_aiscript_hooks(&mut exe, &ctx.aiscript_hooks);
            if ctx.save_extensions_used {
                unsafe fn save_hook(file: *mut c_void) {
                    // TODO ?
                    let _ = samase_plugin::save::call_save_hooks(BwFile(file));
                }
                unsafe fn load_hook() {
                    if *bw::loaded_save != null_mut() {
                        let result =
                            samase_plugin::save::call_load_hooks(BwFile(*bw::loaded_save));
                        if let Err(e) = result {
                            // TODO not crashing
                            panic!("{}", e);
                        }
                    }
                }
                unsafe fn file_pointer_set(val: u32) {
                    LAST_FILE_POINTER.get_or(|| Box::new(Cell::new(0))).set(val as u64);
                }
                exe.call_hook(bw::SaveReady, save_hook);
                exe.hook_closure(bw::InitGame, |orig: &Fn()| {
                    samase_plugin::save::call_init_hooks();
                    orig();
                });
                exe.call_hook(bw::LoadReady, load_hook);
                exe.call_hook(bw::FseekFilePointerSet, file_pointer_set);
            }
        }
        for (addr, data) in ctx.replace_patches {
            unsafe {
                exe.replace(addr, &data);
            }
        }
        let first_file_hooks = FIRST_FILE_ACCESS_HOOKS.lock();
        if !first_file_hooks.is_empty() {
            unsafe fn call_hooks() {
                static ONCE: Once = ONCE_INIT;
                ONCE.call_once(|| {
                    let first_file_hooks = FIRST_FILE_ACCESS_HOOKS.lock();
                    for hook in &*first_file_hooks {
                        hook();
                    }
                });
            }
            unsafe {
                exe.call_hook(bw::FirstFileAccess, call_hooks);
            }
        }
    }
}

fn context() -> RefMut<'static, InternalContext> {
    RefMut::map(CONTEXT.get().unwrap().borrow_mut(), |x| x.as_mut().unwrap())
}

pub unsafe fn on_win_main(f: unsafe fn()) {
    let mut patcher = PATCHER.lock().unwrap();
    let mut exe = patcher.patch_exe(0x00400000);
    bw::init_funcs(&mut exe);
    bw::init_vars(&mut exe);
    exe.call_hook(bw::WinMain, f);
}

pub fn init_1161() -> Context {
    unsafe {
        assert!(CONTEXT.get().is_none());
        let api = PluginApi {
            version: 10,
            padding: 0,
            free_memory,
            write_exe_memory,
            warn_unsupported_feature,
            read_file,
            game,
            rng_seed,
            hook_step_objects,
            hook_aiscript_opcode,
            ai_regions,
            player_ai,
            get_region,
            change_ai_region_state,
            first_active_unit,
            first_hidden_unit,
            issue_order,
            print_text,
            hook_on_first_file_access,
            hook_step_order,
            hook_step_order_hidden,
            dat,
            hook_process_commands,
            hook_process_lobby_commands,
            hook_send_command,
            hook_step_secondary_order,
            extend_save,
            hook_ingame_command,
            units,
            selections,
            client_selection,
            first_ai_script,
            first_guard_ai,
            hook_game_screen_rclick,
            dat_requirements,
            pathing,
            set_first_ai_script,
            first_free_ai_script,
            set_first_free_ai_script,
            player_ai_towns,
        };
        let mut patcher = PATCHER.lock().unwrap();
        {
            let mut storm = patcher.patch_library("storm", 0x15000000);
            bw::init_funcs_storm(&mut storm);
        }
        {
            fn init_mpqs_only_once(orig: &Fn()) {
                static ONCE: Once = ONCE_INIT;
                ONCE.call_once(orig);
            }

            let mut exe = patcher.patch_exe(0x00400000);
            bw::init_funcs(&mut exe);
            bw::init_funcs_cdecl(&mut exe);
            bw::init_vars(&mut exe);
            exe.hook_opt(bw::InitMpqs, init_mpqs_only_once);
        }

        CONTEXT.get_or(|| Box::new(RefCell::new(Some(Default::default()))));
        Context {
            api,
        }
    }
}

unsafe extern fn free_memory(mem: *mut u8) {
    HeapFree(GetProcessHeap(), 0, mem as *mut _);
}

unsafe extern fn write_exe_memory(addr: usize, data: *const u8, len: usize) -> u32 {
    let slice = slice::from_raw_parts(data, len);
    context().replace_patches.push((addr, slice.into()));
    1
}

unsafe extern fn warn_unsupported_feature(feature: *const u8) {
    context().unsupported_features.push(
        CStr::from_ptr(feature as *const i8).to_string_lossy().into()
    );
}

struct SFileHandle(*mut c_void);

impl Drop for SFileHandle {
    fn drop(&mut self) {
        unsafe {
            bw::SFileCloseFile(self.0);
        }
    }
}

unsafe extern fn read_file() -> unsafe extern fn(*const u8, *mut usize) -> *mut u8 {
    unsafe extern fn actual(path: *const u8, size: *mut usize) -> *mut u8 {
        let len = (0..).find(|&x| *path.offset(x) == 0).unwrap() as usize;
        let mut filename = vec![0; len + 1];
        for i in 0..len {
            filename[i] = *path.offset(i as isize);
            if filename[i] == b'/' {
                filename[i] = b'\\';
            }
        }
        bw::init_mpqs();
        let mut handle = null_mut();
        let success = bw::SFileOpenFileEx(null_mut(), filename.as_ptr(), 0, &mut handle);
        if success == 0 || handle.is_null() {
            return null_mut();
        }
        let handle = SFileHandle(handle);
        let mut high = 0;
        let file_size = bw::SFileGetFileSize(handle.0, &mut high);
        if high > 0 || file_size == 0 {
            return null_mut();
        }
        let buf = HeapAlloc(GetProcessHeap(), 0, file_size as usize) as *mut u8;
        let mut read = 0;
        let success = bw::SFileReadFile(handle.0, buf, file_size, &mut read, 0);
        if success == 0 {
            return null_mut();
        }
        *size = file_size as usize;
        buf
    }
    actual
}

unsafe extern fn game() -> Option<unsafe extern fn() -> *mut c_void> {
    unsafe extern fn actual() -> *mut c_void {
        &mut *bw::game as *mut bw::Game as *mut c_void
    }
    Some(actual)
}

unsafe extern fn rng_seed() -> Option<unsafe extern fn() -> u32> {
    unsafe extern fn actual() -> u32 {
        *bw::rng_seed
    }
    Some(actual)
}

unsafe extern fn hook_step_objects(hook: unsafe extern fn(), after: u32) -> u32 {
    context().step_objects.push((hook, after));
    1
}

unsafe extern fn hook_aiscript_opcode(opcode: u32, hook: unsafe extern fn(*mut c_void)) -> u32 {
    if opcode < 0x100 {
        context().aiscript_hooks.push((opcode as u8, hook));
        1
    } else {
        0
    }
}

unsafe extern fn ai_regions() -> Option<unsafe extern fn() -> *mut c_void> {
    unsafe extern fn actual() -> *mut c_void {
        &mut bw::ai_regions[0] as *mut *mut bw::AiRegion as *mut c_void
    }
    Some(actual)
}

unsafe extern fn player_ai() -> Option<unsafe extern fn() -> *mut c_void> {
    unsafe extern fn actual() -> *mut c_void {
        &mut bw::player_ai[0] as *mut bw::PlayerAi as *mut c_void
    }
    Some(actual)
}

unsafe extern fn get_region() -> Option<unsafe extern fn(u32, u32) -> u32> {
    unsafe extern fn actual(x: u32, y: u32) -> u32 {
        bw::get_region(x, y)
    }
    Some(actual)
}

unsafe extern fn change_ai_region_state() -> Option<unsafe extern fn(*mut c_void, u32)> {
    unsafe extern fn actual(region: *mut c_void, state: u32) {
        bw::change_ai_region_state(region, state)
    }
    Some(actual)
}

unsafe extern fn first_active_unit() -> Option<unsafe extern fn() -> *mut c_void> {
    unsafe extern fn actual() -> *mut c_void {
        *bw::first_active_unit as *mut c_void
    }
    Some(actual)
}

unsafe extern fn first_hidden_unit() -> Option<unsafe extern fn() -> *mut c_void> {
    unsafe extern fn actual() -> *mut c_void {
        *bw::first_hidden_unit as *mut c_void
    }
    Some(actual)
}

unsafe extern fn units() -> Option<unsafe extern fn() -> *mut c_void> {
    unsafe extern fn actual() -> *mut c_void {
        &mut bw::units[0] as *mut bw::Unit as *mut c_void
    }
    Some(actual)
}

unsafe extern fn selections() -> Option<unsafe extern fn() -> *mut c_void> {
    unsafe extern fn actual() -> *mut c_void {
        &mut bw::selections[0] as *mut *mut bw::Unit as *mut c_void
    }
    Some(actual)
}

unsafe extern fn client_selection() -> Option<unsafe extern fn() -> *mut c_void> {
    unsafe extern fn actual() -> *mut c_void {
        &mut bw::client_selection[0] as *mut *mut bw::Unit as *mut c_void
    }
    Some(actual)
}

unsafe extern fn first_ai_script() -> Option<unsafe extern fn() -> *mut c_void> {
    unsafe extern fn actual() -> *mut c_void {
        *bw::first_ai_script
    }
    Some(actual)
}

unsafe extern fn set_first_ai_script() -> Option<unsafe extern fn(*mut c_void)> {
    unsafe extern fn actual(value: *mut c_void) {
        *bw::first_ai_script = value
    }
    Some(actual)
}

unsafe extern fn first_free_ai_script() -> Option<unsafe extern fn() -> *mut c_void> {
    unsafe extern fn actual() -> *mut c_void {
        *bw::first_free_ai_script
    }
    Some(actual)
}

unsafe extern fn set_first_free_ai_script() -> Option<unsafe extern fn(*mut c_void)> {
    unsafe extern fn actual(value: *mut c_void) {
        *bw::first_free_ai_script = value
    }
    Some(actual)
}

unsafe extern fn player_ai_towns() -> Option<unsafe extern fn() -> *mut c_void> {
    unsafe extern fn actual() -> *mut c_void {
        &mut bw::active_ai_towns[0] as *mut bw::AiTownList as *mut c_void
    }
    Some(actual)
}

unsafe extern fn first_guard_ai() -> Option<unsafe extern fn() -> *mut c_void> {
    unsafe extern fn actual() -> *mut c_void {
        bw::guard_ais.as_ptr() as *mut c_void
    }
    Some(actual)
}

unsafe extern fn pathing() -> Option<unsafe extern fn() -> *mut c_void> {
    unsafe extern fn actual() -> *mut c_void {
        *bw::pathing
    }
    Some(actual)
}

    // self, order, x, y, target, fow_unit
unsafe extern fn issue_order() ->
    Option<unsafe extern fn(*mut c_void, u32, u32, u32, *mut c_void, u32)>
{
    unsafe extern fn actual(
        unit: *mut c_void,
        order: u32,
        x: u32,
        y: u32,
        target: *mut c_void,
        fow_unit: u32,
    ) {
        let xy = (x as u32 & 0xffff) | ((y as u32) << 16);
        let unit = unit as *mut bw::Unit;
        let target = target as *mut bw::Unit;
        bw::prepare_issue_order(unit, order, xy, target, fow_unit, 1);
        bw::do_next_queued_order(unit);
    }
    Some(actual)
}

unsafe extern fn print_text() -> Option<unsafe extern fn(*const u8)> {
    unsafe extern fn actual(text: *const u8) {
        bw::print_text(text, 8, 0);
    }
    Some(actual)
}

unsafe extern fn hook_on_first_file_access(hook: unsafe extern fn()) {
    FIRST_FILE_ACCESS_HOOKS.lock().push(hook);
}

unsafe extern fn hook_step_order(
    hook: unsafe extern fn(*mut c_void, unsafe extern fn(*mut c_void)),
) -> u32 {
    context().step_order.push(hook);
    1
}

unsafe extern fn hook_step_order_hidden(
    hook: unsafe extern fn(*mut c_void, unsafe extern fn(*mut c_void)),
) -> u32 {
    context().step_order_hidden.push(hook);
    1
}

unsafe extern fn dat(dat: u32) -> Option<unsafe extern fn() -> *mut c_void> {
    macro_rules! dat_fns {
        ($($name:ident,)*) => {
            $(
                unsafe extern fn $name() -> *mut c_void {
                    bw::$name.as_ptr() as *mut c_void
                }
            )*
        }
    }
    dat_fns! {
        units_dat,
        weapons_dat,
        flingy_dat,
        upgrades_dat,
        techdata_dat,
        sprites_dat,
        images_dat,
        orders_dat,
        sfxdata_dat,
        portdata_dat,
    }
    let fun: unsafe extern fn() -> *mut c_void = match dat {
        0 => units_dat,
        1 => weapons_dat,
        2 => flingy_dat,
        3 => upgrades_dat,
        4 => techdata_dat,
        5 => sprites_dat,
        6 => images_dat,
        7 => orders_dat,
        8 => sfxdata_dat,
        9 => portdata_dat,
        _ => return None,
    };
    Some(fun)
}

unsafe extern fn hook_process_commands(
    hook: unsafe extern fn(*const c_void, u32, u32, unsafe extern fn(*const c_void, u32, u32)),
) -> u32 {
    context().process_commands.push(hook);
    1
}

unsafe extern fn hook_process_lobby_commands(
    hook: unsafe extern fn(*const c_void, u32, u32, unsafe extern fn(*const c_void, u32, u32)),
) -> u32 {
    context().process_lobby_commands.push(hook);
    1
}

unsafe extern fn hook_send_command(
    hook: unsafe extern fn(*mut c_void, u32, unsafe extern fn(*mut c_void, u32)),
) -> u32 {
    context().send_command.push(hook);
    1
}

unsafe extern fn hook_step_secondary_order(
    hook: unsafe extern fn(*mut c_void, unsafe extern fn(*mut c_void)),
) -> u32 {
    context().step_secondary_order.push(hook);
    1
}

unsafe extern fn hook_game_screen_rclick(
    hook: unsafe extern fn(*mut c_void, unsafe extern fn(*mut c_void)),
) -> u32 {
    context().game_screen_rclick.push(hook);
    1
}

unsafe extern fn extend_save(
    tag: *const u8,
    save: SaveHook,
    load: LoadHook,
    init: unsafe extern fn(),
) -> u32 {
    let tag = CStr::from_ptr(tag as *const i8).to_string_lossy();
    samase_plugin::save::add_hook(tag.into(), save, load, init);
    context().save_extensions_used = true;
    1
}

unsafe extern fn hook_ingame_command(
    cmd: u32,
    hook: IngameCommandHook,
    len: Option<CommandLength>,
) -> u32 {
    use samase_plugin::commands;

    static INGAME_COMMAND_HOOK: Once = ONCE_INIT;
    if cmd >= 0x100 {
        return 0;
    }
    INGAME_COMMAND_HOOK.call_once(|| {
        unsafe extern fn ingame_hook(
            data: *const c_void,
            len: u32,
            replayed: u32,
            orig: unsafe extern fn(*const c_void, u32, u32),
        ) {
            let globals = commands::IngameHookGlobals {
                is_replay: *bw::is_replay,
                command_user: *bw::command_user,
                unique_command_user: *bw::unique_command_user,
            };
            commands::ingame_hook(data, len, replayed, &globals, orig);
        }
        hook_process_commands(ingame_hook);
        let command_lengths = &(*bw::command_lengths)[..];
        commands::set_default_command_lengths(command_lengths.into());
    });
    commands::add_ingame_hook(cmd as u8, hook);
    if let Some(len) = len {
        commands::add_length_override(cmd as u8, len);
    }
    1
}

unsafe extern fn dat_requirements() -> Option<unsafe extern fn(u32, u32) -> *const u16> {
    unsafe extern fn inner(ty: u32, id: u32) -> *const u16 {
        let arr = match ty {
            0 => *bw::unit_requirement_table,
            1 => *bw::upgrade_requirement_table,
            2 => *bw::tech_research_requirement_table,
            3 => *bw::tech_use_requirement_table,
            4 => *bw::order_requirement_table,
            _ => panic!("Invalid dat req {}", ty),
        };
        let mut pos = arr.offset(1);
        loop {
            let current_id = *pos as u32;
            if current_id == id {
                return pos.offset(1);
            }
            if current_id == 0xffff {
                return null();
            }
            while *pos != 0xffff {
                pos = pos.offset(1);
            }
            pos = pos.offset(1);
        }
    }
    Some(inner)
}

unsafe fn apply_aiscript_hooks(
    exe: &mut whack::ModulePatcher,
    hooks: &[(u8, unsafe extern fn(*mut c_void))],
) {
    if hooks.is_empty() {
        return;
    }
    // Going to set last as !0 so other plugins using this same shim can use it
    // to count patched switch table length
    let unpatched_switch_table =
        *bw::aiscript_switch_table_ptr == bw::aiscript_default_switch_table.as_mut_ptr();
    let old_opcode_count = if unpatched_switch_table {
        0x71
    } else {
        let switch_table = *bw::aiscript_switch_table_ptr;
        (0u32..).find(|&x| *switch_table.offset(x as isize) == !0).unwrap()
    };
    let opcode_count =
        hooks.iter().map(|x| x.0 as u32 + 1).max().unwrap_or(0).max(old_opcode_count);
    let mut switch_table = vec![0; opcode_count as usize + 2];
    switch_table[opcode_count as usize + 1] = !0;
    for i in 0..old_opcode_count {
        let old_switch_table = *bw::aiscript_switch_table_ptr;
        switch_table[i as usize] = *old_switch_table.offset(i as isize);
    }
    let mut asm_offsets = Vec::with_capacity(hooks.len());
    let mut asm = Vec::new();
    for &(opcode, fun) in hooks {
        asm_offsets.push((opcode, asm.len()));
        asm.write_all(&[
            0x60, // pushad
            0x56, // push esi (aiscript)
            0xb8, // mov eax, ...
        ]).unwrap();
        asm.write_u32::<LE>(mem::transmute(fun)).unwrap();
        asm.write_all(&[
            0xff, 0xd0, // call eax
            0x59, // pop ecx
            0x8b, 0x46, 0x0c, // mov eax, [esi + 0xc] (Script wait)
            0x31, 0xc9, // xor ecx, ecx
            0x49, // dec ecx
            0x39, 0xc8, // cmp eax, ecx
            0x74, 0x0d, // je wait not set
            0x61, // popad
            0xc7, 0x44, 0xe4, 0xfc, // Mov [esp - 4], dword ...
        ]).unwrap();
        asm.write_u32::<LE>(bw::AISCRIPT_RET as u32).unwrap();
        // jmp dword [esp - 4]
        asm.write_all(&[0xff, 0x64, 0xe4, 0xfc]).unwrap();
        // wait not set
        asm.write_all(&[
            0x61, // popad
            0xc7, 0x44, 0xe4, 0xfc, // Mov [esp - 4], dword ...
        ]).unwrap();
        asm.write_u32::<LE>(bw::AISCRIPT_LOOP as u32).unwrap();
        // jmp dword [esp - 4]
        asm.write_all(&[0xff, 0x64, 0xe4, 0xfc]).unwrap();
    }
    let exec_asm = exec_alloc(asm.len());
    std::ptr::copy_nonoverlapping(asm.as_ptr(), exec_asm, asm.len());
    for (opcode, offset) in asm_offsets {
        switch_table[opcode as usize] = exec_asm as u32 + offset as u32;
    }

    let opcode_count_patch = [0x90, 0x3c, opcode_count as u8];
    exe.replace(bw::AISCRIPT_OPCODE_CMP, &opcode_count_patch);
    let mut switch_table_ptr = [0u8; 4];
    (&mut switch_table_ptr[..]).write_u32::<LE>(switch_table.as_ptr() as u32).unwrap();
    mem::forget(switch_table);
    exe.replace(bw::AISCRIPT_SWITCH_TABLE, &switch_table_ptr);
}
