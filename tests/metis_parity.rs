use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use metis_core::{CsrGraph, MetisParams, MetisPartitioner, Partitioner};

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

    CsrGraph::new(xadj, adjncy, 1, vec![1; n], None).expect("grid graph is valid")
}

fn find_graph_fixture(name: &str) -> Option<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for dir in [
        manifest.join("../metis/graphs"),
        PathBuf::from(r"C:\src\metis\graphs"),
    ] {
        let path = dir.join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn heavy_parity_enabled() -> bool {
    std::env::var("METIS_CORE_HEAVY_PARITY").is_ok_and(|value| value == "1")
}

fn load_metis_graph(path: &Path) -> Option<CsrGraph> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut lines = content.lines().filter(|line| !line.starts_with('%'));
    let header_line = lines.next().expect("graph file has no header");
    let header: Vec<u64> = header_line
        .split_whitespace()
        .filter_map(|token| token.parse().ok())
        .collect();
    assert!(header.len() >= 2, "invalid graph header: {header_line}");

    let n = header[0] as usize;
    let fmt = header.get(2).copied().unwrap_or(0);
    let ncon = header.get(3).copied().unwrap_or(1).max(1) as usize;
    let has_vwgt = (fmt / 10) % 10 == 1;
    let has_ewgt = fmt % 10 == 1;

    let mut xadj = Vec::with_capacity(n + 1);
    let mut adjncy = Vec::new();
    let mut adjwgt = Vec::new();
    let mut vwgt = Vec::new();
    xadj.push(0);

    for line in lines.take(n) {
        let tokens: Vec<u64> = line
            .split_whitespace()
            .filter_map(|token| token.parse().ok())
            .collect();
        let mut i = 0;

        if has_vwgt {
            for _ in 0..ncon {
                vwgt.push(*tokens.get(i).unwrap_or(&1) as i32);
                i += 1;
            }
        }

        while i < tokens.len() {
            adjncy.push((tokens[i] - 1) as u32);
            i += 1;
            if has_ewgt {
                adjwgt.push(*tokens.get(i).unwrap_or(&1) as i32);
                i += 1;
            }
        }
        xadj.push(adjncy.len() as u32);
    }

    let vwgt = if vwgt.is_empty() {
        vec![1; n]
    } else if ncon > 1 {
        vwgt.chunks(ncon).map(|weights| weights[0].max(1)).collect()
    } else {
        vwgt.into_iter().map(|weight| weight.max(1)).collect()
    };

    let adjwgt = if adjwgt.is_empty() {
        None
    } else {
        Some(adjwgt)
    };

    CsrGraph::new(xadj, adjncy, 1, vwgt, adjwgt).ok()
}

fn parity_temp_dir(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "metis-core-parity-{label}-{}-{stamp}",
        std::process::id()
    ));
    std::fs::create_dir(&dir).expect("create parity temp dir");
    dir
}

