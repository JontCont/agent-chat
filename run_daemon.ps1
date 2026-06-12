# Set MSVC environment variables for Rust compiler
$env:PATH = "C:\Program Files\Microsoft Visual Studio\18\Insiders\SDK\ScopeCppSDK\vc15\VC\bin;" + $env:PATH
$env:LIB = "C:\Program Files\Microsoft Visual Studio\18\Insiders\SDK\ScopeCppSDK\vc15\VC\lib;C:\Program Files\Microsoft Visual Studio\18\Insiders\SDK\ScopeCppSDK\vc15\SDK\lib"
$env:INCLUDE = "C:\Program Files\Microsoft Visual Studio\18\Insiders\SDK\ScopeCppSDK\vc15\VC\include;C:\Program Files\Microsoft Visual Studio\18\Insiders\SDK\ScopeCppSDK\vc15\SDK\include\shared;C:\Program Files\Microsoft Visual Studio\18\Insiders\SDK\ScopeCppSDK\vc15\SDK\include\ucrt;C:\Program Files\Microsoft Visual Studio\18\Insiders\SDK\ScopeCppSDK\vc15\SDK\include\um"
$env:CARGO_INCREMENTAL = "0"
$env:RUSTFLAGS = "-C codegen-units=1"

Write-Host "Starting Local Agent Daemon..." -ForegroundColor Green
cargo run --target-dir target_test -- --daemon
