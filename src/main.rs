use std::{
    os::unix::prelude::AsRawFd,
    panic,
    process::{exit, Child, Command},
    sync::{Arc, Mutex},
};

use options::ContainerOptions;

use crate::push_listener::new_listener;

mod build;
mod options;
mod push_listener;

fn main() {
    let options = options::parse_options();
    println!("Loaded options:\n{options:#?}");

    let mut build_version = 1;

    //Initial build
    build_version = match build::build_child(&options, build_version) {
        Ok(build_version) => build_version,
        Err(err) => {
            println!("Failed to build child. Err: {err}");
            exit(1);
        }
    };

    let child = match spawn_child(&options, build_version) {
        Ok(child) => child,
        Err(err) => {
            println!("Failed to start child. Err: {err}");
            exit(1)
        }
    };

    let child = Arc::new(Mutex::new(child));
    let hook_child_ref = child.clone();
    //Set panic handler to kill child
    panic::set_hook(Box::new(move |_| {
        kill_child(hook_child_ref.clone());
    }));
    let hook_child_ref = child.clone();

    //Spawn http server
    let listener = match new_listener(&options) {
        Ok(listener) => listener,
        Err(err) => {
            println!("Failed to spawn http-server. Err {err}");
            kill_child(child);
            exit(1)
        }
    };
    let listener_clone = listener.clone();
    ctrlc::set_handler(move || {
        kill_child(hook_child_ref.clone());

        let raw_handle = listener_clone.as_raw_fd();
        unsafe {
            libc::shutdown(raw_handle, libc::SHUT_RD);
        }
        exit(0);
    })
    .unwrap();

    //Wait until received a valid http post
    loop {
        push_listener::read_push(listener.clone(), &options);

        build_version = match build::build_child(&options, build_version) {
            Ok(build_version) => build_version,
            Err(err) => {
                println!("Failed to build child. Err: {err}");
                exit(1);
            }
        };

        kill_child(child.clone());

        let new_child = match spawn_child(&options, build_version) {
            Ok(child) => child,
            Err(err) => {
                println!("Failed to start child. Err: {err}");
                exit(1)
            }
        };

        let mut child_lock = child.lock().unwrap();
        *child_lock = new_child;
    }
}

fn kill_child(child: Arc<Mutex<Child>>) {
    let mut child = match child.lock() {
        Ok(child) => child,
        Err(err) => {
            println!("Failed to kill child due to mutex being poisoned. Kill child manually if it still exists. Err {err}");
            exit(1);
        }
    };
    match child.kill() {
        Ok(_) => {}
        Err(err) => {
            println!("Failed to kill child. It is already dead. Err {err}");
        }
    }
}

fn spawn_child(options: &ContainerOptions, build_version: u64) -> Result<Child, std::io::Error> {
    let main_arg =
        options.run_cmd[0].replace("{{build_version}}", format!("{build_version}").as_str());
    let args: Vec<String> = options.run_cmd[1..options.run_cmd.len()]
        .iter()
        .map(|arg| arg.replace("{{build_version}}", format!("{build_version}").as_str()))
        .collect();

    println!("Spawning: {main_arg} {args:?}");

    Ok(Command::new(main_arg).args(args).spawn()?)
}
