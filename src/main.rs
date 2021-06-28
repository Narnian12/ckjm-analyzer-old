extern crate execute;
extern crate fs_extra;
use std::fs::{File, OpenOptions};
use std::io::{self, prelude::*, BufReader};

struct Metric {
    numClasses: i32,
    sumMetric: f64,
}

fn main() -> std::io::Result<()> {
    // Parse command line arguments
    let mut jar_path = "".to_owned();
    let mut get_jar_path = false;
    let mut project_path = "".to_owned();
    let mut get_project_path = false;
    let args: Vec<String> = std::env::args().collect();
    for arg in args {
        if get_project_path {
            project_path = arg.clone(); 
            get_project_path = false;
        }
        if get_jar_path { 
            jar_path = arg.clone();
            get_jar_path = false;
        }
        get_project_path = get_project_path || arg.contains("-path");
        get_jar_path = get_jar_path || arg.contains("-jar");
    }

    // Get all .class files in the path specified
    project_path.push_str("/*.class");

    let ckjm_root_dir = std::env::current_dir()?;
    let mut project_dir = ckjm_root_dir.clone();
    project_dir.push(project_path.clone());

    let mut unix_arg = "java -jar ".to_owned();
    unix_arg.push_str(jar_path.as_str());
    unix_arg.push(' ');
    unix_arg.push_str(project_path.as_str());
    unix_arg.push_str(" > metric_output.txt 2>/dev/null");

    // Execute cross-platform command that performs CKJM analysis, outputs the results in a text file, and ignores error messages
    let mut application = if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
                            .args(&["/C", "java", "-jar", jar_path.as_str(), project_path.as_str(), ">", "metric_output.txt", ">", "NUL", "2>&1"])
                            .current_dir(&ckjm_root_dir)
                            .spawn()
                            .expect("Failed to execute application")
    } else {
        std::process::Command::new("sh")
                            .arg("-c")
                            .arg(unix_arg)
                            .current_dir(&ckjm_root_dir)
                            .spawn()
                            .expect("Failed to execute application")
    };

    let mut complete = false;
    while !complete {
        match application.try_wait() {
            Ok(Some(_status)) => complete = true,
            Ok(None) => complete = true,
            Err(e) => println!("Error attempting to wait for application: {}", e)
        }
    }

    let metric_output = File::open("metric_output.txt")?;
    let reader = BufReader::new(metric_output);

    let mut metrics_output_path = ckjm_root_dir.clone();
    metrics_output_path.push("metrics_output.csv");
    if metrics_output_path.exists() { fs_extra::file::remove(metrics_output_path.clone()).unwrap(); }
    let mut metrics_output_file = OpenOptions::new()
                                    .create_new(true)
                                    .append(true)
                                    .open(metrics_output_path.clone())
                                    .unwrap();
    if let Err(e) = writeln!(metrics_output_file, "WMC,CBO,LCOM,LOC") {
      eprintln!("Could not add headers to metrics_outputs.csv, {}", e);
    }

    let wmc_idx = 0;
    let cbo_idx = 3;
    let lcom_idx = 5;
    let loc_idx = 10;

    let mut metric_vec  = Vec::with_capacity(4 as usize);
    for _ in 0..4 { metric_vec.push(Metric { numClasses: 0, sumMetric: 0.0 }); }

    for metric_line in reader.lines() {
        let mut current_metric_idx = 0;
        for metric_iter in metric_line.unwrap().split_whitespace() {
            if metric_iter == "~" { break; }
            match metric_iter.parse::<f64>() {
                Ok(n) => {
                    if current_metric_idx == wmc_idx { 
                        metric_vec[0].sumMetric += n;
                        metric_vec[0].numClasses += 1;
                    }
                    else if current_metric_idx == cbo_idx {
                        metric_vec[1].sumMetric += n;
                        metric_vec[1].numClasses += 1;
                    }
                    else if current_metric_idx == lcom_idx {
                        metric_vec[2].sumMetric += n;
                        metric_vec[2].numClasses += 1;
                    }
                    else if current_metric_idx == loc_idx {
                        metric_vec[3].sumMetric += n;
                        metric_vec[3].numClasses += 1;
                    }
                    current_metric_idx += 1;
                },
                Err(_e) => {} // Ignore string and other types
            }
        }
    }

    for m in metric_vec {
        println!("numClasses is {}", m.numClasses);
        println!("sumMetric is {}", m.sumMetric);
    }

    Ok(())
}