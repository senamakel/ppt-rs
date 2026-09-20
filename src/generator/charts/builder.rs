//! Chart builder for fluent API

use super::data::{Chart, ChartSeries};
use super::types::ChartType;
use crate::core::{ElementPlacement, validate_chart};
use crate::exc::Result;

/// Chart builder for fluent API
pub struct ChartBuilder {
    title: String,
    chart_type: ChartType,
    categories: Vec<String>,
    series: Vec<ChartSeries>,
    placement: ElementPlacement,
}

impl ChartBuilder {
    /// Create a new chart builder
    pub fn new(title: &str, chart_type: ChartType) -> Self {
        ChartBuilder {
            title: title.to_string(),
            chart_type,
            categories: Vec::new(),
            series: Vec::new(),
            placement: ElementPlacement::chart_defaults(),
        }
    }

    /// Set chart position
    pub fn position(mut self, x: u32, y: u32) -> Self {
        self.placement.set_position(x, y);
        self
    }

    /// Set chart size
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.placement.set_size(width, height);
        self
    }

    /// Add categories
    pub fn categories(mut self, categories: Vec<&str>) -> Self {
        self.categories = categories.into_iter().map(|c| c.to_string()).collect();
        self
    }

    /// Add a data series
    pub fn add_series(mut self, series: ChartSeries) -> Self {
        self.series.push(series);
        self
    }

    /// Build the chart, validating that categories and series are non-empty
    /// and that every series has exactly as many values as there are categories.
    pub fn build(self) -> Result<Chart> {
        let series_names: Vec<&str> = self.series.iter().map(|s| s.name.as_str()).collect();
        let series_lengths: Vec<usize> = self.series.iter().map(|s| s.values.len()).collect();
        validate_chart(&self.categories, &series_names, &series_lengths)?;
        Ok(Chart {
            title: self.title,
            chart_type: self.chart_type,
            categories: self.categories,
            series: self.series,
            x: self.placement.x,
            y: self.placement.y,
            width: self.placement.width,
            height: self.placement.height,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chart_builder() {
        let chart = ChartBuilder::new("Revenue", ChartType::Bar)
            .categories(vec!["Q1", "Q2", "Q3"])
            .add_series(ChartSeries::new("2023", vec![100.0, 150.0, 200.0]))
            .add_series(ChartSeries::new("2024", vec![120.0, 180.0, 220.0]))
            .position(100000, 200000)
            .size(4000000, 3000000)
            .build()
            .expect("valid chart");

        assert_eq!(chart.title, "Revenue");
        assert_eq!(chart.chart_type, ChartType::Bar);
        assert_eq!(chart.category_count(), 3);
        assert_eq!(chart.series_count(), 2);
        assert_eq!(chart.x, 100000);
        assert_eq!(chart.y, 200000);
    }

    #[test]
    fn test_chart_builder_rejects_empty_categories() {
        let res = ChartBuilder::new("Revenue", ChartType::Bar)
            .add_series(ChartSeries::new("2023", vec![]))
            .build();
        assert!(res.is_err());
    }

    #[test]
    fn test_chart_builder_rejects_no_series() {
        let res = ChartBuilder::new("Revenue", ChartType::Bar)
            .categories(vec!["Q1", "Q2"])
            .build();
        assert!(res.is_err());
    }

    #[test]
    fn test_chart_builder_rejects_length_mismatch() {
        let res = ChartBuilder::new("Revenue", ChartType::Bar)
            .categories(vec!["Q1", "Q2", "Q3"])
            .add_series(ChartSeries::new("2023", vec![100.0, 150.0, 200.0]))
            .add_series(ChartSeries::new("2024", vec![120.0, 180.0]))
            .build();
        assert!(res.is_err());
        let msg = format!("{}", res.unwrap_err());
        assert!(msg.contains("'2024'"));
    }
}
