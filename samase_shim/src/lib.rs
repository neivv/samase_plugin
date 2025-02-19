#[macro_use] extern crate whack;

use std::cell::{Cell, RefCell, RefMut};
use std::ffi::{CStr, CString};
use std::io::{self, Write};
use std::marker::PhantomData;
use std::mem;
use std::ptr::{null, null_mut};
use std::slice;
use std::sync::{Once};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use byteorder::{WriteBytesExt, LE};
use libc::c_void;
use thread_local::ThreadLocal;
use once_cell::sync::Lazy;
use parking_lot::{Mutex, RwLock, const_mutex, const_rwlock};
use winapi::um::heapapi::{GetProcessHeap, HeapAlloc, HeapCreate, HeapFree};
use winapi::um::winnt::{HANDLE, HEAP_CREATE_ENABLE_EXECUTE};

use samase_plugin::commands::{CommandLength, IngameCommandHook};
use samase_plugin::save::{SaveHook, LoadHook};

mod bw;
mod windows;

pub use samase_plugin::{PluginApi, FuncId, VarId};

static PATCHER: Mutex<whack::Patcher> = const_mutex(whack::Patcher::new());
static FIRST_FILE_ACCESS_HOOKS: Mutex<Vec<unsafe extern fn()>> = const_mutex(Vec::new());
static FILE_READ_HOOKS: RwLock<Vec<FileReadHook>> = const_rwlock(Vec::new());
static OPEN_HOOKED_FILES: Mutex<Vec<HeapFreeOnDropPtr>> = const_mutex(Vec::new());

static CONTEXT: Lazy<ThreadLocal<RefCell<Option<InternalContext>>>> =
    Lazy::new(|| ThreadLocal::new());
static LAST_FILE_POINTER: Lazy<ThreadLocal<Cell<u64>>> = Lazy::new(|| ThreadLocal::new());

static HAS_HOOKED_FILES_OPEN: AtomicBool = AtomicBool::new(false);

type Hook1Arg = unsafe extern fn(usize, unsafe extern fn(usize) -> usize) -> usize;
type Hook2Arg = unsafe extern fn(usize, usize, unsafe extern fn(usize, usize) -> usize) -> usize;
type Hook3Arg = unsafe extern fn(usize, usize, usize,
    unsafe extern fn(usize, usize, usize) -> usize) -> usize;
type Hook4Arg = unsafe extern fn(usize, usize, usize, usize,
    unsafe extern fn(usize, usize, usize, usize) -> usize) -> usize;
type Hook5Arg = unsafe extern fn(usize, usize, usize, usize, usize,
    unsafe extern fn(usize, usize, usize, usize, usize) -> usize) -> usize;
type Hook6Arg = unsafe extern fn(usize, usize, usize, usize, usize, usize,
    unsafe extern fn(usize, usize, usize, usize, usize, usize) -> usize) -> usize;
type Hook7Arg = unsafe extern fn(usize, usize, usize, usize, usize, usize, usize,
    unsafe extern fn(usize, usize, usize, usize, usize, usize, usize) -> usize) -> usize;
type Hook9Arg = unsafe extern fn(usize, usize, usize, usize, usize, usize, usize, usize, usize,
    unsafe extern fn(usize, usize, usize, usize, usize, usize, usize, usize, usize) -> usize,
    ) -> usize;

struct FileReadHook {
    prefix: Vec<u8>,
    hook: unsafe extern fn(*const u8, *mut u32) -> *mut u8,
    being_called: ThreadLocal<Cell<bool>>,
}

impl FileReadHook {
    unsafe fn matches(&self, filename: *const u8) -> bool {
        (0..self.prefix.len()).all(|x| self.prefix[x].eq_ignore_ascii_case(&*filename.add(x)))
    }
}

struct HeapFreeOnDrop(*mut u8, u32);
#[derive(Copy, Clone)]
struct HeapFreeOnDropPtr(*mut HeapFreeOnDrop);

impl std::ops::Drop for HeapFreeOnDrop {
    fn drop(&mut self) {
        unsafe {
            HeapFree(GetProcessHeap(), 0, self.0 as *mut _);
        }
    }
}

unsafe impl Send for HeapFreeOnDropPtr {}
unsafe impl Sync for HeapFreeOnDropPtr {}

unsafe fn exec_alloc(heap: HANDLE, size: usize) -> *mut u8 {
    HeapAlloc(heap, 0, size) as *mut u8
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
    draw_image: Vec<unsafe extern fn(*mut c_void, unsafe extern fn(*mut c_void))>,
    run_dialog: Vec<unsafe extern fn(
        *mut c_void,
        usize,
        *mut c_void,
        unsafe extern fn(*mut c_void, usize, *mut c_void) -> u32,
    ) -> u32>,
    spawn_dialog: Vec<unsafe extern fn(
        *mut c_void,
        usize,
        *mut c_void,
        unsafe extern fn(*mut c_void, usize, *mut c_void) -> u32,
    ) -> u32>,
    create_bullet: Vec<unsafe extern fn(
        u32, i32, i32, u32, u32, *mut c_void,
        unsafe extern fn(u32, i32, i32, u32, u32, *mut c_void) -> *mut c_void,
    ) -> *mut c_void>,
    create_unit: Vec<unsafe extern fn(
        u32, i32, i32, u32, *const u8,
        unsafe extern fn(u32, i32, i32, u32, *const u8) -> *mut c_void,
    ) -> *mut c_void>,
    init_units: Vec<unsafe extern fn(unsafe extern fn())>,
    aiscript_hooks: Vec<(u8, unsafe extern fn(*mut c_void))>,
    iscript_hooks: Vec<
        (u8, unsafe extern fn(*mut c_void, *mut c_void, *mut c_void, u32, *mut u32))
    >,
    ai_focus_disabled: Vec<unsafe extern fn(*mut c_void, unsafe extern fn(*mut c_void))>,
    ai_focus_air: Vec<unsafe extern fn(*mut c_void, unsafe extern fn(*mut c_void))>,
    func_hooks: Vec<(FuncId, usize)>,
    save_extensions_used: bool,
}

struct FnTraitGlobal<T> {
    state: AtomicUsize,
    phantom: PhantomData<T>,
}

unsafe impl<T> Sync for FnTraitGlobal<T> { }

impl<T: Copy + Clone> FnTraitGlobal<T> {
    const fn new() -> FnTraitGlobal<T> {
        FnTraitGlobal {
            state: AtomicUsize::new(0),
            phantom: PhantomData,
        }
    }

    unsafe fn set(&self, val: T) {
        self.state.store(mem::transmute_copy::<T, usize>(&val), Ordering::Relaxed);
    }

