/// Simple expression parser for functional sources
/// Supports: numbers with SI units, time variable 't', basic functions, conditionals
/// Examples:
///   - "5" -> constant 5V
///   - "if t > 1m then 5 else 0" -> conditional
///   - "5 * sin(2*pi*1k*t)" -> sinusoid
///   - "t < 100u ? 10 : 0" -> ternary conditional
use std::f64::consts::PI;

pub fn parse_expression(expr: &str) -> Result<Box<dyn Fn(f64) -> f64 + Send + Sync>, String> {
    let trimmed = expr.trim();

    // Try to parse as ternary conditional: condition ? true_val : false_val
    if let Ok(closure) = parse_ternary(trimmed) {
        return Ok(closure);
    }

    // Try to parse as if-then-else
    if trimmed.starts_with("if ") {
        return parse_if_then_else(trimmed);
    }

    // Otherwise, try to parse as a number or expression
    parse_expr_simple(trimmed)
}

/// Parse "condition ? expr1 : expr2" style ternary
fn parse_ternary(input: &str) -> Result<Box<dyn Fn(f64) -> f64 + Send + Sync>, String> {
    // Find the positions of ? and :
    if let Some(q_pos) = input.find('?') {
        if let Some(colon_pos) = input[q_pos..].find(':') {
            let colon_pos = q_pos + colon_pos;

            let cond_str = input[..q_pos].trim();
            let true_str = input[q_pos + 1..colon_pos].trim();
            let false_str = input[colon_pos + 1..].trim();

            let cond_closure = parse_comparison(cond_str)?;
            let true_closure = parse_expr_simple(true_str)?;
            let false_closure = parse_expr_simple(false_str)?;

            return Ok(Box::new(move |t| {
                if cond_closure(t) > 0.5 {
                    true_closure(t)
                } else {
                    false_closure(t)
                }
            }));
        }
    }
    Err("Not a ternary expression".to_string())
}

/// Parse "if condition then expr1 else expr2"
fn parse_if_then_else(input: &str) -> Result<Box<dyn Fn(f64) -> f64 + Send + Sync>, String> {
    if !input.starts_with("if ") {
        return Err("Expected 'if'".to_string());
    }

    let rest = &input[3..];

    // Find "then"
    let then_pos = rest
        .find(" then ")
        .ok_or("Missing 'then' in if statement")?;

    let cond_str = rest[..then_pos].trim();
    let after_then = &rest[then_pos + 6..];

    // Find "else"
    let else_pos = after_then
        .rfind(" else ")
        .ok_or("Missing 'else' in if statement")?;

    let true_str = after_then[..else_pos].trim();
    let false_str = after_then[else_pos + 6..].trim();

    let cond_closure = parse_comparison(cond_str)?;
    let true_closure = parse_expr_simple(true_str)?;
    let false_closure = parse_expr_simple(false_str)?;

    Ok(Box::new(move |t| {
        if cond_closure(t) > 0.5 {
            true_closure(t)
        } else {
            false_closure(t)
        }
    }))
}

/// Parse comparison: "a > b", "a < b", "a >= b", "a <= b", "a == b", "a != b"
fn parse_comparison(input: &str) -> Result<Box<dyn Fn(f64) -> f64 + Send + Sync>, String> {
    // Check in order of longest operators first
    if let Some(pos) = input.find("==") {
        let left = parse_expr_simple(input[..pos].trim())?;
        let right = parse_expr_simple(input[pos + 2..].trim())?;
        return Ok(Box::new(move |t| {
            if (left(t) - right(t)).abs() < 1e-10 {
                1.0
            } else {
                0.0
            }
        }));
    }
    if let Some(pos) = input.find("!=") {
        let left = parse_expr_simple(input[..pos].trim())?;
        let right = parse_expr_simple(input[pos + 2..].trim())?;
        return Ok(Box::new(move |t| {
            if (left(t) - right(t)).abs() >= 1e-10 {
                1.0
            } else {
                0.0
            }
        }));
    }
    if let Some(pos) = input.find(">=") {
        let left = parse_expr_simple(input[..pos].trim())?;
        let right = parse_expr_simple(input[pos + 2..].trim())?;
        return Ok(Box::new(
            move |t| {
                if left(t) >= right(t) { 1.0 } else { 0.0 }
            },
        ));
    }
    if let Some(pos) = input.find("<=") {
        let left = parse_expr_simple(input[..pos].trim())?;
        let right = parse_expr_simple(input[pos + 2..].trim())?;
        return Ok(Box::new(
            move |t| {
                if left(t) <= right(t) { 1.0 } else { 0.0 }
            },
        ));
    }
    if let Some(pos) = input.find('>') {
        let left = parse_expr_simple(input[..pos].trim())?;
        let right = parse_expr_simple(input[pos + 1..].trim())?;
        return Ok(Box::new(
            move |t| {
                if left(t) > right(t) { 1.0 } else { 0.0 }
            },
        ));
    }
    if let Some(pos) = input.find('<') {
        let left = parse_expr_simple(input[..pos].trim())?;
        let right = parse_expr_simple(input[pos + 1..].trim())?;
        return Ok(Box::new(
            move |t| {
                if left(t) < right(t) { 1.0 } else { 0.0 }
            },
        ));
    }

    Err("No comparison operator found".to_string())
}

