use metis_core::{
    check_contiguity, CoarseningMethod, CsrGraph, MetisParams, MetisPartitioner, PartitionError,
    Partitioner,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Graph {
    xadj: Vec<u32>,
    adjncy: Vec<u32>,
    #[serde(default)]
    vwgt: Vec<i32>,
    #[serde(default)]
    adjwgt: Vec<i32>,
}
#[derive(Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "snake_case")]
enum Mode {
    #[default]
    Kway,
    Recursive,
}
#[derive(Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "snake_case")]
enum Coarsening {
    #[default]
    Shem,
    Hem,
    MinDegree,
    TwoHop,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    graph_json: String,
    k: u32,
    seed: String,
    #[serde(default)]
    mode: Mode,
    #[serde(default)]
    coarsening: Coarsening,
    ncuts: Option<u32>,
    niter: Option<u32>,
    ufactor: Option<u32>,
    target_weights: Option<Vec<f32>>,
    #[serde(default)]
    contiguity: bool,
    #[serde(default)]
    min_connectivity: bool,
}
fn error(class: &str, detail: Value) -> Value {
    json!({"schema_version":"metis.browser-partition.v1","result":class,"error":detail,"record":null,"target_pointer_bits":usize::BITS})
}
fn native_error(e: PartitionError) -> Value {
    error(
        "native_error",
        json!({"variant":format!("{e:?}"),"message":e.to_string()}),
    )
}
fn run(input: &str) -> Value {
    if input.len() > 3 * 1024 * 1024 {
        return error("browser_refusal", json!("Request framing exceeds 3 MiB"));
    }
    let r: Request = match serde_json::from_str(input) {
        Ok(r) => r,
        Err(e) => return error("invalid_request", json!(e.to_string())),
    };
    if r.graph_json.len() > 1024 * 1024
        || r.k > 16
        || r.ncuts.is_some_and(|n| n > 8)
        || r.niter.is_some_and(|n| n > 20)
        || r.target_weights.as_ref().is_some_and(|w| w.len() > 16)
    {
        return error(
            "browser_refusal",
            json!("Browser bounds: graph1MiB,16parts,8trials,20iterations,16targetweights"),
        );
    }
    if r.seed.is_empty() || r.seed.len() > 20 || !r.seed.bytes().all(|b| b.is_ascii_digit()) {
        return error(
            "invalid_request",
            json!("Seed must be an unsigned decimal u64 string"),
        );
    }
    let seed: u64 = match r.seed.parse() {
        Ok(n) => n,
        Err(_) => return error("invalid_request", json!("Seed exceeds u64")),
    };
    let raw: Graph = match serde_json::from_str(&r.graph_json) {
        Ok(g) => g,
        Err(e) => return error("invalid_graph_json", json!(e.to_string())),
    };
    if raw.xadj.len() > 257
        || raw.adjncy.len() > 4096
        || raw.vwgt.len() > 256
        || raw.adjwgt.len() > 4096
        || raw
            .vwgt
            .iter()
            .chain(raw.adjwgt.iter())
            .any(|w| w.unsigned_abs() > 1000)
    {
        return error(
            "browser_refusal",
            json!("Browser bounds:256vertices,4096adjacency entries,weight magnitude1000"),
        );
    }
    let graph = match CsrGraph::from_csr_strict(&raw.xadj, &raw.adjncy, &raw.vwgt, &raw.adjwgt) {
        Ok(g) => g,
        Err(e) => return native_error(e),
    };
    let mut params = match r.mode {
        Mode::Kway => MetisParams::kway(),
        Mode::Recursive => MetisParams::recursive(),
    }
    .with_seed(seed)
    .with_contiguity(r.contiguity)
    .with_min_connectivity(r.min_connectivity)
    .with_coarsening_method(match r.coarsening {
        Coarsening::Shem => CoarseningMethod::Shem,
        Coarsening::Hem => CoarseningMethod::Hem,
        Coarsening::MinDegree => CoarseningMethod::MinDegree,
        Coarsening::TwoHop => CoarseningMethod::TwoHop,
    });
    if let Some(n) = r.ncuts {
        params = params.with_ncuts(n);
    }
    if let Some(n) = r.niter {
        params = params.with_niter(n);
    }
    if let Some(n) = r.ufactor {
        params = params.with_ufactor(n);
    }
    if let Some(weights) = r.target_weights {
        params = match params.with_target_weights(r.k, weights) {
            Ok(p) => p,
            Err(e) => return native_error(e),
        };
    }
    if let Err(e) = params.validate_for_k(r.k) {
        return native_error(e);
    }
    let controls = json!({"mode":if params.use_recursive(){"recursive"}else{"kway"},"coarsening":format!("{:?}",params.coarsening_method()),"k":r.k,"seed":seed.to_string(),"ncuts":params.ncuts(),"niter":params.niter(),"ufactor":params.ufactor(),"target_weights":params.target_weights(),"contiguity_requested":params.contiguity(),"min_connectivity_requested":params.min_connectivity(),"objective":"edge_cut"});
    let result = match MetisPartitioner::from_params(params).split_result(&graph, r.k, None) {
        Ok(p) => p,
        Err(e) => return native_error(e),
    };
    let partition = result.partition();
    if let Err(e) = partition.validate_for_graph(&graph) {
        return native_error(e);
    }
    let mut counts = vec![0u32; r.k as usize];
    let mut weights = vec![0i64; r.k as usize];
    for (v, &p) in partition.assignment().iter().enumerate() {
        counts[p as usize] += 1;
        weights[p as usize] += i64::from(graph.vwgt()[v]);
    }
    let input_hash = format!("{:x}", Sha256::digest(r.graph_json.as_bytes()));
    json!({"schema_version":"metis.browser-partition.v1","result":"ok","error":null,"target_pointer_bits":usize::BITS,"record":{"supplied_graph_utf8_sha256":input_hash,"controls":controls,"vertices":graph.n(),"adjacency_entries":graph.adjncy().len(),"assignment":partition.assignment(),"edge_cut":result.objective_value(),"part_vertex_counts":counts,"part_weights":weights,"total_vertex_weight":graph.vwgt().iter().map(|w|i64::from(*w)).sum::<i64>(),"all_parts_contiguous":check_contiguity(&graph,partition).is_ok(),"native_partition_validation":"passed"}})
}
#[wasm_bindgen]
pub fn partition_request(input: &str) -> String {
    let result = run(input).to_string();
    if result.len() > 4 * 1024 * 1024 {
        error("browser_refusal", json!("Report exceeds4MiB")).to_string()
    } else {
        result
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn graph() -> Value {
        json!({"xadj":[0,1,3,5,6],"adjncy":[1,0,2,1,3,2]})
    }
    fn request(g: Value) -> Value {
        json!({"graph_json":g.to_string(),"k":2,"seed":"7"})
    }
    #[test]
    fn shared_native_partition_is_valid_and_repeatable() {
        let r = request(graph()).to_string();
        let a = run(&r);
        assert_eq!(a, run(&r));
        assert_eq!(a["result"], "ok");
        assert_eq!(a["record"]["vertices"], 4);
        assert_eq!(a["record"]["native_partition_validation"], "passed");
    }
    #[test]
    fn malformed_csr_retains_native_error() {
        let mut g = graph();
        g["adjncy"] = json!([1, 0, 2]);
        let a = run(&request(g).to_string());
        assert_eq!(a["result"], "native_error");
        assert!(a["record"].is_null());
    }
    #[test]
    fn unsupported_and_resource_errors_are_separate() {
        let mut r = request(graph());
        r["k"] = json!(0);
        assert_eq!(run(&r.to_string())["result"], "native_error");
        r["k"] = json!(17);
        assert_eq!(run(&r.to_string())["result"], "browser_refusal");
        r["k"] = json!(2);
        r["seed"] = json!("18446744073709551616");
        assert_eq!(run(&r.to_string())["result"], "invalid_request");
    }
    #[test]
    fn mode_defaults_come_from_core() {
        let mut r = request(graph());
        r["mode"] = json!("recursive");
        assert_eq!(run(&r.to_string())["record"]["controls"]["ncuts"], 4);
        r["mode"] = json!("kway");
        assert_eq!(run(&r.to_string())["record"]["controls"]["ncuts"], 1);
    }
}