    unsafe fn get(&self) -> T {
        let val = self.state.load(Ordering::Relaxed);
        mem::transmute_copy::<usize, T>(&val)
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
            io::SeekFrom::End(pos) => (2, pos as u32),
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
        let mut patcher = PATCHER.lock();
        let mut exe = patcher.patch_exe(0x00400000);
        unsafe {
            for (hook, after) in ctx.step_objects {
                exe.hook_closure(bw::StepObjects, move |orig| {
                    if after == 0 {
                        *bw::rng_enabled = 1;
                        hook();
                        *bw::rng_enabled = 0;
                        orig();
                    } else {
                        orig();
                        *bw::rng_enabled = 1;
                        hook();
                        *bw::rng_enabled = 0;
                    }
                });
            }
            for hook in ctx.step_order {
                exe.hook_closure(bw::StepOrder, move |unit, orig| {
                    // Sketchy, whack should just give fnptrs as the fn traits it currently gives
                    // are stateless anyways.
                    static ORIG: FnTraitGlobal<unsafe extern fn(*mut c_void)> =
                        FnTraitGlobal::new();
                    ORIG.set(orig);
                    unsafe extern fn call_orig(unit: *mut c_void) {
                        let orig = ORIG.get();
                        orig(unit);
                    }
                    hook(unit, call_orig);
                });
            }
            for hook in ctx.step_order_hidden {
                exe.hook_closure(bw::StepOrder_Hidden, move |unit, orig| {
                    static ORIG: FnTraitGlobal<unsafe extern fn(*mut c_void)> =
                        FnTraitGlobal::new();
                    ORIG.set(orig);
                    unsafe extern fn call_orig(unit: *mut c_void) {
                        let orig = ORIG.get();
                        orig(unit);
                    }
                    hook(unit, call_orig);
                });
            }
            for hook in ctx.step_secondary_order {
                exe.hook_closure(bw::StepSecondaryOrder, move |unit, orig| {
                    static ORIG: FnTraitGlobal<unsafe extern fn(*mut c_void)> =
                        FnTraitGlobal::new();
                    ORIG.set(orig);
                    unsafe extern fn call_orig(unit: *mut c_void) {
                        let orig = ORIG.get();
                        orig(unit);
                    }
                    hook(unit, call_orig);
                });
            }
            for hook in ctx.send_command {
                exe.hook_closure(bw::SendCommand, move |data, len, orig| {
                    static ORIG: FnTraitGlobal<unsafe extern fn(*mut c_void, u32)> =
                        FnTraitGlobal::new();
                    ORIG.set(orig);
                    unsafe extern fn call_orig(data: *mut c_void, len: u32) {
                        let orig = ORIG.get();
                        orig(data, len);
                    }
                    hook(data, len, call_orig);
                });
            }
            for hook in ctx.process_commands {
                exe.hook_closure(
                    bw::ProcessCommands,
                    move |data, len, replay, orig| {
                        static ORIG:
                            FnTraitGlobal<unsafe extern fn(*const c_void, u32, u32)> =
                            FnTraitGlobal::new();
                        ORIG.set(orig);
                        unsafe extern fn call_orig(data: *const c_void, len: u32, replay: u32) {
                            let orig = ORIG.get();
                            orig(data, len, replay);
                        }
                        hook(data, len, replay, call_orig);
                    },
                );
            }
            for hook in ctx.process_lobby_commands {
                exe.hook_closure(
                    bw::ProcessLobbyCommands,
                    move |data, len, replay, orig| {
                        static ORIG:
                            FnTraitGlobal<unsafe extern fn(*const c_void, u32, u32)> =
                            FnTraitGlobal::new();
                        ORIG.set(orig);
                        unsafe extern fn call_orig(data: *const c_void, len: u32, replay: u32) {
                            let orig = ORIG.get();
                            orig(data, len, replay);
                        }
                        hook(data, len, replay, call_orig);
                    },
                );
            }
            for hook in ctx.game_screen_rclick {
                exe.hook_closure(bw::GameScreenRClick, move |event, orig| {
                    static ORIG: FnTraitGlobal<unsafe extern fn(*mut c_void)> =
                        FnTraitGlobal::new();
                    ORIG.set(orig);
                    unsafe extern fn call_orig(event: *mut c_void) {
                        let orig = ORIG.get();
                        orig(event);
                    }
                    hook(event as *mut c_void, call_orig);
                });
            }
            for hook in ctx.draw_image {
                exe.hook_closure(bw::DrawImage, move |image, orig| {
                    static ORIG: FnTraitGlobal<unsafe extern fn(*mut bw::Image)> =
                        FnTraitGlobal::new();
                    ORIG.set(orig);
                    unsafe extern fn call_orig(image: *mut c_void) {
                        let orig = ORIG.get();
                        orig(image as *mut bw::Image)
                    }
                    hook(image as *mut c_void, call_orig);
                });
            }
            for hook in ctx.run_dialog {
                exe.hook_closure(
                    bw::RunDialog,
                    move |dialog, event_handler, orig| {
                        static ORIG:
                            FnTraitGlobal<unsafe extern fn(*mut c_void, *mut c_void) -> u32> =
                            FnTraitGlobal::new();
                        ORIG.set(mem::transmute(orig));
                        unsafe extern fn call_orig(
                            dialog: *mut c_void,
                            _unused: usize,
                            event_handler: *mut c_void,
                        ) -> u32 {
                            let orig = ORIG.get();
                            orig(dialog, event_handler)
                        }
                        hook(dialog, 0, event_handler, call_orig);
                    }
                );
            }
            for hook in ctx.spawn_dialog {
                exe.hook_closure(
                    bw::SpawnDialog,
                    move |dialog, event_handler, orig| {
                        static ORIG:
                            FnTraitGlobal<unsafe extern fn(*mut c_void, *mut c_void) -> u32> =
                            FnTraitGlobal::new();
                        ORIG.set(mem::transmute(orig));
                        unsafe extern fn call_orig(
                            dialog: *mut c_void,
                            _unused: usize,
                            event_handler: *mut c_void,
                        ) -> u32 {
                            let orig = ORIG.get();
                            orig(dialog, event_handler)
                        }
                        hook(dialog, 0, event_handler, call_orig);
                    }
                );
            }
            for hook in ctx.create_bullet {
                exe.hook_closure(
                    bw::CreateBullet,
                    move |id, x, y, player, direction, parent, orig| {
                        static ORIG: FnTraitGlobal<
                                unsafe extern fn(u32, i32, i32, u32, u32, *mut c_void) -> *mut c_void
                            > = FnTraitGlobal::new();
                        ORIG.set(mem::transmute(orig));
                        unsafe extern fn call_orig(
                            id: u32,
                            x: i32,
                            y: i32,
                            player: u32,
                            direction: u32,
                            parent: *mut c_void,
                        ) -> *mut c_void {
                            let orig = ORIG.get();
                            orig(id, x, y, player, direction, parent)
                        }
                        hook(id, x, y, player, direction, parent, call_orig)
                    }
                );
            }
            for hook in ctx.create_unit {
                exe.hook_closure(
                    bw::CreateUnit,
                    move |id, x, y, player, orig| {
                        static ORIG: FnTraitGlobal<
                                unsafe extern fn(u32, i32, i32, u32) -> *mut c_void
                            > = FnTraitGlobal::new();
                        ORIG.set(mem::transmute(orig));
                        unsafe extern fn call_orig(
                            id: u32,
                            x: i32,
                            y: i32,
                            player: u32,
                            _skins: *const u8,
                        ) -> *mut c_void {
                            let orig = ORIG.get();
                            orig(id, x, y, player)
                        }
                        let dummy_skin = [player as u8, player as u8];
                        hook(id, x, y, player, dummy_skin.as_ptr(), call_orig)
                    }
                );
            }
            for hook in ctx.init_units {
                exe.hook_closure(
                    bw::InitUnits,
                    move |orig| {
                        hook(orig)
                    },
                );
            }
            for hook in ctx.ai_focus_disabled {
                exe.hook_closure(
                    bw::AiFocusDisabled,
                    move |unit, orig| {
                        hook(unit, orig)
                    },
                );
            }
            for hook in ctx.ai_focus_air {
                exe.hook_closure(
                    bw::AiFocusAir,
                    move |unit, orig| {
                        hook(unit, orig)
                    },
                );
            }
            for (func, hook) in ctx.func_hooks {
                use samase_plugin::FuncId::*;

                let hook1: Hook1Arg = mem::transmute(hook);
                let hook2: Hook2Arg = mem::transmute(hook);
                let hook3: Hook3Arg = mem::transmute(hook);
                let hook4: Hook4Arg = mem::transmute(hook);
                let hook5: Hook5Arg = mem::transmute(hook);
                let hook6: Hook6Arg = mem::transmute(hook);
                let hook7: Hook7Arg = mem::transmute(hook);
                let hook9: Hook9Arg = mem::transmute(hook);
                match func {
                    UnitCanRally => {
                        exe.hook_closure(bw::H_UnitCanRally, move |a, o| hook1(a, o));
                    }
                    UnitCanBeInfested => {
                        exe.hook_closure(bw::H_UnitCanBeInfested, move |a, o| hook1(a, o));
                    }
                    DoMissileDamage => {
                        exe.hook_closure(bw::H_DoMissileDamage, move |a, o| hook1(a, o));
                    }
                    HitUnit => {
                        exe.hook_closure(bw::H_HitUnit, move |a, b, c, o| hook3(a, b, c, o));
                    }
                    HallucinationHit => {
                        exe.hook_closure(
                            bw::H_HallucinationHit,
                            move |a, b, c, d, o| hook4(a, b & 0xff, c & 0xff, d, o),
                        );
                    }
                    DamageUnit => {
                        exe.hook_closure(
                            bw::H_DamageUnit,
                            move |a, b, c, d, e, o| hook5(a, b, c, d & 0xff, e & 0xff, o),
                        );
                    }
                    KillUnit => {
                        exe.hook_closure(bw::H_KillUnit, move |a, o| hook1(a, o));
                    }
                    UnitSetHp => {
                        exe.hook_closure(bw::H_UnitSetHp, move |a, b, o| hook2(a, b, o));
                    }
                    TransformUnit => {
                        exe.hook_closure(bw::H_TransformUnit, move |a, b, o| hook2(a, b & 0xffff, o));
                    }
                    GiveUnit => {
                        exe.hook_closure(bw::H_GiveUnit, move |a, b, o| hook2(a, b & 0xff, o));
                    }
                    PlaceCreepRect => {
                        exe.hook_closure(
                            bw::H_PlaceCreepRect,
                            move |a, b, c, d, e, o| hook5(a, b, c, d, e & 0xff, o),
                        );
                    }
                    PlaceFinishedUnitCreep => {
                        exe.hook_closure(
                            bw::H_PlaceFinishedUnitCreep,
                            move |a, b, c, o| hook3(a & 0xffff, b & 0xffff, c & 0xffff, o),
                        );
                    }
                    AddAiToTrainedUnit => {
                        exe.hook_closure(
                            bw::H_AddAiToTrainedUnit,
                            move |a, b, o| hook2(a, b, o),
                        );
                    }
                    AddBuildingAi => {
                        exe.hook_closure(
                            bw::H_AddBuildingAi,
                            move |a, b, o| hook2(a, b, o),
                        );
                    }
                    AddTownUnitAi => {
                        exe.hook_closure(
                            bw::H_AddTownUnitAi,
                            move |a, b, o| hook2(a, b, o),
                        );
                    }
                    AddMilitaryAi => {
                        exe.hook_closure(
                            bw::H_AddMilitaryAi,
                            move |a, b, c, o| hook3(a, b, c & 0xff, o),
                        );
                    }
                    AiRemoveUnit => {
                        exe.hook_closure(
                            bw::H_AiRemoveUnit,
                            move |a, b, o| hook2(a, b & 0xff, o),
                        );
                    }
                    AiRemoveUnitMilitary => {
                        exe.hook_closure(
                            bw::H_AiRemoveUnitMilitary,
                            move |a, b, o| hook2(a, b & 0xff, o),
                        );
                    }
                    AiRemoveUnitTown => {
                        exe.hook_closure(
                            bw::H_AiRemoveUnitTown,
                            move |a, b, o| hook2(a, b & 0xff, o),
                        );
                    }
                    UnitMaxEnergy => {
                        exe.hook_closure(
                            bw::H_UnitMaxEnergy,
                            move |a, o| hook1(a, o),
                        );
                    }
                    UnitAttackRange => {
                        exe.hook_closure(
                            bw::H_UnitAttackRange,
                            move |a, b, o| hook2(a, b & 0xffff, o),
                        );
                    }
                    UnitTargetAcquisitionRange => {
                        exe.hook_closure(
                            bw::H_UnitTargetAcquisitionRange,
                            move |a, o| hook1(a, o),
                        );
                    }
                    UnitSightRange => {
                        exe.hook_closure(
                            bw::H_UnitSightRange,
                            move |a, b, o| hook2(a, b & 0xff, o),
                        );
                    }
                    CheckWeaponTargetingFlags => {
                        exe.hook_closure(
                            bw::H_CheckWeaponTargetingFlags,
                            move |a, b, c, o| hook3(a, b & 0xff, c, o),
                        );
                    }
                    CheckTechTargeting => {
                        exe.hook_closure(
                            bw::H_CheckTechTargeting,
                            move |a, b, c, d, e, f, g, o| {
                                hook7(a, b & 0xff, c, d & 0xffff, e & 0xffff, f & 0xffff, g, o)
                            }
                        );
                    }
                    CheckOrderTargeting => {
                        exe.hook_closure(
                            bw::H_CheckOrderTargeting,
                            move |a, b, c, d, e, f, o| {
                                hook6(a, b & 0xff, c, d & 0xffff, e & 0xffff, f, o)
                            }
                        );
                    }
                    CheckFowOrderTargeting => {
                        exe.hook_closure(
                            bw::H_CheckFowOrderTargeting,
                            move |a, b, c, d, e, f, o| {
                                hook6(a, b & 0xff, c & 0xffff, d & 0xffff, e & 0xffff, f, o)
                            }
                        );
                    }
                    HideUnit => {
                        exe.hook_closure(
                            bw::H_HideUnit,
                            move |a, o| hook1(a, o),
                        );
                    }
                    ShowUnit => {
                        exe.hook_closure(
                            bw::H_ShowUnit,
                            move |a, o| hook1(a, o),
                        );
                    }
                    AiAddMilitaryToRegion => {
                        exe.hook_closure(
                            bw::H_AiAddMilitaryToRegion,
                            move |a, b, c, o| hook3(a, b & 0xffff, c & 0xff, o),
                        );
                    }
                    AiTrainMilitary => {
                        exe.hook_closure(
                            bw::H_AiTrainMilitary,
                            move |a, o| hook1(a & 0xff, o),
                        );
                    }
                    AiAttackPrepare => {
                        exe.hook_closure(
                            bw::H_AiAttackPrepare,
                            move |a, b, c, d, e, o| {
                                hook5(a & 0xff, b & 0xffff, c & 0xffff, d & 0xff, e & 0xff, o)
                            },
                        );
                    }
                    AiRegionUpdateStrength => {
                        exe.hook_closure(
                            bw::H_AiRegionUpdateStrength,
                            move |a, o| hook1(a, o),
                        );
                    }
                    AiRegionUpdateTarget => {
                        exe.hook_closure(
                            bw::H_AiRegionUpdateTarget,
                            move |a, o| hook1(a, o),
                        );
                    }
                    AiRegionAbandonIfOverwhelmed => {
                        exe.hook_closure(
                            bw::H_AiRegionAbandonIfOverwhelmed,
                            move |a, o| hook1(a, o),
                        );
                    }
                    AiRegionPickAttackTarget => {
                        exe.hook_closure(
                            bw::H_AiRegionPickAttackTarget,
                            move |a, o| hook1(a, o),
                        );
                    }
                    AiStepRegion => {
                        exe.hook_closure(
                            bw::H_AiStepRegion,
                            move |a, b, o| hook2(a & 0xff, b & 0xffff, o),
                        );
                    }
                    AiTargetExpansion => {
                        exe.hook_closure(
                            bw::H_AiTargetExpansion,
                            move |a, o| hook1(a & 0xff, o),
                        );
                    }
                    StepUnitTimers => {
                        exe.hook_closure(
                            bw::H_StepUnitTimers,
                            move |a, o| hook1(a, o),
                        );
                    }
                    StartCloaking => {
                        exe.hook_closure(
                            bw::H_StartCloaking,
                            move |a, o| hook1(a, o),
                        );
                    }
                    UnitAiWorker => {
                        exe.hook_closure(
                            bw::H_UnitAiWorker,
                            move |a, o| hook1(a, o),
                        );
                    }
                    UnitAiMilitary => {
                        exe.hook_closure(
                            bw::H_UnitAiMilitary,
                            move |a, o| hook1(a, o),
                        );
                    }
                    UnitAiBuilding => {
                        exe.hook_closure(
                            bw::H_AiTryProgressSpendingRequest,
                            move |a, o| hook1(a, o),
                        );
                    }
                    AiCanTargetAttackThis => {
                        exe.hook_closure(
                            bw::H_AiCanTargetAttackThis,
                            move |a, b, o| hook2(a, b, o),
                        );
                    }
                    AiTryReturnHome => {
                        exe.hook_closure(
                            bw::H_AiTryReturnHome,
                            move |a, b, o| hook2(a, b, o),
                        );
                    }
                    IsOutsideAttackRange => {
                        exe.hook_closure(
                            bw::H_IsOutsideAttackRange,
                            move |a, b, o| hook2(a, b, o),
                        );
                    }
                    CanAttackUnit => {
                        exe.hook_closure(
                            bw::H_CanAttackUnit,
                            move |a, b, c, o| hook3(a, b, c & 0xff, o),
                        );
                    }
                    PrepareBuildUnit => {
                        exe.hook_closure(
                            bw::H_PrepareBuildUnit,
                            move |a, b, o| hook2(a, b & 0xffff, o),
                        );
                    }
                    CalculatePath => {
                        exe.hook_closure(
                            bw::H_CalculatePath,
                            move |a, o| hook1(a, o),
                        );
                    }
                    GetChokePointRegions => {
                        exe.hook_closure(
                            bw::H_GetChokePointRegions,
                            move |a, b, c, d, e, f, o| {
                                hook6(a & 0xffff, b & 0xffff, c & 0xff, d, e, f & 0xff, o)
                            },
                        );
                    }
                    AiPlaceBuilding => {
                        exe.hook_closure(
                            bw::H_AiPlaceBuilding,
                            move |a, b, c, d, e, o| {
                                hook5(a, b & 0xffff, c, d, e & 0xff, o)
                            },
                        );
                    }
                    AiUpdateBuildingPlacementState => {
                        exe.hook_closure(
                            bw::H_AiUpdateBuildingPlacementState,
                            move |a, b, c, d, e, o| {
                                hook5(a & 0xffff, b, c & 0xff, d, e & 0xff, o)
                            },
                        );
                    }
                    UpdateBuildingPlacementState => {
                        exe.hook_closure(
                            bw::H_UpdateBuildingPlacementState,
                            move |a, b, c, d, e, f, g, h, i, o| {
                                hook9(a, b & 0xffff, c & 0xffff, d & 0xffff, e & 0xffff,
                                    f & 0xff, g & 0xff, h & 0xff, i & 0xff, o)
                            },
                        );
                    }
                    FindNearestUnitAroundUnit | FindNearestUnitInArea | ForEachUnitInArea |
                        FindNearestUnitInAreaPoint => (),
                    // Not bw functions
                    AiPickBestPlacementPosition | AiPlacementFlags => (),
                    AiAttackClear => (), // Inlined in 1.16.1
                    GetRenderTarget => (),
                    MoveScreen => (),
                    SelectUnits => (),
                    StepGameLogic => (),
                    _Last => (),
                }
            }

            if !ctx.aiscript_hooks.is_empty() || !ctx.iscript_hooks.is_empty() {
                // Heap gets leaked to keep the exec code alive
                let exec_heap = HeapCreate(HEAP_CREATE_ENABLE_EXECUTE, 0, 0);
                apply_aiscript_hooks(&mut exe, &ctx.aiscript_hooks, exec_heap);
                apply_iscript_hooks(&mut exe, &ctx.iscript_hooks, exec_heap);
            }
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
                    LAST_FILE_POINTER.get_or(|| Cell::new(0)).set(val as u64);
                }
                exe.call_hook(bw::SaveReady, save_hook);
                exe.hook_closure(bw::InitGame, |orig| {
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
                static ONCE: Once = Once::new();
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
        drop(exe);
        let file_read_hooks = FILE_READ_HOOKS.read();
        if !file_read_hooks.is_empty() {
            unsafe fn open_hook(
                archive: *mut c_void,
                filename: *const u8,
                flags: u32,
                out: *mut *mut c_void,
                orig: unsafe extern fn(*mut c_void, *const u8, u32, *mut *mut c_void) -> u32,
            ) -> u32 {
                let hooks = FILE_READ_HOOKS.read();
                let mut result = None;
                let mut any_called = false;
                let mut already_calling = false;
                for i in 0..hooks.len() {
                    let hook = &hooks[i];
                    if hook.matches(filename) {
                        let call_marker = hook.being_called.get_or(|| Cell::new(false));
                        if call_marker.get() == false {
                            any_called = true;
                            call_marker.set(true);
                            let mut size = 0u32;
                            let file = (hook.hook)(filename, &mut size);
                            if file != null_mut() {
                                result = Some(Box::new(HeapFreeOnDrop(file, size)));
                                break;
                            }
                        } else {
                            already_calling = true;
                        }
                    }
                }
                if any_called && !already_calling {
                    for hook in &*hooks {
                        let call_marker = hook.being_called.get_or(|| Cell::new(false));
                        call_marker.set(false);
                    }
                }
                if let Some(result) = result {
                    let raw = HeapFreeOnDropPtr(Box::into_raw(result));
                    let mut open_files = OPEN_HOOKED_FILES.lock();
                    HAS_HOOKED_FILES_OPEN.store(true, Ordering::Relaxed);
                    open_files.push(raw);
                    *out = raw.0 as *mut c_void;
                    1
                } else {
                    orig(archive, filename, flags, out)
                }
            }

            unsafe fn size_hook(
                file: *mut c_void,
                out_high: *mut u32,
                orig: unsafe extern fn(*mut c_void, *mut u32) -> u32,
            ) -> u32 {
                if HAS_HOOKED_FILES_OPEN.load(Ordering::Relaxed) {
                    let files = OPEN_HOOKED_FILES.lock();
                    for hooked in &*files {
                        if file == hooked.0 as *mut c_void {
                            if !out_high.is_null() {
                                *out_high = 0;
                            }
                            return (*hooked.0).1;
                        }
                    }
                }
                orig(file, out_high)
            }

            unsafe fn read_hook(
                file: *mut c_void,
                out: *mut u8,
                len: u32,
                out_len: *mut u32,
                overlapped: *mut c_void,
                orig: unsafe extern fn(*mut c_void, *mut u8, u32, *mut u32, *mut c_void) -> u32,
            ) -> u32 {
                if HAS_HOOKED_FILES_OPEN.load(Ordering::Relaxed) {
                    let files = OPEN_HOOKED_FILES.lock();
                    for hooked in &*files {
                        if file == hooked.0 as *mut c_void {
                            assert!(overlapped.is_null());
                            let len = ((*hooked.0).1).min(len);
                            std::ptr::copy_nonoverlapping((*hooked.0).0, out, len as usize);
                            *out_len = len;
                            return 1;
                        }
                    }
                }
                orig(file, out, len, out_len, overlapped)
            }

            unsafe fn close_hook(
                file: *mut c_void,
                orig: unsafe extern fn(*mut c_void),
            ) {
                if HAS_HOOKED_FILES_OPEN.load(Ordering::Relaxed) {
                    let mut files = OPEN_HOOKED_FILES.lock();
                    for i in 0..files.len() {
                        if files[i].0 as *mut c_void == file {
                            drop(Box::from_raw(files[i].0));
                            files.remove(i);
                            if files.is_empty() {
                                HAS_HOOKED_FILES_OPEN.store(false, Ordering::Relaxed);
                            }
                            return;
                        }
                    }
                }
                orig(file)
            }
            unsafe {
                let mut storm = patcher.patch_library("storm", 0x15000000);
                storm.hook_opt(bw::SFileOpenFileEx_Hook, open_hook);
                storm.hook_opt(bw::SFileGetFileSize_Hook, size_hook);
                storm.hook_opt(bw::SFileReadFile_Hook, read_hook);
                storm.hook_opt(bw::SFileCloseFile_Hook, close_hook);
            }
        }
    }
}

fn context() -> RefMut<'static, InternalContext> {
    RefMut::map(CONTEXT.get().unwrap().borrow_mut(), |x| x.as_mut().unwrap())
}

pub unsafe fn on_win_main(f: unsafe fn()) {
    let mut patcher = PATCHER.lock();
    let mut exe = patcher.patch_exe(0x00400000);
    bw::init_funcs(&mut exe);
    bw::init_vars(&mut exe);
    exe.call_hook(bw::WinMain, f);
}

pub fn init_1161() -> Context {
    unsafe {
        assert!(CONTEXT.get().is_none());
        let api = PluginApi {
            version: samase_plugin::VERSION,
            max_func_id: samase_plugin::MAX_FUNC_ID,
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
            map_tile_flags,
            players,
            hook_draw_image,
            hook_renderer,
            get_iscript_bin,
            set_iscript_bin,
            hook_iscript_opcode,
            sprite_hlines,
            sprite_hlines_end,
            hook_file_read,
            first_active_bullet,
            first_lone_sprite,
            add_overlay_iscript,
            set_campaigns,
            hook_run_dialog,
            send_command,
            ai_update_attack_target,
            update_visibility_point,
            create_lone_sprite,
            step_iscript,
            is_outside_game_screen,
            screen_pos,
            ui_scale,
            first_fow_sprite,
            is_replay,
            local_player_id,
            unit_array_len,
            draw_cursor_marker,
            hook_spawn_dialog,
            misc_ui_state,
            create_bullet,
            hook_create_bullet,
            create_unit,
            hook_create_unit,
            finish_unit_pre,
            finish_unit_post,
            get_sprite_position,
            set_sprite_position,
            hook_init_units,
            get_tooltip_draw_func,
            set_tooltip_draw_func,
            hook_layout_draw_text,
            hook_draw_graphic_layers,
            graphic_layers,
            set_prism_shaders,
            crash_with_message,
            ai_attack_prepare,
            hook_ai_step_region,
            extended_arrays,
            extended_dat,
            give_ai,
            hook_play_sound,
            is_multiplayer,
            hook_game_loop_start,
            active_iscript_objects,
            hook_ai_focus_disabled,
            hook_ai_focus_air,
            unit_base_strength,
            read_map_file,
            hook_func,
            get_func,
            load_vars,
            read_vars,
            write_vars,
            debug_ui_add_tab,
            debug_ui_add_log,
            debug_log_add_data,
            debug_log_clear,
            create_extended_unit_field,
            read_extended_unit_field,
            write_extended_unit_field,
        };
        let mut patcher = PATCHER.lock();
        {
            let mut storm = patcher.patch_library("storm", 0x15000000);
            bw::init_funcs_storm(&mut storm);
        }
        {
            unsafe fn init_mpqs_only_once(orig: unsafe extern fn()) {
                static ONCE: Once = Once::new();
                ONCE.call_once(|| orig());
            }

            let mut exe = patcher.patch_exe(0x00400000);
            bw::init_funcs(&mut exe);
            bw::init_funcs_cdecl(&mut exe);
            bw::init_vars(&mut exe);
            exe.hook_opt(bw::InitMpqs, init_mpqs_only_once);
        }

        CONTEXT.get_or(|| RefCell::new(Some(Default::default())));
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

unsafe extern fn unit_array_len() -> Option<unsafe extern fn(*mut *mut c_void, *mut usize)> {
    unsafe extern fn actual(out: *mut *mut c_void, len: *mut usize) {
        let first = &mut bw::units[0] as *mut bw::Unit as *mut c_void;
        *out = first;
        *len = 1700;
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
        bw::guard_ais.ptr() as *mut c_void
    }
    Some(actual)
}

unsafe extern fn pathing() -> Option<unsafe extern fn() -> *mut c_void> {
    unsafe extern fn actual() -> *mut c_void {
        *bw::pathing
    }
    Some(actual)
}

unsafe extern fn map_tile_flags() -> Option<unsafe extern fn() -> *mut u32> {
    unsafe extern fn actual() -> *mut u32 {
        *bw::map_tile_flags
    }
    Some(actual)
}

unsafe extern fn players() -> Option<unsafe extern fn() -> *mut c_void> {
    unsafe extern fn actual() -> *mut c_void {
        &mut bw::players[0] as *mut bw::Player as *mut c_void
    }
    Some(actual)
}

unsafe extern fn draw_cursor_marker() -> Option<unsafe extern fn(u32)> {
    unsafe extern fn actual(val: u32) {
        *bw::draw_cursor_marker = val as u8;
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

unsafe extern fn send_command() -> Option<unsafe extern fn(*const c_void, u32)> {
    unsafe extern fn actual(data: *const c_void, len: u32) {
        bw::send_command(data, len);
    }
    Some(actual)
}

unsafe extern fn ai_update_attack_target() ->
    Option<unsafe extern fn(*mut c_void, u32, u32, u32) -> u32>
{
    unsafe extern fn actual(unit: *mut c_void, a1: u32, a2: u32, a3: u32) -> u32 {
        bw::ai_update_attack_target(unit, a1, a2, a3)
    }
    Some(actual)
}

unsafe extern fn update_visibility_point() -> Option<unsafe extern fn(*mut c_void)> {
    unsafe extern fn actual(lone_sprite: *mut c_void) {
        bw::update_visibility_point(lone_sprite);
    }
    Some(actual)
}

unsafe extern fn create_lone_sprite() ->
    Option<unsafe extern fn(u32, i32, i32, u32) -> *mut c_void>
{
    unsafe extern fn actual(id: u32, x: i32, y: i32, player: u32) -> *mut c_void {
        bw::create_lone_sprite(id, x, y, player)
    }
    Some(actual)
}

unsafe extern fn create_bullet() ->
    Option<unsafe extern fn(u32, i32, i32, u32, u32, *mut c_void) -> *mut c_void>
{
    unsafe extern fn actual(
        id: u32,
        x: i32,
        y: i32,
        player: u32,
        direction: u32,
        parent: *mut c_void,
    ) -> *mut c_void {
        bw::create_bullet(id, x, y, player, direction, parent)
    }
    Some(actual)
}

unsafe extern fn create_unit() ->
    Option<unsafe extern fn(u32, i32, i32, u32, *const u8) -> *mut c_void>
{
    unsafe extern fn actual(
        id: u32,
        x: i32,
        y: i32,
        player: u32,
        _skins: *const u8,
    ) -> *mut c_void {
        bw::create_unit(id, x, y, player)
    }
    Some(actual)
}

unsafe extern fn hook_create_bullet(
    hook: unsafe extern fn(
        u32, i32, i32, u32, u32, *mut c_void,
        unsafe extern fn(u32, i32, i32, u32, u32, *mut c_void) -> *mut c_void,
    ) -> *mut c_void,
) -> u32 {
    context().create_bullet.push(hook);
    1
}

unsafe extern fn hook_create_unit(
    hook: unsafe extern fn(
        u32, i32, i32, u32, *const u8,
        unsafe extern fn(u32, i32, i32, u32, *const u8) -> *mut c_void,
    ) -> *mut c_void,
) -> u32 {
    context().create_unit.push(hook);
    1
}

unsafe extern fn finish_unit_pre() -> Option<unsafe extern fn(*mut c_void)> {
    unsafe extern fn actual(unit: *mut c_void) {
        bw::finish_unit_pre(unit);
    }
    Some(actual)
}

unsafe extern fn finish_unit_post() -> Option<unsafe extern fn(*mut c_void)> {
    unsafe extern fn actual(unit: *mut c_void) {
        bw::finish_unit_post(unit);
    }
    Some(actual)
}

unsafe extern fn get_sprite_position() -> Option<unsafe extern fn(*mut c_void, *mut u16)> {
    unsafe extern fn func(sprite: *mut c_void, pos: *mut u16) {
        *pos.add(0) = (*(sprite as *mut bw::Sprite)).position.x as u16;
        *pos.add(1) = (*(sprite as *mut bw::Sprite)).position.y as u16;
    }

    Some(func)
}

unsafe extern fn set_sprite_position() -> Option<unsafe extern fn(*mut c_void, *const u16)> {
    unsafe extern fn func(sprite: *mut c_void, pos: *const u16) {
        (*(sprite as *mut bw::Sprite)).position.x = *pos.add(0) as i16;
        (*(sprite as *mut bw::Sprite)).position.y = *pos.add(1) as i16;
    }

    Some(func)
}

unsafe extern fn ai_attack_prepare() -> Option<unsafe extern fn(u32, u32, u32, u32, u32) -> u32> {
    unsafe extern fn actual(player: u32, x: u32, y: u32, arg4: u32, arg5: u32) -> u32 {
        bw::ai_attack_prepare(player, x, y, arg4, arg5)
    }
    Some(actual)
}

unsafe extern fn give_ai() -> Option<unsafe extern fn(*mut c_void)> {
    unsafe extern fn actual(unit: *mut c_void) {
        bw::give_ai(unit);
    }
    Some(actual)
}

unsafe extern fn hook_init_units(hook: unsafe extern fn(unsafe extern fn())) -> u32 {
    context().init_units.push(hook);
    1
}

unsafe extern fn hook_ai_step_region(
    hook: unsafe extern fn(u32, u32, unsafe extern fn(u32, u32)),
) -> u32 {
    hook_func(FuncId::AiStepRegion as u16, hook as usize)
}

unsafe extern fn extended_arrays(
    out: *mut *mut samase_plugin::ExtendedArray,
) -> usize {
    *out = null_mut();
    0
}

unsafe extern fn get_tooltip_draw_func() ->
    Option<unsafe extern fn() -> Option<unsafe extern fn(*mut c_void)>>
{
    // Dunno how it works on 1161
    None
}

unsafe extern fn set_tooltip_draw_func() ->
    Option<unsafe extern fn(Option<unsafe extern fn(*mut c_void)>)>
{
    // Dunno how it works on 1161
    None
}

unsafe extern fn hook_layout_draw_text(
    _hook: unsafe extern fn(
        u32, u32, *const u8, *mut u32, u32, *mut u32, u32, u32,
        unsafe extern fn(u32, u32, *const u8, *mut u32, u32, *mut u32, u32, u32) -> *const u8,
    ) -> *const u8,
) -> u32 {
    0
}

unsafe extern fn hook_draw_graphic_layers(
    _hook: unsafe extern fn(u32, unsafe extern fn(u32)),
) -> u32 {
    0
}

unsafe extern fn graphic_layers() -> Option<unsafe extern fn() -> *mut c_void> {
    // Dunno how it works on 1161
    None
}

unsafe extern fn set_prism_shaders(
    _shader_type: u32,
    _id: u32,
    _data: *const u8,
    _size: u32,
) -> u32 {
    0
}

unsafe extern fn crash_with_message(msg: *const u8) -> ! {
    use std::path::Path;
    let path = if Path::new("errors").is_dir() {
        Path::new("errors/plugin_crash")
    } else {
        Path::new("plugin_crash")
    };
    let len = (0..).position(|i| *msg.add(i) == 0).unwrap();
    let slice = std::slice::from_raw_parts(msg, len);
    let _ = std::fs::write(path, slice);
    std::process::exit(5);
}

unsafe extern fn step_iscript() ->
    Option<unsafe extern fn(*mut c_void, *mut c_void, u32, *mut u32)>
{
    unsafe extern fn actual(
        image: *mut c_void,
        iscript: *mut c_void,
        dry_run: u32,
        speed_out: *mut u32,
    ) {
        bw::step_iscript(image, iscript, dry_run, speed_out)
    }
    Some(actual)
}

unsafe extern fn is_outside_game_screen() -> Option<unsafe extern fn(i32, i32) -> u32> {
    unsafe extern fn actual(x: i32, y: i32) -> u32 {
        bw::is_outside_game_screen(x, y)
    }
    Some(actual)
}

unsafe extern fn screen_pos() -> Option<unsafe extern fn(*mut i32, *mut i32)> {
    unsafe extern fn actual(x: *mut i32, y: *mut i32) {
        *x = *bw::screen_x;
        *y = *bw::screen_y;
    }
    Some(actual)
}

unsafe extern fn ui_scale() -> Option<unsafe extern fn() -> f32> {
    unsafe extern fn actual() -> f32 {
        1.0
    }
    Some(actual)
}

unsafe extern fn first_fow_sprite() -> Option<unsafe extern fn() -> *mut c_void> {
    unsafe extern fn actual() -> *mut c_void {
        *bw::first_fow_sprite
    }
    Some(actual)
}

unsafe extern fn is_replay() -> Option<unsafe extern fn() -> u32> {
    unsafe extern fn actual() -> u32 {
        *bw::is_replay
    }
    Some(actual)
}

unsafe extern fn is_multiplayer() -> Option<unsafe extern fn() -> u32> {
    unsafe extern fn actual() -> u32 {
        *bw::is_multiplayer as u32
    }
    Some(actual)
}

unsafe extern fn local_player_id() -> Option<unsafe extern fn() -> u32> {
    unsafe extern fn actual() -> u32 {
        *bw::local_player_id
    }
    Some(actual)
}

unsafe extern fn active_iscript_objects() ->
    Option<unsafe extern fn(*mut *mut c_void, *const *mut c_void)>
{
    unsafe extern fn actual(read: *mut *mut c_void, write: *const *mut c_void) {
        if !read.is_null() {
            *read.add(0) = *bw::active_iscript_flingy;
            *read.add(1) = *bw::active_iscript_unit;
            *read.add(2) = *bw::active_iscript_bullet;
        }
        if !write.is_null() {
            *bw::active_iscript_flingy = *write.add(0);
            *bw::active_iscript_unit = *write.add(1);
            *bw::active_iscript_bullet = *write.add(2);
        }
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
                    bw::$name.ptr() as *mut c_void
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

unsafe extern fn extended_dat(dat: u32) -> Option<unsafe extern fn(*mut usize) -> *mut c_void> {
    macro_rules! dat_fns {
        ($($name:ident, $len:expr,)*) => {
            $(
                unsafe extern fn $name(len: *mut usize) -> *mut c_void {
                    *len = $len;
                    bw::$name.ptr() as *mut c_void
                }
            )*
        }
    }
    dat_fns! {
        units_dat, 0x36,
        weapons_dat, 0x18,
        flingy_dat, 0x7,
        upgrades_dat, 0xc,
        techdata_dat, 0xb,
        sprites_dat, 0x6,
        images_dat, 0xe,
        orders_dat, 0x13,
        sfxdata_dat, 0x5,
        portdata_dat, 0x6,
    }
    let fun: unsafe extern fn(*mut usize) -> *mut c_void = match dat {
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

unsafe extern fn hook_draw_image(
    hook: unsafe extern fn(*mut c_void, unsafe extern fn(*mut c_void)),
) -> u32 {
    context().draw_image.push(hook);
    1
}

unsafe extern fn hook_run_dialog(
    hook: unsafe extern fn(
        *mut c_void,
        usize,
        *mut c_void,
        unsafe extern fn(*mut c_void, usize, *mut c_void) -> u32,
    ) -> u32,
) -> u32 {
    context().run_dialog.push(hook);
    1
}

unsafe extern fn hook_spawn_dialog(
    hook: unsafe extern fn(
        *mut c_void,
        usize,
        *mut c_void,
        unsafe extern fn(*mut c_void, usize, *mut c_void) -> u32,
    ) -> u32,
) -> u32 {
    context().spawn_dialog.push(hook);
    1
}

unsafe extern fn hook_play_sound(
    _hook: unsafe extern fn(
        u32,
        f32,
        *mut c_void,
        *mut i32,
        *mut i32,
        unsafe extern fn(u32, f32, *mut c_void, *mut i32, *mut i32) -> u32,
    ) -> u32,
) -> u32 {
    // 1161 function args aren't same as SCR
    0
}

unsafe extern fn hook_game_loop_start(
    _hook: unsafe extern fn(),
) -> u32 {
    // TODO
    0
}

unsafe extern fn hook_ai_focus_disabled(
    hook: unsafe extern fn(*mut c_void, unsafe extern fn(*mut c_void)),
) -> u32 {
    context().ai_focus_disabled.push(hook);
    1
}

unsafe extern fn hook_ai_focus_air(
    hook: unsafe extern fn(*mut c_void, unsafe extern fn(*mut c_void)),
) -> u32 {
    context().ai_focus_air.push(hook);
    1
}

unsafe extern fn unit_base_strength() -> Option<unsafe extern fn(*mut *mut u32)> {
    unsafe extern fn actual(out: *mut *mut u32) {
        *out = bw::unit_strength.mut_ptr() as *mut u32;
        *out.add(1) = (bw::unit_strength.mut_ptr() as *mut u32).add(0xe4);
    }
    Some(actual)
}

unsafe extern fn read_map_file() -> Option<unsafe extern fn(*const u8, *mut usize) -> *mut u8> {
    // TODO
    None
}

unsafe extern fn hook_func(id: u16, hook: usize) -> u32 {
    if id >= samase_plugin::MAX_FUNC_ID {
        return 0;
    }
    if get_func(id).is_some() {
        let func: samase_plugin::FuncId = mem::transmute(id as u8);
        context().func_hooks.push((func, hook));
        1
    } else {
        0
    }
}

#[allow(bad_style)]
unsafe extern fn get_func(id: u16) -> Option<unsafe extern fn()> {
    if id >= samase_plugin::MAX_FUNC_ID {
        return None;
    }

    unsafe extern fn UnitCanRally(a: usize) -> usize {
        bw::UnitCanRally(a)
    }
    unsafe extern fn UnitCanBeInfested(a: usize) -> usize {
        bw::UnitCanBeInfested(a)
    }
    unsafe extern fn DoMissileDamage(a: usize) -> usize {
        bw::DoMissileDamage(a)
    }
    unsafe extern fn HitUnit(a: usize, b: usize, c: usize) -> usize {
        bw::HitUnit(a, b, c)
    }
    unsafe extern fn HallucinationHit(a: usize, b: usize, c: usize, d: usize) -> usize {
        bw::HallucinationHit(a, b, c, d)
    }
    unsafe extern fn DamageUnit(a: usize, b: usize, c: usize, d: usize, e: usize) -> usize {
        bw::DamageUnit(a, b, c, d, e)
    }
    unsafe extern fn KillUnit(a: usize) -> usize {
        bw::KillUnit(a)
    }
    unsafe extern fn UnitSetHp(a: usize, b: usize) -> usize {
        bw::UnitSetHp(a, b)
    }
    unsafe extern fn TransformUnit(a: usize, b: usize) -> usize {
        bw::TransformUnit(a, b)
    }
    unsafe extern fn GiveUnit(a: usize, b: usize) -> usize {
        bw::GiveUnit(a, b)
    }
    unsafe extern fn PlaceCreepRect(a: usize, b: usize, c: usize, d: usize, e: usize) -> usize {
        bw::PlaceCreepRect(a, b, c, d, e)
    }
    unsafe extern fn PlaceFinishedUnitCreep(a: usize, b: usize, c: usize) -> usize {
        bw::PlaceFinishedUnitCreep(a, b, c)
    }
    unsafe extern fn AddAiToTrainedUnit(a: usize, b: usize) -> usize {
        bw::AddAiToTrainedUnit(a, b)
    }
    unsafe extern fn AddBuildingAi(a: usize, b: usize) -> usize {
        bw::AddBuildingAi(a, b)
    }
    unsafe extern fn AddTownUnitAi(a: usize, b: usize) -> usize {
        bw::AddTownUnitAi(a, b)
    }
    unsafe extern fn AddMilitaryAi(a: usize, b: usize, c: usize) -> usize {
        bw::AddMilitaryAi(a, b, c)
    }
    unsafe extern fn AiRemoveUnit(a: usize, b: usize) -> usize {
        bw::AiRemoveUnit(a, b)
    }
    unsafe extern fn AiRemoveUnitMilitary(a: usize, b: usize) -> usize {
        bw::AiRemoveUnitMilitary(a, b)
    }
    unsafe extern fn AiRemoveUnitTown(a: usize, b: usize) -> usize {
        bw::AiRemoveUnitTown(a, b)
    }
    unsafe extern fn UnitMaxEnergy(a: usize) -> usize {
        bw::UnitMaxEnergy(a)
    }
    unsafe extern fn UnitAttackRange(a: usize, b: usize) -> usize {
        bw::UnitAttackRange(a, b)
    }
    unsafe extern fn UnitTargetAcquisitionRange(a: usize) -> usize {
        bw::UnitTargetAcquisitionRange(a)
    }
    unsafe extern fn UnitSightRange(a: usize, b: usize) -> usize {
        bw::UnitSightRange(a, b)
    }
    unsafe extern fn CheckWeaponTargetingFlags(a: usize, b: usize, c: usize) -> usize {
        bw::CheckWeaponTargetingFlags(a, b, c)
    }
    unsafe extern fn CheckTechTargeting(
        a: usize, b: usize, c: usize, d: usize, e: usize, f: usize, g: usize
    ) -> usize {
        bw::CheckTechTargeting(a, b, c, d, e, f, g)
    }
    unsafe extern fn CheckOrderTargeting(
        a: usize, b: usize, c: usize, d: usize, e: usize, f: usize,
    ) -> usize {
        bw::CheckOrderTargeting(a, b, c, d, e, f)
    }
    unsafe extern fn CheckFowOrderTargeting(
        a: usize, b: usize, c: usize, d: usize, e: usize, f: usize,
    ) -> usize {
        bw::CheckFowOrderTargeting(a, b, c, d, e, f)
    }
    unsafe extern fn HideUnit(a: usize) -> usize {
        bw::HideUnit(a)
    }
    unsafe extern fn ShowUnit(a: usize) -> usize {
        bw::ShowUnit(a)
    }
    unsafe extern fn AiAddMilitaryToRegion(a: usize, b: usize, c: usize) -> usize {
        bw::AiAddMilitaryToRegion(a, b, c)
    }
    unsafe extern fn AiAttackPrepare(a: usize, b: usize, c: usize, d: usize, e: usize) -> usize {
        bw::AiAttackPrepare(a, b, c, d, e)
    }
    unsafe extern fn AiTrainMilitary(a: usize) -> usize {
        bw::AiTrainMilitary(a)
    }
    unsafe extern fn AiRegionUpdateStrength(a: usize) -> usize {
        bw::AiRegionUpdateStrength(a)
    }
    unsafe extern fn AiRegionUpdateTarget(a: usize) -> usize {
        bw::AiRegionUpdateTarget(a)
    }
    unsafe extern fn AiRegionAbandonIfOverwhelmed(a: usize) -> usize {
        bw::AiRegionAbandonIfOverwhelmed(a)
    }
    unsafe extern fn AiRegionPickAttackTarget(a: usize) -> usize {
        bw::AiRegionPickAttackTarget(a)
    }
    unsafe extern fn AiStepRegion(a: usize, b: usize) -> usize {
        bw::AiStepRegion(a, b)
    }
    unsafe extern fn AiTargetExpansion(a: usize) -> usize {
        bw::AiTargetExpansion(a)
    }
    unsafe extern fn StepUnitTimers(a: usize) -> usize {
        bw::StepUnitTimers(a)
    }
    unsafe extern fn StartCloaking(a: usize) -> usize {
        bw::StartCloaking(a)
    }
    unsafe extern fn UnitAiMilitary(a: usize) -> usize {
        bw::UnitAiMilitary(a)
    }
    unsafe extern fn UnitAiWorker(a: usize) -> usize {
        bw::UnitAiWorker(a)
    }
    unsafe extern fn AiTryProgressSpendingRequest(a: usize) -> usize {
        bw::AiTryProgressSpendingRequest(a)
    }
    unsafe extern fn CanAttackUnit(a: usize, b: usize, c: usize) -> usize {
        bw::CanAttackUnit(a, b, c)
    }
    unsafe extern fn IsOutsideAttackRange(a: usize, b: usize) -> usize {
        bw::IsOutsideAttackRange(a, b)
    }
    unsafe extern fn AiCanTargetAttackThis(a: usize, b: usize) -> usize {
        bw::AiCanTargetAttackThis(a, b)
    }
    unsafe extern fn AiTryReturnHome(a: usize, b: usize) -> usize {
        bw::AiTryReturnHome(a, b)
    }
    unsafe extern fn PrepareBuildUnit(a: usize, b: usize) -> usize {
        bw::PrepareBuildUnit(a, b)
    }
    unsafe extern fn CalculatePath(a: usize) -> usize {
        bw::CalculatePath(a)
    }
    unsafe extern fn AiPlaceBuilding(a: usize, b: usize, c: usize, d: usize, e: usize) -> usize {
        bw::AiPlaceBuilding(a, b, c, d, e)
    }
    unsafe extern fn GetChokePointRegions(
        a: usize, b: usize, c: usize, d: usize, e: usize, f: usize,
    ) -> usize {
        bw::GetChokePointRegions(a, b, c, d, e, f)
    }
    unsafe extern fn AiUpdateBuildingPlacementState(
        a: usize, b: usize, c: usize, d: usize, e: usize,
    ) -> usize {
        bw::AiUpdateBuildingPlacementState(a, b, c, d, e)
    }
    unsafe extern fn UpdateBuildingPlacementState(
        a: usize, b: usize, c: usize, d: usize, e: usize, f: usize, g: usize, h: usize, i: usize,
    ) -> usize {
        bw::UpdateBuildingPlacementState(a, b, c, d, e, f, g, h, i)
    }
    unsafe extern fn ForEachUnitInArea(
        area: *mut u16,
        func: unsafe extern fn(usize, *mut c_void) -> u32,
        param: *mut c_void,
    ) -> usize {
        let mut units = bw::FindUnitsRect(area);
        let result = loop {
            let unit = *units;
            if unit == 0 {
                break 0;
            }
            if func(unit, param) != 0 {
                break unit;
            }
            units = units.add(1);
        };
        let count = 0x6bee6c as *mut u32;
        let unit_count = *count.add(*count as usize);
        *count = *count - 1;
        *(0x6bee64 as *mut u32) = unit_count;
        result
    }

    let func: samase_plugin::FuncId = mem::transmute(id as u8);
    let value = match func {
        FuncId::UnitCanRally => UnitCanRally as usize,
        FuncId::UnitCanBeInfested => UnitCanBeInfested as usize,
        FuncId::DoMissileDamage => DoMissileDamage as usize,
        FuncId::HitUnit => HitUnit as usize,
        FuncId::HallucinationHit => HallucinationHit as usize,
        FuncId::DamageUnit => DamageUnit as usize,
        FuncId::KillUnit => KillUnit as usize,
        FuncId::UnitSetHp => UnitSetHp as usize,
        FuncId::TransformUnit => TransformUnit as usize,
        FuncId::GiveUnit => GiveUnit as usize,
        FuncId::PlaceCreepRect => PlaceCreepRect as usize,
        FuncId::PlaceFinishedUnitCreep => PlaceFinishedUnitCreep as usize,
        FuncId::AddAiToTrainedUnit => AddAiToTrainedUnit as usize,
        FuncId::AddBuildingAi => AddBuildingAi as usize,
        FuncId::AddTownUnitAi => AddTownUnitAi as usize,
        FuncId::AddMilitaryAi => AddMilitaryAi as usize,
        FuncId::AiRemoveUnit => AiRemoveUnit as usize,
        FuncId::AiRemoveUnitMilitary => AiRemoveUnitMilitary as usize,
        FuncId::AiRemoveUnitTown => AiRemoveUnitTown as usize,
        FuncId::UnitMaxEnergy => UnitMaxEnergy as usize,
        FuncId::UnitAttackRange => UnitAttackRange as usize,
        FuncId::UnitTargetAcquisitionRange => UnitTargetAcquisitionRange as usize,
        FuncId::UnitSightRange => UnitSightRange as usize,
        FuncId::CheckWeaponTargetingFlags => CheckWeaponTargetingFlags as usize,
        FuncId::CheckTechTargeting => CheckTechTargeting as usize,
        FuncId::CheckOrderTargeting => CheckOrderTargeting as usize,
        FuncId::CheckFowOrderTargeting => CheckFowOrderTargeting as usize,
        FuncId::HideUnit => HideUnit as usize,
        FuncId::ShowUnit => ShowUnit as usize,
        FuncId::AiAddMilitaryToRegion => AiAddMilitaryToRegion as usize,
        FuncId::AiTrainMilitary => AiTrainMilitary as usize,
        FuncId::AiAttackPrepare => AiAttackPrepare as usize,
        FuncId::AiAttackClear => 0,
        FuncId::AiRegionUpdateStrength => AiRegionUpdateStrength as usize,
        FuncId::AiRegionUpdateTarget => AiRegionUpdateTarget as usize,
        FuncId::AiRegionAbandonIfOverwhelmed => AiRegionAbandonIfOverwhelmed as usize,
        FuncId::AiRegionPickAttackTarget => AiRegionPickAttackTarget as usize,
        FuncId::AiStepRegion => AiStepRegion as usize,
        FuncId::AiTargetExpansion => AiTargetExpansion as usize,
        FuncId::StepUnitTimers => StepUnitTimers as usize,
        FuncId::StartCloaking => StartCloaking as usize,
        FuncId::UnitAiWorker => UnitAiWorker as usize,
        FuncId::UnitAiMilitary => UnitAiMilitary as usize,
        FuncId::UnitAiBuilding => AiTryProgressSpendingRequest as usize,
        FuncId::AiCanTargetAttackThis => AiCanTargetAttackThis as usize,
        FuncId::AiTryReturnHome => AiTryReturnHome as usize,
        FuncId::CanAttackUnit => CanAttackUnit as usize,
        FuncId::IsOutsideAttackRange => IsOutsideAttackRange as usize,
        FuncId::ForEachUnitInArea => ForEachUnitInArea as usize,
        FuncId::PrepareBuildUnit => PrepareBuildUnit as usize,
        FuncId::CalculatePath => CalculatePath as usize,
        FuncId::AiPlaceBuilding => AiPlaceBuilding as usize,
        FuncId::GetChokePointRegions => GetChokePointRegions as usize,
        FuncId::AiUpdateBuildingPlacementState => AiUpdateBuildingPlacementState as usize,
        FuncId::UpdateBuildingPlacementState => UpdateBuildingPlacementState as usize,
        FuncId::FindNearestUnitInArea | FuncId::FindNearestUnitAroundUnit |
            FuncId::FindNearestUnitInAreaPoint => 0,
        FuncId::AiPickBestPlacementPosition | FuncId::AiPlacementFlags => 0,
        FuncId::GetRenderTarget => 0,
        FuncId::MoveScreen => 0,
        FuncId::SelectUnits => 0,
        FuncId::StepGameLogic => 0,
        FuncId::_Last => 0,
    };
    mem::transmute(value)
}

fn var_result(var: VarId) -> u8 {
    match var {
        // Structures, assigning usize to struct isn't sensible
        VarId::Game | VarId::AiRegions | VarId::PlayerAi |
            VarId::FirstGuardAi | VarId::ActiveAiTowns | VarId::Selections |
            VarId::ClientSelection | VarId::Players | VarId::SpriteHlines |
            VarId::SpriteHlinesEnd | VarId::GameData | VarId::ReplayData | VarId::ReplayHeader |
            VarId::FirstPlayerUnit | VarId::Units | VarId::ResourceAreas | VarId::MainPalette => 2,
        // Writable variables
        VarId::RngEnable | VarId::FirstActiveUnit | VarId::FirstHiddenUnit |
            VarId::FirstAiScript | VarId::ScMainState | VarId::CommandUser |
            VarId::UniqueCommandUser | VarId::IsReplay | VarId::LocalPlayerId |
            VarId::LocalUniquePlayerId | VarId::IsMultiplayer | VarId::FirstFreeAiScript |
            VarId::Pathing | VarId::LoadedSave | VarId::MapTileFlags | VarId::IscriptBin |
            VarId::FirstActiveBullet | VarId::FirstLoneSprite | VarId::ScreenX |
            VarId::ScreenY | VarId::FirstFowSprite | VarId::DrawCursorMarker |
            VarId::IsPaused | VarId::IsTargeting | VarId::DatRequirementError |
            VarId::IsPlacingBuilding | VarId::UnitShouldRevealArea |
            VarId::ActiveIscriptFlingy | VarId::ActiveIscriptUnit |
            VarId::ActiveIscriptBullet | VarId::RngSeed | VarId::LastLoneSprite |
            VarId::FirstFreeLoneSprite | VarId::LastFreeLoneSprite | VarId::LastFowSprite |
            VarId::FirstFreeFowSprite | VarId::LastFreeFowSprite | VarId::CursorMarker |
            VarId::FirstDialog | VarId::TilesetIndexedMapTiles | VarId::TilesetCv5 |
            VarId::MinitileData | VarId::CreepOriginalTiles | VarId::CreepTileBorders => 3,
        // Unsure / SC:R only
        VarId::Zoom | VarId::TooltipDrawFunc | VarId::GraphicLayers |
            VarId::CmdIconsDdsGrp | VarId::CmdBtnsDdsGrp | VarId::StatusScreenMode |
            VarId::Allocator | VarId::UnitsVector | VarId::DrawCommands | VarId::VertexBuffer |
            VarId::Renderer | VarId::UseRgbColors | VarId::RgbColors | VarId::GameScreenWidthBwpx |
            VarId::GameScreenHeightBwpx | VarId::StepGameFrames |
            VarId::StatportTalkingPortraitActive => 0,
        VarId::_Last => 1,
    }
}

fn var_addr_size(var: VarId) -> (usize, u32) {
    match var {
        VarId::Game => (0x0057F0F0, 0),
        VarId::AiRegions => (0x0069A604, 0),
        VarId::PlayerAi => (0x0068FEE8, 0),
        VarId::Units => (0x0059CCA8, 0),
        VarId::FirstGuardAi => (0x00685108, 0),
        VarId::ActiveAiTowns => (0x006AA050, 0),
        VarId::Selections => (0x006284E8, 0),
        VarId::ClientSelection => (0x00597208, 0),
        VarId::Players => (0x0057EEE0, 0),
        VarId::SpriteHlines => (0x00629688, 0),
        VarId::SpriteHlinesEnd => (0x00629288, 0),
        VarId::GameData => (0x006D0F44, 0),
        VarId::ReplayData => (0x00596BBC, 0),
        VarId::ReplayHeader => (0x006D0F30, 0),
        VarId::FirstPlayerUnit => (0x006283F8, 0),
        VarId::ResourceAreas => (0x00692688, 0),
        // Writable variables
        VarId::RngEnable => (0x006D11C8, 4),
        VarId::FirstActiveUnit => (0x00628430, 4),
        VarId::FirstHiddenUnit => (0x006283EC, 4),
        VarId::FirstAiScript => (0x0068C100, 4),
        VarId::FirstFreeAiScript => (0x0068C0F8, 4),
        VarId::ScMainState => (0x00596904, 4),
        VarId::CommandUser => (0x00512678, 4),
        VarId::UniqueCommandUser => (0x0051267C, 4),
        VarId::IsReplay => (0x006D0F14, 4),
        VarId::LocalPlayerId => (0x00512684, 4),
        VarId::LocalUniquePlayerId => (0x00512688, 4),
        VarId::IsMultiplayer => (0x0057F0B4, 1),
        VarId::Pathing => (0x006D5BFC, 4),
        VarId::LoadedSave => (0x006D1218, 4),
        VarId::MapTileFlags => (0x006D1260, 4),
        VarId::IscriptBin => (0x006D1200, 4),
        VarId::FirstActiveBullet => (0x0064DEC4, 4),
        VarId::FirstLoneSprite => (0x00654874, 4),
        VarId::ScreenX => (0x0062848C, 4),
        VarId::ScreenY => (0x006284A8, 4),
        VarId::FirstFowSprite => (0x00654868, 4),
        VarId::DrawCursorMarker => (0x00652920, 1),
        VarId::IsPaused => (0x006509C4, 4),
        VarId::IsTargeting => (0x00641694, 1),
        VarId::DatRequirementError => (0x0066FF60, 4),
        VarId::IsPlacingBuilding => (0x00640880, 4),
        VarId::UnitShouldRevealArea => (0x0066FF70, 4),
        VarId::ActiveIscriptFlingy => (0x006D11F4, 4),
        VarId::ActiveIscriptUnit => (0x006D11FC, 4),
        VarId::ActiveIscriptBullet => (0x006D11F8, 4),
        VarId::RngSeed => (0x0051CA14, 4),
        VarId::LastLoneSprite => (0x0065291C, 4),
        VarId::FirstFreeLoneSprite => (0x00654878, 4),
        VarId::LastFreeLoneSprite => (0x0065487C, 4),
        VarId::LastFowSprite => (0x0065486C, 4),
        VarId::FirstFreeFowSprite => (0x006509D0, 4),
        VarId::LastFreeFowSprite => (0x00654870, 4),
        VarId::CursorMarker => (0x00652918, 4),
        VarId::TilesetCv5 => (0x006D5EC8, 4),
        VarId::TilesetIndexedMapTiles => (0x005993C4, 4),
        VarId::MinitileData => (0x005993D0, 4),
        VarId::CreepOriginalTiles => (0x006D0C68, 4),
        VarId::CreepTileBorders => (0x006D0E80, 4),
        VarId::FirstDialog => (0x006D5E34, 0),
        VarId::MainPalette => (0x005994E0, 0),
        // Unsure / SC:R only
        VarId::Zoom | VarId::TooltipDrawFunc | VarId::GraphicLayers |
            VarId::CmdIconsDdsGrp | VarId::CmdBtnsDdsGrp | VarId::StatusScreenMode |
            VarId::Allocator | VarId::UnitsVector | VarId::DrawCommands | VarId::VertexBuffer |
            VarId::Renderer | VarId::UseRgbColors | VarId::RgbColors | VarId::GameScreenWidthBwpx |
            VarId::GameScreenHeightBwpx | VarId::StepGameFrames |
            VarId::StatportTalkingPortraitActive => (0, 0),
        VarId::_Last => (0, 0),
    }
}

unsafe extern fn load_vars(vars: *const u16, results: *mut u8, len: usize) {
    let vars = std::slice::from_raw_parts(vars, len);
    let results = std::slice::from_raw_parts_mut(results, len);
    for i in 0..len {
        if vars[i] >= samase_plugin::MAX_VAR_ID {
            results[i] = 1;
        } else {
            let id: VarId = mem::transmute(vars[i]);
            let result = var_result(id);
            results[i] = result;
        }
    }
}

unsafe extern fn read_vars(vars: *const u16, results: *mut usize, len: usize) {
    let vars = std::slice::from_raw_parts(vars, len);
    let results = std::slice::from_raw_parts_mut(results, len);
    for i in 0..len {
        if vars[i] >= samase_plugin::MAX_VAR_ID {
            results[i] = 0;
        } else {
            let id: VarId = mem::transmute(vars[i]);
            let (result, size) = var_addr_size(id);
            match size {
                1 => results[i] = *(result as *mut u8) as usize,
                2 => results[i] = *(result as *mut u16) as usize,
                4 => results[i] = *(result as *mut u32) as usize,
                _ => results[i] = result,
            }
        }
    }
}

unsafe extern fn write_vars(vars: *const u16, values: *const usize, len: usize) {
    let vars = std::slice::from_raw_parts(vars, len);
    let values = std::slice::from_raw_parts(values, len);
    for i in 0..len {
        if vars[i] < samase_plugin::MAX_VAR_ID {
            let id: VarId = mem::transmute(vars[i]);
            let (result, size) = var_addr_size(id);
            match size {
                1 => *(result as *mut u8) = values[i] as u8,
                2 => *(result as *mut u16) = values[i] as u16,
                4 => *(result as *mut u32) = values[i] as u32,
                _ => (),
            }
        }
    }
}

pub unsafe extern fn debug_ui_add_tab(
    _tab: *const samase_plugin::FfiStr,
    _subtab: *const samase_plugin::FfiStr,
    _cb: samase_plugin::DebugUiDrawCb,
    _ctx: *mut c_void,
) -> usize {
    0
}

pub unsafe extern fn debug_ui_add_log() -> *mut samase_plugin::DebugUiLog {
    null_mut()
}

pub unsafe extern fn debug_log_add_data(
    _log: *mut samase_plugin::DebugUiLog,
    _format: *const samase_plugin::FfiStr,
    _param: *const samase_plugin::ComplexLineParam,
    _param_count: usize,
    _unused_extra: *mut c_void,
) {
}

pub unsafe extern fn debug_log_clear(
    _log: *mut samase_plugin::DebugUiLog,
) {
}

unsafe extern fn create_extended_unit_field(_: *const samase_plugin::FfiStr) -> u32 {
    // Not implemented
    0
}

unsafe extern fn read_extended_unit_field(_: u32, _: u32) -> u32 {
    // Not implemented
    0
}

unsafe extern fn write_extended_unit_field(_: u32, _: u32, _: u32) -> u32 {
    // Not implemented
    0
}

unsafe extern fn misc_ui_state(out_size: usize) -> Option<unsafe extern fn(*mut u8)> {
    static OUT_SIZE: AtomicUsize = AtomicUsize::new(0);
    unsafe extern fn actual(out: *mut u8) {
        // NOTE: Leaving open for future updates with larger out_size but not assuming alingment
        // on out
        let out_size = OUT_SIZE.load(Ordering::Acquire);
        let val = [
            *bw::is_paused as u8,
            *bw::is_targeting,
            *bw::is_placing_building as u8,
        ];
        let out = std::slice::from_raw_parts_mut(out, out_size);
        for (value, out) in val.iter().cloned().zip(out.iter_mut()) {
            *out = value;
        }
    }

    if out_size > 3 || out_size == 0 {
        return None;
    }
    OUT_SIZE.store(out_size, Ordering::Release);
    Some(actual)
}

unsafe extern fn hook_renderer(
    _type: u32,
    _hook: unsafe extern fn(),
) -> u32 {
    0
}

unsafe extern fn get_iscript_bin() -> Option<unsafe extern fn() -> *mut c_void> {
    unsafe extern fn actual() -> *mut c_void {
        *bw::iscript_bin
    }
    Some(actual)
}

unsafe extern fn set_iscript_bin() -> Option<unsafe extern fn(*mut c_void)> {
    unsafe extern fn actual(value: *mut c_void) {
        *bw::iscript_bin = value
    }
    Some(actual)
}

unsafe extern fn hook_iscript_opcode(
    opcode: u32,
    hook: unsafe extern fn(*mut c_void, *mut c_void, *mut c_void, u32, *mut u32),
) -> u32 {
    if opcode < 0x100 {
        context().iscript_hooks.push((opcode as u8, hook));
        1
    } else {
        0
    }
}

unsafe extern fn sprite_hlines() -> Option<unsafe extern fn() -> *mut *mut c_void> {
    unsafe extern fn actual() -> *mut *mut c_void {
        &mut bw::sprite_hlines[0] as *mut *mut c_void
    }
    Some(actual)
}

unsafe extern fn sprite_hlines_end() -> Option<unsafe extern fn() -> *mut *mut c_void> {
    unsafe extern fn actual() -> *mut *mut c_void {
        &mut bw::sprite_hlines_end[0] as *mut *mut c_void
    }
    Some(actual)
}

unsafe extern fn first_active_bullet() -> Option<unsafe extern fn() -> *mut c_void> {
    unsafe extern fn actual() -> *mut c_void {
        *bw::first_active_bullet
    }
    Some(actual)
}

unsafe extern fn first_lone_sprite() -> Option<unsafe extern fn() -> *mut c_void> {
    unsafe extern fn actual() -> *mut c_void {
        *bw::first_lone_sprite
    }
    Some(actual)
}

unsafe extern fn add_overlay_iscript() ->
    Option<unsafe extern fn(*mut c_void, u32, i32, i32, u32) -> *mut c_void>
{
    unsafe extern fn actual(
        image: *mut c_void,
        image_id: u32,
        x: i32,
        y: i32,
        above: u32,
    ) -> *mut c_void {
        bw::add_overlay_iscript(image as *mut bw::Image, image_id, x, y, above) as *mut c_void
    }
    Some(actual)
}

unsafe extern fn set_campaigns(val: *const *mut c_void) -> u32 {
    write_exe_memory(&bw::campaigns[0] as *const *mut c_void as usize, val as *const u8, 6 * 4);
    1
}

unsafe extern fn hook_file_read(
    prefix: *const u8,
    hook: unsafe extern fn(*const u8, *mut u32) -> *mut u8,
) {
    let prefix = CStr::from_ptr(prefix as *const i8).to_bytes().into();
    FILE_READ_HOOKS.write().push(FileReadHook {
        prefix,
        hook,
        being_called: ThreadLocal::new(),
    });
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

    static INGAME_COMMAND_HOOK: Once = Once::new();
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
                add_to_replay_data,
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

unsafe extern fn add_to_replay_data(data: *const u8, length: usize) {
    // No code to support replay data for extended commands :l
    if *data.add(0) < 0x5d {
        bw::add_to_replay_data(*bw::replay_data, *bw::storm_command_user, data, length);
    }
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
    exec_heap: HANDLE,
) {
    if hooks.is_empty() {
        return;
    }
    // Going to set last as !0 so other plugins using this same shim can use it
    // to count patched switch table length
    let unpatched_switch_table =
        *bw::aiscript_switch_table_ptr == bw::aiscript_default_switch_table.ptr() as *mut u32;
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
        asm.write_u32::<LE>(fun as usize as u32).unwrap();
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
    let exec_asm = exec_alloc(exec_heap, asm.len());
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

unsafe fn apply_iscript_hooks(
    exe: &mut whack::ModulePatcher,
    hooks: &[(u8, unsafe extern fn(*mut c_void, *mut c_void, *mut c_void, u32, *mut u32))],
    exec_heap: HANDLE,
) {
    if hooks.is_empty() {
        return;
    }
    // Going to set last as !0 so other plugins using this same shim can use it
    // to count patched switch table length
    let unpatched_switch_table =
        *bw::iscript_switch_table_ptr == bw::iscript_default_switch_table.ptr() as *mut u32;
    let old_opcode_count = if unpatched_switch_table {
        0x45
    } else {
        let switch_table = *bw::iscript_switch_table_ptr;
        (0u32..).find(|&x| *switch_table.offset(x as isize) == !0).unwrap()
    };
    let opcode_count =
        hooks.iter().map(|x| x.0 as u32 + 1).max().unwrap_or(0).max(old_opcode_count);
    let mut switch_table = vec![0; opcode_count as usize + 2];
    switch_table[opcode_count as usize + 1] = !0;
    for i in 0..old_opcode_count {
        let old_switch_table = *bw::iscript_switch_table_ptr;
        switch_table[i as usize] = *old_switch_table.offset(i as isize);
    }

    let mut asm_offsets = Vec::with_capacity(hooks.len());
    let mut asm = Vec::new();
    for &(opcode, fun) in hooks {
        asm_offsets.push((opcode, asm.len()));
        asm.write_all(&[
            0x60, // pushad
            0xff, 0x75, 0x10, // push [ebp + c] (out_speed)
            0xff, 0x75, 0x0c, // push [ebp + c] (dry run)
            0x56, // push esi (image)
            0xff, 0x75, 0x08, // push [ebp + 8] (iscript struct)
            0x57, // push edi (iscript_bin)
            0xb8, // mov eax, ...
        ]).unwrap();
        asm.write_u32::<LE>(fun as usize as u32).unwrap();
        asm.write_all(&[
            0xff, 0xd0, // call eax
            0x83, 0xc4, 0x14, // add esp, 14
            0x8b, 0x4d, 0x08, // mov ecx, [ebp + 8] (restore iscript struct)
            0x8a, 0x51, 0x07, // mov dl, byte [ecx + 7] (script wait)
            0xfe, 0xca, // dec dl
            0x31, 0xdb, // xor ebx, ebx
            0x4b, // dec ebx
            0x38, 0xda, // cmp dl, bl
            0x74, 0x0d, // je wait not set
            0x88, 0x51, 0x07, // mov [ecx + 7], dl
            0x61, // popad
            0x5f, // pop edi
            0x5e, // pop esi
            0x5b, // pop ebx
            0x8b, 0xe5, // mov esp, ebp
            0x5d, // pop ebp
            0xc2, 0x0c, 0x00, // ret 0c
            // wait_not_set:
            0xa1, 0x00, 0x12, 0x6d, 0x00, // mov eax [6d1200] (iscript_bin)
            0x0f, 0xb7, 0x79, 0x02, // movzx edi word [ecx + 2] (script pos)
            0x03, 0xf8, // add edi, eax
            0x89, 0x7d, 0xf8, // mov [ebp - 8], edi
            0x61, // popad
            0xc7, 0x44, 0xe4, 0xfc, // Mov [esp - 4], dword ...
        ]).unwrap();
        asm.write_u32::<LE>(bw::ISCRIPT_LOOP as u32).unwrap();
        // jmp dword [esp - 4]
        asm.write_all(&[0xff, 0x64, 0xe4, 0xfc]).unwrap();
    }
    let exec_asm = exec_alloc(exec_heap, asm.len());
    std::ptr::copy_nonoverlapping(asm.as_ptr(), exec_asm, asm.len());
    for (opcode, offset) in asm_offsets {
        switch_table[opcode as usize] = exec_asm as u32 + offset as u32;
    }

    let opcode_count_patch = [0x90, 0x3c, opcode_count as u8];
    exe.replace(bw::ISCRIPT_OPCODE_CMP, &opcode_count_patch);
    let switch_table_ptr = switch_table.as_ptr() as u32;
    mem::forget(switch_table);
    exe.replace_val(bw::ISCRIPT_SWITCH_TABLE, switch_table_ptr);
}
