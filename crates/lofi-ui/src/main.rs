mod app;
mod engine;
mod lcd;
mod pulse;

use lofi_sim::{Simulation, DEFAULT_GROUP_JOIN_US};

fn main() -> eframe::Result<()> {
    // WSLg's Wayland backend crashes winit ("Broken pipe"), and its hardware GL
    // (zink) often fails to create a context. The X11/XWayland path with
    // software GL (llvmpipe) is reliable. Steer there under WSL unless the user
    // has chosen otherwise. Needs the libxkbcommon-x11-0 package.
    if is_wsl() {
        if std::env::var_os("LIBGL_ALWAYS_SOFTWARE").is_none() {
            std::env::set_var("LIBGL_ALWAYS_SOFTWARE", "1");
            std::env::set_var("GALLIUM_DRIVER", "llvmpipe");
        }
        // Force the X11 backend by hiding the (broken) Wayland socket from winit.
        if std::env::var_os("DISPLAY").is_some() {
            std::env::remove_var("WAYLAND_DISPLAY");
            // The X11 backend dlopens libxkbcommon-x11; without it winit panics
            // deep inside the event loop. Fail early with a clear instruction.
            if !x11_keyboard_lib_present() {
                eprintln!(
                    "lofi-ui: the X11 backend needs libxkbcommon-x11, which is not installed.\n\
                     Install it with:  sudo apt-get install libxkbcommon-x11-0\n\
                     (WSLg's Wayland backend crashes winit, so the X11 path is required here.)"
                );
                std::process::exit(1);
            }
        }
    }

    // Four boxes by default: two panned left, two right, mesh active immediately.
    let mut sim = Simulation::new(4, 0x10f1, 0, DEFAULT_GROUP_JOIN_US);
    sim.schedule_demo_drop();
    sim.with_node_mix(0, |m| m.pan = -1.0);
    sim.with_node_mix(1, |m| m.pan = -0.6);
    sim.with_node_mix(2, |m| m.pan = 0.6);
    sim.with_node_mix(3, |m| m.pan = 1.0);

    let engine = engine::Engine::new(sim);
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "lofi mesh simulator",
        options,
        Box::new(|_cc| Ok(Box::new(app::LofiApp::new(engine)))),
    )
}

/// True if `libxkbcommon-x11` is registered with the dynamic linker.
fn x11_keyboard_lib_present() -> bool {
    std::process::Command::new("ldconfig")
        .arg("-p")
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).contains("libxkbcommon-x11.so"))
        .unwrap_or(false)
}

fn is_wsl() -> bool {
    if std::env::var_os("WSL_DISTRO_NAME").is_some() || std::env::var_os("WSL_INTEROP").is_some() {
        return true;
    }
    std::fs::read_to_string("/proc/version")
        .map(|v| v.to_ascii_lowercase().contains("microsoft"))
        .unwrap_or(false)
}
