//! Excel-like formula evaluation against a [`DataTable`].
//!
//! Supports cell references (e.g. `A1`, `BC42`), numeric literals, the four
//! arithmetic operators (`+`, `-`, `*`, `/`) and parenthesised sub-expressions.
//! Re-exported from [`crate::data`] so call sites keep their existing import
//! path (`use octa::data::evaluate_formula`).

use super::{CellValue, DataTable};

/// Snapshot of one referenced cell that could not be coerced to a number
/// during formula evaluation: zero-indexed `(row, col)` plus a short display
/// of the offending content. Used to surface a parse-error banner when the
/// Insert Column dialog runs a formula against a column that turns out to be
/// non-numeric in some rows.
#[derive(Debug, Clone)]
pub struct FormulaBadCell {
    pub row: usize,
    pub col: usize,
    pub content: String,
}

/// Result of evaluating a formula for one row.
///
/// `value` is `Some(_)` when every referenced cell coerced cleanly; `None`
/// when at least one didn't (or when the formula was malformed). `bad_cell`
/// carries the first offender so callers can build a "first non-numeric
/// cell: X" banner without re-walking the row.
#[derive(Debug, Clone)]
pub struct FormulaOutcome {
    pub value: Option<f64>,
    pub bad_cell: Option<FormulaBadCell>,
}

/// Evaluate a simple Excel-like formula.
/// Supports cell references (e.g. A1, B2), numeric literals, and operators +, -, *, /.
/// Column letters: A=0, B=1, ..., Z=25, AA=26, etc. Row numbers are 1-based.
/// Returns the computed f64 result, or None if the formula is invalid.
pub fn evaluate_formula(formula: &str, table: &DataTable) -> Option<f64> {
    evaluate_formula_with_diagnostics(formula, table).value
}

/// Like [`evaluate_formula`] but also reports the first cell reference that
/// pointed at a non-numeric value (e.g. a string column or an unparseable
/// piece of text). Returns `value: None` when any referenced cell failed.
pub fn evaluate_formula_with_diagnostics(formula: &str, table: &DataTable) -> FormulaOutcome {
    let expr = formula.trim();
    if expr.is_empty() {
        return FormulaOutcome {
            value: None,
            bad_cell: None,
        };
    }
    let Some(tokens) = tokenize_formula(expr) else {
        return FormulaOutcome {
            value: None,
            bad_cell: None,
        };
    };
    let mut bad: Option<FormulaBadCell> = None;
    let value = eval_expression(&tokens, 0, table, &mut bad).map(|(val, _)| val);
    FormulaOutcome {
        value,
        bad_cell: bad,
    }
}

/// Resolve a cell reference like "A1", "BC42" to (row, col) zero-indexed.
fn parse_cell_ref(s: &str) -> Option<(usize, usize)> {
    let s = s.trim();
    let col_end = s.bytes().position(|b| b.is_ascii_digit())?;
    if col_end == 0 {
        return None;
    }
    let col_str = &s[..col_end];
    let row_str = &s[col_end..];
    if row_str.is_empty() {
        return None;
    }
    // Column: A=0, B=1, ..., Z=25, AA=26
    let mut col: usize = 0;
    for ch in col_str.chars() {
        if !ch.is_ascii_alphabetic() {
            return None;
        }
        col = col * 26 + (ch.to_ascii_uppercase() as usize - b'A' as usize + 1);
    }
    col -= 1; // zero-indexed
    let row: usize = row_str.parse::<usize>().ok()?;
    if row == 0 {
        return None;
    }
    Some((row - 1, col))
}

/// Try to read a cell as f64. Returns:
///
/// * `Ok(Some(f))` - numeric (int / float, or a Null treated as 0).
/// * `Ok(None)` - the cell is out of bounds (`table.get` returned None);
///   not a "bad value", just a missing one.
/// * `Err(FormulaBadCell)` - the cell exists but isn't a number. Includes a
///   short content snippet so the caller can build a user-facing error.
///
/// Strings are accepted only when they parse cleanly as f64; "abc" returns
/// `Err` rather than silently coercing to 0.0 (which previously corrupted
/// formula results when a referenced column was Utf8).
fn cell_as_f64(table: &DataTable, row: usize, col: usize) -> Result<Option<f64>, FormulaBadCell> {
    let Some(cell) = table.get(row, col) else {
        return Ok(None);
    };
    match cell {
        CellValue::Int(n) => Ok(Some(*n as f64)),
        CellValue::Float(f) => Ok(Some(*f)),
        CellValue::String(s) => match s.trim().parse::<f64>() {
            Ok(v) => Ok(Some(v)),
            Err(_) => Err(FormulaBadCell {
                row,
                col,
                content: snippet(s),
            }),
        },
        CellValue::Null => Ok(Some(0.0)),
        other => Err(FormulaBadCell {
            row,
            col,
            content: snippet(&other.to_string()),
        }),
    }
}

