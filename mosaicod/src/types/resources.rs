use crate::{params, rw, traits};
use std::collections::HashMap;
use std::path;

pub struct ResourceId {
    pub id: i32,
    pub uuid: uuid::Uuid,
}

pub enum ResourceType {
    Sequence,
    Topic,
}

#[derive(Debug, Clone)]
pub struct TopicResourceLocator(String);

impl Resource for TopicResourceLocator {
    fn name(&self) -> &String {
        &self.0
    }

    fn resource_type(&self) -> ResourceType {
        ResourceType::Topic
    }
}

impl<T> From<T> for TopicResourceLocator
where
    T: AsRef<path::Path>,
{
    fn from(value: T) -> Self {
        Self(sanitize_name(&value.as_ref().to_string_lossy()))
    }
}

impl std::fmt::Display for TopicResourceLocator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[topic|{}]", self.0)
    }
}

impl From<TopicResourceLocator> for String {
    fn from(value: TopicResourceLocator) -> Self {
        value.0
    }
}

#[derive(Debug)]
pub struct TopicMetadata<M> {
    pub properties: TopicProperties,
    pub user_metadata: M,
}

impl<M> TopicMetadata<M> {
    pub fn new(props: TopicProperties, user_metadata: M) -> Self
    where
        M: super::MetadataBlob,
    {
        Self {
            properties: props,
            user_metadata,
        }
    }
}

/// Aggregated statistics for a topic's chunks.
#[derive(Debug, Clone, Default)]
pub struct TopicChunksStats {
    pub total_size_bytes: i64,
    pub total_row_count: i64,
}

/// Configuration properties defining the data semantic and encoding for a topic.
#[derive(Debug)]
pub struct TopicProperties {
    pub serialization_format: rw::Format,
    pub ontology_tag: String,
}

impl TopicProperties {
    pub fn new(serialization_format: rw::Format, ontology_tag: String) -> Self {
        Self {
            serialization_format,
            ontology_tag,
        }
    }
}

/// Represents system-level metadata and statistical information for a specific topic.
///
/// This struct provides a snapshot of the topic's physical state on disk, including
/// its size, structure, and lifecycle status.
pub struct TopicSystemInfo {
    /// Number of chunks in the topic
    pub chunks_number: usize,
    /// True is the topic is currently locked, a topic is locked if
    /// some data was uploaded and the connection was closed gracefully
    ///
    /// # Note
    /// (cabba) TODO: evaluate move this into a separate function since is not strictly related to system info
    pub is_locked: bool,
    /// Total size in bytes of the data.
    /// Metadata and other system files are excluded in the count.
    pub total_size_bytes: usize,
    /// Datetime of the topic creation
    pub created_datetime: super::DateTime,
}

#[derive(Debug, Clone)]
pub struct SequenceResourceLocator(String);

impl Resource for SequenceResourceLocator {
    fn name(&self) -> &String {
        &self.0
    }

    fn resource_type(&self) -> ResourceType {
        ResourceType::Sequence
    }
}

impl<T> From<T> for SequenceResourceLocator
where
    T: AsRef<path::Path>,
{
    fn from(value: T) -> Self {
        Self(sanitize_name(&value.as_ref().to_string_lossy()))
    }
}

impl std::fmt::Display for SequenceResourceLocator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[sequence|{}]", self.0)
    }
}

impl From<SequenceResourceLocator> for String {
    fn from(value: SequenceResourceLocator) -> String {
        value.0
    }
}

pub struct SequenceMetadata<M>
where
    M: super::MetadataBlob,
{
    pub user_metadata: M,
}

impl<M> SequenceMetadata<M>
where
    M: super::MetadataBlob,
{
    pub fn new(user_metadata: M) -> Self {
        Self { user_metadata }
    }
}

pub struct SequenceSystemInfo {
    /// Total size in bytes of the data.
    /// This values includes additional system files.
    pub total_size_bytes: usize,
    /// True is the sequence is locked, a sequence is locked if
    /// all its topics are locked and the `sequence_finalize` action
    /// was called.
    pub is_locked: bool,
    /// Datetime of the sequence creation
    pub created_datetime: super::DateTime,
}

#[derive(Debug)]
pub struct SequenceTopicGroup {
    pub sequence: SequenceResourceLocator,
    pub topics: Vec<TopicResourceLocator>,
}

