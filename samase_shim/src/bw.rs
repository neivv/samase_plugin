#![allow(bad_style)]
use libc::c_void;

whack_funcs!(stdcall, init_funcs, 0x00400000,
    0x004D2D10 =>
        read_file(*const u8, u32, u32, *const u8, u32, @ecx u32, @eax *mut usize) -> *mut u8;
    0x0049C9F0 => get_region(@edi u32, @ecx u32) -> u32;
    0x004390A0 => change_ai_region_state(@esi *mut c_void, @ebx u32);
    0x00474810 => prepare_issue_order(@edx *mut Unit, @ecx u32, u32, *mut Unit, u32, @eax u32);
    0x00475000 => do_next_queued_order(@ecx *mut Unit);
    0x0048D1C0 => print_text(*const u8, @eax u32, u32);
    0x004DABD0 => init_mpqs();
    0x004D6D90 => add_overlay_iscript(*mut Image, u32, i32, i32, u32) -> *mut Image;
    0x00485BD0 => send_command(@ecx *const c_void, @edx u32);
    0x00477160 => ai_update_attack_target(@ecx *mut c_void, u32, u32, u32) -> u32;
    0x004878F0 => update_visibility_point(@esi *mut c_void);
    0x00488210 => create_lone_sprite(u32, i32, @edi i32, u32) -> *mut c_void;
    0x004D74C0 => step_iscript(@ecx *mut c_void, *mut c_void, u32, *mut u32);
    0x004D1140 => is_outside_game_screen(@ecx i32, @eax i32) -> u32;

    0x0048C260 => create_bullet(@ecx u32, i32, i32, u32, u32, @eax *mut c_void) -> *mut c_void;
    0x004A09D0 => create_unit(@ecx u32, @eax i32, i32, u32) -> *mut c_void;
    0x004A01F0 => finish_unit_pre(@eax *mut c_void);
    0x0049FA40 => finish_unit_post(@eax *mut c_void);

    0x0043ABB0 => ai_attack_prepare(u32, u32, u32, u32, u32) -> u32;
    0x00463040 => give_ai(@eax *mut c_void);

    0x004CDE70 => add_to_replay_data(@eax *mut c_void, u32, @ebx *const u8, @edi usize);

    0x004100C4 => SFileOpenFileEx(*mut c_void, *const u8, u32, *mut *mut c_void) -> u32;
    0x00410142 => SFileGetFileSize(*mut c_void, *mut u32) -> u32;
    0x004100B8 => SFileCloseFile(*mut c_void);
    0x00410148 => SFileReadFile(*mut c_void, *mut u8, u32, *mut u32, u32) -> u32;

    0x0049FED0 => TransformUnit(@eax usize, usize) -> usize;
    0x00475710 => KillUnit(@eax usize) -> usize;
    0x00479730 => HallucinationHit(@eax usize, usize, usize, @esi usize) -> usize;
    0x004797B0 => DamageUnit(@ecx usize, @eax usize, usize, usize, usize) -> usize;
    0x00479AE0 => HitUnit(@eax usize, @edx usize, usize) -> usize;
    0x0047B2E0 => UnitCanRally(@eax usize) -> usize;
    0x00467340 => UnitSetHp(@eax usize, @ecx usize) -> usize;
    0x00402210 => UnitCanBeInfested(@edx usize) -> usize;
    0x0048B770 => DoMissileDamage(usize) -> usize;
    0x004C8040 => GiveUnit(usize, usize) -> usize;
    0x0045CD50 => PlaceCreepRect(usize, usize, usize, usize, usize) -> usize;
    0x0045CE90 => PlaceFinishedUnitCreep(@ecx usize, usize, usize) -> usize;
    0x004A2830 => AddAiToTrainedUnit(@ecx usize, @eax usize) -> usize;
    0x00435770 => AddBuildingAi(@ecx usize, @eax usize) -> usize;
    0x00433DD0 => AddTownUnitAi(@edi usize, @ebx usize) -> usize;
    0x0043DA20 => AddMilitaryAi(@eax usize, @ebx usize, usize) -> usize;
    0x004A1E50 => AiRemoveUnit(@ecx usize, @edx usize) -> usize;
    0x00439D60 => AiRemoveUnitMilitary(usize, usize) -> usize;
    0x00434C90 => AiRemoveUnitTown(@edi usize, usize) -> usize;
    0x00491870 => UnitMaxEnergy(@eax usize) -> usize;
    0x00475870 => UnitAttackRange(@eax usize, @ebx usize) -> usize;
    0x00476000 => UnitTargetAcquisitionRange(@edx usize) -> usize;
    0x004E5B40 => UnitSightRange(@edx usize, usize) -> usize;
    0x00475CE0 => CheckWeaponTargetingFlags(usize, @eax usize, @edx usize) -> usize;
    0x00492020 => CheckTechTargeting(
        usize, @edx usize, @edi usize, @esi usize, usize, usize, usize) -> usize;
    // ecx, edx really unused
    0x00474D90 => CheckOrderTargeting(
        @ebx usize, @eax usize, @esi usize, @ecx usize, @edx usize, usize) -> usize;
    // eax, esi really unused
    0x004746D0 => CheckFowOrderTargeting(
        @ebx usize, @edx usize, @ecx usize, @eax usize, @esi usize, @edi usize) -> usize;
    0x004E6340 => HideUnit(@eax usize) -> usize;
    0x004E6490 => ShowUnit(@edi usize) -> usize;
    0x0043ABB0 => AiAttackPrepare(usize, usize, usize, usize, usize) -> usize;
    0x0043A390 => AiRegionUpdateStrength(@esi usize) -> usize;
    0x0043CC40 => AiRegionUpdateTarget(@edi usize) -> usize;
    0x0043DE40 => AiRegionAbandonIfOverwhelmed(@edi usize) -> usize;
    0x0043A510 => AiRegionPickAttackTarget(usize) -> usize;
    0x0043F990 => AiTrainMilitary(usize) -> usize;
    0x0043E2E0 => AiAddMilitaryToRegion(@eax usize, usize, usize) -> usize;
    0x0043FC60 => AiStepRegion(@ecx usize, @eax usize) -> usize;
    0x0043B9E0 => AiTargetExpansion(usize) -> usize;
    0x004EC290 => StepUnitTimers(@eax usize) -> usize;
    0x00491B30 => StartCloaking(@eax usize) -> usize;
    0x00435210 => UnitAiWorker(usize) -> usize;
    0x004361A0 => AiTryProgressSpendingRequest(@ecx usize) -> usize;
    0x0043D910 => UnitAiMilitary(@eax usize) -> usize;
    0x00476930 => AiCanTargetAttackThis(@ecx usize, @eax usize) -> usize;
    0x00476730 => CanAttackUnit(@esi usize, @ebx usize, usize) -> usize;
    0x00476430 => IsOutsideAttackRange(@eax usize, usize) -> usize;
    0x00462EA0 => AiTryReturnHome(@eax usize, usize) -> usize;
    0x004308A0 => FindUnitsRect(*mut u16) -> *mut usize;
    0x00467250 => PrepareBuildUnit(@edi usize, usize) -> usize;
    0x004E1D90 => CalculatePath(usize) -> usize;
    0x004465C0 => AiPlaceBuilding(@ecx usize, usize, usize, usize, usize) -> usize;
    0x004E29B0 => GetChokePointRegions(
        @ecx usize, @edx usize, usize, usize, usize, @eax usize) -> usize;
    0x00473FB0 => UpdateBuildingPlacementState(
        usize, usize, usize, usize, usize, usize, usize, usize, usize) -> usize;
    0x004461B0 => AiUpdateBuildingPlacementState(
        @ebx usize, @esi usize, usize, usize, usize) -> usize;
    0x00488210  => CreateLoneSprite(usize, usize, @edi usize, usize) -> usize;
    // Not done since the callback is fastcall and wrapping it would be effort
    //0x004E8740 => FindNearestUnitInArea(@esi usize, usize, usize, usize) -> usize;
    //0x004E8740 => FindNearestUnit(@esi usize, usize, usize, usize) -> usize;
);

