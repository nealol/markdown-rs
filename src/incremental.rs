//! Incremental document parsing and block-level HTML patches.

use crate::mdast::{AttributeContent, AttributeValue, Node, Root};
use crate::{code_hike_blocks, message, parser, to_mdast, Location, Options, ParseOptions};
use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    format,
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};
use core::ops::{Deref, DerefMut};

/// A half-open byte range in the current UTF-8 document.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ByteRange {
    /// First included byte.
    pub start: usize,
    /// First excluded byte.
    pub end: usize,
}

/// One replacement against the document at `EditBatch::base_version`.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Edit {
    /// First byte replaced.
    pub start_byte: usize,
    /// First byte not replaced.
    pub old_end_byte: usize,
    /// UTF-8 replacement text.
    pub replacement: String,
}

/// Atomic edits. Ranges are measured against the same base document.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct EditBatch {
    /// Version to which the edits apply.
    pub base_version: u64,
    /// Sorted, nonoverlapping replacements.
    pub edits: Vec<Edit>,
}

impl EditBatch {
    /// Validate and apply this batch to a source document.
    ///
    /// This is the shared source-edit contract used by incremental consumers
    /// built on top of markdown-rs.
    ///
    /// # Errors
    ///
    /// Returns an error for stale versions, invalid UTF-8 boundaries,
    /// out-of-bounds ranges, or overlapping edits.
    pub fn apply_to(&self, source: &str, version: u64) -> Result<String, message::Message> {
        validate_edits(source, version, self)?;
        Ok(apply_edits(source, &self.edits))
    }
}

/// One block-level DOM/HTML operation.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "op", rename_all = "snake_case"))]
pub enum Patch {
    /// Replace an existing block.
    Replace {
        /// Stable block identity.
        id: String,
        /// Complete replacement HTML.
        html: String,
    },
    /// Insert a block after another block, or at the start when `after` is `None`.
    InsertAfter {
        /// Previous visible block.
        after: Option<String>,
        /// Stable block identity.
        id: String,
        /// Complete block HTML.
        html: String,
    },
    /// Remove an existing block.
    Remove {
        /// Stable block identity.
        id: String,
    },
}

/// Public snapshot of a rendered block.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct RenderedBlock {
    /// Stable identity, preserved when the block is reused.
    pub id: String,
    /// Source range.
    pub range: ByteRange,
    /// HTML for this block.
    pub html: String,
}

/// Parser state recorded at a safe top-level block boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct BlockCheckpoint {
    /// Source byte offset.
    pub byte_offset: usize,
    /// Index of the following block.
    pub block_index: usize,
    /// Hash of definitions visible before this point.
    pub reference_environment_hash: u64,
}

/// Result of applying one edit batch.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ApplyResult {
    /// New renderer version.
    pub version: u64,
    /// Ordered block operations.
    pub patches: Vec<Patch>,
    /// Region actually reparsed to establish structural convergence.
    pub reparsed: ByteRange,
}

/// Result of applying edits to an incremental syntax tree.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct AstApplyResult {
    /// New parser version.
    pub version: u64,
    /// Stable identities of blocks whose syntax trees changed.
    pub changed: Vec<String>,
    /// Stable identities of blocks whose cached syntax trees were reused.
    pub reused: Vec<String>,
    /// Region actually reparsed to establish structural convergence.
    pub reparsed: ByteRange,
}

/// Compact parser result for consumers that validate reuse from a snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct AstApplySummary {
    /// New parser version.
    pub version: u64,
    /// Stable identities of blocks whose syntax trees changed.
    pub changed: Vec<String>,
    /// Number of cached syntax trees reused.
    pub reused_count: usize,
    /// Region actually reparsed to establish structural convergence.
    pub reparsed: ByteRange,
}

/// Borrowed metadata for one cached top-level syntax tree.
#[derive(Clone, Copy)]
pub struct AstBlock<'a> {
    block: &'a Block,
}

impl AstBlock<'_> {
    /// Stable identity.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.block.id
    }

    /// Source range in the snapshot.
    #[must_use]
    pub fn range(&self) -> ByteRange {
        ByteRange {
            start: self.block.start,
            end: self.block.end,
        }
    }

    /// Content and syntax fingerprint used for cache validation.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        self.block.fingerprint
    }

    /// Clone and rebase this block only when a downstream cache misses.
    #[must_use]
    pub fn node(&self, location: &Location) -> Node {
        rebase_block_node(self.block, location)
    }
}

/// Read-only view of a prospective incremental syntax tree.
#[derive(Clone, Copy)]
pub struct AstSnapshot<'a> {
    source: &'a str,
    blocks: &'a [Block],
}

