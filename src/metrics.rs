use std::collections::HashMap;
use array_tool::vec::{Intersect, Union};
use itertools::Itertools;

const OUTLIER: f64 = 0.15;

#[derive(Debug)]
pub struct MetricRange {
    pub min: f64,
    pub max: f64
}

impl MetricRange {
    // Compare min/max between self and parameter value
    pub fn update_min_max(&mut self, value: f64) {
        self.min = self.min.min(value);
        self.max = self.max.max(value);
    }
}

#[derive(Debug)]
pub struct MetricMean {
    pub acc: f64,
    pub count: f64,
    pub mean: f64
}

impl MetricMean {
    pub fn new() -> MetricMean {
        MetricMean { acc: 0.0, count: 0.0, mean: 0.0 }
    }

    pub fn add_value(&mut self, value: f64) {
        self.acc += value;
        self.count += 1.0;
    }

    pub fn compute_mean(&mut self) {
        self.mean = self.acc / self.count;
    }
}

#[derive(Debug)]
pub struct MetricsData {
    pub values: Vec<f64>,
    pub range: MetricRange,
    pub mean: MetricMean,
    pub limits: Vec<f64>
}

impl MetricsData {
    pub fn new() -> MetricsData {
        MetricsData { 
            // Needed for maintainability metric
            values: Vec::new(), 
            // Needed for maintainability metric and limits
            range: MetricRange { min: f64::MAX, max: f64::MIN },
            mean: MetricMean { acc: 0.0, count: 0.0, mean: 0.0 },
            limits: Vec::new()
        }
    }

    pub fn add_metric_value(&mut self, value: f64) {
        self.values.push(value);
        self.range.update_min_max(value);
        self.mean.add_value(value);
    }

    pub fn generate_limits(&mut self) {
        let limit = (self.range.max - self.range.min) * OUTLIER;
        self.limits = vec![self.range.min + limit, self.range.max - limit];
    }

    pub fn compute_mean(&mut self) {
        self.mean.compute_mean();
    }
}

#[derive(Debug)]
pub struct ClassData {
    pub method_params: Vec<String>,
    pub cbo: f64,
    pub di_params: f64,
    pub diw_cbo: f64
}

impl ClassData {
    pub fn new() -> ClassData {
        ClassData {
            method_params: Vec::new(),
            cbo: 0.0,
            di_params: 0.0,
            diw_cbo: 0.0
        }
    }

    pub fn add_method_param(&mut self, param: String) {
        self.method_params.push(param);
    }

    pub fn add_cbo(&mut self, value: f64) {
        self.cbo = value;
    }

    pub fn compute_class_di_metrics(&mut self, class_names: &Vec<String>, xml_di_classes: &Vec<String>) {
        // First find the union of the method params with XML DI params
        // This will be a vector of all params potentially being sent into the class
        let xml_and_method_params = if xml_di_classes.len() > 0 { 
            self.method_params.union(xml_di_classes.clone()) 
        } else {
            self.method_params.to_vec()
        };
        // Next find the intersection of the previous union with all class names
        // This will filter out primitive types from being considered DI
        let mut filtered_xml_and_method_params = xml_and_method_params.intersect(class_names.to_vec());
        // Filter out duplicate params because we will consider two classes to be coupled to each other if they
        // depend on each other at least once
        filtered_xml_and_method_params = filtered_xml_and_method_params.into_iter().unique().collect();
        self.di_params = filtered_xml_and_method_params.len() as f64;
        // Decrease coupling by 0.5 for every DI class-param within class
        self.diw_cbo = self.cbo - (0.5 * filtered_xml_and_method_params.len() as f64);
    }
}

#[derive(Debug)]
pub struct ClassAndMetricStruct {
    pub metrics: HashMap<String, MetricsData>,
    pub classes: HashMap<String, ClassData>,
    pub total_loc: f64,
    pub di_couplings: f64,
    pub total_couplings: f64,
    pub di_proportion: f64,
    // Specifically kept out of metrics HashMap because it requires additional information from classes
    pub diw_cbo_mean: MetricMean
}

impl ClassAndMetricStruct {
    pub fn new() -> ClassAndMetricStruct {
        ClassAndMetricStruct {
            metrics: HashMap::new(),
            classes: HashMap::new(),
            total_loc: 0.0,
            di_couplings: 0.0,
            total_couplings: 0.0,
            di_proportion: 0.0,
            diw_cbo_mean: MetricMean::new()
        }
    }

    pub fn initialize_metrics(&mut self) {
        let metric_names = ["WMC_NOM", "DIT", "NOC", "CBO", "LCOM"];
        for metric in metric_names {
            self.metrics.insert(metric.to_string(), MetricsData::new());
        }
    }

    pub fn generate_di_metrics(&mut self) {
        for (_, class_data) in &self.classes {
            self.di_couplings += class_data.di_params;
            self.diw_cbo_mean.add_value(class_data.diw_cbo);
            // CBO is, in essence, all classes that the current class is coupled to, and thus depends on
            self.total_couplings += class_data.cbo;
        }
        self.di_proportion = self.di_couplings / self.total_couplings;
    }

    pub fn generate_limits(&mut self) {
        let metric_names = ["WMC_NOM", "DIT", "NOC", "CBO", "LCOM"];
        for metric_name in metric_names {
            let metric_data = self.metrics.get_mut(&metric_name.to_string()).unwrap();
            metric_data.generate_limits();
        }
    }

    pub fn compute_means(&mut self) {
        let metric_names = ["WMC_NOM", "DIT", "NOC", "CBO", "LCOM"];
        for metric_name in metric_names {
            let metric_data = self.metrics.get_mut(&metric_name.to_string()).unwrap();
            metric_data.compute_mean();
        }
        // Don't forget DIW-CBO outside of metrics HashMap
        self.diw_cbo_mean.compute_mean();
    }
}