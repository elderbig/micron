use std::time::Duration;
use log::{info, error, debug};
use etcd_client::*;


pub async fn keep_alive_lease(
    client: &mut Client, 
    reg_root: &String, 
    service_name: &Vec<String>,
    lease_time: i64,
    keep_alive: u64) -> Result<(), String> {
    let resp = client.lease_grant(lease_time, None).await;
    let lease = match resp{
        Ok(l)=>l,
        Err(e)=>{
            error!("{}", e);
            return Err("get grant lease faild!".into());
        }
    };
    let osname = match hostname::get(){
        Ok(name) => name,
        Err(e) => {
            error!("{}", e);
            panic!("Can not get hostname")
        }
    };
    let host_name = osname.to_string_lossy();
    info!("Got server host name = [{}]", host_name);
    let lease_id = lease.id();
    for name in service_name{
        let key = format!("{}/{}/members/{}", reg_root, name, host_name);
        let value = format!("{}", lease_id);
        let resp = put_with_failed(client, key.as_str(), value.as_str(), lease_id).await;
        match resp {
            Ok(_)=>{},
            Err(e)=> return Err(e)
        }
    }
    let mut interval = tokio::time::interval(Duration::from_secs(keep_alive));
    loop{
        let resp = keep_alive_with_failed(client, lease_id).await;
        match resp{
            Ok(_)=>{
                interval.tick().await; 
                continue
            },
            Err(e)=> return Err(e)
        };
        
    }
}

async fn put_with_failed(client:&mut Client, key:&str,value:&str, id:i64) -> Result<(), String>{
    for i in 0..=3{
        let options = Some(PutOptions::new().with_lease(id));
        let resp = client.put(key, value, options).await;
        match resp{
            Ok(_)=>{
                info!("put key=[{}],value=[{}]", key, value);
                return Ok(());
            },
            Err(e)=>{
                error!("lease [{}] failed .. [{}]",id, i);
                error!("{}", e);
            }
        }
    }
    return Err("put key failed!".into())
}

async fn keep_alive_with_failed(client:&mut Client, lease_id:i64) -> Result<(), String>{
    for i in 0..=3{
        let resp = client.lease_keep_alive(lease_id).await;
        let (mut keeper, mut stream) = match resp{
            Ok((k,s))=>(k,s),
            Err(e)=>{
                error!("request stream retry times = [{}], error: {}, ", i, e);
                continue;
            }
        };
        let resp = keeper.keep_alive().await;
        match resp{
            Ok(_)=>{},
            Err(e)=>{
                error!("request keeper retry times = [{}], error: {}, ", i, e);
                continue;
            }
        };
        let resp = stream.message().await;
        match resp{
            Ok(r)=>{
                match r{
                    Some(lease_keep_alive_response)=>{
                        debug!("lease_keep_alive_response = {:?}", lease_keep_alive_response);
                        return Ok(())
                    },
                    None=>{
                        error!("keep alive retry times = [{}], response None, connection for lease [{}] maybe lost", i, lease_id);
                        continue;
                    }
                }
            },
            Err(e)=>{
                error!("keep alive retry time = [{}], lease [{}] keep alive", i, lease_id);
                error!("{}", e);
                continue;
            }
        };
    }
    return Err(format!("keep alive lease [{}] failed", lease_id))
}