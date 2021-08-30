// Use reusability multiple regression model from paper here - https://www.scirp.org/pdf/JSEA_2015041418363656.pdf
pub fn compute_reusability_metric(cbo_values: &Vec<f64>, mfa_values: Vec<f64>, dam_values: Vec<f64>) -> f64 {
    let mut reusability_metric = 0.0;
    for metric in 0..(cbo_values.len()) {
        reusability_metric += -37.111 + (3.973 * cbo_values[metric]) + (32.500 * mfa_values[metric]) + (20.709 * dam_values[metric]);
    }
    reusability_metric
}

#[cfg(test)]
mod tests {
    use crate::reusability::compute_reusability_metric;

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_reusability_metric_simple() {
        assert_eq!(compute_reusability_metric(&[0.0].to_vec(), [0.0].to_vec(), [0.0].to_vec()), -37.111);
        assert_eq!(compute_reusability_metric(&[1.0].to_vec(), [1.0].to_vec(), [1.0].to_vec()), 20.071);
    }
}