impl AstSnapshot<'_> {
    /// Snapshot source.
    #[must_use]
    pub fn source(&self) -> &str {
        self.source
    }

    /// Assemble the snapshot root.
    #[must_use]
    pub fn mdast(&self) -> Node {
        mdast_from_blocks(self.source, self.blocks)
    }

    /// Cheap borrowed metadata for cached top-level trees.
    pub fn blocks(&self) -> impl Iterator<Item = AstBlock<'_>> {
        self.blocks.iter().map(|block| AstBlock { block })
    }

    /// Number of cached top-level trees.
    #[must_use]
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// Whether HAST conversion requires document-global state.
    #[must_use]
    pub fn has_global_dependencies(&self) -> bool {
        self.blocks.iter().any(|block| {
            !block.definitions.is_empty() || !block.references.is_empty() || block.footnote
        })
    }

    /// Root position without assembling or cloning child trees.
    #[must_use]
    pub fn root_position(&self) -> crate::unist::Position {
        let location = Location::new(self.source.as_bytes());
        crate::unist::Position {
            start: location.to_point(0).expect("document start exists"),
            end: location
                .to_point(self.source.len())
                .expect("document end exists"),
        }
    }
}

#[derive(Clone, Debug)]
struct Block {
    start: usize,
    end: usize,
    data: Arc<BlockData>,
}

#[derive(Clone, Debug)]
struct BlockData {
    id: String,
    fingerprint: u64,
    kind: u8,
    html: String,
    definitions: Vec<(String, u64)>,
    references: Vec<String>,
    footnote: bool,
    node: Arc<Node>,
}

impl Deref for Block {
    type Target = BlockData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for Block {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.data)
    }
}

/// Persistent incremental Markdown/MDX syntax tree.
pub struct AstSession {
    source: String,
    options: ParseOptions,
    version: u64,
    blocks: Vec<Block>,
    next_id: u64,
}

impl AstSession {
    /// Parse an initial document into cached top-level syntax trees.
    ///
    /// # Errors
    ///
    /// Returns the same syntax errors as [`crate::to_mdast`].
    pub fn open(value: &str, options: ParseOptions) -> Result<Self, message::Message> {
        let mut next_id = 1;
        let blocks = parse_blocks(value, 0, &options, &[], &mut next_id)?;
        Ok(Self {
            source: value.to_string(),
            options,
            version: 0,
            blocks,
            next_id,
        })
    }

    /// Current document version.
    #[must_use]
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Current source.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Stable identities of cached top-level syntax trees.
    pub fn block_ids(&self) -> impl Iterator<Item = &str> {
        self.blocks.iter().map(|block| block.id.as_str())
    }

    /// Assemble the current root while retaining cached block trees.
    #[must_use]
    pub fn mdast(&self) -> Node {
        mdast_from_blocks(&self.source, &self.blocks)
    }

