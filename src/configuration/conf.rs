//struct define for configuration
use serde_derive::Deserialize;
#[derive(Deserialize)]
pub struct Config {
    pub meta: Meta,
    pub register: Register,
    pub service: Vec<Service>,
}

#[derive(Deserialize)]
pub struct Service {
    pub name: String,
    pub schedule: String,
    pub shell: String,
    pub max_wait_for_next: Option<u64> 
}

#[derive(Deserialize)]
pub struct Meta {
    pub schema_version: Option<String>,
}

#[derive(Deserialize)]
pub struct Register {
    pub enable_resister: Option<bool>,
    pub register: Option<String>,
    pub reg_root:Option<String>,
    pub register_auth: Option<String>,
    pub register_keys: Option<String>,
    pub enable_pull: Option<bool>,
    pub enable_push: Option<bool>,
    pub lease_time: Option<i64>,
    pub keep_alive: Option<u64>,
}