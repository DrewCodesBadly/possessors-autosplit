#![no_std]
extern crate alloc;

#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

#[cfg(debug_assertions)]
use alloc::format;
use asr::{
    future::next_tick,
    game_engine::unreal::{Module, UnrealPointer, Version},
    print_message,
    settings::Gui,
    timer::{pause_game_time, resume_game_time, set_variable, split, start},
    Address, Process,
};

asr::async_main!(stable);
asr::panic_handler!();

#[derive(Gui)]
struct Settings {
    /// Autostart
    #[default = true]
    start: bool,

    /// Split on demo end
    #[default = true]
    demo_end: bool,

    /// Split when collecting an ability
    #[default = true]
    abilities: bool,

    /// Split when collecting any item
    #[default = true]
    items: bool,
}

// TODO: Optimize all of this instead of using the unreal functions.
async fn main() {
    let mut settings = Settings::register();

    loop {
        let process = Process::wait_attach("Pose-Win64-Shipping.exe").await;
        process
            .until_closes(async {
                // Load initial info
                let (main_module_address, _main_module_size) =
                    process.get_module_range("Pose-Win64-Shipping.exe").unwrap();
                #[cfg(debug_assertions)]
                print_message("found main module");

                // I think it's V5.5 but it works anyway. 5.5 not supported yet.
                let module =
                    Module::wait_attach(&process, Version::V5_4, main_module_address).await;

                #[cfg(debug_assertions)]
                print_message("Attached to UE game");

                let world = module.g_world();
                let player: UnrealPointer<5> =
                    UnrealPointer::new(world, &["OwningGameInstance", "LocalPlayers"]);
                let mut last_demo_end = 0;
                let mut last_ability = 0;
                let mut last_item = 0;
                let mut luca_has_legs = true;
                // Used to determine when to start the timer.
                // The timer will start when
                // 1. We Have left the title screen (sets this bool to true)
                // 2. Game is finished loading (loading state was 3 and is now not.)
                // 3. Luca does not have legs.
                let mut check_next_load = false;
                let mut game_load_in_progress = false;

                #[cfg(debug_assertions)]
                print_message(&format!("World address: {:?}", world));

                loop {
                    settings.update();

                    // Are we in the game world, or the menu world, or loading the game world?
                    // For now, this only specifies if we're in the game world or not
                    if !module
                        .get_g_world_uobject(&process)
                        .and_then(|world| world.get_fname::<6>(&process, &module).ok())
                        .filter(|name| name.as_bytes() == "Pose_P".as_bytes())
                        .is_some()
                    {
                        check_next_load = true;
                    }
                    #[cfg(debug_assertions)]
                    set_variable("in_game_world", &format!("{}", check_next_load));

                    if let Ok(player_location) = player
                        .deref_offsets(&process, &module)
                        .and_then(|addr| process.read::<u64>(addr))
                    {
                        // INVALID, DEAD, ALIVE, RESPAWNING. Kinda pointless?
                        // If you want something to happen when you die maybe?
                        // let player_state: UnrealPointer<3> = UnrealPointer::new(Address::new(player_location), &[
                        //     "PlayerController",
                        //     "Pawn",
                        //     "CoreState",
                        // ]);
                        // set_variable("player state", &format!("{:?}", player_state.deref::<[u8; 64]>(&process, &module)));

                        // I'm just gonna use the ability unlock screen since I haven't tested
                        // this.
                        // let player_abilities: UnrealPointer<3> = UnrealPointer::new(Address::new(*player_location), &[
                        //     "PlayerController",
                        //     "Pawn",
                        //     "DefaultUnlockedAbilities",
                        // ]);
                        //

                        // ABP_PosePlayerPawn_C has some real fun stuff in there. HUUUUGE
                        // class. of all things here i'm gonna try to use legless luca (lmao)
                        // as a start autosplit.
                        // leglessLuca is a bool in that class which, as you may guess,
                        // represents whether or not luca has her legs. It's pretty safe to say
                        // if the last read was nonexistent or false we are starting a new
                        // game.
                        let legless_luca: UnrealPointer<4> = UnrealPointer::new(
                            Address::new(player_location),
                            &["PlayerController", "Pawn", "LeglessLuca"],
                        );
                        // Not sure what this does yet oopsie spoilers
                        // let hornless_luca: UnrealPointer<4> = UnrealPointer::new(
                        //     Address::new(player_location),
                        //     &["PlayerController", "Pawn", "HornlessLuca"],
                        // );
                        luca_has_legs = legless_luca
                            .deref::<u8>(&process, &module)
                            .ok()
                            .filter(|no_legs| *no_legs == 1)
                            .is_none();

                        let load_status: UnrealPointer<4> = UnrealPointer::new(
                            Address::new(player_location),
                            &[
                                "PlayerController",
                                "MyHUD",
                                "LoadingScreen",
                                "CurrentStatus",
                            ],
                        );

                        // This might be useful later, idk
                        // let player_stats: UnrealPointer<4> = UnrealPointer::new(
                        //     Address::new(player_location),
                        //     &["PlayerController", "Pawn", "PlayerStats"],
                        // );

                        #[cfg(debug_assertions)]
                        set_variable(
                            "load_status",
                            &format!("{:?}", load_status.deref::<u8>(&process, &module)),
                        );

                        // Is loading when this == 3
                        // This also contains the logic for starting the timer.
                        if load_status
                            .deref::<u8>(&process, &module)
                            .ok()
                            .filter(|s| *s == 3)
                            .is_some()
                        {
                            pause_game_time();
                            if check_next_load {
                                game_load_in_progress = true;
                                check_next_load = false;
                            }
                        } else {
                            resume_game_time();
                            if game_load_in_progress {
                                game_load_in_progress = false;
                                if !luca_has_legs {
                                    start();
                                }
                            }
                        }

                        // These three work the same way.
                        // They are either broken (The pointer to the screen is null) before
                        // the screen has triggered once and loaded,
                        // or they are a boolean 1 or 0, 1 when they're shown.
                        let demo_end: UnrealPointer<4> = UnrealPointer::new(
                            Address::new(player_location),
                            &["PlayerController", "MyHUD", "DemoEndScreen", "bIsActive"],
                        );
                        // This logic is kinda jank but it should be fine
                        if let Ok(b) = demo_end.deref::<u8>(&process, &module) {
                            if settings.demo_end && b == 1 && last_demo_end == 0 {
                                last_demo_end = 1;
                                split();
                            } else {
                                last_demo_end = b;
                            }
                        } else {
                            last_demo_end = 0;
                        }

                        let ability_unlock: UnrealPointer<4> = UnrealPointer::new(
                            Address::new(player_location),
                            &[
                                "PlayerController",
                                "MyHUD",
                                "AbilityUnlockedScreen",
                                "bIsActive",
                            ],
                        );
                        if let Ok(b) = ability_unlock.deref::<u8>(&process, &module) {
                            if settings.abilities && b == 1 && last_ability == 0 {
                                last_ability = 1;
                                split();
                            } else {
                                last_ability = b;
                            }
                        } else {
                            last_ability = 0;
                        }

                        let item_pickup: UnrealPointer<4> = UnrealPointer::new(
                            Address::new(player_location),
                            &["PlayerController", "MyHUD", "ItemPickupScreen", "bIsActive"],
                        );
                        if let Ok(b) = item_pickup.deref::<u8>(&process, &module) {
                            if settings.items && b == 1 && last_item == 0 {
                                last_item = 1;
                                split();
                            } else {
                                last_item = b;
                            }
                        } else {
                            last_item = 0;
                        }
                    } else {
                        luca_has_legs = true;
                    }

                    #[cfg(debug_assertions)]
                    set_variable("luca_has_legs", &format!("{}", luca_has_legs));

                    next_tick().await;
                }
            })
            .await;
    }
}
