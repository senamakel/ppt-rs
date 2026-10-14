//! End-to-end tests for chart customization (legend, axes, data labels).
//!
//! python-pptx parity:
//! - `chart.has_legend` / `chart.legend.position` → `ChartBuilder::legend(LegendConfig::…)`
//! - `chart.value_axis.min/max/major_unit` / `log_scale` / `number_format` →
//!   `ChartBuilder::value_axis(AxisConfig::…)`
//! - `chart.plots[0].has_data_labels` / `position` → `ChartBuilder::data_labels(DataLabelsConfig::…)`

use ppt_rs::generator::charts::{
    AxisConfig, ChartBuilder, ChartSeries, ChartType, DataLabelsConfig, LabelPosition,
    LegendConfig, LegendPosition,
};
use ppt_rs::generator::SlideContent;
use ppt_rs::Presentation;
use std::io::Read as _;

fn chart_xml_from_deck(deck: &[u8]) -> String {
    let cursor = std::io::Cursor::new(deck);
    let mut archive = zip::ZipArchive::new(cursor).unwrap();
    let mut s = String::new();
    archive
        .by_name("ppt/charts/chart1.xml")
        .expect("chart1.xml should exist")
        .read_to_string(&mut s)
        .unwrap();
    s
}

fn slide_with_chart(builder: ChartBuilder) -> Vec<u8> {
    let chart = builder.build().expect("valid chart");
    let slide = SlideContent::new("Chart Slide").add_chart(chart);
    Presentation::new()
        .add_slide(slide)
        .build()
        .expect("build deck")
}

