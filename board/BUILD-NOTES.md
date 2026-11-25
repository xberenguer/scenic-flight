# Waveshare firmware

The release targets the round Waveshare ESP32-S3-Touch-AMOLED-1.75C with 32 MB flash. Install the ESP Xtensa Rust toolchain (including Rust source), make `xtensa-esp32s3-elf-gcc` available on `PATH`, and install esptool 4.9 in the Python environment used for the build.

Run `board/build-app.sh` from the repository root. Set `ESP_RUST_TOOLCHAIN` or `ESPTOOL_PYTHON` if your installation uses different names.

The output is an application image. The public release supports the measured Voxel Flight partition map with a factory application at `0x10000`, sized `0x500000`. The web flasher verifies that table and backs up the app before it writes firmware.
