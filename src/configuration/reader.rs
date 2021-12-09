use std::fs::File;
use std::io::prelude::*;
use log::{info, error};
use crate::configuration::conf::Config;
use std::fs;
use std::os::unix::fs::MetadataExt;


pub fn read(file_path: &str) ->Result<Config, &str> {
    let mut file = match File::open(file_path) {
        Ok(f) => f,
        Err(e) => panic!("no such file {} exception:{}", file_path, e)
    };
    let mut str_val = String::new();
    match file.read_to_string(&mut str_val) {
        Ok(s) => s,
        Err(e) => panic!("Error Reading file: {}", e)
    };
    let config:Config  = match toml::from_str(&str_val){
        Ok(c) =>c,
        Err(e) => {
            error!("{}, NOTICE! the line num in error message may incorrect", e);
            return Err("parser config failed!")
        }
    };
    //If enable register, check register config
    match config.register.enable_resister {
        Some(r)=>{
            if r {
                let k = match config.register.keep_alive {
                    Some(k)=>k,
                    None=>0,
                };
                let l = match config.register.lease_time {
                    Some(l)=>l,
                    None=>0,
                };
                if k>0 && l>0 && k>l as u64 - 5 {
                    error!("configed lease = [{0}], keep_alive = [{1}], and lease must be 5(s) more then keep_alive atleast", l, k);
                    return Err("lease&keep_alive relation mismatching");
                }
            }
        },
        None=>{}
    }
    
    match checker(config){
        Ok(_)=>info!("config check pass"),
        Err(e)=>{
            error!("config check error: {}", e);
            return Err("config check failed!");
        }
    }
    let config:Config  = toml::from_str(&str_val).unwrap();
    return Ok(config);
}


fn checker(config:Config)->Result<(), String>{
    let mut i = 0;
    let mut s_list:Vec<String> = Vec::new();
    for s in config.service{
        i = i + 1;
        //check duplicated
        if s_list.contains(&s.name){
            error!("service name [{}] is duplicated", s.name);
            return Err("dupicated service".into())
        }else{
            s_list.push(s.name)
        }
        //check numberic of service
        if i > 100 {
            let err_msg= "numberic of service is limited to 100, check config!";
            error!("{}", err_msg);
            return Err(err_msg.into());
        }
        //check if start with sh
        if !s.shell.starts_with("sh ") {
            let err_msg= "service.shell must be start with 'sh ' in config.toml";
            error!("service[{}].shell check failed!", i);
            return Err(err_msg.into());
        }
        //check numberic of vars in command string
        let shell = String::from(&s.shell);
        let splited:Vec<&str> = shell.split_whitespace().collect();
        if splited.len()<2{
            error!("service[{0}].shell = [{1}] must contain script path", i, &s.shell);
            return Err("shell format error".into())
        }
        //check script file in command string
        let script_path = splited[1];
        match fs::metadata(script_path){
            Ok(m)=>{
                let mode = m.mode();
                let user_have_execute = mode & 0o100;
                let group_has_write_access = mode & 0o020;
                let others_have_write_access = mode & 0o002;
                // only current user can modify script file
                if group_has_write_access!=0 || others_have_write_access!=0{
                    error!("privilege of script [{}] must be write only by owner!", script_path);
                    return Err("script file privilege error!".into())
                }
                // current user must has execute privilege on script file
                if user_have_execute==0{
                    error!("owner must has execute privilege on script [{}] !", script_path);
                    return Err("script file privilege error!".into()) 
                }
                // current user must be owner of script file
                if m.uid() != users::get_current_uid(){
                    error!("The owner of script [{}] [{}] is not current user [{}]", m.uid(), script_path, users::get_current_uid());
                    return Err("script owner is not current user".into())
                }
            },
            Err(e)=>{
                error!("Get privilege of script [{}] error: [{}]", script_path, e);
                return Err("get privilege failed".into())
            }
        };
    }
    return Ok(())
}