whack_funcs!(init_funcs_cdecl, 0x00400000,
    0x004117DE => fread(*mut c_void, u32, u32, *mut c_void) -> u32;
    0x00411931 => fwrite(*const c_void, u32, u32, *mut c_void) -> u32;
    0x00411B6E => fseek(*mut c_void, u32, u32) -> i32;
);

whack_hooks!(stdcall, 0x00400000,
    0x004D94B0 => StepObjects();
    0x004DAA60 => FirstFileAccess();
    0x004DABD0 => InitMpqs();
    0x004E0AE0 => WinMain();
    0x004EC4D0 => StepOrder(@eax *mut c_void);
    0x004EBED0 => StepOrder_Hidden(@eax *mut c_void);
    0x004865D0 => ProcessCommands(@eax *const c_void, u32, u32);
    0x00486040 => ProcessLobbyCommands(@eax *const c_void, u32, u32);
    0x004EC170 => StepSecondaryOrder(@eax *mut c_void);
    0x00485BD0 => SendCommand(@ecx *mut c_void, @edx u32);
    0x004C2E0F => SaveReady(@ebx *mut c_void);
    0x004EEE00 => InitGame();
    0x004D0232 => LoadReady();
    0x0040FE11 => FseekFilePointerSet(@eax u32);
    0x004564E0 => GameScreenRClick(@ecx *mut c_void);
    0x00497CE0 => DrawImage(@esi *mut Image);
    0x0041A080 => RunDialog(@eax *mut c_void, *mut c_void);
    0x00419D20 => SpawnDialog(@esi *mut c_void, @eax *mut c_void);
    0x0048C260 => CreateBullet(@ecx u32, i32, i32, u32, u32, @eax *mut c_void) -> *mut c_void;
    0x004A09D0 => CreateUnit(@ecx u32, @eax i32, i32, u32) -> *mut c_void;
    0x0049F380 => InitUnits();
    0x0043FF00 => AiFocusDisabled(@eax *mut c_void);
    0x0043FE50 => AiFocusAir(@esi *mut c_void);

    0x0049FED0 => H_TransformUnit(@eax usize, usize) -> usize;
    0x00475710 => H_KillUnit(@eax usize) -> usize;
    0x00479730 => H_HallucinationHit(@eax usize, usize, usize, @esi usize) -> usize;
    0x004797B0 => H_DamageUnit(@ecx usize, @eax usize, usize, usize, usize) -> usize;
    0x00479AE0 => H_HitUnit(@eax usize, @edx usize, usize) -> usize;
    0x0047B2E0 => H_UnitCanRally(@eax usize) -> usize;
    0x00467340 => H_UnitSetHp(@eax usize, @ecx usize) -> usize;
    0x00402210 => H_UnitCanBeInfested(@edx usize) -> usize;
    0x0048B770 => H_DoMissileDamage(usize) -> usize;
    0x004C8040 => H_GiveUnit(usize, usize) -> usize;
    0x0045CD50 => H_PlaceCreepRect(usize, usize, usize, usize, usize) -> usize;
    0x0045CE90 => H_PlaceFinishedUnitCreep(@ecx usize, usize, usize) -> usize;
    0x004A2830 => H_AddAiToTrainedUnit(@ecx usize, @eax usize) -> usize;
    0x00435770 => H_AddBuildingAi(@ecx usize, @eax usize) -> usize;
    0x00433DD0 => H_AddTownUnitAi(@edi usize, @ebx usize) -> usize;
    0x0043DA20 => H_AddMilitaryAi(@eax usize, @ebx usize, usize) -> usize;
    0x004A1E50 => H_AiRemoveUnit(@ecx usize, @edx usize) -> usize;
    0x00439D60 => H_AiRemoveUnitMilitary(usize, usize) -> usize;
    0x00434C90 => H_AiRemoveUnitTown(@edi usize, usize) -> usize;
    0x00491870 => H_UnitMaxEnergy(@eax usize) -> usize;
    0x00475870 => H_UnitAttackRange(@eax usize, @ebx usize) -> usize;
    0x00476000 => H_UnitTargetAcquisitionRange(@edx usize) -> usize;
    0x004E5B40 => H_UnitSightRange(@edx usize, usize) -> usize;
    0x00475CE0 => H_CheckWeaponTargetingFlags(usize, @eax usize, @edx usize) -> usize;
    0x00492020 => H_CheckTechTargeting(
        usize, @edx usize, @edi usize, @esi usize, usize, usize, usize) -> usize;
    // ecx, edx really unused
    0x00474D90 => H_CheckOrderTargeting(
        @ebx usize, @eax usize, @esi usize, @ecx usize, @edx usize, usize) -> usize;
    // eax, esi really unused
    0x004746D0 => H_CheckFowOrderTargeting(
        @ebx usize, @edx usize, @ecx usize, @eax usize, @esi usize, @edi usize) -> usize;
    0x004E6340 => H_HideUnit(@eax usize) -> usize;
    0x004E6490 => H_ShowUnit(@edi usize) -> usize;
    0x0043ABB0 => H_AiAttackPrepare(usize, usize, usize, usize, usize) -> usize;
    0x0043A390 => H_AiRegionUpdateStrength(@esi usize) -> usize;
    0x0043CC40 => H_AiRegionUpdateTarget(@edi usize) -> usize;
    0x0043DE40 => H_AiRegionAbandonIfOverwhelmed(@edi usize) -> usize;
    0x0043A510 => H_AiRegionPickAttackTarget(usize) -> usize;
    0x0043F990 => H_AiTrainMilitary(usize) -> usize;
    0x0043E2E0 => H_AiAddMilitaryToRegion(@eax usize, usize, usize) -> usize;
    0x0043FC60 => H_AiStepRegion(@ecx usize, @eax usize) -> usize;
    0x0043B9E0 => H_AiTargetExpansion(usize) -> usize;
    0x004EC290 => H_StepUnitTimers(@eax usize) -> usize;
    0x00491B30 => H_StartCloaking(@eax usize) -> usize;
    0x00435210 => H_UnitAiWorker(usize) -> usize;
    0x004361A0 => H_AiTryProgressSpendingRequest(@ecx usize) -> usize;
    0x0043D910 => H_UnitAiMilitary(@eax usize) -> usize;
    0x00476930 => H_AiCanTargetAttackThis(@ecx usize, @eax usize) -> usize;
    0x00476730 => H_CanAttackUnit(@esi usize, @ebx usize, usize) -> usize;
    0x00476430 => H_IsOutsideAttackRange(@eax usize, usize) -> usize;
    0x00462EA0 => H_AiTryReturnHome(@eax usize, usize) -> usize;
    0x00467250 => H_PrepareBuildUnit(@edi usize, usize) -> usize;
    0x004E1D90 => H_CalculatePath(usize) -> usize;
    0x004465C0 => H_AiPlaceBuilding(@ecx usize, usize, usize, usize, usize) -> usize;
    0x004E29B0 => H_GetChokePointRegions(
        @ecx usize, @edx usize, usize, usize, usize, @eax usize) -> usize;
    0x00473FB0 => H_UpdateBuildingPlacementState(
        usize, usize, usize, usize, usize, usize, usize, usize, usize) -> usize;
    0x004461B0 => H_AiUpdateBuildingPlacementState(
        @ebx usize, @esi usize, usize, usize, usize) -> usize;
    0x00488210  => H_CreateLoneSprite(usize, usize, @edi usize, usize) -> usize;
);