/// Parse simple expressions: numbers, 't', operations, function calls
fn parse_expr_simple(input: &str) -> Result<Box<dyn Fn(f64) -> f64 + Send + Sync>, String> {
    let trimmed = input.trim();

    // Try to parse as a number with optional unit
    if let Ok(val) = parse_number(trimmed) {
        return Ok(Box::new(move |_t| val));
    }

    // If it's just 't', return identity
    if trimmed == "t" {
        return Ok(Box::new(|t| t));
    }

    // If input contains pi, replace it with its numerical value
    let trimmed = trimmed.replace("pi", &std::f64::consts::PI.to_string());

    // Try addition/subtraction (lowest precedence)
    for (i, ch) in trimmed.char_indices() {
        // Skip if inside parentheses
        let mut paren_depth = 0;
        for (_, c) in trimmed[..i].chars().enumerate() {
            if c == '(' {
                paren_depth += 1;
            }
            if c == ')' {
                paren_depth -= 1;
            }
        }
        if paren_depth != 0 {
            continue;
        }

        if ch == '+' {
            let left = parse_expr_simple(trimmed[..i].trim())?;
            let right = parse_expr_simple(trimmed[i + 1..].trim())?;
            return Ok(Box::new(move |t| left(t) + right(t)));
        }
        if ch == '-' && i > 0 {
            let left = parse_expr_simple(trimmed[..i].trim())?;
            let right = parse_expr_simple(trimmed[i + 1..].trim())?;
            return Ok(Box::new(move |t| left(t) - right(t)));
        }
    }

    // Try multiplication/division and powers
    for (i, ch) in trimmed.char_indices() {
        let mut paren_depth = 0;
        for _c in trimmed[..i].chars() {
            if _c == '(' {
                paren_depth += 1;
            }
            if _c == ')' {
                paren_depth -= 1;
            }
        }
        if paren_depth != 0 {
            continue;
        }

        if ch == '*' {
            let left = parse_expr_simple(trimmed[..i].trim())?;
            let right = parse_expr_simple(trimmed[i + 1..].trim())?;
            return Ok(Box::new(move |t| left(t) * right(t)));
        }
        if ch == '/' {
            let left = parse_expr_simple(trimmed[..i].trim())?;
            let right = parse_expr_simple(trimmed[i + 1..].trim())?;
            return Ok(Box::new(move |t| {
                let r = right(t);
                if r.abs() > 1e-20 { left(t) / r } else { 0.0 }
            }));
        }
        if ch == '^' {
            let left = parse_expr_simple(trimmed[..i].trim())?;
            let right = parse_expr_simple(trimmed[i + 1..].trim())?;
            return Ok(Box::new(move |t| left(t).powf(right(t))));
        }
    }

    // Handle special constant pi
    if trimmed == "pi" {
        return Ok(Box::new(move |_t| PI));
    }

    // Try to parse function calls: sin(x), cos(x), etc.
    // Only recognize as function if func_name contains only letters/underscore
    if let Some(paren_pos) = trimmed.find('(') {
        let func_name = trimmed[..paren_pos].trim();
        // Check if func_name is a valid identifier (only letters/underscores)
        let is_valid_ident = !func_name.is_empty() && 
            func_name.chars().all(|c| c.is_alphabetic() || c == '_');
        
        if is_valid_ident {
            if let Some(close_paren) = trimmed.rfind(')') {
                let arg_str = trimmed[paren_pos + 1..close_paren].trim();

                if let Ok(arg_closure) = parse_expr_simple(arg_str) {
                    return match func_name {
                        "sin" => Ok(Box::new(move |t| arg_closure(t).sin())),
                        "cos" => Ok(Box::new(move |t| arg_closure(t).cos())),
                        "tan" => Ok(Box::new(move |t| arg_closure(t).tan())),
                        "exp" => Ok(Box::new(move |t| arg_closure(t).exp())),
                        "sqrt" => Ok(Box::new(move |t| arg_closure(t).sqrt())),
                        "abs" => Ok(Box::new(move |t| arg_closure(t).abs())),
                        "log" => Ok(Box::new(move |t| arg_closure(t).ln())),
                        "log10" => Ok(Box::new(move |t| arg_closure(t).log10())),
                        _ => Err(format!("Unknown function: {}", func_name)),
                    };
                }
            }
        }
    }

    Err(format!("Cannot parse expression: '{}'", trimmed))
}

