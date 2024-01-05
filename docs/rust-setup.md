- ### Install Rust: 

```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```
- ### Install essential dependencies for building a substrate node:

Ubuntu: 
```sh
sudo apt update
# SBP-M2 review: missing protobuf-compiler
sudo apt install -y cmake pkg-config libssl-dev git gcc build-essential git clang libclang-dev
```
Arch Linux:
```sh
pacman -Syu --needed --noconfirm cmake gcc openssl-1.0 pkgconf git clang
export OPENSSL_LIB_DIR="/usr/lib/openssl-1.0";
export OPENSSL_INCLUDE_DIR="/usr/include/openssl-1.0"
```
Mac OS:
```sh
brew update
brew install openssl cmake llvm
```

# SBP-M2 review: use stable
- ### Install the `wasm` target and the `nightly` toolchain for rust

# SBP-M2 review: use stable
```sh
rustup update nightly
rustup target add wasm32-unknown-unknown --toolchain nightly
```