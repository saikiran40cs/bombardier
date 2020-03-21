use crate::cmd;
use crate::file;
use crate::http;
use crate::parser;
use crate::report;

use std::sync::{Arc, Mutex};
use std::{thread, time};
use std::ops::Deref;
use std::collections::HashMap;

use log::{debug};
use reqwest::{blocking::Response};

pub fn execute(args: cmd::Args, env_map: HashMap<String, String>, requests: Vec<parser::Request>) -> Vec<report::Stats> {

    let no_of_threads = args.threads;
    let no_of_iterations = args.iterations;
    let iteration_based_execution = no_of_iterations > 0;
    let thread_delay = args.ramp_up * 1000 / no_of_threads;

    let start_time = time::Instant::now();
    let execution_time = args.execution_time;

    let report_file = file::create_file(&args.report);
   
    let client = http::get_sync_client(&args);
    let client_arc = Arc::new(client);
    let args_arc = Arc::new(args);
    let requests = Arc::new(requests);

    let mut handles = vec![];
    let stats = vec![];
    let stats_arc = Arc::new(Mutex::new(stats));
    let report_arc = Arc::new(Mutex::new(report_file));

    for thread_cnt in 0..no_of_threads {
        let requests_clone = requests.clone();
        let client_clone = client_arc.clone();
        let args_clone = args_arc.clone();
        let stats_clone = stats_arc.clone();
        let mut env_map_clone = env_map.clone();
        let report_clone = report_arc.clone();

        let mut thread_iteration = 0;
        let handle = thread::spawn(move || {
            loop {
                if iteration_based_execution {
                    if thread_iteration >= no_of_iterations {
                        break;
                    }
                } else if is_execution_time_over(start_time, &execution_time) {
                    break;
                }

                thread_iteration += 1; //increment iteration

                //looping thru requests
                for request in requests_clone.deref() {
                    debug!("Executing {}-{} : {}", thread_cnt, thread_iteration, request.name);

                    let processed_request = preprocess(&request, &env_map_clone); //transform request
                    let (res, et) = http::execute(&client_clone, processed_request).unwrap();

                    debug!("Writing stats for {}-{}", thread_cnt, thread_iteration);
                    let new_stats = report::Stats::new(request.name.clone(), res.status().as_u16(), et);
                    file::write_to_file(&mut report_clone.as_ref().lock().unwrap(), &format!("{}", new_stats));
                    stats_clone.lock().unwrap().push(new_stats); //push stats
                    update_env_map(res, &mut env_map_clone); //process response and update env_map
                    
                    thread::sleep(time::Duration::from_millis(args_clone.delay)); //wait per request delay
                }
            }
        });

        handles.push(handle);
        thread::sleep(time::Duration::from_millis(thread_delay)); //wait per thread delay
    }

    for handle in handles {
        handle.join().unwrap();
    }

    match Arc::try_unwrap(stats_arc) {
        Ok(r) =>  r.into_inner().unwrap(),
        Err(_) => panic!("Unable to get report object")
    }
}

fn is_execution_time_over(start_time: time::Instant, duration: &u64) -> bool {
    start_time.elapsed().as_secs() > *duration
}

fn update_env_map(response: Response, env_map: &mut HashMap<String, String>) {
    let resp_body = response.text();
}

fn preprocess(request: &parser::Request, env_map: &HashMap<String, String>) -> parser::Request {
    let mut s_request = serde_json::to_string(request).expect("Request cannot be serialized");
    s_request = file::find_and_replace(s_request, &env_map);
    serde_json::from_str(&s_request).expect("Unable to parse Json")
}