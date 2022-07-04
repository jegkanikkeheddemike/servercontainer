use std::{
    fs::read_to_string,
    process::{exit, Child, Command, ExitStatus},
    sync::Mutex,
};

use actix_web::{dev::Server, post, App, HttpServer, Responder};
use toml::Value;

lazy_static::lazy_static! {
    static ref PROCESS: Mutex<Option<Child>> = Mutex::new(None);
    static ref BUILD_COMMANDS: Mutex<Vec<Vec<String>>> = Mutex::new(vec![vec![]]);
    static ref RUN_COMMAND: Mutex<Vec<String>> = Mutex::new(vec![]);
    static ref CONTAINER_OPTIONS: Mutex<Option<ContainerOptions>> = Mutex::new(None);
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    set_handler();

    pull();
    update_container_options();
    build();
    spawn();
    container().await
}

fn container() -> Server {
    let port = CONTAINER_OPTIONS.lock().unwrap().clone().unwrap().port;

    let container = HttpServer::new(|| App::new().service(webhook_listen))
        .bind(("0.0.0.0", port))
        .expect("Failed to spawn container")
        .run();

    println!("Container spawned on port {port}");
    container
}

fn kill_process() {
    let mut lock = PROCESS.lock().unwrap();

    match &mut *lock {
        Some(child) => match child.kill() {
            Ok(()) => match child.wait() {
                Ok(status) => {
                    println!("Child exited with status: {status}");
                }
                Err(err) => {
                    println!(
                        "Child failed to exit properly with err {err}\nAttempting to continue"
                    );
                }
            },
            Err(_) => {
                println!("Child is already dead. Most likely crashed");
            }
        },
        None => {
            println!("No process to kill");
        }
    }
}

fn pull() {
    fn pull_inner() -> Result<(), Box<dyn std::error::Error>> {
        /*let output = Command::new("git").args(["reset", "--hard"]).output()?;

        println!("Git reset:\n{}", String::from_utf8(output.stdout)?);*/

        let output = Command::new("git").args(["pull"]).output()?;

        if !ExitStatus::success(&output.status) {
            let stderr = String::from_utf8_lossy(&output.stderr);

            eprintln!(
                "FATAL\nFailed to pull from github. Git exited with message\n{stderr}",
            );
            exit(1);
        }

        println!(
            "Pulled from github:\n{}",
            String::from_utf8_lossy(&output.stdout).to_string()
        );
        Ok(())
    }

    match pull_inner() {
        Ok(_) => {}
        Err(err) => {
            eprintln!(
                "FATAL\nFailed to pull from github with err: {}",
                &err
            );
        }
    }
}

#[derive(Clone)]
struct ContainerOptions {
    port: u16,
    build_cmds: Vec<Vec<String>>,
    run_cmd: Vec<String>,
}

fn update_container_options() {
    let file_raw =
        read_to_string("./Container.toml").expect("FATAL:\nFailed to load Container.toml");

    let value: Value = file_raw.parse().unwrap();

    fn parse(value: Value) -> Option<ContainerOptions> {
        let port = value["port"].as_integer()? as u16;

        let run_str = value["run"].as_str()?;

        let mut run_cmd = vec![];

        for arg_ref in run_str.split(" ") {
            run_cmd.push(arg_ref.to_string());
        }

        let build_arr = value["build"].as_array()?;

        let mut build_cmds = vec![];

        for build_cmd_val in build_arr {
            let build_cmd_str = build_cmd_val.as_str()?;

            let mut args = vec![];

            for arg_ref in build_cmd_str.split(" ") {
                args.push(arg_ref.to_string());
            }
            build_cmds.push(args);
        }

        Some(ContainerOptions {
            run_cmd,
            port,
            build_cmds,
        })
    }

    let res = parse(value);

    match res {
        Some(container_options) => {
            let mut lock = CONTAINER_OPTIONS.lock().unwrap();
            *lock = Some(container_options);
        }
        None => {
            eprintln!("FATAL:\nFailed to parse Container.toml");
            exit(1);
        }
    }
}

fn build() {
    fn build_inner() -> Result<(), Box<dyn std::error::Error>> {
        let container_option = CONTAINER_OPTIONS.lock().unwrap().clone();
        match container_option {
            Some(container_options) => {
                for build_cmd in container_options.build_cmds {
                    let child = Command::new(&build_cmd[0]).args(&build_cmd[1..]).spawn()?;

                    let output = child.wait_with_output()?;

                    if !ExitStatus::success(&output.status) {
                        println!(
                            "Build command {:?} failed with stderr:\n{}",
                            &build_cmd,
                            String::from_utf8_lossy(&output.stderr)
                        );
                        println!("Attempting to continue");
                    }
                }
            }
            None => {
                eprintln!(
                    "FATAL\nContainer options are not loaded!"
                );
            }
        }
        Ok(())
    }

    match build_inner() {
        Ok(()) => {
            println!("Build successful");
        }
        Err(err) => {
            eprintln!(
                "FATAL:\nFailed to build with err {}",
                &err
            );
            exit(1);
        }
    }
}

fn spawn() {
    fn spawn_inner() -> Result<Child, Box<dyn std::error::Error>> {
        let container_option = CONTAINER_OPTIONS.lock().unwrap().clone();

        match container_option {
            Some(container_options) => {
                let child = Command::new(&container_options.run_cmd[0])
                    .args(&container_options.run_cmd[1..])
                    .spawn()?;

                println!(
                    "Process started with command {:?}",
                    &container_options.run_cmd
                );

                Ok(child)
            }
            None => {
                eprintln!("FATAL:\nNo container options");
                exit(1);
            }
        }
    }
    match spawn_inner() {
        Ok(child) => {
            *PROCESS.lock().unwrap() = Some(child);
        }
        Err(err) => {
            eprintln!(
                "FATAL\nFailed to spawn child with err {}",
                err
            );
            exit(1)
        }
    }
}

#[post("/")]
async fn webhook_listen(_: String) -> impl Responder {
    kill_process();
    pull();
    update_container_options();
    build();
    spawn();
    "Ok"
}

fn set_handler() {
    ctrlc::set_handler(|| {
        let mut lock = PROCESS.lock().unwrap();
        let mut process = lock.take().unwrap();
        process.kill().unwrap();
        process.wait().unwrap();
        println!("\nClosed server");
    })
    .unwrap();
}