    /// Read-only view of the current cached blocks.
    #[must_use]
    pub fn snapshot(&self) -> AstSnapshot<'_> {
        AstSnapshot {
            source: &self.source,
            blocks: &self.blocks,
        }
    }

    /// Apply an atomic source edit batch and update affected block trees.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid edit batch or invalid syntax. The
    /// session remains unchanged on error.
    pub fn apply(&mut self, batch: EditBatch) -> Result<AstApplyResult, message::Message> {
        self.apply_with(batch, |_| Ok(()))
            .map(|(result, ())| result)
    }

    /// Apply edits and atomically transform the prospective syntax tree.
    ///
    /// The session is committed only when both parsing and `transform`
    /// succeed. This lets downstream compilers retain the parser transaction.
    ///
    /// # Errors
    ///
    /// Returns a parser error or an error returned by `transform`.
    pub fn apply_with<T, F>(
        &mut self,
        batch: EditBatch,
        transform: F,
    ) -> Result<(AstApplyResult, T), message::Message>
    where
        F: FnOnce(AstSnapshot<'_>) -> Result<T, message::Message>,
    {
        self.apply_with_reporting(batch, true, transform)
    }

    /// Apply edits and atomically transform without collecting reused block IDs.
    ///
    /// Downstream caches that validate [`AstSnapshot`] blocks directly can
    /// avoid allocating an ID for every unchanged block.
    pub fn apply_with_changed_only<T, F>(
        &mut self,
        batch: EditBatch,
        transform: F,
    ) -> Result<(AstApplySummary, T), message::Message>
    where
        F: FnOnce(AstSnapshot<'_>) -> Result<T, message::Message>,
    {
        self.apply_with_reporting(batch, false, transform)
            .map(|(result, transformed)| {
                (
                    AstApplySummary {
                        version: result.version,
                        reused_count: self.blocks.len() - result.changed.len(),
                        changed: result.changed,
                        reparsed: result.reparsed,
                    },
                    transformed,
                )
            })
    }

    fn apply_with_reporting<T, F>(
        &mut self,
        batch: EditBatch,
        report_reused: bool,
        transform: F,
    ) -> Result<(AstApplyResult, T), message::Message>
    where
        F: FnOnce(AstSnapshot<'_>) -> Result<T, message::Message>,
    {
        let new_source = batch.apply_to(&self.source, self.version)?;
        let edits = batch.edits;
        let earliest = edits.first().map_or(0, |edit| edit.start_byte);
        let latest_old = edits.last().map_or(0, |edit| edit.old_end_byte);
        let delta = byte_delta(&edits);
        let start_index = checkpoint_before(&self.blocks, earliest);
        let start = self.blocks.get(start_index).map_or(0, |block| block.start);
        let inherited = definitions_before(&self.blocks, start_index);
        let old_definition_ids = definition_ids(&self.blocks);
        let mut next_id = self.next_id;
        let reparse = Reparse {
            source: &new_source,
            options: &self.options,
            old: &self.blocks,
            start_index,
            start,
            latest_old,
            total_delta: delta,
            old_definition_ids: &old_definition_ids,
            inherited: &inherited,
        };
        let (mut parsed, reparsed_end, convergence_old) =
            parse_until_convergence(&reparse, &mut next_id)?;
        let mut blocks = self.blocks[..start_index].to_vec();
        align_ids(&self.blocks[start_index..convergence_old], &mut parsed);
        blocks.append(&mut parsed);

        if convergence_old < self.blocks.len() {
            let shift = delta_at_old_offset(&edits, self.blocks[convergence_old].start);
            for old in &self.blocks[convergence_old..] {
                let mut reused = old.clone();
                reused.start = add_delta(reused.start, shift);
                reused.end = add_delta(reused.end, shift);
                blocks.push(reused);
            }
        }

        if definition_ids(&blocks) != old_definition_ids {
            blocks = parse_blocks(&new_source, 0, &self.options, &[], &mut next_id)?;
            align_ids(&self.blocks, &mut blocks);
        }

        let changed_set = changed_block_ids(&self.blocks, &blocks);
        let changed = blocks
            .iter()
            .filter(|block| changed_set.contains(&block.id))
            .map(|block| block.id.clone())
            .collect();
        let reused = if report_reused {
            blocks
                .iter()
                .filter(|block| !changed_set.contains(&block.id))
                .map(|block| block.id.clone())
                .collect()
        } else {
            vec![]
        };
        let transformed = transform(AstSnapshot {
            source: &new_source,
            blocks: &blocks,
        })?;

        self.source = new_source;
        self.version += 1;
        self.blocks = blocks;
        self.next_id = next_id;

        Ok((
            AstApplyResult {
                version: self.version,
                changed,
                reused,
                reparsed: ByteRange {
                    start,
                    end: reparsed_end,
                },
            },
            transformed,
        ))
    }
}

/// Persistent incremental Markdown renderer.
pub struct Renderer {
    source: String,
    options: Options,
    version: u64,
    blocks: Vec<Block>,
    checkpoints: Vec<BlockCheckpoint>,
    next_id: u64,
    html: String,
    whole_document: bool,
}

impl Renderer {
    /// Parse an initial document and establish stable block identities.
    ///
    /// # Errors
    ///
    /// Returns the same MDX syntax errors as the normal parser.
    pub fn open(value: &str, options: Options) -> Result<Self, message::Message> {
        let mut next_id = 1;
        let mut blocks = parse_blocks(value, 0, &options.parse, &[], &mut next_id)?;
        render_all(value, &options, &mut blocks)?;
        let canonical = crate::to_html_with_options(value, &options)?;
        let segmented = join_html(&blocks);
        let whole_document = blocks.iter().any(|block| block.footnote) || segmented != canonical;

        if whole_document {
            blocks = vec![document_block(value, canonical, &mut next_id)];
        }

        let checkpoints = make_checkpoints(&blocks);
        let html = join_html(&blocks);

        Ok(Self {
            source: value.to_string(),
            options,
            version: 0,
            blocks,
            checkpoints,
            next_id,
            html,
            whole_document,
        })
    }

    /// Current document version.
    #[must_use]
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Current Markdown source.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Current complete HTML.
    #[must_use]
    pub fn html(&self) -> &str {
        &self.html
    }

    /// Current parser checkpoints.
    #[must_use]
    pub fn checkpoints(&self) -> &[BlockCheckpoint] {
        &self.checkpoints
    }

    /// Visible rendered blocks in document order.
    pub fn blocks(&self) -> impl Iterator<Item = RenderedBlock> + '_ {
        self.blocks
            .iter()
            .filter(|block| !block.html.is_empty())
            .map(|block| RenderedBlock {
                id: block.id.clone(),
                range: ByteRange {
                    start: block.start,
                    end: block.end,
                },
                html: block.html.clone(),
            })
    }

    /// Apply an atomic edit batch and return concise block patches.
    ///
    /// # Errors
    ///
    /// Returns an error for stale versions, invalid UTF-8 boundaries,
    /// overlapping edits, or MDX syntax errors. The session is unchanged on
    /// error.
    pub fn apply(&mut self, batch: EditBatch) -> Result<ApplyResult, message::Message> {
        let new_source = batch.apply_to(&self.source, self.version)?;
        let edits = batch.edits;
        let old_visible = visible(&self.blocks);
        let old_definitions = definition_map(&self.blocks);
        let earliest = edits.first().map_or(0, |edit| edit.start_byte);
        let latest_old = edits.last().map_or(0, |edit| edit.old_end_byte);
        let delta = byte_delta(&edits);

        let start_index = checkpoint_before(&self.blocks, earliest);
        let start = self.blocks.get(start_index).map_or(0, |block| block.start);
        let inherited = definitions_before(&self.blocks, start_index);
        let old_definition_ids = definition_ids(&self.blocks);
        let mut next_id = self.next_id;
        let reparse = Reparse {
            source: &new_source,
            options: &self.options.parse,
            old: &self.blocks,
            start_index,
            start,
            latest_old,
            total_delta: delta,
            old_definition_ids: &old_definition_ids,
            inherited: &inherited,
        };
        let (mut parsed, reparsed_end, convergence_old) =
            parse_until_convergence(&reparse, &mut next_id)?;

        let mut blocks = self.blocks[..start_index].to_vec();
        align_ids(&self.blocks[start_index..convergence_old], &mut parsed);
        blocks.append(&mut parsed);

        if convergence_old < self.blocks.len() {
            let shift = delta_at_old_offset(&edits, self.blocks[convergence_old].start);
            for old in &self.blocks[convergence_old..] {
                let mut reused = old.clone();
                reused.start = add_delta(reused.start, shift);
                reused.end = add_delta(reused.end, shift);
                blocks.push(reused);
            }
        }

        let new_definition_ids = definition_ids(&blocks);
        if new_definition_ids != old_definition_ids {
            blocks = parse_blocks(&new_source, 0, &self.options.parse, &[], &mut next_id)?;
            align_ids(&self.blocks, &mut blocks);
        }

        let new_definitions = definition_map(&blocks);
        let changed_definitions = changed_definition_ids(&old_definitions, &new_definitions);
        let changed_ids = changed_block_ids(&self.blocks, &blocks);
        rerender(
            &new_source,
            &self.options,
            &mut blocks,
            &changed_ids,
            &changed_definitions,
        )?;

        let whole_document = self.whole_document
            || requires_whole_document(&blocks)
            || new_source.as_bytes().contains(&b'\r');
        if whole_document {
            let canonical = crate::to_html_with_options(&new_source, &self.options)?;
            let id = self
                .blocks
                .first()
                .map_or_else(|| allocate_id(&mut next_id), |block| block.id.clone());
            blocks = vec![Block {
                start: 0,
                end: new_source.len(),
                data: Arc::new(BlockData {
                    id,
                    fingerprint: fingerprint(0, new_source.as_bytes()),
                    kind: u8::MAX,
                    html: canonical,
                    definitions: vec![],
                    references: vec![],
                    footnote: true,
                    node: Arc::new(Node::Root(Root {
                        children: vec![],
                        position: None,
                    })),
                }),
            }];
        }

        let new_visible = visible(&blocks);
        let patches = diff_visible(&old_visible, &new_visible);
        let html = join_html(&blocks);
        let checkpoints = make_checkpoints(&blocks);

        self.source = new_source;
        self.version += 1;
        self.blocks = blocks;
        self.checkpoints = checkpoints;
        self.next_id = next_id;
        self.html = html;
        self.whole_document = whole_document;

        Ok(ApplyResult {
            version: self.version,
            patches,
            reparsed: ByteRange {
                start,
                end: reparsed_end,
            },
        })
    }
}

struct Reparse<'a> {
    source: &'a str,
    options: &'a ParseOptions,
    old: &'a [Block],
    start_index: usize,
    start: usize,
    latest_old: usize,
    total_delta: (usize, usize),
    old_definition_ids: &'a [String],
    inherited: &'a [String],
}

