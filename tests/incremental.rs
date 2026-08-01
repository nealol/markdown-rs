use markdown::{Edit, EditBatch, Options, Patch, Renderer};

#[test]
fn rich_sections_remain_independently_renderable() {
    let source = "# Incremental body benchmark\n\n## Section 0\n\nParagraph 0 contains **strong text**, *emphasis*, and a [link](/section-0/).\n\n- Item one\n- Item two\n\nBenchmark body token: body-a.\n";
    let mut renderer = Renderer::open(source, Options::default()).unwrap();
    let before: Vec<_> = renderer.blocks().collect();

    assert_eq!(before.len(), 5);
    assert_eq!(renderer.html(), markdown::to_html(source));
    let body = source.find("body-a").unwrap();
    renderer
        .apply(EditBatch {
            base_version: 0,
            edits: vec![Edit {
                start_byte: body,
                old_end_byte: body + "body-a".len(),
                replacement: "body-b".into(),
            }],
        })
        .unwrap();
    let after: Vec<_> = renderer.blocks().collect();
    assert_eq!(after.len(), 5);
    assert_eq!(before[0].id, after[0].id);
    assert_eq!(before[3].id, after[3].id);

    let end = renderer.source().len();
    renderer
        .apply(EditBatch {
            base_version: 1,
            edits: vec![Edit {
                start_byte: end - 1,
                old_end_byte: end,
                replacement: String::new(),
            }],
        })
        .unwrap();
    assert_eq!(renderer.html(), markdown::to_html(renderer.source()));
    let end = renderer.source().len();
    renderer
        .apply(EditBatch {
            base_version: 2,
            edits: vec![Edit {
                start_byte: end,
                old_end_byte: end,
                replacement: "\n".into(),
            }],
        })
        .unwrap();
    assert_eq!(renderer.html(), markdown::to_html(renderer.source()));
}

#[test]
fn edits_one_block_and_preserves_identity() {
    let source = "# Title\n\nAlpha.\n\nBravo.\n\nCharlie.\n\nDelta.";
    let mut renderer = Renderer::open(source, Options::default()).unwrap();
    let before: Vec<_> = renderer.blocks().collect();
    let start = source.find("Bravo").unwrap();

    let result = renderer
        .apply(EditBatch {
            base_version: 0,
            edits: vec![Edit {
                start_byte: start,
                old_end_byte: start + "Bravo".len(),
                replacement: "Beta".into(),
            }],
        })
        .unwrap();
    let after: Vec<_> = renderer.blocks().collect();

    assert_eq!(
        renderer.html(),
        markdown::to_html("# Title\n\nAlpha.\n\nBeta.\n\nCharlie.\n\nDelta.")
    );
    assert_eq!(before[2].id, after[2].id);
    assert_eq!(before[0].id, after[0].id);
    assert_eq!(before[4].id, after[4].id);
    assert_eq!(result.reparsed.start, before[1].range.start);
    assert!(result.reparsed.end < renderer.source().len());
    assert_eq!(
        result.patches,
        vec![Patch::Replace {
            id: before[2].id.clone(),
            html: "<p>Beta.</p>".into(),
        }]
    );
}

#[test]
fn inserts_and_removes_blocks() {
    let mut renderer = Renderer::open("Alpha.\n\nCharlie.", Options::default()).unwrap();
    let insert_at = "Alpha.\n\n".len();
    let inserted = renderer
        .apply(EditBatch {
            base_version: 0,
            edits: vec![Edit {
                start_byte: insert_at,
                old_end_byte: insert_at,
                replacement: "Bravo.\n\n".into(),
            }],
        })
        .unwrap();

    assert!(inserted
        .patches
        .iter()
        .any(|patch| matches!(patch, Patch::InsertAfter { html, .. } if html == "<p>Bravo.</p>")));
    assert_eq!(
        renderer.html(),
        "<p>Alpha.</p>\n<p>Bravo.</p>\n<p>Charlie.</p>"
    );

    let removed = renderer
        .apply(EditBatch {
            base_version: 1,
            edits: vec![Edit {
                start_byte: insert_at,
                old_end_byte: insert_at + "Bravo.\n\n".len(),
                replacement: String::new(),
            }],
        })
        .unwrap();

    assert!(removed
        .patches
        .iter()
        .any(|patch| matches!(patch, Patch::Remove { .. })));
    assert_eq!(renderer.html(), "<p>Alpha.</p>\n<p>Charlie.</p>");
}