/// Parse a number with optional SI unit suffix
/// Examples: "5", "5m", "1u", "1k", "100n", "1M"
fn parse_number(input: &str) -> Result<f64, String> {
    let input = input.trim();

    if input.is_empty() {
        return Err("Empty number".to_string());
    }

    // Find where digits/decimal point end and unit begins
    // Use char_indices for UTF-8 safe splitting
    let mut unit_start = input.len();

    for (i, ch) in input.char_indices() {
        if !ch.is_numeric() && ch != '.' && ch != '-' && ch != '+' && ch != 'e' && ch != 'E' {
            unit_start = i;
            break;
        }
    }

    let num_str = input[..unit_start].trim();
    let unit_str = input[unit_start..].trim();

    if num_str.is_empty() {
        return Err(format!("No numeric value in: {}", input));
    }

    let base: f64 = num_str
        .parse()
        .map_err(|_| format!("Invalid number: {}", num_str))?;

    // Parse SI unit suffix
    let multiplier = match unit_str {
        "" => 1.0,
        "p" => 1e-12,
        "n" => 1e-9,
        "u" | "µ" => 1e-6,  // micro (u or µ)
        "m" => 1e-3,        // milli
        "k" => 1e3,         // kilo
        "M" => 1e6,         // mega
        "G" => 1e9,         // giga
        _ => return Err(format!("Unknown unit: '{}'. Use: p, n, u, m, k, M, G", unit_str)),
    };

    Ok(base * multiplier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant() {
        let f = parse_expr_simple("5").unwrap();
        assert!((f(0.0) - 5.0).abs() < 1e-10);
        assert!((f(1.0) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_unit_milli() {
        let f = parse_expr_simple("5m").unwrap();
        assert!((f(0.0) - 0.005).abs() < 1e-10, "Got {}", f(0.0));
    }

    #[test]
    fn test_unit_micro() {
        let f = parse_expr_simple("100u").unwrap();
        assert!((f(0.0) - 100e-6).abs() < 1e-15);
    }

    #[test]
    fn test_unit_nano() {
        let f = parse_expr_simple("1n").unwrap();
        assert!((f(0.0) - 1e-9).abs() < 1e-18);
    }

    #[test]
    fn test_unit_kilo() {
        let f = parse_expr_simple("1k").unwrap();
        assert!((f(0.0) - 1000.0).abs() < 1e-10);
    }

    #[test]
    fn test_unit_mega() {
        let f = parse_expr_simple("1M").unwrap();
        assert!((f(0.0) - 1e6).abs() < 100.0);
    }

    #[test]
    fn test_variable_t() {
        let f = parse_expr_simple("t").unwrap();
        assert!((f(2.5) - 2.5).abs() < 1e-10);
        assert!((f(0.001) - 0.001).abs() < 1e-10);
    }

    #[test]
    fn test_arithmetic() {
        let f = parse_expr_simple("2 + 3").unwrap();
        assert!((f(0.0) - 5.0).abs() < 1e-10);

        let f = parse_expr_simple("10 - 3").unwrap();
        assert!((f(0.0) - 7.0).abs() < 1e-10);

        let f = parse_expr_simple("2 * 3").unwrap();
        assert!((f(0.0) - 6.0).abs() < 1e-10);

        let f = parse_expr_simple("10 / 2").unwrap();
        assert!((f(0.0) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_if_then_else() {
        let f = parse_expression("if t > 1 then 5 else 0").unwrap();
        assert!((f(0.5) - 0.0).abs() < 1e-10);
        assert!((f(1.5) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_ternary() {
        let f = parse_expression("t > 1 ? 5 : 0").unwrap();
        assert!((f(0.5) - 0.0).abs() < 1e-10);
        assert!((f(1.5) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_sine() {
        let f = parse_expr_simple("sin(0)").unwrap();
        assert!(f(0.0).abs() < 1e-10);

        let f = parse_expr_simple("sin(pi/2)").unwrap();
        assert!((f(0.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_comparison_greater() {
        let f = parse_comparison("t > 1").unwrap();
        assert!(f(0.5) < 0.5);  // false
        assert!(f(1.5) > 0.5);  // true
    }

    #[test]
    fn test_complex_expr() {
        let f = parse_expr_simple("5 * sin(2 * pi * 1k * t)").unwrap();
        let val_at_zero = f(0.0);
        assert!(val_at_zero.abs() < 1e-10);
    }

    #[test]
    fn test_time_step_function() {
        let f = parse_expression("if t > 1m then 5 else 0").unwrap();
        assert!((f(0.0) - 0.0).abs() < 1e-10);
        assert!((f(0.002) - 5.0).abs() < 1e-10);
    }
}
