#[macro_use] extern crate log;

pub mod commands;
pub mod save;

use std::ptr::{null_mut};

use libc::c_void;

use crate::commands::{CommandLength, IngameCommandHook};
use crate::save::{SaveHook, LoadHook};

pub const VERSION: u16 = 43;
pub const MAX_FUNC_ID: u16 = FuncId::_Last as u16;
pub const MAX_VAR_ID: u16 = VarId::_Last as u16;

#[repr(C)]
pub struct ExtendedArray {
    pub pointer: *mut u8,
    pub end: *mut u8,
    pub unused: usize,
    pub unused2: usize,
}

#[repr(C)]
pub struct PluginApi {
    pub version: u16,
    pub max_func_id: u16,
    pub free_memory: unsafe extern "C" fn(*mut u8),
    pub write_exe_memory: unsafe extern "C" fn(usize, *const u8, usize) -> u32,
    pub warn_unsupported_feature: unsafe extern "C" fn(*const u8),
    pub read_file: unsafe extern "C" fn() -> unsafe extern "C" fn(*const u8, *mut usize) -> *mut u8,
    pub game: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut c_void>,
    pub rng_seed: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> u32>,
    pub hook_step_objects: unsafe extern "C" fn(unsafe extern "C" fn(), u32) -> u32,
    pub hook_aiscript_opcode: unsafe extern "C" fn(u32, unsafe extern "C" fn(*mut c_void)) -> u32,
    pub ai_regions: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut c_void>,
    pub player_ai: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut c_void>,
    pub get_region: unsafe extern "C" fn() -> Option<unsafe extern "C" fn(u32, u32) -> u32>,
    pub change_ai_region_state: unsafe extern "C" fn() -> Option<unsafe extern "C" fn(*mut c_void, u32)>,
    pub first_active_unit: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut c_void>,
    pub first_hidden_unit: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut c_void>,
    // self, order, x, y, target, fow_unit
    pub issue_order: unsafe extern "C" fn() ->
        Option<unsafe extern "C" fn(*mut c_void, u32, u32, u32, *mut c_void, u32)>,
    pub print_text: unsafe extern "C" fn() -> Option<unsafe extern "C" fn(*const u8)>,
    pub hook_on_first_file_access: unsafe extern "C" fn(unsafe extern "C" fn()),
    pub hook_step_order:
        unsafe extern "C" fn(unsafe extern "C" fn(*mut c_void, unsafe extern "C" fn(*mut c_void))) -> u32,
    pub hook_step_order_hidden:
        unsafe extern "C" fn(unsafe extern "C" fn(*mut c_void, unsafe extern "C" fn(*mut c_void))) -> u32,
    pub dat: unsafe extern "C" fn(u32) -> Option<unsafe extern "C" fn() -> *mut c_void>,
    pub hook_process_commands: unsafe extern "C" fn(
        unsafe extern "C" fn(*const c_void, u32, u32, unsafe extern "C" fn(*const c_void, u32, u32))
    ) -> u32,
    pub hook_process_lobby_commands: unsafe extern "C" fn(
        unsafe extern "C" fn(*const c_void, u32, u32, unsafe extern "C" fn(*const c_void, u32, u32))
    ) -> u32,
    pub hook_send_command: unsafe extern "C" fn(
        unsafe extern "C" fn(*mut c_void, u32, unsafe extern "C" fn(*mut c_void, u32))
    ) -> u32,
    pub hook_step_secondary_order:
        unsafe extern "C" fn(unsafe extern "C" fn(*mut c_void, unsafe extern "C" fn(*mut c_void))) -> u32,
    pub extend_save: unsafe extern "C" fn(*const u8, SaveHook, LoadHook, unsafe extern "C" fn()) -> u32,
    pub hook_ingame_command:
        unsafe extern "C" fn(u32, IngameCommandHook, Option<CommandLength>) -> u32,
    pub units: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut c_void>,
    pub selections: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut c_void>,
    pub first_ai_script: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut c_void>,
    pub hook_game_screen_rclick:
        unsafe extern "C" fn(unsafe extern "C" fn(*mut c_void, unsafe extern "C" fn(*mut c_void))) -> u32,
    pub client_selection: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut c_void>,
    // type, id
    pub dat_requirements: unsafe extern "C" fn() -> Option<unsafe extern "C" fn(u32, u32) -> *const u16>,
    pub first_guard_ai: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut c_void>,
    pub pathing: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut c_void>,
    pub set_first_ai_script: unsafe extern "C" fn() -> Option<unsafe extern "C" fn(*mut c_void)>,
    pub first_free_ai_script: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut c_void>,
    pub set_first_free_ai_script: unsafe extern "C" fn() -> Option<unsafe extern "C" fn(*mut c_void)>,
    pub player_ai_towns: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut c_void>,
    pub map_tile_flags: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut u32>,
    pub players: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut c_void>,
    pub hook_draw_image: unsafe extern "C" fn(
        unsafe extern "C" fn(*mut c_void, unsafe extern "C" fn(*mut c_void))
    ) -> u32,
    pub hook_renderer: unsafe extern "C" fn(u32, unsafe extern "C" fn()) -> u32,
    pub get_iscript_bin: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut c_void>,
    pub set_iscript_bin: unsafe extern "C" fn() -> Option<unsafe extern "C" fn(*mut c_void)>,
    pub hook_iscript_opcode: unsafe extern "C" fn(
        // Iscript pos, iscript ptr, image ptr, dry_run, speed_out, return new pos
        u32, unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, u32, *mut u32),
    ) -> u32,
    pub sprite_hlines: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut *mut c_void>,
    pub sprite_hlines_end: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut *mut c_void>,
    pub hook_file_read:
        unsafe extern "C" fn(*const u8, unsafe extern "C" fn(*const u8, *mut u32) -> *mut u8),
    pub first_active_bullet: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut c_void>,
    pub first_lone_sprite: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut c_void>,
    // Parent image, image_id, x, y, above
    pub add_overlay_iscript: unsafe extern "C" fn() ->
        Option<unsafe extern "C" fn(*mut c_void, u32, i32, i32, u32) -> *mut c_void>,
    pub set_campaigns: unsafe extern "C" fn(*const *mut c_void) -> u32,
    pub hook_run_dialog: unsafe extern "C" fn(
        unsafe extern "C" fn(
            *mut c_void,
            usize,
            *mut c_void,
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> u32,
        ) -> u32
    ) -> u32,
    pub send_command: unsafe extern "C" fn() -> Option<unsafe extern "C" fn(*const c_void, u32)>,
    pub ai_update_attack_target:
        unsafe extern "C" fn() -> Option<unsafe extern "C" fn(*mut c_void, u32, u32, u32) -> u32>,
    pub update_visibility_point: unsafe extern "C" fn() -> Option<unsafe extern "C" fn(*mut c_void)>,
    pub create_lone_sprite:
        unsafe extern "C" fn() -> Option<unsafe extern "C" fn(u32, i32, i32, u32) -> *mut c_void>,
    pub step_iscript:
        unsafe extern "C" fn() -> Option<unsafe extern "C" fn(*mut c_void, *mut c_void, u32, *mut u32)>,
    pub is_outside_game_screen: unsafe extern "C" fn() -> Option<unsafe extern "C" fn(i32, i32) -> u32>,
    pub screen_pos: unsafe extern "C" fn() -> Option<unsafe extern "C" fn(*mut i32, *mut i32)>,
    pub ui_scale: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> f32>,
    pub first_fow_sprite: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut c_void>,
    pub is_replay: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> u32>,
    pub local_player_id: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> u32>,
    pub unit_array_len:
        unsafe extern "C" fn() -> Option<unsafe extern "C" fn(*mut *mut c_void, *mut usize)>,
    pub draw_cursor_marker: unsafe extern "C" fn() -> Option<unsafe extern "C" fn(u32)>,
    pub hook_spawn_dialog: unsafe extern "C" fn(
        unsafe extern "C" fn(
            *mut c_void,
            usize,
            *mut c_void,
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> u32,
        ) -> u32
    ) -> u32,
    pub misc_ui_state: unsafe extern "C" fn(usize) -> Option<unsafe extern "C" fn(*mut u8)>,
    // bullet_id, x, y, player, direction, parent
    pub create_bullet: unsafe extern "C" fn() ->
        Option<unsafe extern "C" fn(u32, i32, i32, u32, u32, *mut c_void) -> *mut c_void>,
    pub hook_create_bullet: unsafe extern "C" fn(
        unsafe extern "C" fn(
            u32, i32, i32, u32, u32, *mut c_void,
            unsafe extern "C" fn(u32, i32, i32, u32, u32, *mut c_void) -> *mut c_void,
        ) -> *mut c_void,
    ) -> u32,
    // unit_id, x, y, player, skin
    pub create_unit: unsafe extern "C" fn() ->
        Option<unsafe extern "C" fn(u32, i32, i32, u32, *const u8) -> *mut c_void>,
    pub hook_create_unit: unsafe extern "C" fn(
        unsafe extern "C" fn(
            u32, i32, i32, u32, *const u8,
            unsafe extern "C" fn(u32, i32, i32, u32, *const u8) -> *mut c_void,
        ) -> *mut c_void,
    ) -> u32,
    pub finish_unit_pre: unsafe extern "C" fn() -> Option<unsafe extern "C" fn(*mut c_void)>,
    pub finish_unit_post: unsafe extern "C" fn() -> Option<unsafe extern "C" fn(*mut c_void)>,
    pub get_sprite_position:
        unsafe extern "C" fn() -> Option<unsafe extern "C" fn(*mut c_void, *mut u16)>,
    pub set_sprite_position:
        unsafe extern "C" fn() -> Option<unsafe extern "C" fn(*mut c_void, *const u16)>,
    pub hook_init_units: unsafe extern "C" fn(unsafe extern "C" fn(unsafe extern "C" fn())) -> u32,
    pub get_tooltip_draw_func:
        unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> Option<unsafe extern "C" fn(*mut c_void)>>,
    pub set_tooltip_draw_func:
        unsafe extern "C" fn() -> Option<unsafe extern "C" fn(Option<unsafe extern "C" fn(*mut c_void)>)>,
    pub hook_layout_draw_text: unsafe extern "C" fn(
        unsafe extern "C" fn(
            u32, u32, *const u8, *mut u32, u32, *mut u32, u32, u32,
            unsafe extern "C" fn(u32, u32, *const u8, *mut u32, u32, *mut u32, u32, u32) -> *const u8,
        ) -> *const u8,
    ) -> u32,
    pub hook_draw_graphic_layers: unsafe extern "C" fn(
        unsafe extern "C" fn(u32, unsafe extern "C" fn(u32)),
    ) -> u32,
    pub graphic_layers: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> *mut c_void>,
    // Arg 1 shader type (0 = Vertex, 1 = Pixel)
    // Arg 2 shader id
    // Arg 3 pointer (Must be a pointer to the entire set with static lifetime)
    // Arg 4 set size
    pub set_prism_shaders: unsafe extern "C" fn(u32, u32, *const u8, u32) -> u32,
    // Can be stored and called after plugin initialization function.
    pub crash_with_message: unsafe extern "C" fn(*const u8) -> !,
    pub ai_attack_prepare:
        unsafe extern "C" fn() -> Option<unsafe extern "C" fn(u32, u32, u32, u32, u32) -> u32>,
    pub hook_ai_step_region: unsafe extern "C" fn(
        unsafe extern "C" fn(u32, u32, unsafe extern "C" fn(u32, u32))
    ) -> u32,
    pub extended_arrays: unsafe extern "C" fn(*mut *mut ExtendedArray) -> usize,
    pub extended_dat:
        unsafe extern "C" fn(u32) -> Option<unsafe extern "C" fn(*mut usize) -> *mut c_void>,
    pub give_ai: unsafe extern "C" fn() -> Option<unsafe extern "C" fn(*mut c_void)>,
    pub hook_play_sound: unsafe extern "C" fn(
        unsafe extern "C" fn(
            u32, f32, *mut c_void, *mut i32, *mut i32,
            unsafe extern "C" fn(u32, f32, *mut c_void, *mut i32, *mut i32) -> u32,
        ) -> u32
    ) -> u32,
    pub is_multiplayer: unsafe extern "C" fn() -> Option<unsafe extern "C" fn() -> u32>,
    pub hook_game_loop_start: unsafe extern "C" fn(unsafe extern "C" fn()) -> u32,
    pub active_iscript_objects:
        unsafe extern "C" fn() -> Option<unsafe extern "C" fn(*mut *mut c_void, *const *mut c_void)>,
    pub hook_ai_focus_disabled: unsafe extern "C" fn(
        unsafe extern "C" fn(*mut c_void, unsafe extern "C" fn(*mut c_void))
    ) -> u32,
    pub hook_ai_focus_air: unsafe extern "C" fn(
        unsafe extern "C" fn(*mut c_void, unsafe extern "C" fn(*mut c_void))
    ) -> u32,
    // out: 2 pointer array where [0] = *air*, [1] = *ground*
    pub unit_base_strength: unsafe extern "C" fn() -> Option<unsafe extern "C" fn(*mut *mut u32)>,
    pub read_map_file:
        unsafe extern "C" fn() -> Option<unsafe extern "C" fn(*const u8, *mut usize) -> *mut u8>,
    pub hook_func: unsafe extern "C" fn(
        // Func ID (enum)
        u16,
        // Hook function: Takes arguments and then original function callback.
        // Casted from `unsafe extern fn(args..., unsafe extern fn(args...) -> ret) -> ret`
        usize,
    ) -> u32,
    // Func ID -> func, will have to cast for expected calling convention
    pub get_func: unsafe extern "C" fn(u16) -> Option<unsafe extern "C" fn()>,
    // List of VarId, list of results, length of lists
    // Result enum: 0 = Var not found, 1 = Not supported (Old samase),
    //      2 = Read only, 3 = Read / write
    pub load_vars: unsafe extern "C" fn(*const u16, *mut u8, usize),
    // List of VarId, list of results, length of lists
    pub read_vars: unsafe extern "C" fn(*const u16, *mut usize, usize),
    // List of VarId, list of values, length of lists
    pub write_vars: unsafe extern "C" fn(*const u16, *const usize, usize),
    // Main tab name, subtab name, draw fn, draw fn ctx
    // Return 0 if debug ui is disabled.
    pub debug_ui_add_tab:
        unsafe extern "C" fn(*const FfiStr, *const FfiStr, DebugUiDrawCb, *mut c_void) -> usize,
    // Return null if debug ui is disabled.
    pub debug_ui_add_log: unsafe extern "C" fn() -> *mut DebugUiLog,
    // Log, format, format params, format param count, extra (should be null)
    // Expected to be stored and used outside samase_plugin_init.
    // Does nothing if DebugUiLog is null, so caller doesn't have to null check if they
    // don't want to.
    // Format string must be valid until debug_log_clear call
    pub debug_log_add_data: unsafe extern "C" fn(
        *mut DebugUiLog, *const FfiStr, *const ComplexLineParam, usize, *mut c_void,
    ),
    // Expected to be stored and used outside samase_plugin_init.
    pub debug_log_clear: unsafe extern "C" fn(*mut DebugUiLog),
    // Allocates 4 bytes per unit of memory for later use between plugins.
    // Takes in field name string, returns id that is stable for the process lifetime,
    // but unstable between process launches. Repeated calls with same string return
    // same id.
    // Can be called outside plugin init function.
    // Returns 0 if not supported.
    pub create_extended_unit_field: unsafe extern "C" fn(*const FfiStr) -> u32,
    // unit index, field id -> value
    // Can be called outside plugin init function.
    pub read_extended_unit_field: unsafe extern "C" fn(u32, u32) -> u32,
    // unit index, field id, new value -> old value
    // Can be called outside plugin init function.
    pub write_extended_unit_field: unsafe extern "C" fn(u32, u32, u32) -> u32,
}

