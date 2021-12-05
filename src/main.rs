mod configuration;
mod register;
mod runner;
use runner::functions;
use delay_timer;
use anyhow::Result;
use delay_timer::prelude::*;
use std::time::Duration;
use log::{info, error};
use configuration::reader;
use register::utils;
use etcd_client::*;


#[tokio::main]
async fn main() -> Result<()>{
    log4rs::init_file("conf/log4rs.yaml", Default::default()).unwrap();
    let conf = match reader::read("conf/config.toml"){
        Ok(c) =>c,
        Err(e) => {
            error!("{}", e);
            return Ok(())
        }
    };

    info!("micron started, version = [ 0.1.1 ]");
    info!("Begin init task ...");
    let delay_timer = DelayTimerBuilder::default().build();
    let mut s_name_vec:Vec<String> = Vec::new();
    let mut i = 0;
    for s in conf.service{
        i = i + 1;
        let maximum_running_time = match s.max_wait_for_next{
            Some(m)=> m,
            None => {
                let default_num = 5;
                info!("max_await_for_next in task [{0}] use [{1}] by default", i, default_num);
                default_num
            }
        };
        let r = functions::build_task_async_execute_process(i, maximum_running_time, &s.schedule, &s.shell);
        match r {
            Ok(task) => {
                match delay_timer.add_task(task) {
                    Ok(_) => {
                        info!("init successfully,task id = [{0}],task name= [{1}]", i , &s.name);
                        s_name_vec.push(s.name.clone());
                    },
                    Err(e) => info!("add task error,task id = [{0}],task name= [{1}],error detail: \n{2}", i , &s.name, e)
                } 
            },
            Err(e)=> info!("build task error,task id = [{0}],task name= [{1}],error detail: \n{2}",i , &s.name, e)
        }
    }
    info!("==================All job is been inited!================\n");


    let enable_resister = match conf.register.enable_resister{
        Some(x)=>x,
        None=>false
    };
    let _enable_pull = match conf.register.enable_pull{
        Some(x)=>x,
        None=>false
    };
    let _enable_push = match conf.register.enable_push{
        Some(x)=>x,
        None=>false
    };
    let lease_time = match conf.register.lease_time{
        Some(x)=>x,
        None=>10
    };
    let keep_alive = match conf.register.keep_alive{
        Some(x)=>x,
        None=>5
    };
    let mut interval = tokio::time::interval(Duration::from_secs(6));
    if enable_resister {
        let register = conf.register.register.unwrap();
        let reg_root = conf.register.reg_root.unwrap();
        let mut is_conn = false;
        
        while !is_conn {
            let reg_servers:Vec<String> = register.split(",").map(|x|x.to_owned()).collect();
            info!("try to connect to register [{:?}]", &reg_servers);
            let result = Client::connect(reg_servers,None).await;
            let mut client = match result{
                Ok(c)=>c,
                Err(e)=>{
                    is_conn = false;
                    error!("{}", e);
                    interval.tick().await;
                    continue;
                }
            };
            let r = reg_root.clone();
            let s = s_name_vec.clone();
            let result =tokio::task::spawn(async move { utils::keep_alive_lease(&mut client, &r, &s, lease_time, keep_alive).await}).await?;
            match result{
                Ok(_)=>{
                    is_conn = true;
                },
                Err(e)=>{
                    is_conn = false;
                    error!("{}", e);
                    info!("conntect to etcd failed, retry again...");
                    interval.tick().await; 
                }
            }
        }
    }else {
        info!("Register is disable by config,ignore register...");
        loop{
            interval.tick().await;
        }

    }
    Ok(delay_timer.stop_delay_timer()?)
}
