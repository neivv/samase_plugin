#![allow(non_upper_case_globals, non_camel_case_types)]
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
    0x004564E0 => GameScreenRClick(@ecx *const Event);
);

whack_vars!(init_vars, 0x00400000,
    0x0057F0F0 => game: Game;
    0x0051CA14 => rng_seed: u32;
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

    0x006AA050 => active_ai_towns: [AiTownList; 0x8];
);

pub const AISCRIPT_OPCODE_CMP: usize = 0x0045B883;
pub const AISCRIPT_SWITCH_TABLE: usize = 0x0045B892;
pub const AISCRIPT_LOOP: usize = 0x0045B860;
pub const AISCRIPT_RET: usize = 0x0045C9AA;

whack_funcs!(stdcall, init_funcs_storm, 0x15000000,
    0x150205D0 => SMemFree(*mut u8, *const u8, u32, u32);
);

pub struct Game;
pub struct Unit;
pub struct AiRegion;
pub struct AiTownList;
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

#[repr(C)]
pub struct Event {
    pub ext_type: u32,
    pub ext_param: u32,
    pub param: u32,
    pub evt_type: u16,
    pub mouse_x: u16,
    pub mouse_y: u16,
}

pub fn event_to_scr(val: &Event) -> scr::Event {
    scr::Event {
        ext_type: val.ext_type,
        _unk4: 0,
        ext_param: val.ext_param,
        param: val.param,
        evt_type: val.evt_type,
        mouse_x: val.mouse_x,
        mouse_y: val.mouse_y,
        _padding16: 0,
        time: 0,
    }
}

pub fn event_to_1161(val: &scr::Event) -> Event {
    Event {
        ext_type: val.ext_type,
        ext_param: val.ext_param,
        param: val.param,
        evt_type: val.evt_type,
        mouse_x: val.mouse_x,
        mouse_y: val.mouse_y,
    }
}

pub mod scr {
    #[repr(C)]
    pub struct Event {
        pub ext_type: u32,
        pub _unk4: u32,
        pub ext_param: u32,
        pub param: u32,
        pub evt_type: u16,
        pub mouse_x: u16,
        pub mouse_y: u16,
        pub _padding16: u16,
        pub time: u32,
    }
}

#[test]
fn test_sizes() {
    use std::mem;
    assert_eq!(mem::size_of::<scr::Event>(), 0x1c);
    assert_eq!(mem::size_of::<Event>(), 0x12);
}