// Extern struct.
#[repr(C)]
pub struct DebugUiLog(u32);

#[repr(C)]
pub struct FfiStr {
    pub bytes: *const u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct ComplexLineParam {
    pub data: *mut c_void,
    pub ty: u32,
}

#[repr(u32)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ComplexLineParamType {
    Unit = 0,
    UnitId = 1,
    Point = 2,
    AiRegion = 3,
    AiTown = 4,
    TechId = 5,
    UpgradeId = 6,
    I32 = 7,
    PlayerId = 8,
}

#[repr(transparent)]
pub struct DebugUiColor(pub u32);

pub type DebugUiDrawCb = unsafe extern "C" fn(*const DebugUiDraw, *mut c_void);

#[repr(C)]
pub struct DebugUiDraw {
    pub struct_size: usize,
    // Text, return was_clicked
    pub button: unsafe extern "C" fn(*const FfiStr, DebugUiColor) -> u8,
    // Text, state_opt, return state
    // If state_opt is not given uses a variable based on text + tab names.
    pub checkbox: unsafe extern "C" fn(*const FfiStr, *mut u8) -> u8,
    // Label_opt, state_in, state_out
    pub text_entry: unsafe extern "C" fn(*const FfiStr, *const FfiStr, *mut FfiStr),
    // Text, color
    pub label: unsafe extern "C" fn(*const FfiStr, DebugUiColor),
    // Text, color, index, state
    // Writes *state = index when clicked.
    pub clickable_label: unsafe extern "C" fn(*const FfiStr, DebugUiColor, u32, *mut u32),
    // Text, params, param_count
    // "[]" in text gets replaced by params. No fancier formatting specifiers.
    pub complex_line: unsafe extern "C" fn(*const FfiStr, *const ComplexLineParam, usize),
    // Height, callback, callback_pram
    pub scroll_area: unsafe extern "C" fn(u32, DebugUiDrawCb, *mut c_void),
    // Name, id_source_opt, callback, callback_param
    pub collapsing: unsafe extern "C" fn(*const FfiStr, *const FfiStr, DebugUiDrawCb, *mut c_void),
    pub separator: unsafe extern "C" fn(),
    pub debug_log: unsafe extern "C" fn(*mut DebugUiLog),
}

