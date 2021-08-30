const MIN: usize = 0;
const MAX: usize = 1;
// Metric quantities are added based on weights according to this paper - http://www.cs.umd.edu/~pugh/ISSTA08/issta2008/p131.pdf
pub fn compute_maintainability_metric(cbo_values: &Vec<f64>, cbo_limits: Vec<f64>, 
                                    dit_values: Vec<f64>, dit_limits: Vec<f64>, 
                                    lcom_values: Vec<f64>, lcom_limits: Vec<f64>, 
                                    noc_values: Vec<f64>, noc_limits: Vec<f64>, 
                                    wmc_nom_values: Vec<f64>, wmc_nom_limits: Vec<f64>) -> f64 {
  // Sum of weighted metric if they exceed the upper/lower 15% bounds
  let mut analyzability_metric = 0.0;
  let mut changeability_metric = 0.0;
  let mut stability_metric = 0.0;
  let mut testability_metric = 0.0;

  // Add to metric if the value is within the upper/lower 15%
  for i in 0..(cbo_values.len()) {
    if cbo_values[i] <= cbo_limits[MIN] || cbo_values[i] >= cbo_limits[MAX] {
        analyzability_metric += 2.0;
        changeability_metric += 2.0;
        stability_metric += 2.0;
        testability_metric += 2.0;
    }
    if dit_values[i] <= dit_limits[MIN] || dit_values[i] >= dit_limits[MAX] {
        analyzability_metric += 2.0;
        changeability_metric += 2.0;
        stability_metric += 1.0;
        testability_metric += 2.0;
    }
    if lcom_values[i] <= lcom_limits[MIN] || lcom_values[i] >= lcom_limits[MAX] {
        analyzability_metric += 2.0;
        changeability_metric += 2.0;
        stability_metric += 2.0;
        testability_metric += 2.0;
    }
    if noc_values[i] <= noc_limits[MIN] || noc_values[i] >= noc_limits[MAX] {
        analyzability_metric += 1.0;
        changeability_metric += 2.0;
        stability_metric += 1.0;
        testability_metric += 1.0;
    }
    if wmc_nom_values[i] <= wmc_nom_limits[MIN] || wmc_nom_values[i] >= wmc_nom_limits[MAX] {
        analyzability_metric += 2.0;
        changeability_metric += 2.0;
        stability_metric += 1.0;
        testability_metric += 2.0;
    }
  }
  analyzability_metric + changeability_metric + stability_metric + testability_metric
}