#[test]
fn default_chart_emits_legend_block_at_right() {
    // No `.legend()` call → default `LegendConfig::shown()` with `Right`.
    let bytes = slide_with_chart(
        ChartBuilder::new("Revenue", ChartType::Bar)
            .categories(vec!["Q1", "Q2"])
            .add_series(ChartSeries::new("S", vec![1.0, 2.0])),
    );
    let xml = chart_xml_from_deck(&bytes);
    assert!(xml.contains(r#"<c:legendPos val="r"/>"#), "got {xml}");
    assert!(!xml.contains("c:legendPos val=\"b\""));
}

#[test]
fn legend_hidden_emits_no_legend_block() {
    let bytes = slide_with_chart(
        ChartBuilder::new("Revenue", ChartType::Bar)
            .categories(vec!["Q1", "Q2"])
            .add_series(ChartSeries::new("S", vec![1.0, 2.0]))
            .legend(LegendConfig::hidden()),
    );
    let xml = chart_xml_from_deck(&bytes);
    // No `<c:legend>` block at all.
    assert!(!xml.contains("<c:legend>"), "legend block emitted: {xml}");
}

#[test]
fn legend_position_bottom_emits_correct_val() {
    let bytes = slide_with_chart(
        ChartBuilder::new("Revenue", ChartType::Bar)
            .categories(vec!["Q1", "Q2"])
            .add_series(ChartSeries::new("S", vec![1.0, 2.0]))
            .legend(LegendConfig::shown().with_position(LegendPosition::Bottom)),
    );
    let xml = chart_xml_from_deck(&bytes);
    assert!(xml.contains(r#"<c:legendPos val="b"/>"#), "got {xml}");
}

#[test]
fn value_axis_min_max_emits_c_min_max() {
    let bytes = slide_with_chart(
        ChartBuilder::new("Revenue", ChartType::Bar)
            .categories(vec!["Q1", "Q2"])
            .add_series(ChartSeries::new("S", vec![1.0, 2.0]))
            .value_axis(AxisConfig::default().with_min(0.0).with_max(100.0)),
    );
    let xml = chart_xml_from_deck(&bytes);
    assert!(xml.contains(r#"<c:min val="0"/>"#), "got {xml}");
    assert!(xml.contains(r#"<c:max val="100"/>"#), "got {xml}");
}

#[test]
fn value_axis_log_scale_emits_c_log_scale() {
    let bytes = slide_with_chart(
        ChartBuilder::new("Revenue", ChartType::Bar)
            .categories(vec!["Q1", "Q2"])
            .add_series(ChartSeries::new("S", vec![1.0, 2.0]))
            .value_axis(AxisConfig::default().with_log_scale(10)),
    );
    let xml = chart_xml_from_deck(&bytes);
    assert!(xml.contains(r#"<c:logScale base="10"/>"#), "got {xml}");
}

#[test]
fn value_axis_number_format_overrides_general() {
    let bytes = slide_with_chart(
        ChartBuilder::new("Revenue", ChartType::Bar)
            .categories(vec!["Q1", "Q2"])
            .add_series(ChartSeries::new("S", vec![1.0, 2.0]))
            .value_axis(AxisConfig::default().with_number_format("0\"%\"%")),
    );
    let xml = chart_xml_from_deck(&bytes);
    assert!(xml.contains(r#"<c:numFmt formatCode="0""#), "got {xml}");
    assert!(xml.contains(r#"sourceLinked="0""#), "got {xml}");
}

#[test]
fn value_axis_major_unit_emits_c_major_unit() {
    let bytes = slide_with_chart(
        ChartBuilder::new("Revenue", ChartType::Bar)
            .categories(vec!["Q1", "Q2"])
            .add_series(ChartSeries::new("S", vec![1.0, 2.0]))
            .value_axis(AxisConfig::default().with_major_unit(10.0)),
    );
    let xml = chart_xml_from_deck(&bytes);
    assert!(xml.contains(r#"<c:majorUnit val="10"/>"#), "got {xml}");
}

#[test]
fn data_labels_show_values_emits_c_dlbls_with_show_val() {
    let bytes = slide_with_chart(
        ChartBuilder::new("Revenue", ChartType::Bar)
            .categories(vec!["Q1", "Q2"])
            .add_series(ChartSeries::new("S", vec![1.0, 2.0]))
            .data_labels(DataLabelsConfig::show_values()),
    );
    let xml = chart_xml_from_deck(&bytes);
    // Plot-level data labels: one `<c:dLbls>` block at chart level with showVal=1.
    assert!(xml.contains(r#"<c:showVal val="1"/>"#), "got {xml}");
    // Default data labels at series level would all be off (no per-series override).
    assert!(xml.contains(r#"<c:dLblPos val="inEnd"/>"#), "got {xml}");
}

#[test]
fn data_labels_show_percentages_emits_c_show_percent() {
    let bytes = slide_with_chart(
        ChartBuilder::new("Share", ChartType::Pie)
            .categories(vec!["A", "B", "C"])
            .add_series(ChartSeries::new("S", vec![10.0, 20.0, 30.0]))
            .data_labels(DataLabelsConfig::show_percentages_outside()),
    );
    let xml = chart_xml_from_deck(&bytes);
    assert!(xml.contains(r#"<c:showPercent val="1"/>"#), "got {xml}");
    assert!(xml.contains(r#"<c:dLblPos val="outEnd"/>"#), "got {xml}");
}

#[test]
fn data_labels_off_emits_no_block() {
    let bytes = slide_with_chart(
        ChartBuilder::new("Revenue", ChartType::Bar)
            .categories(vec!["Q1", "Q2"])
            .add_series(ChartSeries::new("S", vec![1.0, 2.0]))
            // No `.data_labels()` → default off.
            .data_labels(DataLabelsConfig::default()),
    );
    let xml = chart_xml_from_deck(&bytes);
    assert!(!xml.contains("<c:dLbls>"), "plot-level dLbls emitted: {xml}");
}

#[test]
fn all_customizations_together_round_trip() {
    // Combine every config in one chart and verify the XML has all flags.
    let bytes = slide_with_chart(
        ChartBuilder::new("Sales", ChartType::Bar)
            .categories(vec!["Q1", "Q2", "Q3"])
            .add_series(ChartSeries::new("2024", vec![100.0, 200.0, 300.0]))
            .legend(LegendConfig::shown().with_position(LegendPosition::Top))
            .value_axis(
                AxisConfig::default()
                    .with_min(0.0)
                    .with_max(500.0)
                    .with_major_unit(100.0)
                    .with_log_scale(10)
                    .with_number_format("0\"k\"k\"")
                    .with_tick_label_size_pt(12)
                    .with_tick_label_bold(true),
            )
            .category_axis(AxisConfig::default().with_tick_label_rotation(45))
            .data_labels(DataLabelsConfig::show_values()),
    );
    let xml = chart_xml_from_deck(&bytes);
    assert!(xml.contains(r#"<c:legendPos val="t"/>"#), "legend pos");
    assert!(xml.contains(r#"<c:min val="0"/>"#));
    assert!(xml.contains(r#"<c:max val="500"/>"#));
    assert!(xml.contains(r#"<c:majorUnit val="100"/>"#));
    assert!(xml.contains(r#"<c:logScale base="10"/>"#));
    assert!(xml.contains(r#"<c:numFmt formatCode="0""#));
    assert!(xml.contains(r#"<c:tickLblPos val="low"/>"#));
    assert!(xml.contains(r#"<c:lblAlgn val="ctr"/>"#), "rotation");
    assert!(xml.contains(r#"<c:showVal val="1"/>"#), "data labels");
}

#[test]
fn chart_customization_does_not_break_package_validation() {
    // Save and run the zero-repair gate.
    let bytes = slide_with_chart(
        ChartBuilder::new("Sales", ChartType::Bar)
            .categories(vec!["Q1"])
            .add_series(ChartSeries::new("S", vec![10.0]))
            .value_axis(AxisConfig::default().with_min(0.0).with_max(50.0))
            .data_labels(DataLabelsConfig::show_values()),
    );
    // Build, then save.
    let scratch = std::env::temp_dir().join(format!(
        "ppt_rs_chart_custom_{}",
        std::process::id()
    ));
    let _ = std::fs::create_dir_all(&scratch);
    let path = scratch.join("out.pptx");
    std::fs::write(&path, &bytes).unwrap();

    let saved = std::fs::read(&path).unwrap();
    let report = ppt_rs::validate_package_bytes(&saved);
    let errors: Vec<_> = report
        .issues
        .iter()
        .filter(|i| matches!(i.severity, ppt_rs::ValidationSeverity::Error))
        .collect();
    assert!(
        errors.is_empty(),
        "chart-customization regressed package validation: {errors:?}"
    );
}

#[test]
fn label_position_str_renders_all_variants() {
    // Sanity check the LabelPosition → OOXML enum mapping.
    for (variant, expected) in [
        (LabelPosition::BestFit, "bestFit"),
        (LabelPosition::Center, "ctr"),
        (LabelPosition::InsideBase, "inBase"),
        (LabelPosition::InsideEnd, "inEnd"),
        (LabelPosition::OutsideEnd, "outEnd"),
        (LabelPosition::Left, "l"),
        (LabelPosition::Right, "r"),
        (LabelPosition::Top, "t"),
        (LabelPosition::Bottom, "b"),
    ] {
        assert_eq!(variant.as_str(), expected, "{variant:?}");
    }
}

#[test]
fn legend_position_str_renders_all_variants() {
    for (variant, expected) in [
        (LegendPosition::Right, "r"),
        (LegendPosition::Left, "l"),
        (LegendPosition::Top, "t"),
        (LegendPosition::TopRight, "tr"),
        (LegendPosition::Bottom, "b"),
    ] {
        assert_eq!(variant.as_str(), expected, "{variant:?}");
    }
}