# A CERTAIN TURN BASED ANIME GAME EMULATOR

## Supported Version: 4.6.5x

### Check ingame chat for added functionalities over base RobinSR

Run the game by clicking run.bat file.

Alternatively, run each in two different terminals:
```
cargo run --bin sdkserver
cargo run --bin gameserver
```

Tool website: [https://srtools.neonteam.dev](https://srtools.neonteam.dev)

Start battle by entering any calyx or cavern in the map.

Some scenes might not loaded properly. If you stuck at loading screen, remove `persistent` file.

# RobinSR

Original:

[https://git.xeondev.com/reversedrooms/RobinSR](https://git.xeondev.com/reversedrooms/RobinSR)

[https://git.xeondev.com/reversedrooms/JadeSR](https://git.xeondev.com/reversedrooms/JadeSR)

NeonTeam Version:
[https://git.neonteam.dev/amizing/robinsr](https://git.neonteam.dev/amizing/robinsr)

## Installation

#### Requirements

- [Rust](https://www.rust-lang.org/tools/install)

**NOTE**: Nightly Rust is required to build the project. To install it, first install
Rust itself, then run the following command:

```sh
rustup toolchain install nightly
rustup default nightly
```

## Usage

To begin using the server, you need to run both the SDK server and the game server. Follow the instruction above

# Thanks to NeonTeam for the base RobinSR