fn parse_until_convergence(
    reparse: &Reparse,
    next_id: &mut u64,
) -> Result<(Vec<Block>, usize, usize), message::Message> {
    let minimum_old = reparse.latest_old.max(reparse.start);
    let mut candidate_index = reparse.start_index;
    while candidate_index < reparse.old.len() && reparse.old[candidate_index].end <= minimum_old {
        candidate_index += 1;
    }
    candidate_index = (candidate_index + 2).min(reparse.old.len());

    loop {
        let end = if candidate_index == reparse.old.len() {
            reparse.source.len()
        } else {
            add_delta(reparse.old[candidate_index - 1].end, reparse.total_delta)
                .min(reparse.source.len())
        };
        if end < reparse.start || !reparse.source.is_char_boundary(end) {
            candidate_index = (candidate_index + 1).min(reparse.old.len());
            continue;
        }

        let mut seed = reparse.inherited.to_vec();
        for identifier in reparse.old_definition_ids {
            if !seed.contains(identifier) {
                seed.push(identifier.clone());
            }
        }
        let parsed_result = parse_blocks(
            &reparse.source[reparse.start..end],
            reparse.start,
            reparse.options,
            &seed,
            next_id,
        );
        let parsed = match parsed_result {
            Ok(parsed) => parsed,
            Err(_) if candidate_index < reparse.old.len() => {
                candidate_index += 1;
                continue;
            }
            Err(error) => return Err(error),
        };

        if candidate_index == reparse.old.len()
            || has_converged(reparse.old, reparse.start_index, candidate_index, &parsed)
        {
            return Ok((parsed, end, candidate_index));
        }
        candidate_index += 1;
    }
}

