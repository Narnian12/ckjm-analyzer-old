extern crate execute;
extern crate fs_extra;
extern crate clap;
extern crate minidom;
use minidom::Element;
use std::fs::OpenOptions;
use std::io::prelude::*;
use clap::{Arg, App};
use walkdir::WalkDir;
use crate::metrics::{ClassAndMetricStruct, ClassData};
mod metrics;
mod maintainability;

// Indices for metrics
// WMC is the same as NOM
const WMC_NOM: i32 = 0;
const DIT: i32 = 1;
const NOC: i32 = 2;
const CBO: i32 = 3;
const RFC: i32 = 4;
const LCOM: i32 = 5;
const CA: i32 = 6;
const CE: i32 = 7;
const NPM: i32 = 8;
const LCOM3: i32 = 9;
const LOC: i32 = 10;
const DAM: i32 = 11;
const MOA: i32 = 12;
const MFA: i32 = 13;
const CAM: i32 = 14;
const IC: i32 = 15;
const CBM: i32 = 16;
const AMC: i32 = 17;
const CC: i32 = 18;

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

    let metrics_headers = "Project,DI,MAI,DIW-MAI,LOC,CBO,DIW-CBO,DAM,MOA,DIT,MFA";
    if let Err(e) = writeln!(metrics_output_file, "{}", metrics_headers) {
        eprintln!("Could not add headers to metrics_output.csv, {}", e);
    }

    for project_dir in std::fs::read_dir(projects_root_path.clone()).expect("Could not access subdirectory") {
        let project_dir = project_dir.expect("Could not unwrap subdirectory");
        // Skip all files because we only care about the folders containing .class files
        if !std::fs::metadata(project_dir.path())?.is_dir() { continue; }
        let project_path = project_dir.path().clone();

        // Find classes injected via XML-injection as well as all class files
        let mut xml_di_string = String::new();
        let mut class_files_string = String::new();
        let mut class_names_string = String::new();
        for entry in WalkDir::new(project_path.clone()) {
            let file = entry.unwrap();
            // Find all xml files within the project
            if file.path().extension().is_some() && file.path().extension().unwrap() == "xml" {
                let xml = std::fs::read_to_string(file.path())?;
                let xml_root: Element;
                match xml.parse() {
                    Ok(v) => {
                        xml_root = v;
                        for child in xml_root.children() {
                          if child.is("bean", xml_root.ns().as_str()) {
                              xml_di_string.push_str(&vec![",".to_string(), child.attr("class").unwrap().to_string()].join(""));
                          }
                        }
                    }
                    Err(e) => eprintln!("{}", e),
                }
            }
            else if file.path().extension().is_some() && file.path().extension().unwrap() == "class" {
                // These strings contain the absolute path of the class file
                class_files_string.push_str(&vec![file.path().to_str().unwrap(), ","].join(""));
                let file_name = file.file_name().to_str().unwrap();
                class_names_string.push_str(&vec![&file_name[0..file_name.len() - 6], ","].join(""));
            }
        }

        let unprocessed_xml_di_classes: Vec<&str> = xml_di_string.split(',').collect();
        // Contains final processed class names injected via XML
        let mut xml_di_classes: Vec<String> = Vec::new();
        for unprocessed_class in unprocessed_xml_di_classes {
            let split_xml_di_classes: Vec<&str> = unprocessed_class.split('.').collect();
            xml_di_classes.push(split_xml_di_classes[split_xml_di_classes.len() - 1].to_string());
        }

        let mut class_files: Vec<&str> = class_files_string.split(',').collect();
        // Last element is empty because of comma-delimiter
        class_files.pop();

        let mut class_names: Vec<&str> = class_names_string.split(',').collect();
        class_names.pop();
        let mut owned_class_names: Vec<String> = Vec::new();
        for class_name in class_names {
            owned_class_names.push(class_name.to_string());
        }

        let project_name = project_dir.file_name();
        // Contains all information for DI and metrics analysis
        let mut class_and_metrics_struct: ClassAndMetricStruct = ClassAndMetricStruct::new();
        class_and_metrics_struct.initialize_metrics();

        for class_file in class_files {
            let application = if cfg!(target_os = "windows") {
                std::process::Command::new("cmd")
                    .args(&["/C", "java", "-jar", jar_path, class_file, "2>", "nul"])
                    .current_dir(&project_dir.path())
                    .output()
                    .expect("Failed to execute application")
            } else {
                std::process::Command::new("sh")
                    .arg("-c")
                    .arg(vec!["java -jar", jar_path, class_file, "2>/dev/null"].join(" ").to_string())
                    .current_dir(&ckjm_root_dir)
                    .output()
                    .expect("Failed to execute application")
            };

            let ckjm_output = String::from_utf8_lossy(&application.stdout);
            let metric_lines: Vec<&str> = ckjm_output.split("\n").collect();
    
            // Iterate through CKJM-Extended output
            for metric_line in metric_lines {
                let mut current_metric_idx = 0; // Iterate through every metric
                if metric_line.contains("~") { continue; }
                else if metric_line.contains("method_params - ") {
                    let mut name = "";
                    for name_or_param in metric_line.split_whitespace().into_iter().skip(2) {
                        // First string is class name
                        if name == "" { 
                            let split_name: Vec<&str> = name_or_param.split('.').collect();
                            name = split_name[split_name.len() - 1];
                            class_and_metrics_struct.classes.insert(name.to_string(), ClassData::new());
                        }
                        else {
                            class_and_metrics_struct.classes.get_mut(name).unwrap().add_method_param(name_or_param.to_string());
                        }
                    }
                }
                else if metric_line.contains("metrics - ") {
                    // Metric Analysis
                    let mut class_name = "";
                    for metric_or_name in metric_line.split_whitespace().into_iter().skip(2) {
                        let float_parse = metric_or_name.parse::<f64>();
                        if float_parse.is_ok() {
                            let metric_val = float_parse.unwrap();
                            match current_metric_idx {
                                CBO => {
                                    let class_elem = class_and_metrics_struct.classes.get_mut(class_name).unwrap();
                                    class_elem.add_cbo(metric_val);
                                    class_elem.compute_class_di_metrics(&owned_class_names, &xml_di_classes);
                                    class_and_metrics_struct.metrics.get_mut("CBO").unwrap().add_metric_value(metric_val);
                                }
                                DAM => { class_and_metrics_struct.metrics.get_mut("DAM").unwrap().add_metric_value(metric_val); }
                                MOA => { class_and_metrics_struct.metrics.get_mut("MOA").unwrap().add_metric_value(metric_val); }
                                DIT => { class_and_metrics_struct.metrics.get_mut("DIT").unwrap().add_metric_value(metric_val); }
                                MFA => { class_and_metrics_struct.metrics.get_mut("MFA").unwrap().add_metric_value(metric_val); }
                                LOC => { class_and_metrics_struct.total_loc += metric_val; }
                                _ => {}
                            }
                            current_metric_idx += 1;
                        }
                        else {
                            let split_class_name: Vec<&str> = metric_or_name.split('.').collect();
                            class_name = split_class_name[split_class_name.len() - 1];
                        }
                    }
                }
            }
        }

        // Finish iterating through CKJM-Extended output
        class_and_metrics_struct.generate_di_metrics();
        class_and_metrics_struct.compute_means();

        let maintainability_metric = maintainability::compute_maintainability_metric(
            class_and_metrics_struct.metrics["CBO"].mean.mean,
            class_and_metrics_struct.metrics["DAM"].mean.mean,
            class_and_metrics_struct.metrics["MOA"].mean.mean,
            class_and_metrics_struct.metrics["DIT"].mean.mean,
            class_and_metrics_struct.metrics["MFA"].mean.mean
        );

        let diw_cbo_maintainability_metric = maintainability::compute_maintainability_metric(
            class_and_metrics_struct.diw_cbo_mean.mean,
            class_and_metrics_struct.metrics["DAM"].mean.mean,
            class_and_metrics_struct.metrics["MOA"].mean.mean,
            class_and_metrics_struct.metrics["DIT"].mean.mean,
            class_and_metrics_struct.metrics["MFA"].mean.mean
        );
        
        let mut metric_analysis = String::from(format!("{:?}{}", project_name, ","));
        metric_analysis.push_str(&vec![(class_and_metrics_struct.di_proportion).to_string(), ','.to_string()].join(""));
        metric_analysis.push_str(&vec![maintainability_metric.to_string(), ','.to_string()].join(""));
        metric_analysis.push_str(&vec![diw_cbo_maintainability_metric.to_string(), ','.to_string()].join(""));
        metric_analysis.push_str(&vec![class_and_metrics_struct.total_loc.to_string(), ','.to_string()].join(""));
        metric_analysis.push_str(&vec![class_and_metrics_struct.metrics["CBO"].mean.mean.to_string(), ','.to_string()].join(""));
        metric_analysis.push_str(&vec![class_and_metrics_struct.diw_cbo_mean.mean.to_string(), ','.to_string()].join(""));
        metric_analysis.push_str(&vec![class_and_metrics_struct.metrics["DAM"].mean.mean.to_string(), ','.to_string()].join(""));
        metric_analysis.push_str(&vec![class_and_metrics_struct.metrics["MOA"].mean.mean.to_string(), ','.to_string()].join(""));
        metric_analysis.push_str(&vec![class_and_metrics_struct.metrics["DIT"].mean.mean.to_string(), ','.to_string()].join(""));
        metric_analysis.push_str(&vec![class_and_metrics_struct.metrics["MFA"].mean.mean.to_string(), ','.to_string()].join(""));

        if let Err(e) = writeln!(metrics_output_file, "{}", metric_analysis) {
            eprintln!("Could not add metrics to metrics_output.csv, {}", e);
        }
    }

    Ok(())
}

// TODO : Use this once CKJM-DI is fixed to allow for parallel execution
// let mut unix_arg = "find ".to_owned();
// unix_arg.push_str(&vec![project_path.to_str().unwrap(), "-name '*.class' -print | java -jar", jar_path, "2>/dev/null"].join(" ").to_string());

// // Execute cross-platform command that performs CKJM analysis, outputs the results in a text file, and ignores error messages
// let application = if cfg!(target_os = "windows") {
//     std::process::Command::new("cmd")
//                         .args(&["/C", "dir", "/b", "/s", "*.class", "|", "findstr", "/v", ".class.", "|", "java", "-jar", jar_path, "2>", "nul"])
//                         .current_dir(&project_dir.path())
//                         .output()
//                         .expect("Failed to execute application")
// } else {
//     std::process::Command::new("sh")
//                         .arg("-c")
//                         .arg(unix_arg)
//                         .current_dir(&ckjm_root_dir)
//                         .output()
//                         .expect("Failed to execute application")
// };