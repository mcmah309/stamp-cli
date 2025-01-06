use std::env;

pub struct Config {
    pub host_address: String,
}

impl Config {
    /// Load the configuration from the environment. Panics if a required variable is not set.
    pub fn from_env() -> Self {
        return Self {
            host_address: env("SERVER_HOST_ADDRESS"),
        };
    }
}

fn env(var: &str) -> String {
    return env::var(var).expect(&*format!("`{var}` must be set."));
}