impl SequenceTopicGroup {
    pub fn new(sequence: SequenceResourceLocator, topics: Vec<TopicResourceLocator>) -> Self {
        Self { sequence, topics }
    }

    pub fn into_parts(self) -> (SequenceResourceLocator, Vec<TopicResourceLocator>) {
        (self.sequence, self.topics)
    }
}

#[derive(Debug)]
pub struct SequenceTopicGroups(Vec<SequenceTopicGroup>);

impl SequenceTopicGroups {
    pub fn new(groups: Vec<SequenceTopicGroup>) -> Self {
        Self(groups)
    }

    pub fn empty() -> Self {
        Self(Vec::new())
    }

    /// Consumes the current group and a provided group to produce a new group in which
    /// the sequences are intersected while the topics are joined
    pub fn merge(self, group: Self) -> Self {
        let mut result = Vec::new();

        // We use an HashMap for O(1) lookup and avoid cloning.
        // We consume the second group, extracting topics keyed by sequence name.
        let mut group_map: HashMap<String, Vec<TopicResourceLocator>> = group
            .0
            .into_iter()
            .map(|g| {
                let (seq, topics) = g.into_parts();
                (seq.into(), topics)
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
}

impl Default for SequenceTopicGroups {
    fn default() -> Self {
        Self::empty()
    }
}

impl From<Vec<SequenceTopicGroup>> for SequenceTopicGroups {
    fn from(value: Vec<SequenceTopicGroup>) -> Self {
        Self::new(value)
    }
}

impl From<SequenceTopicGroups> for Vec<SequenceTopicGroup> {
    fn from(value: SequenceTopicGroups) -> Self {
        value.0
    }
}

pub trait Resource: std::fmt::Display + Send + Sync {
    fn name(&self) -> &String;

    fn resource_type(&self) -> ResourceType;

    /// Returns the location of the metadata file associated with the resource.
    ///
    /// The metadata file may or may not exists, no check if performed by this function.
    fn metadata(&self) -> path::PathBuf {
        let mut path = path::Path::new(self.name()).join("metadata");
        path.set_extension(params::ext::JSON);
        path
    }

    fn datafile(&self, chunk_number: usize, extension: &dyn traits::AsExtension) -> path::PathBuf {
        let filename = format!("data-{:05}", chunk_number);
        let mut path = path::Path::new(self.name()).join(filename);

        path.set_extension(extension.as_extension());

        path
    }

    fn is_sub_resource(&self, parent: &dyn Resource) -> bool {
        self.name().starts_with(parent.name())
    }
}

/// Returns a sanitized resource name by trimming whitespace and ensuring it does **not** start with a `/`.
///
/// This function is useful when normalizing resource paths or identifiers to ensure consistency
/// across the application by making them relative paths.
fn sanitize_name(name: &str) -> String {
    let trimmed = name.trim();
    trimmed.trim_start_matches('/').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_name() {
        let target = "my/resource/name";
        let san = sanitize_name("/my/resource/name");
        assert_eq!(san, target);

        let san = sanitize_name("    my/resource/name   ");
        assert_eq!(san, target);

        let san = sanitize_name("//my/resource/name");
        assert_eq!(san, target);
    }

    #[test]
    fn merge_sequence_topic_groups() {
        // Group 1: seq_a con topic1, seq_b con topic2
        let group1 = SequenceTopicGroups::new(vec![
            SequenceTopicGroup::new(
                SequenceResourceLocator::from("seq_a"),
                vec![TopicResourceLocator::from("topic1")],
            ),
            SequenceTopicGroup::new(
                SequenceResourceLocator::from("seq_b"),
                vec![TopicResourceLocator::from("topic2")],
            ),
        ]);

        // Group 2: seq_a con topic3 (match), seq_c con topic4 (no match)
        let group2 = SequenceTopicGroups::new(vec![
            SequenceTopicGroup::new(
                SequenceResourceLocator::from("seq_a"),
                vec![TopicResourceLocator::from("topic3")],
            ),
            SequenceTopicGroup::new(
                SequenceResourceLocator::from("seq_c"),
                vec![TopicResourceLocator::from("topic4")],
            ),
        ]);

        let merged: Vec<SequenceTopicGroup> = group1.merge(group2).into();

        // Solo seq_a dovrebbe sopravvivere (intersezione)
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].sequence.name(), "seq_a");
        // topic1 + topic3 merged
        assert_eq!(merged[0].topics.len(), 2);
    }
}
