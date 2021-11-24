// Metric quantities are added based on weights according to this paper - http://ijcce.org/papers/38-Z012.pdf
pub fn compute_maintainability_metric(mean_cbo: f64, mean_dam: f64, mean_moa: f64, mean_dit: f64, mean_mfa: f64) -> f64 {
    (0.5 * ((0.25 * mean_dam) - (0.25 * mean_cbo) + (0.5 * mean_moa))) + 
    (0.5 * ((0.5 * mean_dit) - (0.5 * mean_cbo) + (0.5 * mean_mfa)))
}