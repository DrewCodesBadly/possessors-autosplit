# possessors-autosplit

An auto splitter for Possessor(s) (The demo)

Some details are in the comments in lib.rs.
The current autosplitter is uses features from the asr crate
specific to unreal instead of just getting the pointer paths because i'm lazy, but going through the
CXX headers for the offsets and swapping everything out might improve performance.

The autosplitter tracks data related to the HUD - loads are removed whenever WBP_LoadingScreen_C.CurrentStatus == 3.
Similarly, it tracks if the ability screen and demo end screen exist and are being shown.
Finally, it tracks the variable leglessLuca in the player pawn to track when a new game starts.

Dumping the CXX headers and inspecting classes can be done using UE4SS just fine,
using the experimental release with 5.5 support. However, a different AOB was needed to
find GUObjectArray - the settings and AOBs used are in this repository.

Below is taken from the auto splitter template.
## Compilation

This auto splitter is written in Rust. In order to compile it, you need to
install the Rust compiler: [Install Rust](https://www.rust-lang.org/tools/install).

Afterwards install the WebAssembly target:
```sh
rustup target add wasm32-unknown-unknown --toolchain stable
```

The auto splitter can now be compiled:
```sh
cargo b --release
```

The auto splitter is then available at:
```
target/wasm32-unknown-unknown/release/possessors_autosplit.wasm
```

Make sure to look into the [API documentation](https://livesplit.org/asr/asr/) for the `asr` crate.

## Development

You can use the [debugger](https://github.com/LiveSplit/asr-debugger) while
developing the auto splitter to more easily see the log messages, statistics,
dump memory, step through the code and more.

The repository comes with preconfigured Visual Studio Code tasks. During
development it is recommended to use the `Debug Auto Splitter` launch action to
run the `asr-debugger`. You need to install the `CodeLLDB` extension to run it.

You can then use the `Build Auto Splitter (Debug)` task to manually build the
auto splitter. This will automatically hot reload the auto splitter in the
`asr-debugger`.

Alternatively you can install the [`cargo
watch`](https://github.com/watchexec/cargo-watch?tab=readme-ov-file#install)
subcommand and run the `Watch Auto Splitter` task for it to automatically build
when you save your changes.

The debugger is able to step through the code. You can set breakpoints in VSCode
and it should stop there when the breakpoint is hit. Inspecting variables may
not work all the time.
