extern crate execute;
extern crate fs_extra;
extern crate clap;
use std::fs::OpenOptions;
use std::io::prelude::*;
use clap::{Arg, App};
use array_tool::vec::Intersect;

struct MetricRange {
  min: f64,
  max: f64
}

// Indices for metrics
// WMC is the same as NOM
const WMC_NOM: i32 = 0;
const DIT: i32 = 1;
const NOC: i32 = 2;
const CBO: i32 = 3;
const LCOM: i32 = 5;
const LOC: i32 = 10;
const DAM: i32 = 11;
const MFA: i32 = 13;

const OUTLIER: f64 = 0.15;
const MAINTAINABILITY_TOTAL: f64 = 35.0;
const MIN: usize = 0;
const MAX: usize = 1;

fn main() -> std::io::Result<()> {
    // Parse command line arguments
    let matches = App::new("CKJM Analyzer")
                        .version("0.1")
                        .author("Peter Sun, <pysun@oakland.edu")
                        .about("Application used to analyze specific metrics from the CKJM Extended Tool.")
                        .arg(Arg::with_name("jar")
                            .short("j")
                            .long("jar")
                            .required(true)
                            .value_name("JAR_PATH")
                            .help("Sets the path to the CKJM Extended JAR file. Must be an absolute path."))
                        .arg(Arg::with_name("path")
                            .short("p")
                            .long("path")
                            .required(true)
                            .value_name("PROJECTS_PATH")
                            .help("Sets the path to a folder with sub-folders of projects containing the .class files to analyze. Must be an absolute path"))
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

    let metrics_headers = "Project,DI,MAI,REU,LOC";
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

        // Variables for DI analysis
        let mut fields: Vec<&str> = Vec::new();
        let mut methods: Vec<&str> = Vec::new();
        let mut field_method_int: Vec<Vec<&str>> = Vec::new();
        let mut class_names: Vec<&str> = Vec::new();

        // Variables for maintainability analysis
        let mut cbo_values: Vec<f64> = Vec::new();
        let mut cbo_range = MetricRange { min: -999.0, max: 999.0 };
        let mut dit_values: Vec<f64> = Vec::new();
        let mut dit_range = MetricRange { min: -999.0, max: 999.0 };
        let mut lcom_values: Vec<f64> = Vec::new();
        let mut lcom_range = MetricRange { min: -999.0, max: 999.0 };
        let mut noc_values: Vec<f64> = Vec::new();
        let mut noc_range = MetricRange { min: -999.0, max: 999.0 };
        let mut wmc_nom_values: Vec<f64> = Vec::new();
        let mut wmc_nom_range = MetricRange { min: -999.0, max: 999.0 };

        // Variables for reusability analysis
        let mut dam_values: Vec<f64> = Vec::new();
        let mut mfa_values: Vec<f64> = Vec::new();

        for metric_line in metric_lines {
            let mut current_metric_idx = 0; // Iterate through every metric
            if metric_line.contains("~") { continue; }
            else if metric_line.contains("fieldTypes - ,") {
                let types: Vec<&str> = metric_line.split(',').collect();
                for field_type in types.iter().skip(1) { fields.push(&field_type); }
            }
            else if metric_line.contains("methodTypes - ,") {
                let types: Vec<&str> = metric_line.split(',').collect();
                for method_type in types.iter().skip(1) { methods.push(&method_type); }
                field_method_int.push(fields.intersect(methods.clone()));
            }
            else {
                for metric_or_name in metric_line.split_whitespace() {
                    let float_parse = metric_or_name.parse::<f64>();
                    if float_parse.is_ok() {
                        let metric_val = float_parse.unwrap();
                        match current_metric_idx {
                            WMC_NOM => {
                                wmc_nom_values.push(metric_val);
                                wmc_nom_range.min = wmc_nom_range.min.min(metric_val);
                                wmc_nom_range.max = wmc_nom_range.max.max(metric_val);
                            }
                            DIT => {
                                dit_values.push(metric_val);
                                dit_range.min = dit_range.min.min(metric_val);
                                dit_range.max = dit_range.max.max(metric_val);
                            }
                            NOC => {
                                noc_values.push(metric_val);
                                noc_range.min = noc_range.min.min(metric_val);
                                noc_range.max = noc_range.max.max(metric_val);
                            }
                            CBO => {
                                cbo_values.push(metric_val);
                                cbo_range.min = cbo_range.min.min(metric_val);
                                cbo_range.max = cbo_range.max.max(metric_val);
                            }
                            LCOM => {
                                lcom_values.push(metric_val);
                                lcom_range.min = lcom_range.min.min(metric_val);
                                lcom_range.max = lcom_range.max.max(metric_val);
                            }
                            LOC => { total_loc += metric_val; }
                            DAM => { dam_values.push(metric_val); }
                            MFA => { mfa_values.push(metric_val); }
                            _ => {}
                        }
                        current_metric_idx += 1;
                    }
                    // Non-float will be class name
                    else { class_names.push(metric_or_name); }
                }
            }
        }
        let cbo_limit = (cbo_range.max - cbo_range.min) * OUTLIER;
        let dit_limit = (dit_range.max - dit_range.min) * OUTLIER;
        let lcom_limit = (lcom_range.max - lcom_range.min) * OUTLIER;
        let noc_limit = (noc_range.max - noc_range.min) * OUTLIER;
        let wmc_nom_limit = (wmc_nom_range.max - wmc_nom_range.min) * OUTLIER;

        // Use this to determine whether the current class' metric is an outlier
        let cbo_limits = vec![cbo_range.min + cbo_limit, cbo_range.max - cbo_limit];
        let dit_limits = vec![dit_range.min + dit_limit, dit_range.max - dit_limit];
        let lcom_limits = vec![lcom_range.min + lcom_limit, lcom_range.max - lcom_limit];
        let noc_limits = vec![noc_range.min + noc_limit, noc_range.max - noc_limit];
        let wmc_nom_limits = vec![wmc_nom_range.min + wmc_nom_limit, wmc_nom_range.max - wmc_nom_limit];

        // Total number of classes that involve DI
        let mut di_classes = 0;
        // Sum of weighted metric if they exceed the upper/lower 15% bounds
        let mut analyzability_metric = 0.0;
        let mut changeability_metric = 0.0;
        let mut stability_metric = 0.0;
        let mut testability_metric = 0.0;
        // Sum of all reusability computations across classes
        let mut reusability_metric = 0.0;

        // Iterate through all classes and perform DI, maintainability and reusability analysis
        for i in 0..(class_names.len() - 1) {
            if i == field_method_int.len() { break; }
            // If class implements constructor-based or setter-based dependency injection, include it as a DI class
            if field_method_int[i].intersect(class_names.clone()).len() > 1 { di_classes += 1; }
            // Metric quantities are added based on weights according to this paper - http://www.cs.umd.edu/~pugh/ISSTA08/issta2008/p131.pdf
            if cbo_values[i] >= cbo_limits[MIN] || cbo_values[i] <= cbo_limits[MAX] { 
                analyzability_metric += 2.0;
                changeability_metric += 2.0;
                stability_metric += 2.0;
                testability_metric += 2.0;
            }
            if dit_values[i] >= dit_limits[MIN] || dit_values[i] <= dit_limits[MAX] {
                analyzability_metric += 2.0;
                changeability_metric += 2.0;
                stability_metric += 1.0;
                testability_metric += 2.0;
            }
            if lcom_values[i] >= lcom_limits[MIN] || lcom_values[i] <= lcom_limits[MAX] {
                analyzability_metric += 2.0;
                changeability_metric += 2.0;
                stability_metric += 2.0;
                testability_metric += 2.0;
            }
            if noc_values[i] >= noc_limits[MIN] || noc_values[i] <= noc_limits[MAX] {
                analyzability_metric += 1.0;
                changeability_metric += 2.0;
                stability_metric += 1.0;
                testability_metric += 1.0;
            }
            if wmc_nom_values[i] >= wmc_nom_limits[MIN] || wmc_nom_values[i] <= wmc_nom_limits[MAX] {
                analyzability_metric += 2.0;
                changeability_metric += 2.0;
                stability_metric += 1.0;
                testability_metric += 2.0;
            }
            // Use reusability multiple regression model from paper here - https://www.scirp.org/pdf/JSEA_2015041418363656.pdf
            reusability_metric += -37.111 + (3.973 * cbo_values[i]) + (32.500 * mfa_values[i]) + (20.709 * dam_values[i]);
        }
        
        let maintainability_metric = analyzability_metric + changeability_metric + stability_metric + testability_metric;
        let mut metric_analysis = String::from(format!("{:?}{}", project_name, ","));
        metric_analysis.push_str(&(di_classes as f64 / class_names.len() as f64).to_string());
        metric_analysis.push(',');
        metric_analysis.push_str(&(maintainability_metric / (MAINTAINABILITY_TOTAL * class_names.len() as f64)).to_string());
        metric_analysis.push(',');
        metric_analysis.push_str(&(reusability_metric / class_names.len() as f64).to_string());
        metric_analysis.push(',');
        metric_analysis.push_str(&total_loc.to_string());

        if let Err(e) = writeln!(metrics_output_file, "{}", metric_analysis) {
            eprintln!("Could not add metrics to metrics_output.csv, {}", e);
        }
    }

    Ok(())
}