fn has_converged(old: &[Block], start: usize, candidate: usize, parsed: &[Block]) -> bool {
    let available = candidate.saturating_sub(start).min(parsed.len());
    let count = available.min(1);
    if count == 0 {
        return false;
    }
    let old_start = candidate - count;
    let new_start = parsed.len() - count;
    old[old_start..candidate]
        .iter()
        .zip(&parsed[new_start..])
        .all(|(left, right)| left.fingerprint == right.fingerprint && left.kind == right.kind)
}

fn parse_blocks(
    value: &str,
    base: usize,
    options: &ParseOptions,
    inherited_definitions: &[String],
    next_id: &mut u64,
) -> Result<Vec<Block>, message::Message> {
    let (events, state) =
        parser::parse_with_definitions(value, options, inherited_definitions.to_vec(), vec![])?;
    let mut tree = to_mdast::compile(&events, state.bytes)?;
    if options.constructs.code_hike_blocks {
        code_hike_blocks::transform(&mut tree);
    }

    let mut result = vec![];
    if let Some(children) = tree.children() {
        for node in children {
            let position = node.position().expect("top-level nodes have positions");
            let start = base + position.start.offset;
            let end = base + position.end.offset;
            let mut definitions = vec![];
            let mut references = vec![];
            let mut footnote = false;
            collect_dependencies(node, &mut definitions, &mut references, &mut footnote);
            references.sort();
            references.dedup();
            let kind = node_kind(node);
            result.push(Block {
                start,
                end,
                data: Arc::new(BlockData {
                    id: allocate_id(next_id),
                    fingerprint: fingerprint(
                        kind,
                        &value.as_bytes()[position.start.offset..position.end.offset],
                    ),
                    kind,
                    html: String::new(),
                    definitions,
                    references,
                    footnote,
                    node: Arc::new(node.clone()),
                }),
            });
        }
    }
    Ok(result)
}

fn collect_dependencies(
    node: &Node,
    definitions: &mut Vec<(String, u64)>,
    references: &mut Vec<String>,
    footnote: &mut bool,
) {
    match node {
        Node::Definition(definition) => definitions.push((
            definition.identifier.clone(),
            fingerprint(
                0,
                format!("{}\0{:?}", definition.url, definition.title).as_bytes(),
            ),
        )),
        Node::LinkReference(reference) => references.push(reference.identifier.clone()),
        Node::ImageReference(reference) => references.push(reference.identifier.clone()),
        Node::FootnoteDefinition(_) | Node::FootnoteReference(_) => *footnote = true,
        _ => {}
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_dependencies(child, definitions, references, footnote);
        }
    }
}

fn node_kind(node: &Node) -> u8 {
    match node {
        Node::Root(_) => 0,
        Node::Blockquote(_) => 1,
        Node::FootnoteDefinition(_) => 2,
        Node::MdxJsxFlowElement(_) => 3,
        Node::List(_) => 4,
        Node::MdxjsEsm(_) => 5,
        Node::Toml(_) => 6,
        Node::Yaml(_) => 7,
        Node::Break(_) => 8,
        Node::InlineCode(_) => 9,
        Node::InlineMath(_) => 10,
        Node::Delete(_) => 11,
        Node::Emphasis(_) => 12,
        Node::MdxTextExpression(_) => 13,
        Node::FootnoteReference(_) => 14,
        Node::Html(_) => 15,
        Node::Image(_) => 16,
        Node::ImageReference(_) => 17,
        Node::MdxJsxTextElement(_) => 18,
        Node::Link(_) => 19,
        Node::LinkReference(_) => 20,
        Node::Strong(_) => 21,
        Node::Text(_) => 22,
        Node::Code(_) => 23,
        Node::Math(_) => 24,
        Node::MdxFlowExpression(_) => 25,
        Node::Heading(_) => 26,
        Node::Table(_) => 27,
        Node::ThematicBreak(_) => 28,
        Node::TableRow(_) => 29,
        Node::TableCell(_) => 30,
        Node::ListItem(_) => 31,
        Node::Definition(_) => 32,
        Node::Paragraph(_) => 33,
        Node::ObsidianWikilink(_) => 34,
        Node::ObsidianEmbed(_) => 35,
        Node::ObsidianComment(_) => 36,
        Node::ObsidianBlockId(_) => 37,
        Node::ObsidianHighlight(_) => 38,
        Node::ObsidianCallout(_) => 39,
        Node::CodeHikeBlock(_) => 40,
        Node::CodeHikeText(_) => 41,
        Node::CodeHikeImage(_) => 42,
        Node::CodeHikeCode(_) => 43,
    }
}