/// Trim long cell content to a short display snippet for error messages.
fn snippet(s: &str) -> String {
    const MAX: usize = 40;
    if s.chars().count() <= MAX {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(MAX).collect();
        t.push_str("...");
        t
    }
}

#[derive(Debug, Clone)]
enum FormulaToken {
    Number(f64),
    CellRef(usize, usize), // (row, col)
    Op(char),              // +, -, *, /
    LParen,
    RParen,
}

fn tokenize_formula(expr: &str) -> Option<Vec<FormulaToken>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            ' ' => {
                i += 1;
            }
            '+' | '-' | '*' | '/' => {
                // Unary minus: treat as part of number if at start or after operator/lparen
                if chars[i] == '-' {
                    let is_unary = tokens.is_empty()
                        || matches!(
                            tokens.last(),
                            Some(FormulaToken::Op(_)) | Some(FormulaToken::LParen)
                        );
                    if is_unary {
                        // Collect the number after the minus
                        i += 1;
                        let start = i;
                        while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                            i += 1;
                        }
                        if i == start {
                            return None;
                        }
                        let num_str: String = chars[start..i].iter().collect();
                        let val: f64 = num_str.parse().ok()?;
                        tokens.push(FormulaToken::Number(-val));
                        continue;
                    }
                }
                tokens.push(FormulaToken::Op(chars[i]));
                i += 1;
            }
            '(' => {
                tokens.push(FormulaToken::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(FormulaToken::RParen);
                i += 1;
            }
            c if c.is_ascii_digit() || c == '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let num_str: String = chars[start..i].iter().collect();
                let val: f64 = num_str.parse().ok()?;
                tokens.push(FormulaToken::Number(val));
            }
            c if c.is_ascii_alphabetic() => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric()) {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                let (row, col) = parse_cell_ref(&word)?;
                tokens.push(FormulaToken::CellRef(row, col));
            }
            _ => return None,
        }
    }
    Some(tokens)
}

/// Recursive descent parser: expression = term (('+' | '-') term)*
///
/// `bad` captures the first non-numeric cell ref encountered during the
/// walk. Once set, evaluation still continues so we can drain the full
/// token stream, but the final value is dropped by the caller (None).
fn eval_expression(
    tokens: &[FormulaToken],
    pos: usize,
    table: &DataTable,
    bad: &mut Option<FormulaBadCell>,
) -> Option<(f64, usize)> {
    let (mut left, mut p) = eval_term(tokens, pos, table, bad)?;
    while p < tokens.len() {
        match &tokens[p] {
            FormulaToken::Op('+') => {
                let (right, np) = eval_term(tokens, p + 1, table, bad)?;
                left += right;
                p = np;
            }
            FormulaToken::Op('-') => {
                let (right, np) = eval_term(tokens, p + 1, table, bad)?;
                left -= right;
                p = np;
            }
            _ => break,
        }
    }
    Some((left, p))
}

/// term = factor (('*' | '/') factor)*
fn eval_term(
    tokens: &[FormulaToken],
    pos: usize,
    table: &DataTable,
    bad: &mut Option<FormulaBadCell>,
) -> Option<(f64, usize)> {
    let (mut left, mut p) = eval_factor(tokens, pos, table, bad)?;
    while p < tokens.len() {
        match &tokens[p] {
            FormulaToken::Op('*') => {
                let (right, np) = eval_factor(tokens, p + 1, table, bad)?;
                left *= right;
                p = np;
            }
            FormulaToken::Op('/') => {
                let (right, np) = eval_factor(tokens, p + 1, table, bad)?;
                if right == 0.0 {
                    return None; // division by zero
                }
                left /= right;
                p = np;
            }
            _ => break,
        }
    }
    Some((left, p))
}

/// factor = Number | CellRef | '(' expression ')'
fn eval_factor(
    tokens: &[FormulaToken],
    pos: usize,
    table: &DataTable,
    bad: &mut Option<FormulaBadCell>,
) -> Option<(f64, usize)> {
    if pos >= tokens.len() {
        return None;
    }
    match &tokens[pos] {
        FormulaToken::Number(n) => Some((*n, pos + 1)),
        FormulaToken::CellRef(row, col) => {
            match cell_as_f64(table, *row, *col) {
                Ok(Some(v)) => Some((v, pos + 1)),
                // Out-of-bounds reference: treat as 0 so a partly-overrun
                // formula still produces a result for the rows that do fit.
                Ok(None) => Some((0.0, pos + 1)),
                Err(b) => {
                    if bad.is_none() {
                        *bad = Some(b);
                    }
                    // Tell the caller "this row can't be numerically
                    // resolved" by aborting the walk.
                    None
                }
            }
        }
        FormulaToken::LParen => {
            let (val, p) = eval_expression(tokens, pos + 1, table, bad)?;
            if p < tokens.len() && matches!(tokens[p], FormulaToken::RParen) {
                Some((val, p + 1))
            } else {
                None
            }
        }
        _ => None,
    }
}