#[test]
fn definition_changes_only_rerender_consumers() {
    let source = "[One][docs]\n\nUnrelated.\n\n[Two][docs]\n\n[docs]: /old";
    let mut renderer = Renderer::open(source, Options::default()).unwrap();
    let before: Vec<_> = renderer.blocks().collect();
    assert_eq!(
        before.len(),
        3,
        "html={:?}, blocks={:?}",
        renderer.html(),
        before
    );
    let start = source.find("/old").unwrap();

    let result = renderer
        .apply(EditBatch {
            base_version: 0,
            edits: vec![Edit {
                start_byte: start,
                old_end_byte: start + 4,
                replacement: "/new".into(),
            }],
        })
        .unwrap();

    let replaced: Vec<_> = result
        .patches
        .iter()
        .filter_map(|patch| match patch {
            Patch::Replace { id, .. } => Some(id.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(replaced, vec![before[0].id.as_str(), before[2].id.as_str()]);
    assert!(!replaced.contains(&before[1].id.as_str()));
    assert_eq!(
        renderer.html(),
        "<p><a href=\"/new\">One</a></p>\n<p>Unrelated.</p>\n<p><a href=\"/new\">Two</a></p>\n"
    );
}

#[test]
fn adding_a_definition_reclassifies_earlier_references() {
    let source = "[One][docs]\n\nUnrelated.";
    let mut renderer = Renderer::open(source, Options::default()).unwrap();
    let insert_at = source.len();

    renderer
        .apply(EditBatch {
            base_version: 0,
            edits: vec![Edit {
                start_byte: insert_at,
                old_end_byte: insert_at,
                replacement: "\n\n[docs]: /new".into(),
            }],
        })
        .unwrap();

    assert_eq!(
        renderer.html(),
        "<p><a href=\"/new\">One</a></p>\n<p>Unrelated.</p>\n"
    );
}

#[test]
fn removing_a_definition_restores_literal_reference_text() {
    let source = "[One][docs]\n\nUnrelated.\n\n[docs]: /old";
    let mut renderer = Renderer::open(source, Options::default()).unwrap();
    let definition_start = source.find("[docs]:").unwrap();

    let result = renderer
        .apply(EditBatch {
            base_version: 0,
            edits: vec![Edit {
                start_byte: definition_start - 2,
                old_end_byte: source.len(),
                replacement: String::new(),
            }],
        })
        .unwrap();

    assert_eq!(renderer.html(), "<p>[One][docs]</p>\n<p>Unrelated.</p>");
    assert!(result
        .patches
        .iter()
        .any(|patch| matches!(patch, Patch::Replace { html, .. } if html == "<p>[One][docs]</p>")));
}

#[test]
fn an_open_fence_invalidates_the_suffix() {
    let source = "Alpha.\n\nBravo.\n\nCharlie.\n\nDelta.";
    let mut renderer = Renderer::open(source, Options::default()).unwrap();
    let start = source.find("Bravo").unwrap();
    let result = renderer
        .apply(EditBatch {
            base_version: 0,
            edits: vec![Edit {
                start_byte: start,
                old_end_byte: start,
                replacement: "```\n".into(),
            }],
        })
        .unwrap();

    assert_eq!(result.reparsed.end, renderer.source().len());
    assert_eq!(
        renderer.html(),
        markdown::to_html("Alpha.\n\n```\nBravo.\n\nCharlie.\n\nDelta.")
    );
}

#[test]
fn changing_to_crlf_keeps_canonical_output() {
    let mut renderer = Renderer::open("a\n\nb", Options::default()).unwrap();
    renderer
        .apply(EditBatch {
            base_version: 0,
            edits: vec![Edit {
                start_byte: 1,
                old_end_byte: 2,
                replacement: "\r\n".into(),
            }],
        })
        .unwrap();

    assert_eq!(
        renderer.html(),
        markdown::to_html(renderer.source()),
        "incremental output must use the document-wide inferred line ending"
    );
}

#[test]
fn invalid_batches_are_atomic() {
    let mut renderer = Renderer::open("🌍", Options::default()).unwrap();
    let error = renderer
        .apply(EditBatch {
            base_version: 0,
            edits: vec![Edit {
                start_byte: 1,
                old_end_byte: 1,
                replacement: "x".into(),
            }],
        })
        .unwrap_err();

    assert!(error.reason.contains("UTF-8"));
    assert_eq!(renderer.version(), 0);
    assert_eq!(renderer.source(), "🌍");
}

#[test]
fn footnotes_use_a_correct_whole_document_patch() {
    let source = "Call[^a].\n\n[^a]: old";
    let mut renderer = Renderer::open(source, Options::gfm()).unwrap();
    assert_eq!(renderer.blocks().count(), 1);
    let start = source.find("old").unwrap();
    let result = renderer
        .apply(EditBatch {
            base_version: 0,
            edits: vec![Edit {
                start_byte: start,
                old_end_byte: start + 3,
                replacement: "new".into(),
            }],
        })
        .unwrap();

    assert_eq!(
        renderer.html(),
        markdown::to_html_with_options("Call[^a].\n\n[^a]: new", &Options::gfm()).unwrap()
    );
    assert_eq!(result.patches.len(), 1);
    assert!(matches!(result.patches[0], Patch::Replace { .. }));
}