fn render_all(
    source: &str,
    options: &Options,
    blocks: &mut [Block],
) -> Result<(), message::Message> {
    let definitions = definition_sources(source, blocks);
    for block in blocks {
        block.html = render_block(source, block, &definitions, options)?;
    }
    Ok(())
}

fn rerender(
    source: &str,
    options: &Options,
    blocks: &mut [Block],
    changed_blocks: &ChangedBlocks,
    changed_definitions: &BTreeSet<String>,
) -> Result<(), message::Message> {
    let definitions = definition_sources(source, blocks);
    for block in blocks {
        let dependency_changed = block
            .references
            .iter()
            .any(|identifier| changed_definitions.contains(identifier));
        if changed_blocks.contains(&block.id) || dependency_changed {
            block.html = render_block(source, block, &definitions, options)?;
        }
    }
    Ok(())
}

fn render_block(
    source: &str,
    block: &Block,
    definitions: &str,
    options: &Options,
) -> Result<String, message::Message> {
    if !block.definitions.is_empty() {
        return Ok(String::new());
    }
    let mut input = source[block.start..block.end].to_string();
    let appended_definitions = !definitions.is_empty() && !block.references.is_empty();
    if appended_definitions {
        input.push_str("\n\n");
        input.push_str(definitions);
    }
    let mut html = crate::to_html_with_options(&input, options)?;
    if appended_definitions && html.ends_with('\n') {
        html.pop();
        if html.ends_with('\r') {
            html.pop();
        }
    }
    Ok(html)
}

fn definition_sources(source: &str, blocks: &[Block]) -> String {
    let mut result = String::new();
    for block in blocks {
        if !block.definitions.is_empty() {
            if !result.is_empty() {
                result.push_str("\n\n");
            }
            result.push_str(&source[block.start..block.end]);
        }
    }
    result
}

fn align_ids(old: &[Block], new: &mut [Block]) {
    let rows = old.len() + 1;
    let columns = new.len() + 1;
    let mut table = vec![0usize; rows * columns];
    for old_index in (0..old.len()).rev() {
        for new_index in (0..new.len()).rev() {
            let value = if old[old_index].fingerprint == new[new_index].fingerprint
                && old[old_index].kind == new[new_index].kind
            {
                1 + table[(old_index + 1) * columns + new_index + 1]
            } else {
                table[(old_index + 1) * columns + new_index]
                    .max(table[old_index * columns + new_index + 1])
            };
            table[old_index * columns + new_index] = value;
        }
    }

    let mut old_index = 0;
    let mut new_index = 0;
    while old_index < old.len() && new_index < new.len() {
        if old[old_index].fingerprint == new[new_index].fingerprint
            && old[old_index].kind == new[new_index].kind
        {
            new[new_index].id.clone_from(&old[old_index].id);
            new[new_index].html.clone_from(&old[old_index].html);
            old_index += 1;
            new_index += 1;
        } else if table[(old_index + 1) * columns + new_index]
            >= table[old_index * columns + new_index + 1]
        {
            old_index += 1;
        } else {
            new_index += 1;
        }
    }

    // Preserve identity for a directly edited block between aligned neighbors.
    for index in 0..old.len().min(new.len()) {
        if new[index].html.is_empty()
            && old[index].kind == new[index].kind
            && !new.iter().any(|block| block.id == old[index].id)
        {
            new[index].id.clone_from(&old[index].id);
        }
    }
}

struct ChangedBlocks {
    ids: Vec<String>,
}

impl ChangedBlocks {
    fn contains(&self, id: &str) -> bool {
        self.ids
            .binary_search_by(|candidate| candidate.as_str().cmp(id))
            .is_ok()
    }
}

fn changed_block_ids(old: &[Block], new: &[Block]) -> ChangedBlocks {
    let mut old_by_id: Vec<_> = old.iter().collect();
    old_by_id.sort_unstable_by(|left, right| left.id.cmp(&right.id));
    let mut ids: Vec<_> = new
        .iter()
        .filter(|block| {
            old_by_id
                .binary_search_by(|old| old.id.as_str().cmp(&block.id))
                .map_or(true, |index| {
                    let old = old_by_id[index];
                    old.fingerprint != block.fingerprint || old.references != block.references
                })
        })
        .map(|block| block.id.clone())
        .collect();
    ids.sort_unstable();
    ChangedBlocks { ids }
}

