use std::time::Duration;
// use chrono::format;
use futures::StreamExt;
use etcd_rs::{Client, LeaseGrantRequest, PutRequest, LeaseKeepAliveRequest};
use log::{info, error, debug};
use std::thread;


pub async fn keep_alive_lease(
    client: &Client, 
    reg_root: &String, 
    service_name: &Vec<String>,
    lease_time: u64,
    keep_alive: u64) -> Result<(), String> {
    let osname = match hostname::get(){
        Ok(name) => name,
        Err(e) => {
            error!("{}", e);
            panic!("Can not get hostname")
        }
    };
    let host_name = osname.to_string_lossy();
    info!("Got server host name = [{}]", host_name);
    info!("grant lease and keep alive ...");   
    // grant lease
    let lease = client
        .lease()
        .grant(LeaseGrantRequest::new(Duration::from_secs(lease_time)))
        .await;
    let lease = match lease {
        Ok(l)=> l,
        Err(e)=> {
            error!("grant lease error: {}", e);
            return Err("grant lease  failed!".into());
        }
    };
    let lease_id = lease.id();
    // One lease can be used by may key, So lease need only one for usually
    {
        // watch keep alive event
        let result = client.lease().keep_alive_responses().await;
        let mut inbound = match result {
            Ok(i)=> i,
            Err(e)=> {
                error!("alive_responses error: {}", e);
                return Err("".into());
            }
        };
        tokio::spawn(async move {
            loop {
                match inbound.next().await {
                    Some(resp) => {
                        match resp {
                            //ignore ok response,but under debug model
                            Ok(o) => debug!("{:?}", o),
                            Err(e) => {
                                error!("{:?}", e);
                                // If error got,like network to etcd is interrupted, give up loop and wait for next connect in main
                                break;
                            }
                        }
                    }
                    None => {
                        error!("keep alive response None!");
                        //unknown error maybe need reconnect to etcd
                        break;
                    }
                };
                //lease_time must longer then keep_alive (time)
                thread::sleep(Duration::from_secs(keep_alive));
            }
        });
    }
    for name in service_name{
        let reg_path = format!("{0}/{1}/members/{2}", reg_root, name, host_name);
        // set lease for key
        let result = client
        .kv()
        .put({
            let mut req = PutRequest::new(reg_path.as_str(), "bar");
            req.set_lease(lease_id);
            req
        })
        .await;
        match result{
            Ok(_)=>{},
            Err(e)=>{
                error!("{}", e);
                return Err(format!("set lease for key {}", reg_path));
            }
        };
        info!("service regist path = [{}]", reg_path);
    }
    let client = client.clone();
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        // lease for all keys one time
        let result = client
            .lease()
            .keep_alive(LeaseKeepAliveRequest::new(lease_id))
            .await;
        match result{
            Ok(_)=>debug!("keep alive is ok ..., wait for next time"),
            Err(e)=>{
                error!("{}", e);
                return Err(format!("keep alive the lease id {} failed", lease_id));
            }
        }
        interval.tick().await;
    }

}
