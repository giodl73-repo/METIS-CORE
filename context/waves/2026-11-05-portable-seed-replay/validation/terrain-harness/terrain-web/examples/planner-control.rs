//! Native reference for the unchanged public planning function, not a new model.
use serde::Deserialize;
use serde_json::Value;
use std::io::{self, BufRead, Write};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    csv: String,
    count: usize,
    graph: bool,
    demand_limit: Value,
    revenue_limit: Value,
}

// JSON cannot carry these f64 values. Named tokens belong only to this test
// transport; both references receive the same actual typed f64 argument.
fn number(value: Value) -> io::Result<f64> {
    if let Some(number) = value.as_f64() {
        return Ok(number);
    }
    match value.as_str() {
        Some("NaN") => Ok(f64::NAN),
        Some("Infinity") => Ok(f64::INFINITY),
        Some("-Infinity") => Ok(f64::NEG_INFINITY),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid control f64",
        )),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut output = io::BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        let control: Control = serde_json::from_str(&line?)?;
        let result = terrain_web::plan(
            &control.csv,
            control.count,
            control.graph,
            number(control.demand_limit)?,
            number(control.revenue_limit)?,
        );
        serde_json::to_writer(&mut output, &result)?;
        writeln!(output)?;
    }
    Ok(())
}