#[derive(Copy, Clone)]
pub struct DebugUiDrawHelper(pub *const DebugUiDraw);

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum FuncId {
    // this = unit
    UnitCanRally = 0,
    // this = unit
    UnitCanBeInfested,
    // this = bullet
    DoMissileDamage,
    // this = target, bullet, damage_divisor
    HitUnit,
    // this = target, weapon_id, direction, attacker_unit
    HallucinationHit,
    // this = target, damage, attacker_unit, attacker_player, show_attacker
    DamageUnit,
    // this = unit
    KillUnit,
    // this = unit, value
    UnitSetHp,
    // this = unit, new_unit_id
    TransformUnit,
    // a1 = unit, player
    GiveUnit,
    // a1 = x_tile, y_tile, w_tile, h_tile, bool_round_corners
    PlaceCreepRect,
    // a1 = unit_id, x, y
    PlaceFinishedUnitCreep,
    // a1 = parent_unit, unit (a1 == a2 if unit morph)
    AddAiToTrainedUnit,
    // a1 = parent_unit, unit (a1 == a2 if unit morph)
    AddBuildingAi,
    // a1 town, unit
    AddTownUnitAi,
    // a1 = ai_region, unit, bool_always_use_this_region (Otherwise won't if region.state == 3)
    AddMilitaryAi,
    // a1 = unit, bool_was_killed
    AiRemoveUnit,
    // a1 = unit, bool_was_killed
    AiRemoveUnitMilitary,
    // a1 = unit, bool_for_finished_unit_morph (Force BuildingAi deletion and never delete town)
    AiRemoveUnitTown,
    // this = unit, returns 256 * displayed value
    UnitMaxEnergy,
    // this = unit, weapon_id
    UnitAttackRange,
    // this = unit
    UnitTargetAcquisitionRange,
    // this = unit, bool_ignore_blind
    UnitSightRange,
    // this = unit, weapon_id, target_unit
    CheckWeaponTargetingFlags,
    // this = unit, tech_id, target_unit, fow_unit_id, x, y, u16 *opt_error_string_id
    CheckTechTargeting,
    // this = unit, order, target_unit, x, y, u16 *opt_error_string_id
    CheckOrderTargeting,
    // this = unit, order, fow_unit_id, x, y, u16 *opt_error_string_id
    CheckFowOrderTargeting,
    // this = unit
    HideUnit,
    // this = unit
    ShowUnit,
    // a1 = integer id (0 / 1 / 2)
    GetRenderTarget,
    // a1 = x, y (Top left)
    MoveScreen,
    // a1 = length, units, bool, bool
    SelectUnits,
    // a1 = AiRegion *, unit_id, priority
    AiAddMilitaryToRegion,
    // a1 = player
    AiTrainMilitary,
    // a1 = player, x, y, bool always_override, bool allow_air_fallback
    AiAttackPrepare,
    // a1 = player, bool zero_last_attack_second
    AiAttackClear,
    // a1 = AiRegion *
    AiRegionUpdateStrength,
    // a1 = AiRegion *
    AiRegionUpdateTarget,
    // a1 = AiRegion *
    AiRegionAbandonIfOverwhelmed,
    // a1 = AiRegion *
    AiRegionPickAttackTarget,
    // a1 = player, region_id
    AiStepRegion,
    // a1 = player
    AiTargetExpansion,
    // a1 = Related to replay seeking?
    StepGameLogic,
    // this = unit
    StepUnitTimers,
    // this = unit
    StartCloaking,
    // a1 = unit
    UnitAiWorker,
    // a1 = unit
    UnitAiMilitary,
    // a1 = unit (May be called for workers too but nop, may be called several times to try first
    // value in spending request queue?)
    UnitAiBuilding,
    // this = unit, rect, filter_func, filter_param
    FindNearestUnitInArea,
    // this = unit, radius, filter_func, filter_param
    FindNearestUnitAroundUnit,
    // this = unit, target, check_detection
    CanAttackUnit,
    // this = unit, target
    IsOutsideAttackRange,
    // this = unit, target_opt
    AiCanTargetAttackThis,
    // a1 = unit, dont_issue_order
    AiTryReturnHome,
    // a1 = rect, filter_func, func_param
    ForEachUnitInArea,
    // this = builder_unit, unit_id
    PrepareBuildUnit,
    // a1 = path_context
    CalculatePath,
    // a1 = worker, unit_id, pos_xy, out_xy, area_tiles
    AiPlaceBuilding,
    // a1 = source_region, dest_region, max_distance_regions, [u16; 3] *out_regions,
    // u32 *out_error, min_distance_regions
    GetChokePointRegions,
    // a1 = unit_id, u8 *placement_data[0x1000], player, pos_xy, radius_tiles
    AiUpdateBuildingPlacementState,
    // a1 = unit_opt, player, x_tile, y_tile, unit_id, placement_entry,
    // check_vision, also_invisible, without_vision
    UpdateBuildingPlacementState,
    // a1 = x, y, rect, filter_func, filter_param
    FindNearestUnitInAreaPoint,
    // a1 = builder_unit, u8 *placement_data[0x1000], player, unit_id, placement_center,
    // search_pos, u32 *out
    // Return 1 for ok, 0 for none
    AiPickBestPlacementPosition,
    // a1 = builder_unit, player, unit_id
    AiPlacementFlags,

    _Last,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(u16)]
