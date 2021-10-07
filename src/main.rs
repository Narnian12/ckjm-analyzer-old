extern crate execute;
extern crate fs_extra;
extern crate clap;
extern crate minidom;
use minidom::Element;
use std::fs::OpenOptions;
use std::io::prelude::*;
use clap::{Arg, App};
use array_tool::vec::{Intersect, Union};
use walkdir::WalkDir;
mod maintainability;

struct MetricRange {
    min: f64,
    max: f64
}

struct MetricMean {
    acc: f64,
    count: f64
}

// Indices for metrics
// WMC is the same as NOM
const WMC_NOM: i32 = 0;
const DIT: i32 = 1;
const NOC: i32 = 2;
const CBO: i32 = 3;
const LCOM: i32 = 5;
const LOC: i32 = 10;

const OUTLIER: f64 = 0.15;
const MAINTAINABILITY_TOTAL: f64 = 35.0;

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

    let metrics_headers = "Project,DI,MAI,LOC,CBO,DIT,LCOM,NOC,WMC-NOM";
    if let Err(e) = writeln!(metrics_output_file, "{}", metrics_headers) {
        eprintln!("Could not add headers to metrics_output.csv, {}", e);
    }

    for project_dir in std::fs::read_dir(projects_root_path.clone()).expect("Could not access subdirectory") {
        let project_dir = project_dir.expect("Could not unwrap subdirectory");
        // Skip all files because we only care about the folders containing .class files
        if !std::fs::metadata(project_dir.path())?.is_dir() { continue; }
        let project_path = project_dir.path().clone();

        // Find classes injected via XML-injection
        let mut xml_di_string: String = String::new();
        for entry in WalkDir::new(project_path.clone()) {
            let class_file = entry.unwrap();
            // Find all class files within the project
            if class_file.path().extension().is_some() && class_file.path().extension().unwrap() == "xml" {
                let xml = std::fs::read_to_string(class_file.path())?;
                let xml_root: Element = xml.parse().unwrap();
                for child in xml_root.children() {
                    if child.is("bean", xml_root.ns().as_str()) {
                        xml_di_string.push_str(&vec![",".to_string(), child.attr("class").unwrap().to_string()].join(""));
                    }
                }
            }
        }
        let unprocessed_xml_di_classes: Vec<&str> = xml_di_string.split(',').collect();
        // Contains final processed class names injected via XML
        let mut xml_di_classes: Vec<&str> = Vec::new();
        for unprocessed_class in unprocessed_xml_di_classes {
            let split_xml_di_classes: Vec<&str> = unprocessed_class.split('.').collect();
            xml_di_classes.push(split_xml_di_classes[split_xml_di_classes.len() - 1]);
        }

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
        let mut cbo_range = MetricRange { min: f64::MAX, max: f64::MIN };
        let mut dit_values: Vec<f64> = Vec::new();
        let mut dit_range = MetricRange { min: f64::MAX, max: f64::MIN };
        let mut lcom_values: Vec<f64> = Vec::new();
        let mut lcom_range = MetricRange { min: f64::MAX, max: f64::MIN };
        let mut noc_values: Vec<f64> = Vec::new();
        let mut noc_range = MetricRange { min: f64::MAX, max: f64::MIN };
        let mut wmc_nom_values: Vec<f64> = Vec::new();
        let mut wmc_nom_range = MetricRange { min: f64::MAX, max: f64::MIN };

        // Variables for metrics mean analysis
        let mut mean_cbo = MetricMean { acc: 0.0, count: 0.0 };
        let mut mean_dit = MetricMean { acc: 0.0, count: 0.0 };
        let mut mean_lcom = MetricMean { acc: 0.0, count: 0.0 };
        let mut mean_noc = MetricMean { acc: 0.0, count: 0.0 };
        let mut mean_wmc_nom = MetricMean { acc: 0.0, count: 0.0 };

        // Iterate through CKJM-Extended output
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
                let mut intersection = fields.intersect(methods.clone());
                // Trim to remove carriage return escape characters
                for int in intersection.iter_mut() { *int = int.trim(); }
                field_method_int.push(intersection);
            }
            else if metric_line.contains("metrics - ") {
                for metric_or_name in metric_line.split_whitespace().into_iter().skip(2) {
                    let float_parse = metric_or_name.parse::<f64>();
                    if float_parse.is_ok() {
                        let metric_val = float_parse.unwrap();
                        match current_metric_idx {
                            WMC_NOM => {
                                wmc_nom_values.push(metric_val);
                                wmc_nom_range.min = wmc_nom_range.min.min(metric_val);
                                wmc_nom_range.max = wmc_nom_range.max.max(metric_val);
                                mean_wmc_nom.acc += metric_val;
                                mean_wmc_nom.count += 1.0;
                            }
                            DIT => {
                                dit_values.push(metric_val);
                                dit_range.min = dit_range.min.min(metric_val);
                                dit_range.max = dit_range.max.max(metric_val);
                                mean_dit.acc += metric_val;
                                mean_dit.count += 1.0;
                            }
                            NOC => {
                                noc_values.push(metric_val);
                                noc_range.min = noc_range.min.min(metric_val);
                                noc_range.max = noc_range.max.max(metric_val);
                                mean_noc.acc += metric_val;
                                mean_noc.count += 1.0;
                            }
                            CBO => {
                                cbo_values.push(metric_val);
                                cbo_range.min = cbo_range.min.min(metric_val);
                                cbo_range.max = cbo_range.max.max(metric_val);
                                mean_cbo.acc += metric_val;
                                mean_cbo.count += 1.0;
                            }
                            LCOM => {
                                lcom_values.push(metric_val);
                                lcom_range.min = lcom_range.min.min(metric_val);
                                lcom_range.max = lcom_range.max.max(metric_val);
                                mean_lcom.acc += metric_val;
                                mean_lcom.count += 1.0;
                            }
                            LOC => { total_loc += metric_val; }
                            _ => {}
                        }
                        current_metric_idx += 1;
                    }
                    // Non-float will be class name
                    else { class_names.push(metric_or_name); }
                }
            }
        }
        // Finish iterating through CKJM-Extended output

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

        // Iterate through all classes and perform DI analysis
        for i in 0..(class_names.len()) {
            if i == field_method_int.len() { break; }
            // Generate complete set of DI classes
            let union_di_classes = xml_di_classes.union(field_method_int[i].clone());
            // If class implements constructor-based or setter-based dependency injection, include it as a DI class
            if union_di_classes.intersect(class_names.clone()).len() > 0 { di_classes += 1; }
        }
        
        let maintainability_metric = maintainability::compute_maintainability_metric(&cbo_values, cbo_limits, dit_values, dit_limits, lcom_values, lcom_limits, 
            noc_values, noc_limits, wmc_nom_values, wmc_nom_limits);

        let mut metric_analysis = String::from(format!("{:?}{}", project_name, ","));
        metric_analysis.push_str(&vec![(di_classes as f64 / class_names.len() as f64).to_string(), ','.to_string()].join(""));
        metric_analysis.push_str(&vec![(1.0 - (maintainability_metric / (MAINTAINABILITY_TOTAL * class_names.len() as f64))).to_string(), ','.to_string()].join(""));
        metric_analysis.push_str(&vec![total_loc.to_string(), ','.to_string()].join(""));
        metric_analysis.push_str(&vec![(mean_cbo.acc / mean_cbo.count).to_string(), ','.to_string()].join(""));
        metric_analysis.push_str(&vec![(mean_dit.acc / mean_dit.count).to_string(), ','.to_string()].join(""));
        metric_analysis.push_str(&vec![(mean_lcom.acc / mean_lcom.count).to_string(), ','.to_string()].join(""));
        metric_analysis.push_str(&vec![(mean_noc.acc / mean_noc.count).to_string(), ','.to_string()].join(""));
        metric_analysis.push_str(&vec![(mean_wmc_nom.acc / mean_wmc_nom.count).to_string(), ','.to_string()].join(""));

        if let Err(e) = writeln!(metrics_output_file, "{}", metric_analysis) {
            eprintln!("Could not add metrics to metrics_output.csv, {}", e);
        }
    }

    Ok(())
}
