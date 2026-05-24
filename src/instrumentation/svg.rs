/// Generate an SVG scatter plot.
///
/// `data` is a sequence of `(index, value)` pairs.  The X axis represents
/// the insertion order (event index) and the Y axis represents the value
/// (e.g. latency in ms).
///
/// The returned string is a self-contained `<svg>` element.
pub fn scatter_plot_svg(data: &[(usize, u64)]) -> String {
    if data.is_empty() {
        return empty_svg();
    }

    let width = 800;
    let height = 400;
    let padding = 60;
    let plot_w = width - 2 * padding;
    let plot_h = height - 2 * padding;

    let max_value = data.iter().map(|(_, v)| *v).max().unwrap_or(1).max(1);
    let max_x = data.len().max(1);

    let mut svg = svg_open(width, height);
    draw_axes(&mut svg, width, height, padding);
    draw_y_ticks(&mut svg, height, padding, plot_h, max_value, 5);
    draw_x_ticks(&mut svg, width, height, padding, plot_w, max_x as u64, 5);

    for (i, (_, value)) in data.iter().enumerate() {
        let x = padding + ((i as f64 / max_x as f64) * plot_w as f64) as i32;
        let y = height - padding - ((*value as f64 / max_value as f64) * plot_h as f64) as i32;
        svg.push_str(&format!(
            r##"<circle cx="{}" cy="{}" r="3" fill="#007bff"/>"##,
            x, y
        ));
    }

    draw_axis_labels(&mut svg, width, height, "Event Index", "Value");
    svg.push_str("</svg>");
    svg
}

/// Generate an SVG histogram.
///
/// `data` is a sequence of `(index, value)` pairs.  Values are binned into
/// `buckets` equal-width buckets and drawn as vertical bars.
///
/// The returned string is a self-contained `<svg>` element.
pub fn histogram_svg(data: &[(usize, u64)], buckets: usize) -> String {
    if data.is_empty() {
        return empty_svg();
    }

    let width = 800;
    let height = 400;
    let padding = 60;
    let plot_w = width - 2 * padding;
    let plot_h = height - 2 * padding;

    let max_value = data.iter().map(|(_, v)| *v).max().unwrap_or(1).max(1);
    let buckets = buckets.max(1);
    let mut counts = vec![0usize; buckets];

    for (_, value) in data {
        let bucket = ((*value as f64 / max_value as f64) * (buckets as f64 - 1.0))
            .min(buckets as f64 - 1.0) as usize;
        counts[bucket] += 1;
    }

    let max_count = counts.iter().copied().max().unwrap_or(1).max(1);

    let mut svg = svg_open(width, height);
    draw_axes(&mut svg, width, height, padding);
    draw_y_ticks(&mut svg, height, padding, plot_h, max_count as u64, 5);
    draw_x_ticks(&mut svg, width, height, padding, plot_w, max_value, 5);

    let bar_w = plot_w as f64 / buckets as f64;
    for (i, count) in counts.iter().enumerate() {
        let x = padding + (i as f64 * bar_w) as i32;
        let bar_h = ((*count as f64 / max_count as f64) * plot_h as f64) as i32;
        let y = height - padding - bar_h;
        svg.push_str(&format!(
            r##"<rect x="{}" y="{}" width="{}" height="{}" fill="#28a745" stroke="#fff" stroke-width="1"/>"##,
            x,
            y,
            bar_w as i32 - 1,
            bar_h
        ));
    }

    draw_axis_labels(&mut svg, width, height, "Value", "Count");
    svg.push_str("</svg>");
    svg
}

fn empty_svg() -> String {
    r##"<svg width="200" height="100" xmlns="http://www.w3.org/2000/svg">
<text x="100" y="55" text-anchor="middle" font-size="14" fill="#999">No data</text>
</svg>"##
        .to_string()
}

fn svg_open(width: i32, height: i32) -> String {
    format!(
        r##"<svg width="{}" height="{}" viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg">"##,
        width, height, width, height
    )
}

fn draw_axes(svg: &mut String, width: i32, height: i32, padding: i32) {
    svg.push_str(&format!(
        r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#333" stroke-width="2"/>"##,
        padding,
        height - padding,
        width - padding,
        height - padding
    ));
    svg.push_str(&format!(
        r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#333" stroke-width="2"/>"##,
        padding,
        height - padding,
        padding,
        padding
    ));
}

fn draw_y_ticks(
    svg: &mut String,
    height: i32,
    padding: i32,
    plot_h: i32,
    max_value: u64,
    ticks: usize,
) {
    for i in 0..=ticks {
        let v = (max_value as f64 * i as f64 / ticks as f64) as u64;
        let y = height - padding - ((i as f64 / ticks as f64) * plot_h as f64) as i32;
        svg.push_str(&format!(
            r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#333" stroke-width="1"/>"##,
            padding - 5, y, padding, y
        ));
        svg.push_str(&format!(
            r##"<text x="{}" y="{}" text-anchor="end" font-size="10" dominant-baseline="middle">{}</text>"##,
            padding - 8, y, v
        ));
    }
}

fn draw_x_ticks(
    svg: &mut String,
    _width: i32,
    height: i32,
    padding: i32,
    plot_w: i32,
    max_value: u64,
    ticks: usize,
) {
    for i in 0..=ticks {
        let v = (max_value as f64 * i as f64 / ticks as f64) as u64;
        let x = padding + ((i as f64 / ticks as f64) * plot_w as f64) as i32;
        svg.push_str(&format!(
            r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#333" stroke-width="1"/>"##,
            x, height - padding, x, height - padding + 5
        ));
        svg.push_str(&format!(
            r##"<text x="{}" y="{}" text-anchor="middle" font-size="10">{}</text>"##,
            x, height - padding + 18, v
        ));
    }
}

fn draw_axis_labels(svg: &mut String, width: i32, height: i32, x_label: &str, y_label: &str) {
    svg.push_str(&format!(
        r##"<text x="{}" y="{}" text-anchor="middle" font-size="12">{}</text>"##,
        width / 2,
        height - 2,
        x_label
    ));
    svg.push_str(&format!(
        r##"<text x="{}" y="{}" text-anchor="middle" font-size="12" transform="rotate(-90, {}, {})">{}</text>"##,
        12, height / 2, 12, height / 2, y_label
    ));
}