pub enum VarId {
    Game = 0,
    RngSeed,
    RngEnable,
    AiRegions,
    PlayerAi,
    FirstActiveUnit,
    FirstHiddenUnit,
    // *mut Unit, not *mut vector<Unit>
    Units,
    // *mut vector<Unit>
    UnitsVector,
    FirstAiScript,
    FirstFreeAiScript,
    FirstGuardAi,
    ActiveAiTowns,
    Pathing,
    Selections,
    ClientSelection,
    LoadedSave,
    MapTileFlags,
    Players,
    IscriptBin,
    SpriteHlines,
    SpriteHlinesEnd,
    FirstActiveBullet,
    FirstLoneSprite,
    ScreenX,
    ScreenY,
    Zoom,
    FirstFowSprite,
    DrawCursorMarker,
    IsPaused,
    IsTargeting,
    IsPlacingBuilding,
    TooltipDrawFunc,
    GraphicLayers,
    ActiveIscriptFlingy,
    ActiveIscriptUnit,
    ActiveIscriptBullet,
    CmdIconsDdsGrp,
    CmdBtnsDdsGrp,
    StatusScreenMode,
    DatRequirementError,
    FirstPlayerUnit,
    UnitShouldRevealArea,
    Allocator,
    GameData,
    ReplayData,
    ReplayHeader,
    ScMainState,
    CommandUser,
    UniqueCommandUser,
    IsReplay,
    LocalPlayerId,
    LocalUniquePlayerId,
    IsMultiplayer,
    LastLoneSprite,
    FirstFreeLoneSprite,
    LastFreeLoneSprite,
    LastFowSprite,
    FirstFreeFowSprite,
    LastFreeFowSprite,
    CursorMarker,
    ResourceAreas,
    DrawCommands,
    VertexBuffer,
    Renderer,
    FirstDialog,
    MainPalette,
    RgbColors,
    UseRgbColors,
    GameScreenWidthBwpx,
    GameScreenHeightBwpx,
    StepGameFrames,
    StatportTalkingPortraitActive,
    TilesetCv5,
    MinitileData,
    TilesetIndexedMapTiles,
    CreepOriginalTiles,
    CreepTileBorders,

