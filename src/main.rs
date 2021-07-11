extern crate execute;
extern crate fs_extra;
extern crate clap;
use std::fs::OpenOptions;
use std::io::prelude::*;
use clap::{Arg, App};

struct Metric {
    num_classes: f64,
    sum_metric: f64,
}

const NUM_METRICS: usize = 17;

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
                        .arg(Arg::with_name("path")
                            .short("p")
                            .long("path")
                            .required(true)
                            .value_name("PROJECTS_PATH")
                            .help("Sets the path to a folder with sub-folders of projects containing the .class files to analyze"))
                        .get_matches();

    let jar_path = matches.value_of("jar").unwrap();
    let mut projects_root_path = std::path::PathBuf::new();
    projects_root_path.push(matches.value_of("path").unwrap());

    let ckjm_root_dir = std::env::current_dir()?;

    let mut metrics_output_path = ckjm_root_dir.clone();
    metrics_output_path.push("metrics_output.csv");
    if metrics_output_path.exists() { fs_extra::file::remove(metrics_output_path.clone()).unwrap(); }
    let mut metrics_output_file = OpenOptions::new()
                                    .create_new(true)
                                    .append(true)
                                    .open(metrics_output_path.clone())
                                    .unwrap();

    let metrics_headers = "Project,WMC,DIT,NOC,CBO,RFC,LCOM,Ca,Ce,NPM,LCOM3,DAM,MOA,MFA,CAM,IC,CBM,AMC,LOC";
    if let Err(e) = writeln!(metrics_output_file, "{}", metrics_headers) {
        eprintln!("Could not add headers to metrics_output.csv, {}", e);
    }

    for project_dir in std::fs::read_dir(projects_root_path.clone()).expect("Could not access subdirectory") {
        let project_dir = project_dir.expect("Could not unwrap subdirectory");
        // Skip all files because we only care about the folders containing .class files
        if !std::fs::metadata(project_dir.path())?.is_dir() { continue; }

        let project_path = project_dir.path().clone();
        let project_name = project_dir.file_name();

        let mut unix_arg = "find ".to_owned();
        unix_arg.push_str(&vec![project_path.to_str().unwrap(), "-name '*.class' -print | java -jar", jar_path, "2>/dev/null"].join(" ").to_string());

        // Execute cross-platform command that performs CKJM analysis, outputs the results in a text file, and ignores error messages
        let application = if cfg!(target_os = "windows") {
            std::process::Command::new("cmd")
                                .args(&["/C", "dir", "/b", "/s", "*.class", "|", "findstr", "/v", ".class.", "|", "java", "-jar", jar_path, "2>", "nul"])
                                .current_dir(&project_dir.path())
                                .output()
                                .expect("Failed to execute application")
        } else {
            std::process::Command::new("sh")
                                .arg("-c")
                                .arg(unix_arg)
                                .current_dir(&ckjm_root_dir)
                                .output()
                                .expect("Failed to execute application")
        };
        
        let ckjm_output = String::from_utf8_lossy(&application.stdout);
        let metric_lines: Vec<&str> = ckjm_output.split("\n").collect();
        let mut total_loc = 0.0;
        let mut metric_vec  = Vec::with_capacity(NUM_METRICS);
        for _ in 0..NUM_METRICS { metric_vec.push(Metric { num_classes: 0.0, sum_metric: 0.0 }); } // Initialize metrics

        for metric_line in metric_lines {
            let mut current_metric_idx = 0; // Iterate through every metric
            let mut added_metric_idx = 0; // Increment once metric is added to vector
            if metric_line.contains("~") { continue; }
            for metric in metric_line.split_whitespace() {
                match metric.parse::<f64>() {
                    Ok(n) => {
                        // The 10th index in the CKJM metrics is LOC
                        if current_metric_idx == 10 { total_loc += n; }
                        else {
                            metric_vec[added_metric_idx].sum_metric += n;
                            metric_vec[added_metric_idx].num_classes += 1.0;
                            added_metric_idx += 1;
                        }
                        current_metric_idx += 1;
                    },
                    Err(_e) => {} // Ignore string and other types
                }
            }
        }
        
        let mut metric_analysis = String::from(format!("{:?}{}", project_name, ","));
        for i in 0..NUM_METRICS {
            metric_analysis.push_str(&(metric_vec[i].sum_metric / metric_vec[i].num_classes).to_string());
            metric_analysis.push(',');
        }
        metric_analysis.push_str(&total_loc.to_string());
        metric_analysis.push(',');

        if let Err(e) = writeln!(metrics_output_file, "{}", metric_analysis) {
            eprintln!("Could not add metrics to metrics_output.csv, {}", e);
        }
    }

    Ok(())
}