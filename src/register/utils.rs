use std::time::Duration;
// use chrono::format;
use futures::StreamExt;
use etcd_rs::{Client, LeaseGrantRequest, PutRequest, LeaseKeepAliveRequest, LeaseGrantResponse};
use log::{info, error, debug};
// use std::thread;


pub async fn keep_alive_lease(
    client: &Client, 
    reg_root: &String, 
    service_name: &Vec<String>,
    lease_time: u64,
    keep_alive: u64) -> Result<(), String> {
    //lease_time must longer then keep_alive (time)
    // let mut interval = tokio::time::interval(Duration::from_secs(keep_alive));
    // grant lease
    let lease = retry_lease(client, lease_time).await?;
    let lease_id = lease.id();
    info!("got lease id = [{}]", lease_id);
    // One lease can be used by may key, So lease need only one for usually
    reg_keep_alive(client, keep_alive).await?;

    let osname = match hostname::get(){
        Ok(name) => name,
        Err(e) => {
            error!("{}", e);
            panic!("Can not get hostname")
        }
    };
    let host_name = osname.to_string_lossy();
    info!("Got server host name = [{}]", host_name);
    for name in service_name{
        let reg_path = format!("{0}/{1}/members/{2}", reg_root, name, host_name);
        put_key2lease(client, lease_id, reg_path).await?;
    }
        
    // let client = client.clone();
    let mut interval = tokio::time::interval(Duration::from_secs(keep_alive));
    loop {
        // lease for all keys one times
        let result = client
            .lease()
            .keep_alive(LeaseKeepAliveRequest::new(lease_id))
            .await;
        match result{
            Ok(_)=>debug!("keep alive is ok ..., wait for next time"),
            Err(e)=>{
                error!("{}", e);
                //NOTICE! retry is no usefull when connect is intrupted,must bo reconnect etcd
                return Err(format!("keep alive the lease id {} failed", lease_id));
            }
        }
        interval.tick().await;
    }

}

async fn retry_lease(
    client:&Client,
    lease_time:u64)->Result<LeaseGrantResponse, String>{
    for i in 0..=3{
        let lease_rst = client
        .lease()
        .grant(LeaseGrantRequest::new(Duration::from_secs(lease_time)))
        .await;
        match lease_rst {
            Ok(l)=> {return Ok(l)},
            Err(e)=> {
                error!("grant lease & retry [{}] times error: {}", i,e);
            }
        };
    }
    return Err("grant lease  failed!".into())
}

async fn reg_keep_alive(client:&Client, keep_alive:u64)->Result<(), String>{
    // watch keep alive event
    let result = client.lease().keep_alive_responses().await;
    let mut inbound = match result {
        Ok(i)=> i,
        Err(e)=> {
            error!("alive_responses error: {}", e);
            client.shutdown().await.unwrap_err();
            return Err("".into());
        }
    };
    let mut interval = tokio::time::interval(Duration::from_secs(keep_alive));
    info!("grant lease and keep alive ...");
    tokio::spawn(async move {
        loop {
            match inbound.next().await {
                Some(resp) => {
                    match resp {
                        //ignore ok response,but under debug model
                        Ok(o) => debug!("{:?}", o),
                        Err(e) => {
                            debug!("{:?}", e);
                            // If error got,like network to etcd is interrupted, etcd-rs retry auto
                            // nothing need to do, only waiting interval
                        }
                    }
                }
                None => {
                    debug!("keep alive response None!");
                    //nothing to do
                }
            };
            interval.tick().await;
        }
    });
    Ok(())
}

async fn put_key2lease(client:&Client,lease_id:u64, reg_path:String)->Result<(), String>{
    //retry put key for lease while connect is interupted, as some etcd node is down
    for i in 0..=3{
        // set lease for key
        //may panicked when connect lost,throw by etcd-rs crate
        let result = client
        .kv()
        .put({
            let mut req = PutRequest::new(reg_path.as_str(), format!("lease_id=[{}]", lease_id));
            req.set_lease(lease_id);
            req
        })
        .await;
        match result{
            Ok(_)=>{
                info!("service regist path = [{}] successfully with lease id = [{}]", reg_path, lease_id);
                return Ok(())
            },
            Err(e)=>{
                error!("put key [{}] with lease [{}] fail times [{}], error: {}", reg_path, lease_id, i, e);
            }
        };
        
    }
    return Err(format!("set lease {} for key {} failed!", lease_id, reg_path));
}