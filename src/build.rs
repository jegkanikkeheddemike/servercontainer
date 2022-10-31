use crate::options::ContainerOptions;
use std::error::Error;
use std::fmt::Display;
use std::process::Command;

pub fn build_child(options: &ContainerOptions, build_version: u64) -> Result<u64, Box<dyn Error>> {
    //First pull from git
    Command::new("git").args(["reset", "--hard"]).output()?;
    Command::new("git").args(["pull"]).output()?;

    for build_step in &options.build_cmds {
        let mut attempts = 0;
        loop {
            let build_step: Vec<String> = build_step
                .iter()
                .map(|arg| {
                    arg.replace(
                        "{{build_version}}",
                        format!("{}", build_version + 1).as_str(),
                    )
                })
                .collect();

            println!("Building: {build_step:?}");

            let main_arg = build_step[0].clone();
            let args = &build_step[1..build_step.len()];
            let build_process = Command::new(main_arg).args(args).spawn()?;

            let output = build_process.wait_with_output()?;
            if output.status.success() {
                break;
            } else if attempts == options.build_attempts {
                return Err(Box::new(BuildError {
                    build_step: build_step.clone(),
                }));
            } else {
                attempts += 1;
            }
        }
    }
    Ok(build_version + 1)
}

#[derive(Debug)]
struct BuildError {
    build_step: Vec<String>,
}
impl Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Failed to build argument {:?}",
            self.build_step
        ))
    }
}

impl Error for BuildError {}
