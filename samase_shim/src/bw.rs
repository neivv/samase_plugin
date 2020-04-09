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

    0x004100C4 => SFileOpenFileEx(*mut c_void, *const u8, u32, *mut *mut c_void) -> u32;
    0x00410142 => SFileGetFileSize(*mut c_void, *mut u32) -> u32;
    0x004100B8 => SFileCloseFile(*mut c_void);
    0x00410148 => SFileReadFile(*mut c_void, *mut u8, u32, *mut u32, u32) -> u32;
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
    0x00512678 => command_user: u32;
    0x0051267C => unique_command_user: u32;
    0x005005F8 => command_lengths: [u32; 0x60];
    0x006284E8 => selections: [*mut Unit; 0xc * 0x8];
    0x00597208 => client_selection: [*mut Unit; 0xc];
    0x0059CCA8 => units: [Unit; 0x6a4];
    0x0068C100 => first_ai_script: *mut c_void;
    0x0068C0F8 => first_free_ai_script: *mut c_void;
    0x00685108 => guard_ais: [[*mut c_void; 0x2]; 0x8];
    0x006D5BFC => pathing: *mut c_void;

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
