use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use metis_core::api::{MetisParams, MetisPartitioner, Partitioner};
use metis_core::graph::CsrGraph;

fn find_gpmetis() -> Option<PathBuf> {
    for key in ["METIS_GPMETIS", "METIS_CORE_GPMETIS"] {
        if let Ok(path) = std::env::var(key) {
            let path = PathBuf::from(path);
            if path.is_file() {
                return Some(path);
            }
        }
    }

    for path in [
        r"C:\src\metis\build\programs\gpmetis.exe",
        r"C:\src\metis\build\Windows\programs\Release\gpmetis.exe",
        r"C:\src\metis\build\programs\Release\gpmetis.exe",
        r"C:\src\apportionment\bin\gpmetis.exe",
    ] {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    if Command::new("gpmetis").arg("-help").output().is_ok() {
        return Some(PathBuf::from("gpmetis"));
    }

    None
}

fn grid_graph(width: usize, height: usize) -> CsrGraph {
    let n = width * height;
    let mut xadj = Vec::with_capacity(n + 1);
    let mut adjncy = Vec::new();
    xadj.push(0);

    for y in 0..height {
        for x in 0..width {
            let mut push = |nx: usize, ny: usize| {
                adjncy.push((ny * width + nx) as u32);
            };
            if x > 0 {
                push(x - 1, y);
            }
            if x + 1 < width {
                push(x + 1, y);
            }
            if y > 0 {
                push(x, y - 1);
            }
            if y + 1 < height {
                push(x, y + 1);
            }
            xadj.push(adjncy.len() as u32);
        }
    }

    CsrGraph {
        xadj,
        adjncy,
        ncon: 1,
        vwgt: vec![1; n],
        adjwgt: None,
    }
}

fn write_metis_graph(g: &CsrGraph, path: &Path) {
    let mut out = String::new();
    out.push_str(&format!("{} {}\n", g.n(), g.adjncy.len() / 2));
    for v in 0..g.n() {
        for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
            if j > g.xadj[v] as usize {
                out.push(' ');
            }
            out.push_str(&(g.adjncy[j] + 1).to_string());
        }
        out.push('\n');
    }
    std::fs::write(path, out).expect("write METIS graph file");
}

fn run_gpmetis(gpmetis: &Path, graph_path: &Path, k: u32, seed: u32) -> Vec<u32> {
    let output = Command::new(gpmetis)
        .arg("-ptype=kway")
        .arg("-ctype=shem")
        .arg("-objtype=cut")
        .arg("-ncuts=1")
        .arg("-niter=10")
        .arg("-ufactor=30")
        .arg(format!("-seed={seed}"))
        .arg(graph_path)
        .arg(k.to_string())
        .output()
        .expect("run gpmetis");

    assert!(
        output.status.success(),
        "gpmetis failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let part_path = graph_path.with_extension(format!("graph.part.{k}"));
    std::fs::read_to_string(&part_path)
        .expect("read gpmetis partition output")
        .lines()
        .map(|line| line.trim().parse::<u32>().expect("part id"))
        .collect()
}

fn edge_cut(g: &CsrGraph, assignment: &[u32]) -> u64 {
    let mut cut = 0u64;
    for v in 0..g.n() {
        for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
            let u = g.adjncy[j] as usize;
            if assignment[v] != assignment[u] {
                cut += 1;
            }
        }
    }
    cut / 2
}

fn max_imbalance(assignment: &[u32], k: u32) -> f64 {
    let target = assignment.len() as f64 / k as f64;
    let mut counts = vec![0usize; k as usize];
    for &part in assignment {
        counts[part as usize] += 1;
    }
    *counts.iter().max().unwrap_or(&0) as f64 / target
}

fn assert_basic_partition(g: &CsrGraph, assignment: &[u32], k: u32, label: &str) {
    assert_eq!(assignment.len(), g.n(), "{label}: assignment length");
    assert!(
        assignment.iter().all(|&part| part < k),
        "{label}: part id out of range"
    );
    for part in 0..k {
        assert!(
            assignment.contains(&part),
            "{label}: part {part} should be occupied"
        );
    }
}

#[test]
fn gpmetis_grid_quality_envelope() {
    let Some(gpmetis) = find_gpmetis() else {
        eprintln!("Skipping gpmetis_grid_quality_envelope: gpmetis not found");
        return;
    };

    let g = grid_graph(24, 24);
    assert!(g.is_valid());

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let dir =
        std::env::temp_dir().join(format!("metis-core-parity-{}-{stamp}", std::process::id()));
    std::fs::create_dir(&dir).expect("create parity temp dir");
    let graph_path = dir.join("grid.graph");
    write_metis_graph(&g, &graph_path);

    let k = 8;
    let seed = 42;
    let c_assignment = run_gpmetis(&gpmetis, &graph_path, k, seed);
    let params = MetisParams {
        seed: Some(seed as u64),
        ufactor: 30,
        ..MetisParams::default()
    };
    let rust_assignment = MetisPartitioner::with_params(params, k)
        .split(&g, k, None)
        .expect("rust partition")
        .assignment;

    assert_basic_partition(&g, &c_assignment, k, "gpmetis");
    assert_basic_partition(&g, &rust_assignment, k, "rust");

    let c_cut = edge_cut(&g, &c_assignment);
    let rust_cut = edge_cut(&g, &rust_assignment);
    let c_imbalance = max_imbalance(&c_assignment, k);
    let rust_imbalance = max_imbalance(&rust_assignment, k);

    eprintln!(
        "gpmetis grid k={k}: cut={c_cut}, imbalance={c_imbalance:.3}; \
         rust cut={rust_cut}, imbalance={rust_imbalance:.3}"
    );

    assert!(
        rust_imbalance <= c_imbalance.max(1.03) + 0.20,
        "rust imbalance {rust_imbalance:.3} should stay near gpmetis {c_imbalance:.3}"
    );
    assert!(
        rust_cut <= c_cut.saturating_mul(2).saturating_add(k as u64 * 4),
        "rust cut {rust_cut} should stay within the gpmetis quality envelope {c_cut}"
    );

    let _ = std::fs::remove_file(graph_path.with_extension(format!("graph.part.{k}")));
    let _ = std::fs::remove_file(graph_path);
    let _ = std::fs::remove_dir(dir);
}
