use std::fs::read_to_string;

use toml::Value;

#[derive(Clone, Debug)]
pub struct ContainerOptions {
    pub port: u16,
    pub build_cmds: Vec<Vec<String>>,
    pub run_cmd: Vec<String>,
    pub build_attempts: u64,
    pub delayed_kill: bool,
    pub secret_key: Option<String>,
}

pub fn parse_options() -> ContainerOptions {
    let file_raw =
        read_to_string("./Container.toml").expect("FATAL:\nFailed to load Container.toml");

    let value: Value = file_raw
        .parse()
        .expect("Failed to parse Conatiner.toml to valid toml data");
    //Parse build commands
    let build_commands_value: &Vec<Value> = value
        .get("build")
        .expect("Failed to find \"build\" array in options")
        .as_array()
        .expect("Failed to parse \"build\" to array");

    let mut build_cmds = vec![];

    for value_vec in build_commands_value {
        let mut cmd_vec = vec![];
        for arg_value in value_vec
            .as_array()
            .expect("Failed to parse \"build\" cmds to vec")
        {
            let arg = arg_value
                .as_str()
                .expect("Failed to pass build arg to string");
            cmd_vec.push(arg.to_string());
        }
        build_cmds.push(cmd_vec);
    }

    //Parse run cmd
    let run_arg_vec = value
        .get("run")
        .expect("Failed to find \"run\" in options")
        .as_array()
        .expect("Failed to parse \"run\" to vec");

    let mut run_cmd = vec![];

    for arg_value in run_arg_vec {
        let arg = arg_value
            .as_str()
            .expect("Failed to parse \"run\" arg to string");
        run_cmd.push(arg.to_string());
    }

    let build_attempts = match value.get("build_attemps") {
        Some(value) => match value.as_integer() {
            Some(value) => value as u64,
            None => panic!("Failed to parse \"build_attemps\" to u64"),
        },
        None => 1,
    };

    let port = value
        .get("port")
        .expect("Failed to find \"port\" in options")
        .as_integer()
        .expect("Failed to parse \"port\" to u16") as u16;

    let delayed_kill = match value.get("delayed_kill") {
        Some(value) => value
            .as_bool()
            .expect("Failed to parse \"delayed_kill\" to boolean "),
        None => false,
    };

    let secret_key = match value.get("secret") {
        Some(value) => match value.as_str() {
            Some(str) => Some(str.to_string()),
            None => None,
        },
        None => None,
    };

    ContainerOptions {
        port,
        build_cmds,
        run_cmd,
        build_attempts,
        delayed_kill,
        secret_key,
    }
}