fn definition_map(blocks: &[Block]) -> BTreeMap<String, u64> {
    let mut result = BTreeMap::new();
    for block in blocks {
        for (identifier, value) in &block.definitions {
            if !result.contains_key(identifier) {
                result.insert(identifier.clone(), *value);
            }
        }
    }
    result
}

fn definition_ids(blocks: &[Block]) -> Vec<String> {
    definition_map(blocks).keys().cloned().collect()
}

fn definitions_before(blocks: &[Block], end: usize) -> Vec<String> {
    let mut result = vec![];
    for block in &blocks[..end] {
        for (identifier, _) in &block.definitions {
            if !result.contains(identifier) {
                result.push(identifier.clone());
            }
        }
    }
    result
}

fn changed_definition_ids(
    old: &BTreeMap<String, u64>,
    new: &BTreeMap<String, u64>,
) -> BTreeSet<String> {
    old.keys()
        .chain(new.keys())
        .filter(|identifier| old.get(*identifier) != new.get(*identifier))
        .cloned()
        .collect()
}

fn visible(blocks: &[Block]) -> Vec<RenderedBlock> {
    blocks
        .iter()
        .filter(|block| !block.html.is_empty())
        .map(|block| RenderedBlock {
            id: block.id.clone(),
            range: ByteRange {
                start: block.start,
                end: block.end,
            },
            html: block.html.clone(),
        })
        .collect()
}

fn diff_visible(old: &[RenderedBlock], new: &[RenderedBlock]) -> Vec<Patch> {
    let old_ids: BTreeSet<&str> = old.iter().map(|block| block.id.as_str()).collect();
    let new_ids: BTreeSet<&str> = new.iter().map(|block| block.id.as_str()).collect();
    let old_html: BTreeMap<&str, &str> = old
        .iter()
        .map(|block| (block.id.as_str(), block.html.as_str()))
        .collect();
    let mut patches = vec![];

    for block in old {
        if !new_ids.contains(block.id.as_str()) {
            patches.push(Patch::Remove {
                id: block.id.clone(),
            });
        }
    }

    let mut after = None;
    for block in new {
        if !old_ids.contains(block.id.as_str()) {
            patches.push(Patch::InsertAfter {
                after: after.clone(),
                id: block.id.clone(),
                html: block.html.clone(),
            });
        } else if old_html.get(block.id.as_str()) != Some(&block.html.as_str()) {
            patches.push(Patch::Replace {
                id: block.id.clone(),
                html: block.html.clone(),
            });
        }
        after = Some(block.id.clone());
    }
    patches
}

fn make_checkpoints(blocks: &[Block]) -> Vec<BlockCheckpoint> {
    let mut result = vec![BlockCheckpoint {
        byte_offset: 0,
        block_index: 0,
        reference_environment_hash: FNV_OFFSET,
    }];
    let mut environment = FNV_OFFSET;
    for (index, block) in blocks.iter().enumerate() {
        for (identifier, value) in &block.definitions {
            environment = hash_bytes(environment, identifier.as_bytes());
            environment = hash_bytes(environment, &value.to_le_bytes());
        }
        result.push(BlockCheckpoint {
            byte_offset: block.end,
            block_index: index + 1,
            reference_environment_hash: environment,
        });
    }
    result
}

fn checkpoint_before(blocks: &[Block], offset: usize) -> usize {
    let containing = blocks
        .iter()
        .position(|block| block.end >= offset)
        .unwrap_or(blocks.len());
    containing.saturating_sub(1)
}

fn join_html(blocks: &[Block]) -> String {
    let mut result = String::new();
    let mut seen_visible = false;
    let mut trailing_definition = false;
    for block in blocks.iter().filter(|block| !block.html.is_empty()) {
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(&block.html);
        seen_visible = true;
    }
    if seen_visible {
        if let Some(last_visible) = blocks.iter().rposition(|block| !block.html.is_empty()) {
            trailing_definition = blocks[last_visible + 1..]
                .iter()
                .any(|block| !block.definitions.is_empty());
        }
    }
    if trailing_definition {
        result.push('\n');
    }
    result
}

fn requires_whole_document(blocks: &[Block]) -> bool {
    blocks
        .iter()
        .any(|block| block.footnote || matches!(block.kind, 5 | 6 | 7 | u8::MAX))
}

fn document_block(value: &str, html: String, next_id: &mut u64) -> Block {
    Block {
        start: 0,
        end: value.len(),
        data: Arc::new(BlockData {
            id: allocate_id(next_id),
            fingerprint: fingerprint(u8::MAX, value.as_bytes()),
            kind: u8::MAX,
            html,
            definitions: vec![],
            references: vec![],
            footnote: true,
            node: Arc::new(Node::Root(Root {
                children: vec![],
                position: None,
            })),
        }),
    }
}

