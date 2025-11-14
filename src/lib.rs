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

    /// Split on ending
    #[default = true]
    end: bool,

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
                let mut last_ability = 0;
                let mut last_item = 0;
                let mut in_intro_cutscene = false;
                let mut credits_running = false;
                // Used to determine when to start the timer.
                // The timer will start when
                // 1. We Have left the title screen (sets this bool to true)
                // 2. Game is finished loading (loading state was 3 and is now not.)
                // 3. Luca does not have legs.

                #[cfg(debug_assertions)]
                print_message(&format!("World address: {:?}", world));

                loop {
                    settings.update();

                    // True when on the title screen.
                    // if module
                    //     .get_g_world_uobject(&process)
                    //     .and_then(|world| world.get_fname::<6>(&process, &module).ok())
                    //     .filter(|name| name.as_bytes() == "Pose_P".as_bytes())
                    //     .is_none()
                    // {
                    //     #[cfg(debug_assertions)]
                    //     set_variable("in_game_world", "true");
                    // } else {
                    //     #[cfg(debug_assertions)]
                    //     set_variable("in_game_world", "false");
                    // }

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

                        // let legless_luca: UnrealPointer<4> = UnrealPointer::new(
                        //     Address::new(player_location),
                        //     &["PlayerController", "Pawn", "LeglessLuca"],
                        // );
                        // let hornless_ptr: UnrealPointer<4> = UnrealPointer::new(
                        //     Address::new(player_location),
                        //     &["PlayerController", "Pawn", "HornlessLuca"],
                        // );

                        let load_status: UnrealPointer<4> = UnrealPointer::new(
                            Address::new(player_location),
                            &[
                                "PlayerController",
                                "MyHUD",
                                "LoadingScreen",
                                "CurrentStatus",
                            ],
                        );

                        // Useful for testing, I guess.
                        #[cfg(debug_assertions)]
                        {
                            let player_stats: UnrealPointer<4> = UnrealPointer::new(
                                Address::new(player_location),
                                &["PlayerController", "Pawn", "PlayerStats"],
                            );
                            if let Ok(addr) = player_stats.deref_offsets(&process, &module) {
                                set_variable(
                                    "sprint_mult",
                                    &format!(
                                        "{}",
                                        &process.read::<f32>(addr + 0x18).unwrap_or(-1.0),
                                    ),
                                );
                            }
                        }

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
                        } else {
                            resume_game_time();
                        }

                        // Start autosplit: Checks for the intro cutscene object disappearing
                        let intro_cutscene_active_ptr: UnrealPointer<4> = UnrealPointer::new(
                            Address::new(player_location),
                            &["PlayerController", "MyHUD", "IntroMovieScreen", "bIsActive"],
                        );
                        if intro_cutscene_active_ptr
                            .deref::<u8>(&process, &module)
                            .ok()
                            .filter(|v| *v == 1)
                            .is_some()
                        {
                            in_intro_cutscene = true;
                        } else if in_intro_cutscene {
                            in_intro_cutscene = false;
                            if settings.start {
                                start();
                            }
                        }
                        #[cfg(debug_assertions)]
                        set_variable(
                            "intro",
                            &format!(
                                "{:?}",
                                intro_cutscene_active_ptr.deref::<u8>(&process, &module)
                            ),
                        );

                        // End autosplit: Checks for the credits screen
                        let credits_cutscene_ptr: UnrealPointer<4> = UnrealPointer::new(
                            Address::new(player_location),
                            &["PlayerController", "MyHUD", "EndCreditsScreen", "bIsActive"],
                        );
                        if settings.end
                            && credits_cutscene_ptr
                                .deref::<u8>(&process, &module)
                                .ok()
                                .filter(|v| *v == 1)
                                .is_some()
                        {
                            if !credits_running {
                                split();
                            }
                            credits_running = true;
                        } else {
                            credits_running = false;
                        }
                        #[cfg(debug_assertions)]
                        set_variable(
                            "credits",
                            &format!("{:?}", credits_cutscene_ptr.deref::<u8>(&process, &module)),
                        );

                        // These three work the same way.
                        // They are either broken (The pointer to the screen is null) before
                        // the screen has triggered once and loaded,
                        // or they are a boolean 1 or 0, 1 when they're shown.
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
                    }

                    next_tick().await;
                }
            })
            .await;
    }
}
