/// Math expression evaluator & graph point sampler.
/// Handles natural-language patterns like "15% of 300", implicit
/// multiplication ("2(3)", "3x"), and all standard math functions.

use serde::{Deserialize, Serialize};

// ─── Types ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalcResult {
    pub expression: String,
    pub result: f64,
    pub display: String,
    pub has_variable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPoint {
    pub x: f64,
    pub y: f64,
}

// ─── Preprocessor ────────────────────────────────────────

/// Convert human-friendly math into meval-compatible expressions.
fn preprocess(input: &str) -> String {
    let mut s = input.trim().to_lowercase();

    // Replace unicode operators
    s = s.replace('×', "*").replace('÷', "/");

    // Handle "log(" → "log10(" (common convention)
    // Protect existing log10/log2 first
    s = s
        .replace("log10(", "\x01LOGTEN\x02")
        .replace("log2(", "\x01LOGTWO\x02")
        .replace("log(", "log10(")
        .replace("\x01LOGTEN\x02", "log10(")
        .replace("\x01LOGTWO\x02", "log2(");

    // Handle "X% of Y" → "(X/100)*(Y)"
    if let Some(pct_idx) = s.find('%') {
        let before = s[..pct_idx].trim().to_string();
        let after = s[pct_idx + 1..].trim().to_string();

        if after.starts_with("of ") {
            let y_part = after[3..].trim();
            s = format!("({})/100*({})", before, y_part);
        } else if after.is_empty() {
            // "15%" → "(15)/100"
            s = format!("({})/100", before);
        } else if after.starts_with('+')
            || after.starts_with('-')
            || after.starts_with('*')
            || after.starts_with('/')
        {
            // "50% * 200" → "(50/100) * 200"
            s = format!("({})/100{}", before, after);
        }
    }

    // Insert implicit multiplication where appropriate:
    //   digit(  → digit*(
    //   )(      → )*(
    //   )digit  → )*digit
    //   digit x → digit*x   (variable)
    //   digit pi → digit*pi (constant)
    let mut result = String::with_capacity(s.len() + 16);
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();

    for i in 0..len {
        result.push(chars[i]);
        if i + 1 < len {
            let c = chars[i];
            let n = chars[i + 1];

            // digit or ')' followed by '(' or letter → insert '*'
            let curr_is_value = c.is_ascii_digit() || c == '.' || c == ')';
            let next_is_start = n == '(' || n == 'x';

            // ')' followed by digit
            let paren_digit = c == ')' && n.is_ascii_digit();

            if (curr_is_value && next_is_start) || paren_digit {
                result.push('*');
            }
        }
    }

    result
}

// ─── Evaluator ───────────────────────────────────────────

/// Try to evaluate a string as a math expression.
/// Returns `None` if the string isn't valid math or is just a bare number.
pub fn evaluate(input: &str) -> Option<CalcResult> {
    let raw = input.trim();
    if raw.is_empty() || raw.len() < 2 {
        return None;
    }

    // Quick reject: search-mode prefixes
    if raw.starts_with('>') || raw.starts_with('?') {
        return None;
    }

    // Must contain at least one digit
    if !raw.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }

    let processed = preprocess(raw);

    // Check for variable 'x' (graphable equation)
    let has_x = processed.contains('x');

    if has_x {
        // Validate by evaluating at x = 1
        let test = processed.replace('x', "(1)");
        match meval::eval_str(&test) {
            Ok(v) if v.is_finite() => Some(CalcResult {
                expression: raw.to_string(),
                result: 0.0,
                display: format!("f(x) = {}", raw),
                has_variable: true,
            }),
            _ => None,
        }
    } else {
        let val = match meval::eval_str(&processed) {
            Ok(v) if v.is_finite() => v,
            _ => return None,
        };

        // Reject bare numbers (no operator / function) — "42" should search, not calc
        let has_operator = processed.contains('+')
            || processed.contains('-')
            || processed.contains('*')
            || processed.contains('/')
            || processed.contains('^')
            || processed.contains('(');
        let is_constant = processed == "pi" || processed == "e";
        let had_percent = raw.contains('%');

        if !has_operator && !is_constant && !had_percent {
            return None;
        }

        Some(CalcResult {
            expression: raw.to_string(),
            result: val,
            display: format_number(val),
            has_variable: false,
        })
    }
}

/// Sample a function f(x) over [x_min, x_max] for graphing.
pub fn evaluate_graph(expr: &str, x_min: f64, x_max: f64, steps: usize) -> Vec<GraphPoint> {
    let processed = preprocess(expr.trim());
    let steps = steps.min(1000).max(10); // clamp to sane range
    let mut points = Vec::with_capacity(steps + 1);

    let step_size = (x_max - x_min) / steps as f64;

    for i in 0..=steps {
        let x = x_min + step_size * i as f64;
        let substituted = processed.replace('x', &format!("({})", x));

        if let Ok(y) = meval::eval_str(&substituted) {
            if y.is_finite() {
                points.push(GraphPoint { x, y });
            }
        }
    }

    points
}

// ─── Formatting ──────────────────────────────────────────

fn format_number(val: f64) -> String {
    if val == val.floor() && val.abs() < 1e15 {
        format!("{}", val as i64)
    } else {
        // Up to 10 decimal places, strip trailing zeros
        let s = format!("{:.10}", val);
        let s = s.trim_end_matches('0');
        let s = s.trim_end_matches('.');
        s.to_string()
    }
}

// ─── Tests ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_arithmetic() {
        let r = evaluate("2+2").unwrap();
        assert_eq!(r.display, "4");
        assert!(!r.has_variable);
    }

    #[test]
    fn percentage_of() {
        let r = evaluate("15% of 300").unwrap();
        assert!((r.result - 45.0).abs() < 0.001);
    }

    #[test]
    fn trig_function() {
        let r = evaluate("sin(0)").unwrap();
        assert!((r.result - 0.0).abs() < 0.001);
    }

    #[test]
    fn power() {
        let r = evaluate("2^10").unwrap();
        assert_eq!(r.display, "1024");
    }

    #[test]
    fn graphable() {
        let r = evaluate("x^2 + 1").unwrap();
        assert!(r.has_variable);
    }

    #[test]
    fn bare_number_rejected() {
        assert!(evaluate("42").is_none());
    }

    #[test]
    fn plain_text_rejected() {
        assert!(evaluate("hello world").is_none());
    }

    #[test]
    fn graph_points() {
        let pts = evaluate_graph("x^2", -2.0, 2.0, 100);
        assert!(pts.len() == 101); // 100 steps → 101 points
        // First point: x=-2 → y=4
        assert!((pts[0].x - (-2.0)).abs() < 0.001);
        assert!((pts[0].y - 4.0).abs() < 0.001);
        // Last point: x=2 → y=4
        assert!((pts[100].x - 2.0).abs() < 0.001);
        assert!((pts[100].y - 4.0).abs() < 0.001);
    }
}
