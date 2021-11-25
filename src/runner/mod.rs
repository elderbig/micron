pub mod functions{
    // use chrono::format;
    use delay_timer;
    use anyhow::Result;
    use delay_timer::prelude::*;
    use delay_timer::utils::convenience::functions as dt_functions;
    use log::{info};


    pub fn build_task_async_execute_process(
        task_id:u64, 
        maximum_running_time:u64,
        schedule:&str, 
        cmd_string:&str,) -> Result<Task, TaskError> {
        let mut task_builder = TaskBuilder::default();
        let formated_cmd = format!("{0} {1}", &cmd_string, task_id);
        info!("formated_cmd = [{}]", &formated_cmd);
        let body = dt_functions::unblock_process_task_fn(formated_cmd);
        task_builder
            .set_frequency_repeated_by_cron_str(&schedule)
            .set_task_id(task_id)
            .set_maximum_running_time(maximum_running_time)
            .set_maximum_parallel_runnable_num(1)
            .spawn(body)
    }
}