    _Last,
}

impl FfiStr {
    pub fn from_str(input: &str) -> FfiStr {
        Self::from_bytes(input.as_bytes())
    }

    pub fn from_bytes(input: &[u8]) -> FfiStr {
        let len = input.len();
        FfiStr {
            bytes: input.as_ptr(),
            len,
        }
    }

    pub unsafe fn to_bytes<'a>(&self) -> &'a [u8] {
        // Allowing null / unaligned when length is 0.
        if self.len == 0 {
            &[]
        } else {
            std::slice::from_raw_parts(self.bytes, self.len)
        }
    }

    pub unsafe fn string_lossy<'a>(&self) -> std::borrow::Cow<'a, str> {
        String::from_utf8_lossy(self.to_bytes())
    }
}

macro_rules! debug_ui_draw_ptr {
    ($ptr:expr, $field:ident) => {
        if $ptr.is_null() {
            None
        } else {
            let offset = std::mem::offset_of!(DebugUiDraw, $field);
            let size = (*$ptr).struct_size;
            // Technically size >= offset + sizeof(usize) is more accurate but w/e
            if size > offset {
                Some((*$ptr).$field)
            } else {
                None
            }
        }
    };
}

impl DebugUiDrawHelper {
    pub unsafe fn button(self, text: &str, color: DebugUiColor) -> bool {
        if let Some(func) = debug_ui_draw_ptr!(self.0, button) {
            let text = FfiStr::from_str(text);
            func(&text, color) != 0
        } else {
            false
        }
    }

