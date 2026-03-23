// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::Diagnostics::Debug::IsDebuggerPresent;
        if unsafe { IsDebuggerPresent() }.as_bool() {
            std::process::exit(0);
        }
    }

    cybersnatcher_lib::run()
}
