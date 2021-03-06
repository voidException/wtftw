extern crate libc;

use core::Workspaces;
use window_manager::WindowManager;
use window_system::WindowSystem;
use window_system::Window;
use config::{ GeneralConfig, Config };

pub type KeyHandler<'a> = Box<Fn<(WindowManager<'a>, &'a (WindowSystem + 'a), &'a GeneralConfig<'a>), WindowManager<'a>> + 'a>;
pub type MouseHandler<'a> = Box<Fn<(WindowManager<'a>, &'a (WindowSystem + 'a), &'a GeneralConfig<'a>, Window), WindowManager<'a>> + 'a>;
pub type ManageHook<'a> = Box<Fn<(Workspaces<'a>, &'a (WindowSystem + 'a), Window), Workspaces<'a>> + 'a>;
pub type StartupHook<'a> = Box<Fn<(WindowManager<'a>, &'a (WindowSystem + 'a), &'a Config<'a>), WindowManager<'a>> + 'a>;
pub type LogHook<'a> = Box<FnMut<(WindowManager<'a>, &'a (WindowSystem + 'a)), ()> + 'a>;

extern {
    pub fn waitpid(fd: libc::pid_t, status: *mut libc::c_int, options: libc::c_int) -> libc::pid_t;
}

/// Some default handlers for easier config scripts
pub mod default {
    use std::os;
    use std::ptr::null;
    use std::io::process::Command;
    use std::thread::Thread;
    use std::path::BytesContainer;
    use serialize::json;
    use core::Workspaces;
    use window_manager::WindowManager;
    use window_system::WindowSystem;
    use window_system::Window;
    use config::GeneralConfig;
    use handlers::libc::funcs::posix88::unistd::execvp;
    use std::ffi::CString;

    pub fn start_terminal<'a>(window_manager: WindowManager<'a>, _: &WindowSystem,
                          config: &GeneralConfig) -> WindowManager<'a> {
        let (terminal, args) = config.terminal.clone();
        let arguments : Vec<String> = if args.is_empty() {
            Vec::new()
        } else {
            args.split(' ').map(String::from_str).collect()
        };

        Thread::scoped(move || {
            debug!("spawning terminal");
            let command = if arguments.is_empty() {
                Command::new(terminal).detached().spawn()
            } else {
                Command::new(terminal).args(arguments.as_slice()).detached().spawn()
            };

            if let Err(_) = command {
                panic!("unable to start terminal")
            }
        }).detach();

        window_manager.clone()
    }

    pub fn start_launcher<'a>(window_manager: WindowManager<'a>, _: &WindowSystem,
                          config: &GeneralConfig) -> WindowManager<'a> {
        let launcher = config.launcher.clone();
        Thread::scoped(move || {
            debug!("spawning launcher");
            match Command::new(launcher).detached().spawn() {
                Ok(_) => (),
                _     => panic!("unable to start launcher")
            }
        }).detach();

        window_manager.clone()
    }

    pub fn switch_to_workspace<'a>(window_manager: WindowManager<'a>, window_system: &WindowSystem,
                               config: &GeneralConfig<'a>, index: usize) -> WindowManager<'a> {
        window_manager.view(window_system, index as u32, config)
    }

    pub fn move_window_to_workspace<'a>(window_manager: WindowManager<'a>, window_system: &WindowSystem,
                                    config: &GeneralConfig<'a>, index: usize) -> WindowManager<'a> {
        window_manager.move_window_to_workspace(window_system, config, index as u32)
    }

    /// Restart the window manager by calling execvp and replacing the current binary
    /// with the new one in memory.
    /// Pass a list of all windows to it via command line arguments
    /// so it may resume work as usual.
    pub fn restart<'a>(window_manager: WindowManager<'a>, _: &WindowSystem, c: &GeneralConfig<'a>) -> WindowManager<'a> {
        // Get absolute path to binary
        let filename = os::make_absolute(&Path::new(os::args()[0].clone())).unwrap();
        // Collect all managed windows
        let window_ids : String = json::encode(&window_manager.workspaces.all_windows_with_workspaces());

        // Create arguments
        let program_name = os::args()[0].clone();
        let resume = &"--resume";
        let windows = window_ids;
        let filename_c = CString::from_slice(filename.container_as_str().unwrap().as_bytes());

        for ref p in c.pipes.iter() {
            match p.write().unwrap().wait() {
                _ => ()
            }
        }

        unsafe {
            let mut slice : &mut [*const i8; 4] = &mut [
                CString::from_slice(program_name.as_bytes()).as_ptr(),
                CString::from_slice(resume.as_bytes()).as_ptr(),
                CString::from_slice(windows.as_bytes()).as_ptr(),
                null()
            ];
            execvp(filename_c.as_ptr(), slice.as_mut_ptr());
        }

        window_manager.clone()
    }

    /// Stop the window manager
    pub fn exit<'a>(w: WindowManager<'a>, _: &WindowSystem, _: &GeneralConfig<'a>) -> WindowManager<'a> {
        WindowManager { running: false, dragging: None, workspaces: w.workspaces }
    }

    pub fn shift<'a>(index: u32, workspace: Workspaces<'a>, window: Window) -> Workspaces<'a> {
        workspace.shift_window(index, window)
    }
}