whack_vars!(init_vars, 0x00400000,
    0x0057F0F0 => game: Game;
    0x0051CA14 => rng_seed: u32;
    0x006D11C8 => rng_enabled: u32;
    0x006283EC => first_hidden_unit: *mut Unit;
    0x00628430 => first_active_unit: *mut Unit;
    0x0069A604 => ai_regions: [*mut AiRegion; 8];
    0x0068FEE8 => player_ai: [PlayerAi; 0x8];
    0x006D1218 => loaded_save: *mut c_void;
    0x006D0F14 => is_replay: u32;
    0x0057F0B4 => is_multiplayer: u8;
    0x00512678 => command_user: u32;
    0x0051267C => unique_command_user: u32;
    0x00512680 => storm_command_user: u32;
    0x005005F8 => command_lengths: [u32; 0x60];
    0x006284E8 => selections: [*mut Unit; 0xc * 0x8];
    0x00597208 => client_selection: [*mut Unit; 0xc];
    0x0059CCA8 => units: [Unit; 0x6a4];
    0x0068C100 => first_ai_script: *mut c_void;
    0x0068C0F8 => first_free_ai_script: *mut c_void;
    0x00685108 => guard_ais: [[*mut c_void; 0x2]; 0x8];
    0x006D5BFC => pathing: *mut c_void;
    0x00596BBC => replay_data: *mut c_void;

    0x00513C30 => units_dat: [DatTable; 0x1];
    0x00513EC8 => orders_dat: [DatTable; 0x1];
    0x005136E0 => upgrades_dat: [DatTable; 0x1];
    0x005137D8 => techdata_dat: [DatTable; 0x1];
    0x00515A38 => flingy_dat: [DatTable; 0x7];
    0x00513FB8 => sprites_dat: [DatTable; 0x1];
    0x00513868 => weapons_dat: [DatTable; 0x1];
    0x00514010 => images_dat: [DatTable; 0x1];
    0x00513780 => portdata_dat: [DatTable; 0x6];
    0x00515498 => sfxdata_dat: [DatTable; 0x5];

    0x0046E2D8 => unit_requirement_table: *const u16;
    0x0046E0C7 => upgrade_requirement_table: *const u16;
    0x0046DE76 => tech_use_requirement_table: *const u16;
    0x0046DF96 => tech_research_requirement_table: *const u16;
    0x0046DD6D => order_requirement_table: *const u16;

    0x0045CA0C => aiscript_default_switch_table: [u32; 0x71];
    0x0045B892 => aiscript_switch_table_ptr: *mut u32;
    0x004D835C => iscript_default_switch_table: [u32; 0x45];
    0x004D750F => iscript_switch_table_ptr: *mut u32;

    0x006AA050 => active_ai_towns: [AiTownList; 0x8];
    0x006D1260 => map_tile_flags: *mut u32;
    0x0057EEE0 => players: [Player; 0xc];
    0x006D1200 => iscript_bin: *mut c_void;

    0x00629288 => sprite_hlines_end: [*mut c_void; 0x100];
    0x00629688 => sprite_hlines: [*mut c_void; 0x100];
    0x0064DEC4 => first_active_bullet: *mut c_void;
    0x00654874 => first_lone_sprite: *mut c_void;

    0x005122A0 => campaigns: [*mut c_void; 6];
    0x0062848C => screen_x: i32;
    0x006284A8 => screen_y: i32;
    0x00654868 => first_fow_sprite: *mut c_void;

    0x00512684 => local_player_id: u32;
    0x00652920 => draw_cursor_marker: u8;

    0x006509C4 => is_paused: u32;
    0x00641694 => is_targeting: u8;
    0x00640880 => is_placing_building: u32;

    0x006D11FC => active_iscript_unit: *mut c_void;
    0x006D11F4 => active_iscript_flingy: *mut c_void;
    0x006D11F8 => active_iscript_bullet: *mut c_void;

    0x006BB210 => unit_strength: [u32; 0xe4 * 2];
);