fn rebase_node(node: &mut Node, shift: (usize, usize), location: &Location) {
    if let Some(position) = node.position_mut() {
        let start = add_delta(position.start.offset, shift);
        let end = add_delta(position.end.offset, shift);
        position.start = location.to_point(start).expect("node start is in document");
        position.end = location.to_point(end).expect("node end is in document");
    }
    match node {
        Node::MdxjsEsm(node) => rebase_stops(&mut node.stops, shift),
        Node::MdxFlowExpression(node) => rebase_stops(&mut node.stops, shift),
        Node::MdxTextExpression(node) => rebase_stops(&mut node.stops, shift),
        Node::MdxJsxFlowElement(node) => rebase_attributes(&mut node.attributes, shift),
        Node::MdxJsxTextElement(node) => rebase_attributes(&mut node.attributes, shift),
        _ => {}
    }
    if let Some(children) = node.children_mut() {
        for child in children {
            rebase_node(child, shift, location);
        }
    }
}

fn rebase_attributes(attributes: &mut [AttributeContent], shift: (usize, usize)) {
    for attribute in attributes {
        match attribute {
            AttributeContent::Expression(expression) => {
                rebase_stops(&mut expression.stops, shift);
            }
            AttributeContent::Property(property) => {
                if let Some(AttributeValue::Expression(expression)) = &mut property.value {
                    rebase_stops(&mut expression.stops, shift);
                }
            }
        }
    }
}

fn rebase_stops(stops: &mut [(usize, usize)], shift: (usize, usize)) {
    for (_, absolute) in stops {
        *absolute = add_delta(*absolute, shift);
    }
}

fn mdast_from_blocks(source: &str, blocks: &[Block]) -> Node {
    let location = Location::new(source.as_bytes());
    let children = blocks
        .iter()
        .map(|block| rebase_block_node(block, &location))
        .collect();
    Node::Root(Root {
        children,
        position: Some(crate::unist::Position {
            start: location.to_point(0).expect("document start exists"),
            end: location
                .to_point(source.len())
                .expect("document end exists"),
        }),
    })
}

fn rebase_block_node(block: &Block, location: &Location) -> Node {
    let mut node = block.node.as_ref().clone();
    let current_start = node
        .position()
        .map_or(block.start, |position| position.start.offset);
    rebase_node(&mut node, (block.start, current_start), location);
    node
}

fn validate_edits(source: &str, version: u64, batch: &EditBatch) -> Result<(), message::Message> {
    if batch.base_version != version {
        return Err(incremental_error(format!(
            "stale edit batch: expected version {}, received {}",
            version, batch.base_version
        )));
    }
    let mut previous_end = 0;
    for (index, edit) in batch.edits.iter().enumerate() {
        if edit.start_byte > edit.old_end_byte || edit.old_end_byte > source.len() {
            return Err(incremental_error(
                "edit range is outside the document".to_string(),
            ));
        }
        if !source.is_char_boundary(edit.start_byte) || !source.is_char_boundary(edit.old_end_byte)
        {
            return Err(incremental_error(
                "edit range splits a UTF-8 code point".to_string(),
            ));
        }
        if index > 0 && edit.start_byte < previous_end {
            return Err(incremental_error(
                "edits must be sorted and nonoverlapping".to_string(),
            ));
        }
        previous_end = edit.old_end_byte;
    }
    Ok(())
}

fn apply_edits(source: &str, edits: &[Edit]) -> String {
    let capacity = add_delta(source.len(), byte_delta(edits));
    let mut result = String::with_capacity(capacity);
    let mut cursor = 0;
    for edit in edits {
        result.push_str(&source[cursor..edit.start_byte]);
        result.push_str(&edit.replacement);
        cursor = edit.old_end_byte;
    }
    result.push_str(&source[cursor..]);
    result
}

fn byte_delta(edits: &[Edit]) -> (usize, usize) {
    edits.iter().fold((0, 0), |(added, removed), edit| {
        (
            added + edit.replacement.len(),
            removed + edit.old_end_byte - edit.start_byte,
        )
    })
}

fn delta_at_old_offset(edits: &[Edit], offset: usize) -> (usize, usize) {
    edits
        .iter()
        .filter(|edit| edit.old_end_byte <= offset)
        .fold((0, 0), |(added, removed), edit| {
            (
                added + edit.replacement.len(),
                removed + edit.old_end_byte - edit.start_byte,
            )
        })
}

fn add_delta(value: usize, (added, removed): (usize, usize)) -> usize {
    value - removed + added
}

fn allocate_id(next: &mut u64) -> String {
    let result = format!("b_{:016x}", *next);
    *next += 1;
    result
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fingerprint(kind: u8, bytes: &[u8]) -> u64 {
    hash_bytes(hash_bytes(FNV_OFFSET, &[kind]), bytes)
}

fn hash_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn incremental_error(reason: String) -> message::Message {
    message::Message {
        place: None,
        reason,
        rule_id: Box::new("edit".to_string()),
        source: Box::new("markdown-incremental".to_string()),
    }
}
