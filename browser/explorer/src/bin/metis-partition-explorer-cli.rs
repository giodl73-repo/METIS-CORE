use std::io::{Read, Write};
fn main() {
    let mut input = String::new();
    std::io::stdin()
        .take(3 * 1024 * 1024 + 1)
        .read_to_string(&mut input)
        .expect("UTF-8 input");
    let result = metis_partition_explorer::partition_request(&input);
    std::io::stdout()
        .write_all(result.as_bytes())
        .expect("JSON output");
}