pub const AISCRIPT_OPCODE_CMP: usize = 0x0045B883;
pub const AISCRIPT_SWITCH_TABLE: usize = 0x0045B892;
pub const AISCRIPT_LOOP: usize = 0x0045B860;
pub const AISCRIPT_RET: usize = 0x0045C9AA;
pub const ISCRIPT_LOOP: usize = 0x004D74F4;
pub const ISCRIPT_OPCODE_CMP: usize = 0x004D7504;
pub const ISCRIPT_SWITCH_TABLE: usize = 0x004D750F;

whack_funcs!(stdcall, init_funcs_storm, 0x15000000,
    0x150205D0 => SMemFree(*mut u8, *const u8, u32, u32);
);

whack_hooks!(stdcall, 0x15000000,
    0x15017960 => SFileOpenFileEx_Hook(*mut c_void, *const u8, u32, *mut *mut c_void) -> u32;
    0x15013F50 => SFileGetFileSize_Hook(*mut c_void, *mut u32) -> u32;
    0x15016360 => SFileReadFile_Hook(*mut c_void, *mut u8, u32, *mut u32, *mut c_void) -> u32;
    0x150152B0 => SFileCloseFile_Hook(*mut c_void);
);

pub struct Game;
pub struct Unit;
pub struct AiRegion;

#[repr(C, packed)]
pub struct Sprite {
    pub prev: *mut Sprite,
    pub next: *mut Sprite,
    pub sprite_id: u16,
    pub player: u8,
    pub selection_index: u8,
    pub visibility_mask: u8,
    pub elevation_level: u8,
    pub flags: u8,
    pub selection_flash_timer: u8,
    pub index: u16,
    pub width: u8,
    pub height: u8,
    pub position: Point,
    pub main_image: *mut Image,
    pub first_image: *mut Image,
    pub last_image: *mut Image,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

#[repr(C)]
pub struct Image {
    _data: [u8; 0x40],
}

#[repr(C)]
pub struct AiTownList {
    _data: [u8; 8],
}

#[repr(C)]
pub struct Player {
    _data: [u8; 0x24],
}

#[repr(C)]
pub struct PlayerAi {
    _data: [u8; 0x4e8],
}

#[repr(C)]
pub struct DatTable {
    data: *mut c_void,
    entry_size: u32,
    entry_count: u32,
}