    pub unsafe fn checkbox(self, text: &str) -> bool {
        if let Some(func) = debug_ui_draw_ptr!(self.0, checkbox) {
            let text = FfiStr::from_str(text);
            func(&text, null_mut()) != 0
        } else {
            false
        }
    }

    pub unsafe fn checkbox_state(self, text: &str, state: &mut bool) {
        if let Some(func) = debug_ui_draw_ptr!(self.0, checkbox) {
            let text = FfiStr::from_str(text);
            let mut s = *state as u8;
            func(&text, &mut s);
            *state = s != 0;
        }
    }

    pub unsafe fn text_entry(self, label: &str, state: &mut String) {
        if let Some(func) = debug_ui_draw_ptr!(self.0, text_entry) {
            let label = FfiStr::from_str(label);
            let input = FfiStr::from_str(state);
            let mut out = FfiStr::from_str("");
            func(&label, &input, &mut out);
            *state = out.string_lossy().into();
        }
    }

    pub unsafe fn label(self, text: &str) {
        self.label_colored(text, DebugUiColor::none())
    }

    pub unsafe fn label_colored(self, text: &str, color: DebugUiColor) {
        if let Some(func) = debug_ui_draw_ptr!(self.0, label) {
            let text = FfiStr::from_str(text);
            func(&text, color);
        }
    }

