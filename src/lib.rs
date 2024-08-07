#[macro_use] extern crate log;

pub mod commands;
pub mod save;

use libc::c_void;

use crate::commands::{CommandLength, IngameCommandHook};
use crate::save::{SaveHook, LoadHook};

pub const VERSION: u16 = 40;
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
    pub free_memory: unsafe extern fn(*mut u8),
    pub write_exe_memory: unsafe extern fn(usize, *const u8, usize) -> u32,
    pub warn_unsupported_feature: unsafe extern fn(*const u8),
    pub read_file: unsafe extern fn() -> unsafe extern fn(*const u8, *mut usize) -> *mut u8,
    pub game: unsafe extern fn() -> Option<unsafe extern fn() -> *mut c_void>,
    pub rng_seed: unsafe extern fn() -> Option<unsafe extern fn() -> u32>,
    pub hook_step_objects: unsafe extern fn(unsafe extern fn(), u32) -> u32,
    pub hook_aiscript_opcode: unsafe extern fn(u32, unsafe extern fn(*mut c_void)) -> u32,
    pub ai_regions: unsafe extern fn() -> Option<unsafe extern fn() -> *mut c_void>,
    pub player_ai: unsafe extern fn() -> Option<unsafe extern fn() -> *mut c_void>,
    pub get_region: unsafe extern fn() -> Option<unsafe extern fn(u32, u32) -> u32>,
    pub change_ai_region_state: unsafe extern fn() -> Option<unsafe extern fn(*mut c_void, u32)>,
    pub first_active_unit: unsafe extern fn() -> Option<unsafe extern fn() -> *mut c_void>,
    pub first_hidden_unit: unsafe extern fn() -> Option<unsafe extern fn() -> *mut c_void>,
    // self, order, x, y, target, fow_unit
    pub issue_order: unsafe extern fn() ->
        Option<unsafe extern fn(*mut c_void, u32, u32, u32, *mut c_void, u32)>,
    pub print_text: unsafe extern fn() -> Option<unsafe extern fn(*const u8)>,
    pub hook_on_first_file_access: unsafe extern fn(unsafe extern fn()),
    pub hook_step_order:
        unsafe extern fn(unsafe extern fn(*mut c_void, unsafe extern fn(*mut c_void))) -> u32,
    pub hook_step_order_hidden:
        unsafe extern fn(unsafe extern fn(*mut c_void, unsafe extern fn(*mut c_void))) -> u32,
    pub dat: unsafe extern fn(u32) -> Option<unsafe extern fn() -> *mut c_void>,
    pub hook_process_commands: unsafe extern fn(
        unsafe extern fn(*const c_void, u32, u32, unsafe extern fn(*const c_void, u32, u32))
    ) -> u32,
    pub hook_process_lobby_commands: unsafe extern fn(
        unsafe extern fn(*const c_void, u32, u32, unsafe extern fn(*const c_void, u32, u32))
    ) -> u32,
    pub hook_send_command: unsafe extern fn(
        unsafe extern fn(*mut c_void, u32, unsafe extern fn(*mut c_void, u32))
    ) -> u32,
    pub hook_step_secondary_order:
        unsafe extern fn(unsafe extern fn(*mut c_void, unsafe extern fn(*mut c_void))) -> u32,
    pub extend_save: unsafe extern fn(*const u8, SaveHook, LoadHook, unsafe extern fn()) -> u32,
    pub hook_ingame_command:
        unsafe extern fn(u32, IngameCommandHook, Option<CommandLength>) -> u32,
    pub units: unsafe extern fn() -> Option<unsafe extern fn() -> *mut c_void>,
    pub selections: unsafe extern fn() -> Option<unsafe extern fn() -> *mut c_void>,
    pub first_ai_script: unsafe extern fn() -> Option<unsafe extern fn() -> *mut c_void>,
    pub hook_game_screen_rclick:
        unsafe extern fn(unsafe extern fn(*mut c_void, unsafe extern fn(*mut c_void))) -> u32,
    pub client_selection: unsafe extern fn() -> Option<unsafe extern fn() -> *mut c_void>,
    // type, id
    pub dat_requirements: unsafe extern fn() -> Option<unsafe extern fn(u32, u32) -> *const u16>,
    pub first_guard_ai: unsafe extern fn() -> Option<unsafe extern fn() -> *mut c_void>,
    pub pathing: unsafe extern fn() -> Option<unsafe extern fn() -> *mut c_void>,
    pub set_first_ai_script: unsafe extern fn() -> Option<unsafe extern fn(*mut c_void)>,
    pub first_free_ai_script: unsafe extern fn() -> Option<unsafe extern fn() -> *mut c_void>,
    pub set_first_free_ai_script: unsafe extern fn() -> Option<unsafe extern fn(*mut c_void)>,
    pub player_ai_towns: unsafe extern fn() -> Option<unsafe extern fn() -> *mut c_void>,
    pub map_tile_flags: unsafe extern fn() -> Option<unsafe extern fn() -> *mut u32>,
    pub players: unsafe extern fn() -> Option<unsafe extern fn() -> *mut c_void>,
    pub hook_draw_image: unsafe extern fn(
        unsafe extern fn(*mut c_void, unsafe extern fn(*mut c_void))
    ) -> u32,
    pub hook_renderer: unsafe extern fn(u32, unsafe extern fn()) -> u32,
    pub get_iscript_bin: unsafe extern fn() -> Option<unsafe extern fn() -> *mut c_void>,
    pub set_iscript_bin: unsafe extern fn() -> Option<unsafe extern fn(*mut c_void)>,
    pub hook_iscript_opcode: unsafe extern fn(
        // Iscript pos, iscript ptr, image ptr, dry_run, speed_out, return new pos
        u32, unsafe extern fn(*mut c_void, *mut c_void, *mut c_void, u32, *mut u32),
    ) -> u32,
    pub sprite_hlines: unsafe extern fn() -> Option<unsafe extern fn() -> *mut *mut c_void>,
    pub sprite_hlines_end: unsafe extern fn() -> Option<unsafe extern fn() -> *mut *mut c_void>,
    pub hook_file_read:
        unsafe extern fn(*const u8, unsafe extern fn(*const u8, *mut u32) -> *mut u8),
    pub first_active_bullet: unsafe extern fn() -> Option<unsafe extern fn() -> *mut c_void>,
    pub first_lone_sprite: unsafe extern fn() -> Option<unsafe extern fn() -> *mut c_void>,
    // Parent image, image_id, x, y, above
    pub add_overlay_iscript: unsafe extern fn() ->
        Option<unsafe extern fn(*mut c_void, u32, i32, i32, u32) -> *mut c_void>,
    pub set_campaigns: unsafe extern fn(*const *mut c_void) -> u32,
    pub hook_run_dialog: unsafe extern fn(
        unsafe extern fn(
            *mut c_void,
            usize,
            *mut c_void,
            unsafe extern fn(*mut c_void, usize, *mut c_void) -> u32,
        ) -> u32
    ) -> u32,
    pub send_command: unsafe extern fn() -> Option<unsafe extern fn(*const c_void, u32)>,
    pub ai_update_attack_target:
        unsafe extern fn() -> Option<unsafe extern fn(*mut c_void, u32, u32, u32) -> u32>,
    pub update_visibility_point: unsafe extern fn() -> Option<unsafe extern fn(*mut c_void)>,
    pub create_lone_sprite:
        unsafe extern fn() -> Option<unsafe extern fn(u32, i32, i32, u32) -> *mut c_void>,
    pub step_iscript:
        unsafe extern fn() -> Option<unsafe extern fn(*mut c_void, *mut c_void, u32, *mut u32)>,
    pub is_outside_game_screen: unsafe extern fn() -> Option<unsafe extern fn(i32, i32) -> u32>,
    pub screen_pos: unsafe extern fn() -> Option<unsafe extern fn(*mut i32, *mut i32)>,
    pub ui_scale: unsafe extern fn() -> Option<unsafe extern fn() -> f32>,
    pub first_fow_sprite: unsafe extern fn() -> Option<unsafe extern fn() -> *mut c_void>,
    pub is_replay: unsafe extern fn() -> Option<unsafe extern fn() -> u32>,
    pub local_player_id: unsafe extern fn() -> Option<unsafe extern fn() -> u32>,
    pub unit_array_len:
        unsafe extern fn() -> Option<unsafe extern fn(*mut *mut c_void, *mut usize)>,
    pub draw_cursor_marker: unsafe extern fn() -> Option<unsafe extern fn(u32)>,
    pub hook_spawn_dialog: unsafe extern fn(
        unsafe extern fn(
            *mut c_void,
            usize,
            *mut c_void,
            unsafe extern fn(*mut c_void, usize, *mut c_void) -> u32,
        ) -> u32
    ) -> u32,
    pub misc_ui_state: unsafe extern fn(usize) -> Option<unsafe extern fn(*mut u8)>,
    // bullet_id, x, y, player, direction, parent
    pub create_bullet: unsafe extern fn() ->
        Option<unsafe extern fn(u32, i32, i32, u32, u32, *mut c_void) -> *mut c_void>,
    pub hook_create_bullet: unsafe extern fn(
        unsafe extern fn(
            u32, i32, i32, u32, u32, *mut c_void,
            unsafe extern fn(u32, i32, i32, u32, u32, *mut c_void) -> *mut c_void,
        ) -> *mut c_void,
    ) -> u32,
    // unit_id, x, y, player, skin
    pub create_unit: unsafe extern fn() ->
        Option<unsafe extern fn(u32, i32, i32, u32, *const u8) -> *mut c_void>,
    pub hook_create_unit: unsafe extern fn(
        unsafe extern fn(
            u32, i32, i32, u32, *const u8,
            unsafe extern fn(u32, i32, i32, u32, *const u8) -> *mut c_void,
        ) -> *mut c_void,
    ) -> u32,
    pub finish_unit_pre: unsafe extern fn() -> Option<unsafe extern fn(*mut c_void)>,
    pub finish_unit_post: unsafe extern fn() -> Option<unsafe extern fn(*mut c_void)>,
    pub get_sprite_position:
        unsafe extern fn() -> Option<unsafe extern fn(*mut c_void, *mut u16)>,
    pub set_sprite_position:
        unsafe extern fn() -> Option<unsafe extern fn(*mut c_void, *const u16)>,
    pub hook_init_units: unsafe extern fn(unsafe extern fn(unsafe extern fn())) -> u32,
    pub get_tooltip_draw_func:
        unsafe extern fn() -> Option<unsafe extern fn() -> Option<unsafe extern fn(*mut c_void)>>,
    pub set_tooltip_draw_func:
        unsafe extern fn() -> Option<unsafe extern fn(Option<unsafe extern fn(*mut c_void)>)>,
    pub hook_layout_draw_text: unsafe extern fn(
        unsafe extern fn(
            u32, u32, *const u8, *mut u32, u32, *mut u32, u32, u32,
            unsafe extern fn(u32, u32, *const u8, *mut u32, u32, *mut u32, u32, u32) -> *const u8,
        ) -> *const u8,
    ) -> u32,
    pub hook_draw_graphic_layers: unsafe extern fn(
        unsafe extern fn(u32, unsafe extern fn(u32)),
    ) -> u32,
    pub graphic_layers: unsafe extern fn() -> Option<unsafe extern fn() -> *mut c_void>,
    // Arg 1 shader type (0 = Vertex, 1 = Pixel)
    // Arg 2 shader id
    // Arg 3 pointer (Must be a pointer to the entire set with static lifetime)
    // Arg 4 set size
    pub set_prism_shaders: unsafe extern fn(u32, u32, *const u8, u32) -> u32,
    // Can be stored and called after plugin initialization function.
    pub crash_with_message: unsafe extern fn(*const u8) -> !,
    pub ai_attack_prepare:
        unsafe extern fn() -> Option<unsafe extern fn(u32, u32, u32, u32, u32) -> u32>,
    pub hook_ai_step_region: unsafe extern fn(
        unsafe extern fn(u32, u32, unsafe extern fn(u32, u32))
    ) -> u32,
    pub extended_arrays: unsafe extern fn(*mut *mut ExtendedArray) -> usize,
    pub extended_dat:
        unsafe extern fn(u32) -> Option<unsafe extern fn(*mut usize) -> *mut c_void>,
    pub give_ai: unsafe extern fn() -> Option<unsafe extern fn(*mut c_void)>,
    pub hook_play_sound: unsafe extern fn(
        unsafe extern fn(
            u32, f32, *mut c_void, *mut i32, *mut i32,
            unsafe extern fn(u32, f32, *mut c_void, *mut i32, *mut i32) -> u32,
        ) -> u32
    ) -> u32,
    pub is_multiplayer: unsafe extern fn() -> Option<unsafe extern fn() -> u32>,
    pub hook_game_loop_start: unsafe extern fn(unsafe extern fn()) -> u32,
    pub active_iscript_objects:
        unsafe extern fn() -> Option<unsafe extern fn(*mut *mut c_void, *const *mut c_void)>,
    pub hook_ai_focus_disabled: unsafe extern fn(
        unsafe extern fn(*mut c_void, unsafe extern fn(*mut c_void))
    ) -> u32,
    pub hook_ai_focus_air: unsafe extern fn(
        unsafe extern fn(*mut c_void, unsafe extern fn(*mut c_void))
    ) -> u32,
    // out: 2 pointer array where [0] = *air*, [1] = *ground*
    pub unit_base_strength: unsafe extern fn() -> Option<unsafe extern fn(*mut *mut u32)>,
    pub read_map_file:
        unsafe extern fn() -> Option<unsafe extern fn(*const u8, *mut usize) -> *mut u8>,
    pub hook_func: unsafe extern fn(
        // Func ID (enum)
        u16,
        // Hook function: Takes arguments and then original function callback.
        // Casted from `unsafe extern fn(args..., unsafe extern fn(args...) -> ret) -> ret`
        usize,
    ) -> u32,
    // Func ID -> func, will have to cast for expected calling convention
    pub get_func: unsafe extern fn(u16) -> Option<unsafe extern fn()>,
    // List of VarId, list of results, length of lists
    // Result enum: 0 = Var not found, 1 = Not supported (Old samase),
    //      2 = Read only, 3 = Read / write
    pub load_vars: unsafe extern fn(*const u16, *mut u8, usize),
    // List of VarId, list of results, length of lists
    pub read_vars: unsafe extern fn(*const u16, *mut usize, usize),
    // List of VarId, list of values, length of lists
    pub write_vars: unsafe extern fn(*const u16, *const usize, usize),
}

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
