## LogSimGUI
Was originally a typo, but is now the name of a Logic Gate simulator. The app is coded in [Rust](https://www.rust-lang.org/) with [eframe](https://crates.io/crates/eframe).

## Running
MacOS is the most supported, but it is also tested on Windows and Linux (debian based).
In `/releases/latest_date/` there should be built binaries for MacOS, Windows, and Linux.

You can also use LogSimGUI in the browser [here](https://logsimgui.ga).

You can also build and run the native application from source:
```sh
git clone "https://github.com/MasonFeurer/LogSimGUI.git"
cd LogSimGUI/native
cargo run --release
```
On Linux, you may have to install a few packages first:
```sh
sudo apt install libglib2.0-dev
sudo apt install libatk1.0-dev
sudo apt install libcairo2-dev
sudo apt install libpango1.0-dev
sudo apt install librust-gdk-dev
```