    pub unsafe fn clickable_label(
        self,
        text: &str,
        color: DebugUiColor,
        index: u32,
        state: &mut u32,
    ) {
        if let Some(func) = debug_ui_draw_ptr!(self.0, clickable_label) {
            let text = FfiStr::from_str(text);
            func(&text, color, index, state);
        }
    }

    pub unsafe fn complex_line(self, text: &str, params: &[ComplexLineParam]) {
        if let Some(func) = debug_ui_draw_ptr!(self.0, complex_line) {
            let text = FfiStr::from_str(text);
            let len = params.len();
            func(&text, params.as_ptr(), len);
        }
    }

    pub unsafe fn scroll_area<F: FnOnce(DebugUiDrawHelper)>(self, height: u32, cb: F) {
        if let Some(func) = debug_ui_draw_ptr!(self.0, scroll_area) {
            let mut ctx = Some(cb);
            let ctx: &mut Option<F> = &mut ctx;
            func(height, Self::ui_draw_cb::<F>, ctx as *mut Option<F> as *mut c_void);
        }
    }

    unsafe extern "C" fn ui_draw_cb<F: FnOnce(DebugUiDrawHelper)>(
        api: *const DebugUiDraw,
        ctx: *mut c_void,
    ) {
        let ctx = ctx as *mut Option<F>;
        let ctx = &mut *ctx;
        (ctx.take().unwrap())(Self(api));
    }

