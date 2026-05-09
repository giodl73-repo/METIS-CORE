use std::sync::Arc;

use metis_core::api::{MetisParams, MetisPartitioner, Partitioner};
use metis_core::graph::{CoarseMap, CsrGraph, Partition};

fn assert_send_sync<T: Send + Sync>() {}

fn path_graph(n: usize) -> CsrGraph {
    let mut xadj = Vec::with_capacity(n + 1);
    let mut adjncy = Vec::with_capacity(n.saturating_sub(1) * 2);
    xadj.push(0);
    for v in 0..n {
        if v > 0 {
            adjncy.push((v - 1) as u32);
        }
        if v + 1 < n {
            adjncy.push((v + 1) as u32);
        }
        xadj.push(adjncy.len() as u32);
    }

    CsrGraph::new(xadj, adjncy, 1, vec![1; n], None).expect("path graph is valid")
}

#[test]
fn public_partitioning_types_are_send_sync() {
    assert_send_sync::<CsrGraph>();
    assert_send_sync::<Partition>();
    assert_send_sync::<CoarseMap>();
    assert_send_sync::<MetisParams>();
    assert_send_sync::<MetisPartitioner>();
    assert_send_sync::<Box<dyn Partitioner>>();
}

#[test]
fn partitioner_can_be_shared_across_threads() {
    let graph = Arc::new(path_graph(96));
    let partitioner = Arc::new(MetisPartitioner::with_params(
        MetisParams {
            seed: Some(1234),
            ..MetisParams::default()
        },
        4,
    ));

    let expected = partitioner
        .split(&graph, 4, None)
        .expect("baseline partition")
        .into_assignment();

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let graph = Arc::clone(&graph);
            let partitioner = Arc::clone(&partitioner);
            std::thread::spawn(move || {
                partitioner
                    .split(&graph, 4, None)
                    .expect("threaded partition")
                    .into_assignment()
            })
        })
        .collect();

    for handle in handles {
        assert_eq!(handle.join().expect("thread should not panic"), expected);
    }
}