fn write_metis_graph(g: &CsrGraph, path: &Path) {
    let mut out = String::new();
    out.push_str(&format!("{} {}\n", g.n(), g.adjncy().len() / 2));
    for v in 0..g.n() {
        for j in g.xadj()[v] as usize..g.xadj()[v + 1] as usize {
            if j > g.xadj()[v] as usize {
                out.push(' ');
            }
            out.push_str(&(g.adjncy()[j] + 1).to_string());
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

    let part_path = gpmetis_part_path(graph_path, k);
    std::fs::read_to_string(&part_path)
        .expect("read gpmetis partition output")
        .lines()
        .map(|line| line.trim().parse::<u32>().expect("part id"))
        .collect()
}

fn gpmetis_part_path(graph_path: &Path, k: u32) -> PathBuf {
    let mut path = graph_path.as_os_str().to_os_string();
    path.push(format!(".part.{k}"));
    PathBuf::from(path)
}

fn edge_cut(g: &CsrGraph, assignment: &[u32]) -> u64 {
    let mut cut = 0u64;
    for v in 0..g.n() {
        for j in g.xadj()[v] as usize..g.xadj()[v + 1] as usize {
            let u = g.adjncy()[j] as usize;
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

fn max_weight_imbalance(g: &CsrGraph, assignment: &[u32], k: u32) -> f64 {
    let total: i64 = g.vwgt().iter().map(|&weight| weight as i64).sum();
    let target = total as f64 / k as f64;
    let mut counts = vec![0i64; k as usize];
    for (v, &part) in assignment.iter().enumerate() {
        counts[part as usize] += g.vwgt()[v] as i64;
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

    let dir = parity_temp_dir("grid");
    let graph_path = dir.join("grid.graph");
    write_metis_graph(&g, &graph_path);

    let k = 8;
    let seed = 42;
    let c_assignment = run_gpmetis(&gpmetis, &graph_path, k, seed);
    let params = MetisParams::default()
        .with_seed(seed as u64)
        .with_ufactor(30);
    let rust_assignment = MetisPartitioner::with_params(params, k)
        .split(&g, k, None)
        .expect("rust partition")
        .into_assignment();

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

    let _ = std::fs::remove_file(gpmetis_part_path(&graph_path, k));
    let _ = std::fs::remove_file(graph_path);
    let _ = std::fs::remove_dir(dir);
}

#[test]
fn gpmetis_4elt_quality_envelope() {
    let Some(gpmetis) = find_gpmetis() else {
        eprintln!("Skipping gpmetis_4elt_quality_envelope: gpmetis not found");
        return;
    };
    let Some(source_graph_path) = find_graph_fixture("4elt.graph") else {
        eprintln!("Skipping gpmetis_4elt_quality_envelope: 4elt.graph not found");
        return;
    };

    let g = load_metis_graph(&source_graph_path).expect("load 4elt.graph");
    assert_eq!(g.n(), 15_606);
    assert!(g.is_valid());

    let dir = parity_temp_dir("4elt");
    let graph_path = dir.join("4elt.graph");
    std::fs::copy(&source_graph_path, &graph_path).expect("copy 4elt.graph");

    let k = 8;
    let seed = 42;
    let c_assignment = run_gpmetis(&gpmetis, &graph_path, k, seed);
    let params = MetisParams::default()
        .with_seed(seed as u64)
        .with_ufactor(30);
    let rust_assignment = MetisPartitioner::with_params(params, k)
        .split(&g, k, None)
        .expect("rust 4elt partition")
        .into_assignment();

    assert_basic_partition(&g, &c_assignment, k, "gpmetis 4elt");
    assert_basic_partition(&g, &rust_assignment, k, "rust 4elt");

    let c_cut = edge_cut(&g, &c_assignment);
    let rust_cut = edge_cut(&g, &rust_assignment);
    let c_imbalance = max_imbalance(&c_assignment, k);
    let rust_imbalance = max_imbalance(&rust_assignment, k);

    eprintln!(
        "gpmetis 4elt k={k}: cut={c_cut}, imbalance={c_imbalance:.3}; \
         rust cut={rust_cut}, imbalance={rust_imbalance:.3}"
    );

    assert!(
        rust_imbalance <= c_imbalance.max(1.03) + 0.20,
        "rust 4elt imbalance {rust_imbalance:.3} should stay near gpmetis {c_imbalance:.3}"
    );
    assert!(
        rust_cut <= c_cut.saturating_mul(3).saturating_add(k as u64 * 16),
        "rust 4elt cut {rust_cut} should stay within the gpmetis envelope {c_cut}"
    );

    let _ = std::fs::remove_file(gpmetis_part_path(&graph_path, k));
    let _ = std::fs::remove_file(graph_path);
    let _ = std::fs::remove_dir(dir);
}

#[test]
fn gpmetis_test_mgraph_structural_reference() {
    let Some(gpmetis) = find_gpmetis() else {
        eprintln!("Skipping gpmetis_test_mgraph_structural_reference: gpmetis not found");
        return;
    };
    let Some(source_graph_path) = find_graph_fixture("test.mgraph") else {
        eprintln!("Skipping gpmetis_test_mgraph_structural_reference: test.mgraph not found");
        return;
    };

    let g = load_metis_graph(&source_graph_path).expect("load test.mgraph");
    assert_eq!(g.n(), 766);
    assert!(g.is_valid());

    let dir = parity_temp_dir("test-mgraph");
    let graph_path = dir.join("test.mgraph");
    std::fs::copy(&source_graph_path, &graph_path).expect("copy test.mgraph");

    let k = 4;
    let seed = 42;
    let c_assignment = run_gpmetis(&gpmetis, &graph_path, k, seed);
    let params = MetisParams::default()
        .with_seed(seed as u64)
        .with_ufactor(30);
    let rust_assignment = MetisPartitioner::with_params(params, k)
        .split(&g, k, None)
        .expect("rust test.mgraph partition")
        .into_assignment();

    assert_basic_partition(&g, &c_assignment, k, "gpmetis test.mgraph");
    assert_basic_partition(&g, &rust_assignment, k, "rust test.mgraph");

    let c_cut = edge_cut(&g, &c_assignment);
    let rust_cut = edge_cut(&g, &rust_assignment);
    let c_count_imbalance = max_imbalance(&c_assignment, k);
    let rust_count_imbalance = max_imbalance(&rust_assignment, k);
    let c_weight_imbalance = max_weight_imbalance(&g, &c_assignment, k);
    let rust_weight_imbalance = max_weight_imbalance(&g, &rust_assignment, k);

    eprintln!(
        "gpmetis test.mgraph k={k}: cut={c_cut}, count_imbalance={c_count_imbalance:.3}, \
         primary_weight_imbalance={c_weight_imbalance:.3}; rust cut={rust_cut}, \
         count_imbalance={rust_count_imbalance:.3}, \
         primary_weight_imbalance={rust_weight_imbalance:.3}"
    );

    assert!(
        rust_weight_imbalance <= c_weight_imbalance.max(1.03) + 0.35,
        "rust test.mgraph primary-weight imbalance {rust_weight_imbalance:.3} \
         should stay near gpmetis {c_weight_imbalance:.3}"
    );
    assert!(
        rust_cut <= c_cut.saturating_mul(4).saturating_add(k as u64 * 16),
        "rust test.mgraph cut {rust_cut} should stay within the gpmetis envelope {c_cut}"
    );

    let _ = std::fs::remove_file(gpmetis_part_path(&graph_path, k));
    let _ = std::fs::remove_file(graph_path);
    let _ = std::fs::remove_dir(dir);
}

#[test]
fn gpmetis_copter2_quality_smoke() {
    if !heavy_parity_enabled() {
        eprintln!("Skipping gpmetis_copter2_quality_smoke: set METIS_CORE_HEAVY_PARITY=1 to run");
        return;
    }

    let Some(gpmetis) = find_gpmetis() else {
        eprintln!("Skipping gpmetis_copter2_quality_smoke: gpmetis not found");
        return;
    };
    let Some(source_graph_path) = find_graph_fixture("copter2.graph") else {
        eprintln!("Skipping gpmetis_copter2_quality_smoke: copter2.graph not found");
        return;
    };

    let g = load_metis_graph(&source_graph_path).expect("load copter2.graph");
    assert_eq!(g.n(), 55_476);
    assert!(g.is_valid());

    let dir = parity_temp_dir("copter2");
    let graph_path = dir.join("copter2.graph");
    std::fs::copy(&source_graph_path, &graph_path).expect("copy copter2.graph");

    let k = 8;
    let seed = 42;
    let c_assignment = run_gpmetis(&gpmetis, &graph_path, k, seed);
    let params = MetisParams::default()
        .with_seed(seed as u64)
        .with_ufactor(30);
    let rust_assignment = MetisPartitioner::with_params(params, k)
        .split(&g, k, None)
        .expect("rust copter2 partition")
        .into_assignment();

    assert_basic_partition(&g, &c_assignment, k, "gpmetis copter2");
    assert_basic_partition(&g, &rust_assignment, k, "rust copter2");

    let c_cut = edge_cut(&g, &c_assignment);
    let rust_cut = edge_cut(&g, &rust_assignment);
    let c_imbalance = max_imbalance(&c_assignment, k);
    let rust_imbalance = max_imbalance(&rust_assignment, k);

    eprintln!(
        "gpmetis copter2 k={k}: cut={c_cut}, imbalance={c_imbalance:.3}; \
         rust cut={rust_cut}, imbalance={rust_imbalance:.3}"
    );

    assert!(
        rust_imbalance <= c_imbalance.max(1.03) + 0.20,
        "rust copter2 imbalance {rust_imbalance:.3} should stay near gpmetis {c_imbalance:.3}"
    );
    assert!(
        rust_cut <= c_cut.saturating_mul(4).saturating_add(k as u64 * 32),
        "rust copter2 cut {rust_cut} should stay within the gpmetis envelope {c_cut}"
    );

    let _ = std::fs::remove_file(gpmetis_part_path(&graph_path, k));
    let _ = std::fs::remove_file(graph_path);
    let _ = std::fs::remove_dir(dir);
}