    pub unsafe fn collapsing<F: FnOnce(DebugUiDrawHelper)>(
        self,
        text: &str,
        id_source: Option<&str>,
        cb: F,
    ) {
        if let Some(func) = debug_ui_draw_ptr!(self.0, collapsing) {
            let text = FfiStr::from_str(text);
            let mut ctx = Some(cb);
            let ctx: &mut Option<F> = &mut ctx;
            if let Some(id_source) = id_source {
                let id_source = FfiStr::from_str(id_source);
                func(
                    &text,
                    &id_source,
                    Self::ui_draw_cb::<F>,
                    ctx as *mut Option<F> as *mut c_void,
                );
            }
        }
    }

    pub unsafe fn separator(self) {
        if let Some(func) = debug_ui_draw_ptr!(self.0, separator) {
            func();
        }
    }

    pub unsafe fn debug_log(self, log: *mut DebugUiLog) {
        if let Some(func) = debug_ui_draw_ptr!(self.0, debug_log) {
            func(log);
        }
    }
}

impl DebugUiColor {
    pub fn none() -> DebugUiColor {
        DebugUiColor(0)
    }

    pub fn rgb(color: u32) -> DebugUiColor {
        DebugUiColor(0x0100_0000 | color)
    }

    pub fn player(player: u8) -> DebugUiColor {
        DebugUiColor(0x0200_0000 | (player as u32))
    }
}
