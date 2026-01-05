//! Benchmark: SequenceTopicGroups::merge - HashMap O(n+m) vs Linear O(n*m)
//!
//! Run: cargo run --release --example merge_benchmark

use std::collections::HashMap;
use std::time::{Duration, Instant};

// Standalone replica of project types

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct TopicResourceLocator(String);

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Debug)]
struct SequenceResourceLocator(String);

impl SequenceResourceLocator {
    fn name(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug)]
struct SequenceTopicGroup {
    sequence: SequenceResourceLocator,
    topics: Vec<TopicResourceLocator>,
}

impl SequenceTopicGroup {
    fn new(sequence: SequenceResourceLocator, topics: Vec<TopicResourceLocator>) -> Self {
        Self { sequence, topics }
    }

    fn into_parts(self) -> (SequenceResourceLocator, Vec<TopicResourceLocator>) {
        (self.sequence, self.topics)
    }
}

#[derive(Clone)]
struct SequenceTopicGroups(Vec<SequenceTopicGroup>);

impl SequenceTopicGroups {
    fn new(groups: Vec<SequenceTopicGroup>) -> Self {
        Self(groups)
    }

    /// HashMap-based merge: O(n+m) time complexity
    fn merge_hashmap(self, group: Self) -> Self {
        let mut result = Vec::new();

        let mut group_map: HashMap<String, Vec<TopicResourceLocator>> = group
            .0
            .into_iter()
            .map(|g| {
                let (seq, topics) = g.into_parts();
                (seq.0, topics)
            })
            .collect();

        for mut grp1 in self.0 {
            if let Some(topics2) = group_map.remove(grp1.sequence.name()) {
                grp1.topics.extend(topics2);
                result.push(grp1);
            }
        }

        Self(result)
    }

    /// Linear search merge: O(n*m) time complexity
    fn merge_linear(self, mut group: Self) -> Self {
        // Set vector capacity to the maximum beween the two group to avoiding allocations
        let max_capacity = group.0.len().max(self.0.len());
        let mut result = Vec::with_capacity(max_capacity);

        group
            .0
            .sort_by(|a, b| a.sequence.name().cmp(b.sequence.name()));

        for mut self_grp in self.0 {
            let found = group
                .0
                .binary_search_by(|grp_aux| self_grp.sequence.name().cmp(grp_aux.sequence.name()));

            if let Ok(found) = found {
                self_grp.topics.extend(group.0[found].topics.clone());
                result.push(self_grp);
            }
        }

        Self(result)
    }
}

/// Generate test data with partial overlap between groups.
/// Topics use realistic hierarchical format: sequence_name/sensor/metric
fn generate_groups(
    num_sequences: usize,
    topics_per_seq: usize,
    overlap_ratio: f64,
) -> (SequenceTopicGroups, SequenceTopicGroups) {
    let overlap_count = (num_sequences as f64 * overlap_ratio) as usize;

    let make_seq_name = |i: usize| format!("project/dataset_{}/series_{}", i / 10, i % 10);

    let make_topics = |seq_name: &str, count: usize, suffix: &str| -> Vec<TopicResourceLocator> {
        (0..count)
            .map(|t| {
                TopicResourceLocator(format!(
                    "{}/sensor_{}/metric_{}{}",
                    seq_name,
                    t / 5,
                    t % 5,
                    suffix
                ))
            })
            .collect()
    };

    let group1 = SequenceTopicGroups::new(
        (0..num_sequences)
            .map(|i| {
                let seq_name = make_seq_name(i);
                SequenceTopicGroup::new(
                    SequenceResourceLocator(seq_name.clone()),
                    make_topics(&seq_name, topics_per_seq, "_a"),
                )
            })
            .collect(),
    );

    let group2 = SequenceTopicGroups::new(
        (0..num_sequences)
            .map(|i| {
                let seq_idx = if i < overlap_count {
                    i
                } else {
                    num_sequences + i
                };
                let seq_name = make_seq_name(seq_idx);
                SequenceTopicGroup::new(
                    SequenceResourceLocator(seq_name.clone()),
                    make_topics(&seq_name, topics_per_seq, "_b"),
                )
            })
            .collect(),
    );

    (group1, group2)
}

fn measure<F>(iterations: usize, warmup: usize, mut f: F) -> Duration
where
    F: FnMut(),
{
    for _ in 0..warmup {
        f();
    }

    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    start.elapsed() / iterations as u32
}

fn format_duration(d: Duration) -> String {
    let nanos = d.as_nanos();
    if nanos < 1_000 {
        format!("{} ns", nanos)
    } else if nanos < 1_000_000 {
        format!("{:.2} us", nanos as f64 / 1_000.0)
    } else if nanos < 1_000_000_000 {
        format!("{:.2} ms", nanos as f64 / 1_000_000.0)
    } else {
        format!("{:.2} s", nanos as f64 / 1_000_000_000.0)
    }
}

fn main() {
    println!("SequenceTopicGroups::merge benchmark");
    println!("HashMap O(n+m) vs Linear O(n*m)");
    println!();

    let scenarios = [
        (5, 10),
        (10, 20),
        (20, 50),
        (50, 100),
        (80, 500),
        (100, 1000),
    ];

    const ITERATIONS: usize = 2000;
    const WARMUP: usize = 200;
    const OVERLAP: f64 = 0.5;

    println!(
        "Config: {} iterations, {} warmup, {:.0}% overlap",
        ITERATIONS,
        WARMUP,
        OVERLAP * 100.0
    );
    println!();

    println!(
        "{:>8} {:>8} {:>14} {:>14} {:>12}",
        "seqs", "topics", "hashmap", "linear", "ratio"
    );
    println!("{}", "-".repeat(60));

    for (num_seq, topics_per_seq) in scenarios {
        let (g1, g2) = generate_groups(num_seq, topics_per_seq, OVERLAP);

        // Measure clone overhead
        let clone_time = measure(ITERATIONS, WARMUP, || {
            let _ = std::hint::black_box((g1.clone(), g2.clone()));
        });

        // Measure hashmap merge (includes clone)
        let hashmap_total = measure(ITERATIONS, WARMUP, || {
            let _ = std::hint::black_box(g1.clone().merge_hashmap(g2.clone()));
        });

        // Measure linear merge (includes clone)
        let linear_total = measure(ITERATIONS, WARMUP, || {
            let _ = std::hint::black_box(g1.clone().merge_linear(g2.clone()));
        });

        // Subtract clone overhead
        let hashmap_time = hashmap_total.saturating_sub(clone_time);
        let linear_time = linear_total.saturating_sub(clone_time);

        // Compute ratio: >1 means HashMap is faster
        let (ratio, winner) = if hashmap_time.as_nanos() == 0 || linear_time.as_nanos() == 0 {
            (1.0, "=")
        } else {
            let r = linear_time.as_nanos() as f64 / hashmap_time.as_nanos() as f64;
            let w = if r > 1.05 {
                "H"
            } else if r < 0.95 {
                "L"
            } else {
                "="
            };
            (r, w)
        };

        let ratio_str = format!("{:.2}x", ratio);

        println!(
            "{:>8} {:>8} {:>14} {:>14} {:>10} {}",
            num_seq,
            topics_per_seq,
            format_duration(hashmap_time),
            format_duration(linear_time),
            ratio_str,
            winner
        );
    }

    println!();
    println!("H = HashMap faster, L = Linear faster, = = within 5%");
    println!("Clone overhead subtracted from measurements");
}
