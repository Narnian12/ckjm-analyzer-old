extern crate execute;
extern crate fs_extra;
extern crate clap;
use std::fs::{File, OpenOptions};
use std::io::{prelude::*, BufReader};
use clap::{Arg, App};

struct Metric {
    num_classes: f64,
    sum_metric: f64,
}

fn main() -> std::io::Result<()> {
    // Parse command line arguments
    let matches = App::new("CKJM Analyzer")
                        .version("0.1")
                        .author("Peter Sun, <pysun@oakland.edu")
                        .about("Application used to analyze specific metrics from the CKJM Extended Tool")
                        .arg(Arg::with_name("jar")
                            .short("j")
                            .long("jar")
                            .required(true)
                            .value_name("JAR_PATH")
                            .help("Sets the path to the CKJM Extended JAR file"))
                        .arg(Arg::with_name("project")
                            .short("p")
                            .long("project")
                            .required(true)
                            .value_name("PROJECT_PATH")
                            .help("Sets the path to the project folder containing the .class files to analyze"))
                        .arg(Arg::with_name("name")
                            .short("n")
                            .long("name")
                            .required(true)
                            .value_name("PROJECT_NAME")
                            .help("Sets the name of the project to be analyzed"))
                        .get_matches();

    let jar_path = matches.value_of("jar").unwrap();
    let mut project_path = std::path::PathBuf::new();
    project_path.push(matches.value_of("project").unwrap());
    project_path.push("*.class");
    let project_name = matches.value_of("name").unwrap();

    let ckjm_root_dir = std::env::current_dir()?;

    let mut unix_arg = "java -jar ".to_owned();
    unix_arg.push_str(jar_path);
    unix_arg.push(' ');
    unix_arg.push_str(project_path.to_str().unwrap());
    unix_arg.push_str(" > metric_output.txt 2>/dev/null");

    // Execute cross-platform command that performs CKJM analysis, outputs the results in a text file, and ignores error messages
    let mut application = if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
                            .args(&["/C", "java", "-jar", jar_path, project_path.to_str().unwrap(), ">", "metric_output.txt", "2>", "nul"])
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
    if let Err(e) = writeln!(metrics_output_file, "Project,WMC,CBO,LCOM,LOC") {
      eprintln!("Could not add headers to metrics_output.csv, {}", e);
    }

    let wmc_idx = 0;
    let cbo_idx = 3;
    let lcom_idx = 5;
    let loc_idx = 10;

    let mut metric_vec  = Vec::with_capacity(4 as usize);
    for _ in 0..4 { metric_vec.push(Metric { num_classes: 0.0, sum_metric: 0.0 }); }

    for metric_line in reader.lines() {
        let mut current_metric_idx = 0;
        for metric_iter in metric_line.unwrap().split_whitespace() {
            if metric_iter == "~" { break; }
            match metric_iter.parse::<f64>() {
                Ok(n) => {
                    if current_metric_idx == wmc_idx { 
                        metric_vec[0].sum_metric += n;
                        metric_vec[0].num_classes += 1.0;
                    }
                    else if current_metric_idx == cbo_idx {
                        metric_vec[1].sum_metric += n;
                        metric_vec[1].num_classes += 1.0;
                    }
                    else if current_metric_idx == lcom_idx {
                        metric_vec[2].sum_metric += n;
                        metric_vec[2].num_classes += 1.0;
                    }
                    else if current_metric_idx == loc_idx {
                        metric_vec[3].sum_metric += n;
                        metric_vec[3].num_classes += 1.0;
                    }
                    current_metric_idx += 1;
                },
                Err(_e) => {} // Ignore string and other types
            }
        }
    }

    let mut metric_analysis = String::from(format!("{}{}", project_name, ","));
    metric_analysis.push_str(&(metric_vec[0].sum_metric / metric_vec[0].num_classes).to_string());
    metric_analysis.push(',');
    metric_analysis.push_str(&(metric_vec[1].sum_metric / metric_vec[1].num_classes).to_string());
    metric_analysis.push(',');
    metric_analysis.push_str(&(metric_vec[2].sum_metric / metric_vec[2].num_classes).to_string());
    metric_analysis.push(',');
    metric_analysis.push_str(&metric_vec[3].sum_metric.to_string());
    metric_analysis.push(',');

    if let Err(e) = writeln!(metrics_output_file, "{}", metric_analysis) {
      eprintln!("Could not add metrics to metrics_output.csv, {}", e);
    }

    Ok(())
}