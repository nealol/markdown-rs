use crate::util::{
    line_ending::LineEnding,
    mdx::{EsmParse as MdxEsmParse, ExpressionParse as MdxExpressionParse},
};
use alloc::{boxed::Box, fmt, string::String, sync::Arc};

/// Control which constructs are enabled.
///
/// Not all constructs can be configured.
/// Notably, blank lines and paragraphs cannot be turned off.
///
/// ## Examples
///
/// ```
/// use markdown::Constructs;
/// # fn main() {
///
/// // Use the default trait to get `CommonMark` constructs:
/// let commonmark = Constructs::default();
///
/// // To turn on all of GFM, use the `gfm` method:
/// let gfm = Constructs::gfm();
///
/// // Or, mix and match:
/// let custom = Constructs {
///   math_flow: true,
///   math_text: true,
///   ..Constructs::gfm()
/// };
/// # }
/// ```
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct Constructs {
    /// Attention.
    ///
    /// ```markdown
    /// > | a *b* c **d**.
    ///       ^^^   ^^^^^
    /// ```
    pub attention: bool,
    /// Autolink.
    ///
    /// ```markdown
    /// > | a <https://example.com> b <user@example.org>.
    ///       ^^^^^^^^^^^^^^^^^^^^^   ^^^^^^^^^^^^^^^^^^
    /// ```
    pub autolink: bool,
    /// Block quote.
    ///
    /// ```markdown
    /// > | > a
    ///     ^^^
    /// ```
    pub block_quote: bool,
    /// Character escape.
    ///
    /// ```markdown
    /// > | a \* b
    ///       ^^
    /// ```
    pub character_escape: bool,
    /// Character reference.
    ///
    /// ```markdown
    /// > | a &amp; b
    ///       ^^^^^
    /// ```
    pub character_reference: bool,
    /// Code (indented).
    ///
    /// ```markdown
    /// > |     a
    ///     ^^^^^
    /// ```
    pub code_indented: bool,
    /// Code (fenced).
    ///
    /// ```markdown
    /// > | ~~~js
    ///     ^^^^^
    /// > | console.log(1)
    ///     ^^^^^^^^^^^^^^
    /// > | ~~~
    ///     ^^^
    /// ```
    pub code_fenced: bool,
    /// Code (text).
    ///
    /// ```markdown
    /// > | a `b` c
    ///       ^^^
    /// ```
    pub code_text: bool,
    /// Definition.
    ///
    /// ```markdown
    /// > | [a]: b "c"
    ///     ^^^^^^^^^^
    /// ```
    pub definition: bool,
    /// Frontmatter.
    ///
    /// ````markdown
    /// > | ---
    ///     ^^^
    /// > | title: Neptune
    ///     ^^^^^^^^^^^^^^
    /// > | ---
    ///     ^^^
    /// ````
    pub frontmatter: bool,
    /// GFM: autolink literal.
    ///
    /// ```markdown
    /// > | https://example.com
    ///     ^^^^^^^^^^^^^^^^^^^
    /// ```
    pub gfm_autolink_literal: bool,
    /// GFM: footnote definition.
    ///
    /// ```markdown
    /// > | [^a]: b
    ///     ^^^^^^^
    /// ```
    pub gfm_footnote_definition: bool,
    /// GFM: footnote label start.
    ///
    /// ```markdown
    /// > | a[^b]
    ///      ^^
    /// ```
    pub gfm_label_start_footnote: bool,
    ///
    /// ```markdown
    /// > | a ~b~ c.
    ///       ^^^
    /// ```
    pub gfm_strikethrough: bool,
    /// GFM: table.
    ///
    /// ```markdown
    /// > | | a |
    ///     ^^^^^
    /// > | | - |
    ///     ^^^^^
    /// > | | b |
    ///     ^^^^^
    /// ```
    pub gfm_table: bool,
    /// GFM: task list item.
    ///
    /// ```markdown
    /// > | * [x] y.
    ///       ^^^
    /// ```
    pub gfm_task_list_item: bool,
    /// Hard break (escape).
    ///
    /// ```markdown
    /// > | a\
    ///      ^
    ///   | b
    /// ```
    pub hard_break_escape: bool,
    /// Hard break (trailing).
    ///
    /// ```markdown
    /// > | a␠␠
    ///      ^^
    ///   | b
    /// ```
    pub hard_break_trailing: bool,
    /// Heading (atx).
    ///
    /// ```markdown
    /// > | # a
    ///     ^^^
    /// ```
    pub heading_atx: bool,
    /// Heading (setext).
    ///
    /// ```markdown
    /// > | a
    ///     ^^
    /// > | ==
    ///     ^^
    /// ```
    pub heading_setext: bool,
    /// HTML (flow).
    ///
    /// ```markdown
    /// > | <div>
    ///     ^^^^^
    /// ```
    pub html_flow: bool,
    /// HTML (text).
    ///
    /// ```markdown
    /// > | a <b> c
    ///       ^^^
    /// ```
    pub html_text: bool,
    /// Label start (image).
    ///
    /// ```markdown
    /// > | a ![b](c) d
    ///       ^^
    /// ```
    pub label_start_image: bool,
    /// Label start (link).
    ///
    /// ```markdown
    /// > | a [b](c) d
    ///       ^
    /// ```
    pub label_start_link: bool,
    /// Label end.
    ///
    /// ```markdown
    /// > | a [b](c) d
    ///         ^^^^
    /// ```
    pub label_end: bool,
    /// List items.
    ///
    /// ```markdown
    /// > | * a
    ///     ^^^
    /// ```
    pub list_item: bool,
    /// Math (flow).
    ///
    /// ```markdown
    /// > | $$
    ///     ^^
    /// > | \frac{1}{2}
    ///     ^^^^^^^^^^^
    /// > | $$
    ///     ^^
    /// ```
    pub math_flow: bool,
    /// Math (text).
    ///
    /// ```markdown
    /// > | a $b$ c
    ///       ^^^
    /// ```
    pub math_text: bool,
    /// MDX: ESM.
    ///
    /// ```markdown
    /// > | import a from 'b'
    ///     ^^^^^^^^^^^^^^^^^
    /// ```
    ///
    /// > 👉 **Note**: to support ESM, you *must* pass
    /// > [`mdx_esm_parse`][MdxEsmParse] in [`ParseOptions`][] too.
    /// > Otherwise, ESM is treated as normal markdown.
    pub mdx_esm: bool,
    /// MDX: expression (flow).
    ///
    /// ```markdown
    /// > | {Math.PI}
    ///     ^^^^^^^^^
    /// ```
    ///
    /// > 👉 **Note**: You *can* pass
    /// > [`mdx_expression_parse`][MdxExpressionParse] in [`ParseOptions`][]
    /// > too, to parse expressions according to a certain grammar (typically,
    /// > a programming language).
    /// > Otherwise, expressions are parsed with a basic algorithm that only
    /// > cares about braces.
    pub mdx_expression_flow: bool,
    /// MDX: expression (text).
    ///
    /// ```markdown
    /// > | a {Math.PI} c
    ///       ^^^^^^^^^
    /// ```
    ///
    /// > 👉 **Note**: You *can* pass
    /// > [`mdx_expression_parse`][MdxExpressionParse] in [`ParseOptions`][]
    /// > too, to parse expressions according to a certain grammar (typically,
    /// > a programming language).
    /// > Otherwise, expressions are parsed with a basic algorithm that only
    /// > cares about braces.
    pub mdx_expression_text: bool,
    /// MDX: JSX (flow).
    ///
    /// ```markdown
    /// > | <Component />
    ///     ^^^^^^^^^^^^^
    /// ```
    ///
    /// > 👉 **Note**: You *must* pass `html_flow: false` to use this,
    /// > as it’s preferred when on over `mdx_jsx_flow`.
    ///
    /// > 👉 **Note**: You *can* pass
    /// > [`mdx_expression_parse`][MdxExpressionParse] in [`ParseOptions`][]
    /// > too, to parse expressions in JSX according to a certain grammar
    /// > (typically, a programming language).
    /// > Otherwise, expressions are parsed with a basic algorithm that only
    /// > cares about braces.
    pub mdx_jsx_flow: bool,
    /// MDX: JSX (text).
    ///
    /// ```markdown
    /// > | a <Component /> c
    ///       ^^^^^^^^^^^^^
    /// ```
    ///
    /// > 👉 **Note**: You *must* pass `html_text: false` to use this,
    /// > as it’s preferred when on over `mdx_jsx_text`.
    ///
    /// > 👉 **Note**: You *can* pass
    /// > [`mdx_expression_parse`][MdxExpressionParse] in [`ParseOptions`][]
    /// > too, to parse expressions in JSX according to a certain grammar
    /// > (typically, a programming language).
    /// > Otherwise, expressions are parsed with a basic algorithm that only
    /// > cares about braces.
    pub mdx_jsx_text: bool,
    /// Thematic break.
    ///
    /// ```markdown
    /// > | ***
    ///     ^^^
    /// ```
    pub thematic_break: bool,
    /// Obsidian: wikilink.
    ///
    /// ```markdown
    /// > | a [[Note]] b
    ///       ^^^^^^^^
    /// ```
    pub obsidian_wikilink: bool,
    /// Obsidian: embed.
    ///
    /// ```markdown
    /// > | a ![[Note]] b
    ///       ^^^^^^^^^^
    /// ```
    pub obsidian_embed: bool,
    /// Obsidian: block id.
    ///
    /// ```markdown
    /// > | a
    ///   | ^id
    ///     ^^^
    /// ```
    pub obsidian_block_id: bool,
    /// Obsidian: comment.
    ///
    /// ```markdown
    /// > | a %%b%% c
    ///       ^^^^^
    /// ```
    pub obsidian_comment: bool,
    /// Obsidian: highlight.
    ///
    /// ```markdown
    /// > | a ==b== c
    ///       ^^^^^
    /// ```
    pub obsidian_highlight: bool,
    /// Obsidian: callout.
    ///
    /// ```markdown
    /// > | > [!note] Title
    ///     ^^^^^^^^^^^^^^^
    /// ```
    pub obsidian_callout: bool,
    /// `CodeHike`: decorated blocks.
    ///
    /// When enabled, the mdast produced by [`to_mdast`][crate::to_mdast] is
    /// post-processed to turn `CodeHike`-style decorations (`!name`, `!!name`)
    /// on headings, paragraphs, images, and fenced code blocks into dedicated
    /// `CodeHike*` mdast nodes.
    ///
    /// This is a structural mdast transform only. It does not change
    /// `to_html` / `to_html_with_options` output, which compiles directly
    /// from parser events.
    ///
    /// ```markdown
    /// > | ## !mordor Barad-dur
    ///     ^^^^^^^^^^^^^^^^^^^^
    /// > | The Dark Tower
    ///     ^^^^^^^^^^^^^^
    /// ```
    pub code_hike_blocks: bool,
}

impl Default for Constructs {
    /// `CommonMark`.
    ///
    /// `CommonMark` is a relatively strong specification of how markdown
    /// works.
    /// Most markdown parsers try to follow it.
    ///
    /// For more information, see the `CommonMark` specification:
    /// <https://spec.commonmark.org>.
    fn default() -> Self {
        Self {
            attention: true,
            autolink: true,
            block_quote: true,
            character_escape: true,
            character_reference: true,
            code_indented: true,
            code_fenced: true,
            code_text: true,
            definition: true,
            frontmatter: false,
            gfm_autolink_literal: false,
            gfm_label_start_footnote: false,
            gfm_footnote_definition: false,
            gfm_strikethrough: false,
            gfm_table: false,
            gfm_task_list_item: false,
            hard_break_escape: true,
            hard_break_trailing: true,
            heading_atx: true,
            heading_setext: true,
            html_flow: true,
            html_text: true,
            label_start_image: true,
            label_start_link: true,
            label_end: true,
            list_item: true,
            math_flow: false,
            math_text: false,
            mdx_esm: false,
            mdx_expression_flow: false,
            mdx_expression_text: false,
            mdx_jsx_flow: false,
            mdx_jsx_text: false,
            thematic_break: true,
            obsidian_wikilink: false,
            obsidian_embed: false,
            obsidian_block_id: false,
            obsidian_comment: false,
            obsidian_highlight: false,
            obsidian_callout: false,
            code_hike_blocks: false,
        }
    }
}

impl Constructs {
    /// GFM.
    ///
    /// GFM stands for **GitHub flavored markdown**.
    /// GFM extends `CommonMark` and adds support for autolink literals,
    /// footnotes, strikethrough, tables, and tasklists.
    ///
    /// For more information, see the GFM specification:
    /// <https://github.github.com/gfm/>.
    pub fn gfm() -> Self {
        Self {
            gfm_autolink_literal: true,
            gfm_footnote_definition: true,
            gfm_label_start_footnote: true,
            gfm_strikethrough: true,
            gfm_table: true,
            gfm_task_list_item: true,
            ..Self::default()
        }
    }

    /// MDX.
    ///
    /// This turns on `CommonMark`, turns off some conflicting constructs
    /// (autolinks, code (indented), and HTML), and turns on MDX (ESM,
    /// expressions, and JSX).
    ///
    /// For more information, see the MDX website:
    /// <https://mdxjs.com>.
    ///
    /// > 👉 **Note**: to support ESM, you *must* pass
    /// > [`mdx_esm_parse`][MdxEsmParse] in [`ParseOptions`][] too.
    /// > Otherwise, ESM is treated as normal markdown.
    /// >
    /// > You *can* pass
    /// > [`mdx_expression_parse`][MdxExpressionParse]
    /// > to parse expressions according to a certain grammar (typically, a
    /// > programming language).
    /// > Otherwise, expressions are parsed with a basic algorithm that only
    /// > cares about braces.
    pub fn mdx() -> Self {
        Self {
            autolink: false,
            code_indented: false,
            html_flow: false,
            html_text: false,
            mdx_esm: true,
            mdx_expression_flow: true,
            mdx_expression_text: true,
            mdx_jsx_flow: true,
            mdx_jsx_text: true,
            ..Self::default()
        }
    }

    /// Obsidian Flavored Markdown.
    ///
    /// OFM extends `CommonMark` (and is typically combined with GFM) and adds
    /// support for wikilinks, embeds, block ids, comments, highlights, and
    /// callouts.
    ///
    /// For more information, see the Obsidian Flavored Markdown documentation:
    /// <https://obsidian.md/help/obsidian-flavored-markdown>.
    ///
    /// > 👉 **Note**: OFM is typically combined with GFM. Use
    /// > `Constructs { ..Constructs::obsidian() }` mixed with `Constructs::gfm()`
    /// > (or use [`Options::obsidian()`], which turns on both) to get the full
    /// > Obsidian experience.
    pub fn obsidian() -> Self {
        Self {
            obsidian_wikilink: true,
            obsidian_embed: true,
            obsidian_block_id: true,
            obsidian_comment: true,
            obsidian_highlight: true,
            obsidian_callout: true,
            ..Self::default()
        }
    }

    /// `CodeHike` blocks.
    ///
    /// Enables the `CodeHike`-style decorated block transform on the mdast
    /// produced by [`to_mdast`][crate::to_mdast]. See
    /// [`Constructs::code_hike_blocks`] for details.
    ///
    /// For more information on `CodeHike`, see:
    /// <https://codehike.org>.
    pub fn code_hike() -> Self {
        Self {
            code_hike_blocks: true,
            ..Self::default()
        }
    }
}

/// Configuration that describes how to compile to HTML.
///
/// You likely either want to turn on the dangerous options
/// (`allow_dangerous_html`, `allow_dangerous_protocol`) when dealing with
/// input you trust, or want to customize how GFM footnotes are compiled
/// (typically because the input markdown is not in English).
///
/// ## Examples
///
/// ```
/// use markdown::CompileOptions;
/// # fn main() {
///
/// // Use the default trait to get safe defaults:
/// let safe = CompileOptions::default();
///
/// // Live dangerously / trust the author:
/// let danger = CompileOptions {
///   allow_dangerous_html: true,
///   allow_dangerous_protocol: true,
///   ..CompileOptions::default()
/// };
///
/// // In French:
/// let enFrançais = CompileOptions {
///   gfm_footnote_back_label: Some("Arrière".into()),
///   gfm_footnote_label: Some("Notes de bas de page".into()),
///   ..CompileOptions::default()
/// };
/// # }
/// ```
/// Parsed target of an Obsidian wikilink or embed.
///
/// This is the shared shape used by both [`ObsidianWikilink`][crate::mdast::Node::ObsidianWikilink]
/// and [`ObsidianEmbed`][crate::mdast::Node::ObsidianEmbed] nodes, and is what
/// resolvers receive to decide how to render them.
///
/// * `path` — the note path/file name (e.g. `Note`, `Daily notes/2026-06-22`).
///   `None` for same-file references such as `[[#Heading]]`.
/// * `heading` — a heading anchor within the target note (e.g. `Heading` from
///   `[[Note#Heading]]` or `[[#Heading]]`).
/// * `block_id` — a block reference within the target note (e.g. `abc` from
///   `[[Note#^abc]]` or `[[#^abc]]`).
/// * `alias` — display text (e.g. `Alias` from `[[Note|Alias]]`).
///
/// At most one of `heading` and `block_id` is `Some`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct ObsidianLinkTarget {
    /// Note path/file name, or `None` for same-file references.
    pub path: Option<String>,
    /// Heading anchor within the target note.
    pub heading: Option<String>,
    /// Block reference id within the target note.
    pub block_id: Option<String>,
    /// Display text (alias).
    pub alias: Option<String>,
}

/// Result of resolving an Obsidian wikilink to HTML.
///
/// * `href` — the (already-encoded) `href` attribute value.
/// * `text` — optional display text. If `None`, the caller falls back to the
///   alias / path / heading.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ObsidianLinkResolution {
    /// Already-encoded `href` attribute value.
    pub href: String,
    /// Optional display text (raw; the caller encodes it).
    pub text: Option<String>,
}

/// Result of resolving an Obsidian embed to HTML.
///
/// * `html` — the full HTML to emit for the embed (already-encoded/sanitized
///   by the resolver). The caller emits it verbatim.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ObsidianEmbedResolution {
    /// Full HTML to emit for the embed (resolver is responsible for encoding).
    pub html: String,
}

/// Resolver for Obsidian wikilinks.
///
/// A boxed closure that receives an [`ObsidianLinkTarget`] and returns an
/// [`ObsidianLinkResolution`]. Use this to resolve wikilinks against a vault
/// index, base URL, or other configuration.
pub type ObsidianLinkResolver = dyn Fn(&ObsidianLinkTarget) -> ObsidianLinkResolution + Send + Sync;

/// Resolver for Obsidian embeds.
///
/// A boxed closure that receives an [`ObsidianLinkTarget`] and returns an
/// [`ObsidianEmbedResolution`]. Use this to perform transclusion or generate
/// custom embed HTML.
pub type ObsidianEmbedResolver =
    dyn Fn(&ObsidianLinkTarget) -> ObsidianEmbedResolution + Send + Sync;

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default, rename_all = "camelCase")
)]
pub struct CompileOptions {
    /// Whether to allow all values in images.
    ///
    /// The default is `false`,
    /// which lets `allow_dangerous_protocol` control protocol safety for
    /// both links and images.
    ///
    /// Pass `true` to allow all values as `src` on images,
    /// regardless of `allow_dangerous_protocol`.
    /// This is safe because the
    /// [HTML specification][whatwg-html-image-processing]
    /// does not allow executable code in images.
    ///
    /// [whatwg-html-image-processing]: https://html.spec.whatwg.org/multipage/images.html#images-processing-model
    ///
    /// ## Examples
    ///
    /// ```
    /// use markdown::{to_html_with_options, CompileOptions, Options};
    /// # fn main() -> Result<(), markdown::message::Message> {
    ///
    /// // By default, some protocols in image sources are dropped:
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "![](data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==)",
    ///         &Options::default()
    ///     )?,
    ///     "<p><img src=\"\" alt=\"\" /></p>"
    /// );
    ///
    /// // Turn `allow_any_img_src` on to allow all values as `src` on images.
    /// // This is safe because browsers do not execute code in images.
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "![](javascript:alert(1))",
    ///         &Options {
    ///             compile: CompileOptions {
    ///               allow_any_img_src: true,
    ///               ..CompileOptions::default()
    ///             },
    ///             ..Options::default()
    ///         }
    ///     )?,
    ///     "<p><img src=\"javascript:alert(1)\" alt=\"\" /></p>"
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub allow_any_img_src: bool,

    /// Whether to allow (dangerous) HTML.
    ///
    /// The default is `false`, which still parses the HTML according to
    /// `CommonMark` but shows the HTML as text instead of as elements.
    ///
    /// Pass `true` for trusted content to get actual HTML elements.
    ///
    /// When using GFM, make sure to also turn off `gfm_tagfilter`.
    /// Otherwise, some dangerous HTML is still ignored.
    ///
    /// ## Examples
    ///
    /// ```
    /// use markdown::{to_html, to_html_with_options, CompileOptions, Options};
    /// # fn main() -> Result<(), markdown::message::Message> {
    ///
    /// // `markdown-rs` is safe by default:
    /// assert_eq!(
    ///     to_html("Hi, <i>venus</i>!"),
    ///     "<p>Hi, &lt;i&gt;venus&lt;/i&gt;!</p>"
    /// );
    ///
    /// // Turn `allow_dangerous_html` on to allow potentially dangerous HTML:
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "Hi, <i>venus</i>!",
    ///         &Options {
    ///             compile: CompileOptions {
    ///               allow_dangerous_html: true,
    ///               ..CompileOptions::default()
    ///             },
    ///             ..Options::default()
    ///         }
    ///     )?,
    ///     "<p>Hi, <i>venus</i>!</p>"
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub allow_dangerous_html: bool,

    /// Whether to allow dangerous protocols in links and images.
    ///
    /// The default is `false`, which drops URLs in links and images that use
    /// dangerous protocols.
    ///
    /// Pass `true` for trusted content to support all protocols.
    ///
    /// URLs that have no protocol (which means it’s relative to the current
    /// page, such as `./some/page.html`) and URLs that have a safe protocol
    /// (for images: `http`, `https`; for links: `http`, `https`, `irc`,
    /// `ircs`, `mailto`, `xmpp`), are safe.
    /// All other URLs are dangerous and dropped.
    ///
    /// When the option `allow_all_protocols_in_img` is enabled,
    /// `allow_dangerous_protocol` only applies to links.
    ///
    /// This is safe because the
    /// [HTML specification][whatwg-html-image-processing]
    /// does not allow executable code in images.
    /// All modern browsers respect this.
    ///
    /// [whatwg-html-image-processing]: https://html.spec.whatwg.org/multipage/images.html#images-processing-model
    ///
    /// ## Examples
    ///
    /// ```
    /// use markdown::{to_html, to_html_with_options, CompileOptions, Options};
    /// # fn main() -> Result<(), markdown::message::Message> {
    ///
    /// // `markdown-rs` is safe by default:
    /// assert_eq!(
    ///     to_html("<javascript:alert(1)>"),
    ///     "<p><a href=\"\">javascript:alert(1)</a></p>"
    /// );
    ///
    /// // Turn `allow_dangerous_protocol` on to allow potentially dangerous protocols:
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "<javascript:alert(1)>",
    ///         &Options {
    ///             compile: CompileOptions {
    ///               allow_dangerous_protocol: true,
    ///               ..CompileOptions::default()
    ///             },
    ///             ..Options::default()
    ///         }
    ///     )?,
    ///     "<p><a href=\"javascript:alert(1)\">javascript:alert(1)</a></p>"
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub allow_dangerous_protocol: bool,

    // To do: `doc_markdown` is broken.
    #[allow(clippy::doc_markdown)]
    /// Default line ending to use when compiling to HTML, for line endings not
    /// in `value`.
    ///
    /// Generally, `markdown-rs` copies line endings (`\r`, `\n`, `\r\n`) in
    /// the markdown document over to the compiled HTML.
    /// In some cases, such as `> a`, CommonMark requires that extra line
    /// endings are added: `<blockquote>\n<p>a</p>\n</blockquote>`.
    ///
    /// To create that line ending, the document is checked for the first line
    /// ending that is used.
    /// If there is no line ending, `default_line_ending` is used.
    /// If that isn’t configured, `\n` is used.
    ///
    /// ## Examples
    ///
    /// ```
    /// use markdown::{to_html, to_html_with_options, CompileOptions, LineEnding, Options};
    /// # fn main() -> Result<(), markdown::message::Message> {
    ///
    /// // `markdown-rs` uses `\n` by default:
    /// assert_eq!(
    ///     to_html("> a"),
    ///     "<blockquote>\n<p>a</p>\n</blockquote>"
    /// );
    ///
    /// // Define `default_line_ending` to configure the default:
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "> a",
    ///         &Options {
    ///             compile: CompileOptions {
    ///               default_line_ending: LineEnding::CarriageReturnLineFeed,
    ///               ..CompileOptions::default()
    ///             },
    ///             ..Options::default()
    ///         }
    ///     )?,
    ///     "<blockquote>\r\n<p>a</p>\r\n</blockquote>"
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub default_line_ending: LineEnding,

    /// Textual label to describe the backreference back to footnote calls.
    ///
    /// The default value is `"Back to content"`.
    /// Change it when the markdown is not in English.
    ///
    /// This label is used in the `aria-label` attribute on each backreference
    /// (the `↩` links).
    /// It affects users of assistive technology.
    ///
    /// ## Examples
    ///
    /// ```
    /// use markdown::{to_html_with_options, CompileOptions, Options, ParseOptions};
    /// # fn main() -> Result<(), markdown::message::Message> {
    ///
    /// // `"Back to content"` is used by default:
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "[^a]\n\n[^a]: b",
    ///         &Options::gfm()
    ///     )?,
    ///     "<p><sup><a href=\"#user-content-fn-a\" id=\"user-content-fnref-a\" data-footnote-ref=\"\" aria-describedby=\"footnote-label\">1</a></sup></p>\n<section data-footnotes=\"\" class=\"footnotes\"><h2 id=\"footnote-label\" class=\"sr-only\">Footnotes</h2>\n<ol>\n<li id=\"user-content-fn-a\">\n<p>b <a href=\"#user-content-fnref-a\" data-footnote-backref=\"\" aria-label=\"Back to content\" class=\"data-footnote-backref\">↩</a></p>\n</li>\n</ol>\n</section>\n"
    /// );
    ///
    /// // Pass `gfm_footnote_back_label` to use something else:
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "[^a]\n\n[^a]: b",
    ///         &Options {
    ///             parse: ParseOptions::gfm(),
    ///             compile: CompileOptions {
    ///               gfm_footnote_back_label: Some("Arrière".into()),
    ///               ..CompileOptions::gfm()
    ///             }
    ///         }
    ///     )?,
    ///     "<p><sup><a href=\"#user-content-fn-a\" id=\"user-content-fnref-a\" data-footnote-ref=\"\" aria-describedby=\"footnote-label\">1</a></sup></p>\n<section data-footnotes=\"\" class=\"footnotes\"><h2 id=\"footnote-label\" class=\"sr-only\">Footnotes</h2>\n<ol>\n<li id=\"user-content-fn-a\">\n<p>b <a href=\"#user-content-fnref-a\" data-footnote-backref=\"\" aria-label=\"Arrière\" class=\"data-footnote-backref\">↩</a></p>\n</li>\n</ol>\n</section>\n"
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub gfm_footnote_back_label: Option<String>,

    /// Prefix to use before the `id` attribute on footnotes to prevent them
    /// from *clobbering*.
    ///
    /// The default is `"user-content-"`.
    /// Pass `Some("".into())` for trusted markdown and when you are careful
    /// with polyfilling.
    /// You could pass a different prefix.
    ///
    /// DOM clobbering is this:
    ///
    /// ```html
    /// <p id="x"></p>
    /// <script>alert(x) // `x` now refers to the `p#x` DOM element</script>
    /// ```
    ///
    /// The above example shows that elements are made available by browsers,
    /// by their ID, on the `window` object.
    /// This is a security risk because you might be expecting some other
    /// variable at that place.
    /// It can also break polyfills.
    /// Using a prefix solves these problems.
    ///
    /// ## Examples
    ///
    /// ```
    /// use markdown::{to_html_with_options, CompileOptions, Options, ParseOptions};
    /// # fn main() -> Result<(), markdown::message::Message> {
    ///
    /// // `"user-content-"` is used by default:
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "[^a]\n\n[^a]: b",
    ///         &Options::gfm()
    ///     )?,
    ///     "<p><sup><a href=\"#user-content-fn-a\" id=\"user-content-fnref-a\" data-footnote-ref=\"\" aria-describedby=\"footnote-label\">1</a></sup></p>\n<section data-footnotes=\"\" class=\"footnotes\"><h2 id=\"footnote-label\" class=\"sr-only\">Footnotes</h2>\n<ol>\n<li id=\"user-content-fn-a\">\n<p>b <a href=\"#user-content-fnref-a\" data-footnote-backref=\"\" aria-label=\"Back to content\" class=\"data-footnote-backref\">↩</a></p>\n</li>\n</ol>\n</section>\n"
    /// );
    ///
    /// // Pass `gfm_footnote_clobber_prefix` to use something else:
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "[^a]\n\n[^a]: b",
    ///         &Options {
    ///             parse: ParseOptions::gfm(),
    ///             compile: CompileOptions {
    ///               gfm_footnote_clobber_prefix: Some("".into()),
    ///               ..CompileOptions::gfm()
    ///             }
    ///         }
    ///     )?,
    ///     "<p><sup><a href=\"#fn-a\" id=\"fnref-a\" data-footnote-ref=\"\" aria-describedby=\"footnote-label\">1</a></sup></p>\n<section data-footnotes=\"\" class=\"footnotes\"><h2 id=\"footnote-label\" class=\"sr-only\">Footnotes</h2>\n<ol>\n<li id=\"fn-a\">\n<p>b <a href=\"#fnref-a\" data-footnote-backref=\"\" aria-label=\"Back to content\" class=\"data-footnote-backref\">↩</a></p>\n</li>\n</ol>\n</section>\n"
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub gfm_footnote_clobber_prefix: Option<String>,

    /// Attributes to use on the footnote label.
    ///
    /// The default value is `"class=\"sr-only\""`.
    /// Change it to show the label and add other attributes.
    ///
    /// This label is typically hidden visually (assuming a `sr-only` CSS class
    /// is defined that does that), and thus affects screen readers only.
    /// If you do have such a class, but want to show this section to everyone,
    /// pass an empty string.
    /// You can also add different attributes.
    ///
    /// > 👉 **Note**: `id="footnote-label"` is always added, because footnote
    /// > calls use it with `aria-describedby` to provide an accessible label.
    ///
    /// ## Examples
    ///
    /// ```
    /// use markdown::{to_html_with_options, CompileOptions, Options, ParseOptions};
    /// # fn main() -> Result<(), markdown::message::Message> {
    ///
    /// // `"class=\"sr-only\""` is used by default:
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "[^a]\n\n[^a]: b",
    ///         &Options::gfm()
    ///     )?,
    ///     "<p><sup><a href=\"#user-content-fn-a\" id=\"user-content-fnref-a\" data-footnote-ref=\"\" aria-describedby=\"footnote-label\">1</a></sup></p>\n<section data-footnotes=\"\" class=\"footnotes\"><h2 id=\"footnote-label\" class=\"sr-only\">Footnotes</h2>\n<ol>\n<li id=\"user-content-fn-a\">\n<p>b <a href=\"#user-content-fnref-a\" data-footnote-backref=\"\" aria-label=\"Back to content\" class=\"data-footnote-backref\">↩</a></p>\n</li>\n</ol>\n</section>\n"
    /// );
    ///
    /// // Pass `gfm_footnote_label_attributes` to use something else:
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "[^a]\n\n[^a]: b",
    ///         &Options {
    ///             parse: ParseOptions::gfm(),
    ///             compile: CompileOptions {
    ///               gfm_footnote_label_attributes: Some("class=\"footnote-heading\"".into()),
    ///               ..CompileOptions::gfm()
    ///             }
    ///         }
    ///     )?,
    ///     "<p><sup><a href=\"#user-content-fn-a\" id=\"user-content-fnref-a\" data-footnote-ref=\"\" aria-describedby=\"footnote-label\">1</a></sup></p>\n<section data-footnotes=\"\" class=\"footnotes\"><h2 id=\"footnote-label\" class=\"footnote-heading\">Footnotes</h2>\n<ol>\n<li id=\"user-content-fn-a\">\n<p>b <a href=\"#user-content-fnref-a\" data-footnote-backref=\"\" aria-label=\"Back to content\" class=\"data-footnote-backref\">↩</a></p>\n</li>\n</ol>\n</section>\n"
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub gfm_footnote_label_attributes: Option<String>,

    /// HTML tag name to use for the footnote label element.
    ///
    /// The default value is `"h2"`.
    /// Change it to match your document structure.
    ///
    /// This label is typically hidden visually (assuming a `sr-only` CSS class
    /// is defined that does that), and thus affects screen readers only.
    /// If you do have such a class, but want to show this section to everyone,
    /// pass different attributes with the `gfm_footnote_label_attributes`
    /// option.
    ///
    /// ## Examples
    ///
    /// ```
    /// use markdown::{to_html_with_options, CompileOptions, Options, ParseOptions};
    /// # fn main() -> Result<(), markdown::message::Message> {
    ///
    /// // `"h2"` is used by default:
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "[^a]\n\n[^a]: b",
    ///         &Options::gfm()
    ///     )?,
    ///     "<p><sup><a href=\"#user-content-fn-a\" id=\"user-content-fnref-a\" data-footnote-ref=\"\" aria-describedby=\"footnote-label\">1</a></sup></p>\n<section data-footnotes=\"\" class=\"footnotes\"><h2 id=\"footnote-label\" class=\"sr-only\">Footnotes</h2>\n<ol>\n<li id=\"user-content-fn-a\">\n<p>b <a href=\"#user-content-fnref-a\" data-footnote-backref=\"\" aria-label=\"Back to content\" class=\"data-footnote-backref\">↩</a></p>\n</li>\n</ol>\n</section>\n"
    /// );
    ///
    /// // Pass `gfm_footnote_label_tag_name` to use something else:
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "[^a]\n\n[^a]: b",
    ///         &Options {
    ///             parse: ParseOptions::gfm(),
    ///             compile: CompileOptions {
    ///               gfm_footnote_label_tag_name: Some("h1".into()),
    ///               ..CompileOptions::gfm()
    ///             }
    ///         }
    ///     )?,
    ///     "<p><sup><a href=\"#user-content-fn-a\" id=\"user-content-fnref-a\" data-footnote-ref=\"\" aria-describedby=\"footnote-label\">1</a></sup></p>\n<section data-footnotes=\"\" class=\"footnotes\"><h1 id=\"footnote-label\" class=\"sr-only\">Footnotes</h1>\n<ol>\n<li id=\"user-content-fn-a\">\n<p>b <a href=\"#user-content-fnref-a\" data-footnote-backref=\"\" aria-label=\"Back to content\" class=\"data-footnote-backref\">↩</a></p>\n</li>\n</ol>\n</section>\n"
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub gfm_footnote_label_tag_name: Option<String>,

    /// Textual label to use for the footnotes section.
    ///
    /// The default value is `"Footnotes"`.
    /// Change it when the markdown is not in English.
    ///
    /// This label is typically hidden visually (assuming a `sr-only` CSS class
    /// is defined that does that), and thus affects screen readers only.
    /// If you do have such a class, but want to show this section to everyone,
    /// pass different attributes with the `gfm_footnote_label_attributes`
    /// option.
    ///
    /// ## Examples
    ///
    /// ```
    /// use markdown::{to_html_with_options, CompileOptions, Options, ParseOptions};
    /// # fn main() -> Result<(), markdown::message::Message> {
    ///
    /// // `"Footnotes"` is used by default:
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "[^a]\n\n[^a]: b",
    ///         &Options::gfm()
    ///     )?,
    ///     "<p><sup><a href=\"#user-content-fn-a\" id=\"user-content-fnref-a\" data-footnote-ref=\"\" aria-describedby=\"footnote-label\">1</a></sup></p>\n<section data-footnotes=\"\" class=\"footnotes\"><h2 id=\"footnote-label\" class=\"sr-only\">Footnotes</h2>\n<ol>\n<li id=\"user-content-fn-a\">\n<p>b <a href=\"#user-content-fnref-a\" data-footnote-backref=\"\" aria-label=\"Back to content\" class=\"data-footnote-backref\">↩</a></p>\n</li>\n</ol>\n</section>\n"
    /// );
    ///
    /// // Pass `gfm_footnote_label` to use something else:
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "[^a]\n\n[^a]: b",
    ///         &Options {
    ///             parse: ParseOptions::gfm(),
    ///             compile: CompileOptions {
    ///               gfm_footnote_label: Some("Notes de bas de page".into()),
    ///               ..CompileOptions::gfm()
    ///             }
    ///         }
    ///     )?,
    ///     "<p><sup><a href=\"#user-content-fn-a\" id=\"user-content-fnref-a\" data-footnote-ref=\"\" aria-describedby=\"footnote-label\">1</a></sup></p>\n<section data-footnotes=\"\" class=\"footnotes\"><h2 id=\"footnote-label\" class=\"sr-only\">Notes de bas de page</h2>\n<ol>\n<li id=\"user-content-fn-a\">\n<p>b <a href=\"#user-content-fnref-a\" data-footnote-backref=\"\" aria-label=\"Back to content\" class=\"data-footnote-backref\">↩</a></p>\n</li>\n</ol>\n</section>\n"
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub gfm_footnote_label: Option<String>,

    /// Whether or not GFM task list html `<input>` items are enabled.
    ///
    /// This determines whether or not the user of the browser is able
    /// to click and toggle generated checkbox items. The default is false.
    ///
    /// ## Examples
    ///
    /// ```
    /// use markdown::{to_html_with_options, CompileOptions, Options, ParseOptions};
    /// # fn main() -> Result<(), markdown::message::Message> {
    ///
    /// // With `gfm_task_list_item_checkable`, generated `<input type="checkbox" />`
    /// // tags do not contain the attribute `disabled=""` and are thus toggleable by
    /// // browser users.
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "* [x] y.",
    ///         &Options {
    ///             parse: ParseOptions::gfm(),
    ///             compile: CompileOptions {
    ///                 gfm_task_list_item_checkable: true,
    ///                 ..CompileOptions::gfm()
    ///             }
    ///         }
    ///     )?,
    ///     "<ul>\n<li><input type=\"checkbox\" checked=\"\" /> y.</li>\n</ul>"
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub gfm_task_list_item_checkable: bool,

    /// Whether to support the GFM tagfilter.
    ///
    /// This option does nothing if `allow_dangerous_html` is not turned on.
    /// The default is `false`, which does not apply the GFM tagfilter to HTML.
    /// Pass `true` for output that is a bit closer to GitHub’s actual output.
    ///
    /// The tagfilter is kinda weird and kinda useless.
    /// The tag filter is a naïve attempt at XSS protection.
    /// You should use a proper HTML sanitizing algorithm instead.
    ///
    /// ## Examples
    ///
    /// ```
    /// use markdown::{to_html_with_options, CompileOptions, Options, ParseOptions};
    /// # fn main() -> Result<(), markdown::message::Message> {
    ///
    /// // With `allow_dangerous_html`, `markdown-rs` passes HTML through untouched:
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "<iframe>",
    ///         &Options {
    ///             parse: ParseOptions::gfm(),
    ///             compile: CompileOptions {
    ///               allow_dangerous_html: true,
    ///               ..CompileOptions::default()
    ///             }
    ///         }
    ///     )?,
    ///     "<iframe>"
    /// );
    ///
    /// // Pass `gfm_tagfilter: true` to make some of that safe:
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "<iframe>",
    ///         &Options {
    ///             parse: ParseOptions::gfm(),
    ///             compile: CompileOptions {
    ///               allow_dangerous_html: true,
    ///               gfm_tagfilter: true,
    ///               ..CompileOptions::default()
    ///             }
    ///         }
    ///     )?,
    ///     "&lt;iframe>"
    /// );
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## References
    ///
    /// * [*§ 6.1 Disallowed Raw HTML (extension)* in GFM](https://github.github.com/gfm/#disallowed-raw-html-extension-)
    /// * [`cmark-gfm#extensions/tagfilter.c`](https://github.com/github/cmark-gfm/blob/master/extensions/tagfilter.c)
    pub gfm_tagfilter: bool,

    /// Resolver for Obsidian wikilinks.
    ///
    /// When `None` (the default), wikilinks are rendered with a best-effort
    /// `<a href="{path}{#heading|#^blockid}">{alias|path|heading}</a>`.
    /// Pass a closure to resolve wikilinks against a vault index, base URL,
    /// or other configuration.
    ///
    /// This field is not serialized by serde and is always `None` after
    /// deserialization.
    ///
    /// ## Examples
    ///
    /// ```
    /// use markdown::{
    ///     to_html_with_options, CompileOptions, Options, ParseOptions,
    ///     ObsidianLinkResolution, ObsidianLinkTarget,
    /// };
    /// # fn main() -> Result<(), markdown::message::Message> {
    ///
    /// let options = Options {
    ///     parse: ParseOptions::obsidian(),
    ///     compile: CompileOptions {
    ///         obsidian_link_resolver: Some(std::sync::Arc::new(
    ///             |target: &ObsidianLinkTarget| ObsidianLinkResolution {
    ///                 href: format!("/notes/{}", target.path.clone().unwrap_or_default()),
    ///                 text: target.alias.clone(),
    ///             },
    ///         )),
    ///         ..CompileOptions::default()
    ///     },
    /// };
    ///
    /// assert_eq!(
    ///     to_html_with_options("[[Note|Alias]]", &options)?,
    ///     "<p><a href=\"/notes/Note\">Alias</a></p>"
    /// );
    /// # Ok(())
    /// # }
    /// ```
    #[cfg_attr(feature = "serde", serde(skip))]
    pub obsidian_link_resolver: Option<Arc<ObsidianLinkResolver>>,

    /// Resolver for Obsidian embeds.
    ///
    /// When `None` (the default), embeds are rendered with best-effort media
    /// sniffing (`<img>`, `<audio>`, `<video>`, `<iframe>` for known
    /// extensions, else an `<a>` to the note).
    /// Pass a closure to perform transclusion or generate custom embed HTML.
    ///
    /// This field is not serialized by serde and is always `None` after
    /// deserialization.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub obsidian_embed_resolver: Option<Arc<ObsidianEmbedResolver>>,
}

impl fmt::Debug for CompileOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompileOptions")
            .field("allow_any_img_src", &self.allow_any_img_src)
            .field("allow_dangerous_html", &self.allow_dangerous_html)
            .field("allow_dangerous_protocol", &self.allow_dangerous_protocol)
            .field("default_line_ending", &self.default_line_ending)
            .field("gfm_footnote_back_label", &self.gfm_footnote_back_label)
            .field(
                "gfm_footnote_clobber_prefix",
                &self.gfm_footnote_clobber_prefix,
            )
            .field(
                "gfm_footnote_label_attributes",
                &self.gfm_footnote_label_attributes,
            )
            .field(
                "gfm_footnote_label_tag_name",
                &self.gfm_footnote_label_tag_name,
            )
            .field("gfm_footnote_label", &self.gfm_footnote_label)
            .field(
                "gfm_task_list_item_checkable",
                &self.gfm_task_list_item_checkable,
            )
            .field("gfm_tagfilter", &self.gfm_tagfilter)
            .field(
                "obsidian_link_resolver",
                &self.obsidian_link_resolver.as_ref().map(|_d| "[Function]"),
            )
            .field(
                "obsidian_embed_resolver",
                &self.obsidian_embed_resolver.as_ref().map(|_d| "[Function]"),
            )
            .finish()
    }
}

impl CompileOptions {
    /// GFM.
    ///
    /// GFM stands for **GitHub flavored markdown**.
    /// On the compilation side, GFM turns on the GFM tag filter.
    /// The tagfilter is useless, but it’s included here for consistency, and
    /// this method exists for parity to parse options.
    ///
    /// For more information, see the GFM specification:
    /// <https://github.github.com/gfm/>.
    pub fn gfm() -> Self {
        Self {
            gfm_tagfilter: true,
            ..Self::default()
        }
    }

    /// Obsidian Flavored Markdown.
    ///
    /// On the compilation side, OFM uses best-effort rendering for wikilinks
    /// and embeds (resolvers are `None`). Pass custom resolvers to customize
    /// rendering. This preset exists for parity with
    /// [`ParseOptions::obsidian()`].
    ///
    /// For more information, see the Obsidian Flavored Markdown documentation:
    /// <https://obsidian.md/help/obsidian-flavored-markdown>.
    pub fn obsidian() -> Self {
        Self::default()
    }
}

/// Configuration that describes how to parse from markdown.
///
/// You can use this:
///
/// * To control what markdown constructs are turned on and off
/// * To control some of those constructs
/// * To add support for certain programming languages when parsing MDX
///
/// In most cases, you will want to use the default trait or `gfm` method.
///
/// ## Examples
///
/// ```
/// use markdown::ParseOptions;
/// # fn main() {
///
/// // Use the default trait to parse markdown according to `CommonMark`:
/// let commonmark = ParseOptions::default();
///
/// // Use the `gfm` method to parse markdown according to GFM:
/// let gfm = ParseOptions::gfm();
/// # }
/// ```
#[allow(clippy::struct_excessive_bools)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default, rename_all = "camelCase")
)]
pub struct ParseOptions {
    // Note: when adding fields, don’t forget to add them to `fmt::Debug` below.
    /// Which constructs to enable and disable.
    ///
    /// The default is to follow `CommonMark`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use markdown::{to_html, to_html_with_options, Constructs, Options, ParseOptions};
    /// # fn main() -> Result<(), markdown::message::Message> {
    ///
    /// // `markdown-rs` follows CommonMark by default:
    /// assert_eq!(
    ///     to_html("    indented code?"),
    ///     "<pre><code>indented code?\n</code></pre>"
    /// );
    ///
    /// // Pass `constructs` to choose what to enable and disable:
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "    indented code?",
    ///         &Options {
    ///             parse: ParseOptions {
    ///               constructs: Constructs {
    ///                 code_indented: false,
    ///                 ..Constructs::default()
    ///               },
    ///               ..ParseOptions::default()
    ///             },
    ///             ..Options::default()
    ///         }
    ///     )?,
    ///     "<p>indented code?</p>"
    /// );
    /// # Ok(())
    /// # }
    /// ```
    #[cfg_attr(feature = "serde", serde(default))]
    pub constructs: Constructs,

    /// Whether to support GFM strikethrough with a single tilde
    ///
    /// This option does nothing if `gfm_strikethrough` is not turned on in
    /// `constructs`.
    /// This option does not affect strikethrough with double tildes.
    ///
    /// The default is `true`, which follows how markdown on `github.com`
    /// works, as strikethrough with single tildes is supported.
    /// Pass `false`, to follow the GFM spec more strictly, by not allowing
    /// strikethrough with single tildes.
    ///
    /// ## Examples
    ///
    /// ```
    /// use markdown::{to_html_with_options, Constructs, Options, ParseOptions};
    /// # fn main() -> Result<(), markdown::message::Message> {
    ///
    /// // `markdown-rs` supports single tildes by default:
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "~a~",
    ///         &Options {
    ///             parse: ParseOptions {
    ///               constructs: Constructs::gfm(),
    ///               ..ParseOptions::default()
    ///             },
    ///             ..Options::default()
    ///         }
    ///     )?,
    ///     "<p><del>a</del></p>"
    /// );
    ///
    /// // Pass `gfm_strikethrough_single_tilde: false` to turn that off:
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "~a~",
    ///         &Options {
    ///             parse: ParseOptions {
    ///               constructs: Constructs::gfm(),
    ///               gfm_strikethrough_single_tilde: false,
    ///               ..ParseOptions::default()
    ///             },
    ///             ..Options::default()
    ///         }
    ///     )?,
    ///     "<p>~a~</p>"
    /// );
    /// # Ok(())
    /// # }
    /// ```
    #[cfg_attr(feature = "serde", serde(default))]
    pub gfm_strikethrough_single_tilde: bool,

    /// Whether to support math (text) with a single dollar
    ///
    /// This option does nothing if `math_text` is not turned on in
    /// `constructs`.
    /// This option does not affect math (text) with two or more dollars.
    ///
    /// The default is `true`, which is more close to how code (text) and
    /// Pandoc work, as it allows math with a single dollar to form.
    /// However, single dollars can interfere with “normal” dollars in text.
    /// Pass `false`, to only allow math (text) to form when two or more
    /// dollars are used.
    /// If you pass `false`, you can still use two or more dollars for text
    /// math.
    ///
    /// ## Examples
    ///
    /// ```
    /// use markdown::{to_html_with_options, Constructs, Options, ParseOptions};
    /// # fn main() -> Result<(), markdown::message::Message> {
    ///
    /// // `markdown-rs` supports single dollars by default:
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "$a$",
    ///         &Options {
    ///             parse: ParseOptions {
    ///               constructs: Constructs {
    ///                 math_text: true,
    ///                 ..Constructs::default()
    ///               },
    ///               ..ParseOptions::default()
    ///             },
    ///             ..Options::default()
    ///         }
    ///     )?,
    ///     "<p><code class=\"language-math math-inline\">a</code></p>"
    /// );
    ///
    /// // Pass `math_text_single_dollar: false` to turn that off:
    /// assert_eq!(
    ///     to_html_with_options(
    ///         "$a$",
    ///         &Options {
    ///             parse: ParseOptions {
    ///               constructs: Constructs {
    ///                 math_text: true,
    ///                 ..Constructs::default()
    ///               },
    ///               math_text_single_dollar: false,
    ///               ..ParseOptions::default()
    ///             },
    ///             ..Options::default()
    ///         }
    ///     )?,
    ///     "<p>$a$</p>"
    /// );
    /// # Ok(())
    /// # }
    /// ```
    #[cfg_attr(feature = "serde", serde(default))]
    pub math_text_single_dollar: bool,

    /// Function to parse expressions with.
    ///
    /// This function can be used to add support for arbitrary programming
    /// languages within expressions.
    ///
    /// It only makes sense to pass this when compiling to a syntax tree
    /// with [`to_mdast()`][crate::to_mdast()].
    ///
    /// For an example that adds support for JavaScript with SWC, see
    /// `tests/test_utils/mod.rs`.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub mdx_expression_parse: Option<Box<MdxExpressionParse>>,

    /// Function to parse ESM with.
    ///
    /// This function can be used to add support for arbitrary programming
    /// languages within ESM blocks, however, the keywords (`export`,
    /// `import`) are currently hardcoded JavaScript-specific.
    ///
    /// > 👉 **Note**: please raise an issue if you’re interested in working on
    /// > MDX that is aware of, say, Rust, or other programming languages.
    ///
    /// It only makes sense to pass this when compiling to a syntax tree
    /// with [`to_mdast()`][crate::to_mdast()].
    ///
    /// For an example that adds support for JavaScript with SWC, see
    /// `tests/test_utils/mod.rs`.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub mdx_esm_parse: Option<Box<MdxEsmParse>>,
    // Note: when adding fields, don’t forget to add them to `fmt::Debug` below.
}

impl fmt::Debug for ParseOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParseOptions")
            .field("constructs", &self.constructs)
            .field(
                "gfm_strikethrough_single_tilde",
                &self.gfm_strikethrough_single_tilde,
            )
            .field("math_text_single_dollar", &self.math_text_single_dollar)
            .field(
                "mdx_expression_parse",
                &self.mdx_expression_parse.as_ref().map(|_d| "[Function]"),
            )
            .field(
                "mdx_esm_parse",
                &self.mdx_esm_parse.as_ref().map(|_d| "[Function]"),
            )
            .finish()
    }
}

impl Default for ParseOptions {
    /// `CommonMark` defaults.
    fn default() -> Self {
        Self {
            constructs: Constructs::default(),
            gfm_strikethrough_single_tilde: true,
            math_text_single_dollar: true,
            mdx_expression_parse: None,
            mdx_esm_parse: None,
        }
    }
}

impl ParseOptions {
    /// GFM.
    ///
    /// GFM stands for GitHub flavored markdown.
    /// GFM extends `CommonMark` and adds support for autolink literals,
    /// footnotes, strikethrough, tables, and tasklists.
    ///
    /// For more information, see the GFM specification:
    /// <https://github.github.com/gfm/>
    pub fn gfm() -> Self {
        Self {
            constructs: Constructs::gfm(),
            ..Self::default()
        }
    }

    /// MDX.
    ///
    /// This turns on `CommonMark`, turns off some conflicting constructs
    /// (autolinks, code (indented), and HTML), and turns on MDX (ESM,
    /// expressions, and JSX).
    ///
    /// For more information, see the MDX website:
    /// <https://mdxjs.com>.
    ///
    /// > 👉 **Note**: to support ESM, you *must* pass
    /// > [`mdx_esm_parse`][MdxEsmParse] in [`ParseOptions`][] too.
    /// > Otherwise, ESM is treated as normal markdown.
    /// >
    /// > You *can* pass
    /// > [`mdx_expression_parse`][MdxExpressionParse]
    /// > to parse expressions according to a certain grammar (typically, a
    /// > programming language).
    /// > Otherwise, expressions are parsed with a basic algorithm that only
    /// > cares about braces.
    pub fn mdx() -> Self {
        Self {
            constructs: Constructs::mdx(),
            ..Self::default()
        }
    }

    /// Obsidian Flavored Markdown.
    ///
    /// OFM extends `CommonMark` with wikilinks, embeds, block ids, comments,
    /// highlights, and callouts.
    ///
    /// For more information, see the Obsidian Flavored Markdown documentation:
    /// <https://obsidian.md/help/obsidian-flavored-markdown>.
    pub fn obsidian() -> Self {
        Self {
            constructs: Constructs::obsidian(),
            ..Self::default()
        }
    }

    /// `CodeHike` blocks.
    ///
    /// Enables the `CodeHike`-style decorated block transform on the mdast
    /// produced by [`to_mdast`][crate::to_mdast]. See
    /// [`Constructs::code_hike_blocks`] for details.
    pub fn code_hike() -> Self {
        Self {
            constructs: Constructs::code_hike(),
            ..Self::default()
        }
    }
}

/// Configuration that describes how to parse from markdown and compile to
/// HTML.
///
/// In most cases, you will want to use the default trait or `gfm` method.
///
/// ## Examples
///
/// ```
/// use markdown::Options;
/// # fn main() {
///
/// // Use the default trait to compile markdown to HTML according to `CommonMark`:
/// let commonmark = Options::default();
///
/// // Use the `gfm` method to compile markdown to HTML according to GFM:
/// let gfm = Options::gfm();
/// # }
/// ```
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
pub struct Options {
    /// Configuration that describes how to parse from markdown.
    pub parse: ParseOptions,
    /// Configuration that describes how to compile to HTML.
    pub compile: CompileOptions,
}

impl Options {
    /// GFM.
    ///
    /// GFM stands for GitHub flavored markdown.
    /// GFM extends `CommonMark` and adds support for autolink literals,
    /// footnotes, strikethrough, tables, and tasklists.
    /// On the compilation side, GFM turns on the GFM tag filter.
    /// The tagfilter is useless, but it’s included here for consistency.
    ///
    /// For more information, see the GFM specification:
    /// <https://github.github.com/gfm/>
    pub fn gfm() -> Self {
        Self {
            parse: ParseOptions::gfm(),
            compile: CompileOptions::gfm(),
        }
    }

    /// Obsidian Flavored Markdown.
    ///
    /// OFM extends `CommonMark` with wikilinks, embeds, block ids, comments,
    /// highlights, and callouts. This preset turns on the OFM parse constructs
    /// and uses best-effort compilation (resolvers are `None`).
    ///
    /// For more information, see the Obsidian Flavored Markdown documentation:
    /// <https://obsidian.md/help/obsidian-flavored-markdown>.
    pub fn obsidian() -> Self {
        Self {
            parse: ParseOptions::obsidian(),
            compile: CompileOptions::obsidian(),
        }
    }

    /// `CodeHike` blocks.
    ///
    /// Enables the `CodeHike`-style decorated block transform on the mdast
    /// produced by [`to_mdast`][crate::to_mdast]. See
    /// [`Constructs::code_hike_blocks`] for details.
    ///
    /// > 👉 **Note**: `CodeHike` blocks currently affect `to_mdast()` only.
    /// > HTML output (`to_html` / `to_html_with_options`) remains normal
    /// > markdown because it compiles directly from parser events.
    pub fn code_hike() -> Self {
        Self {
            parse: ParseOptions::code_hike(),
            compile: CompileOptions::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::mdx::Signal;
    use alloc::format;

    #[test]
    fn test_constructs() {
        Constructs::default();
        Constructs::gfm();
        Constructs::mdx();
        Constructs::obsidian();
        Constructs::code_hike();

        let constructs = Constructs::default();
        assert!(constructs.attention, "should default to `CommonMark` (1)");
        assert!(
            !constructs.gfm_autolink_literal,
            "should default to `CommonMark` (2)"
        );
        assert!(
            !constructs.mdx_jsx_flow,
            "should default to `CommonMark` (3)"
        );
        assert!(
            !constructs.frontmatter,
            "should default to `CommonMark` (4)"
        );
        assert!(
            !constructs.code_hike_blocks,
            "should default to `CommonMark` (5)"
        );

        let constructs = Constructs::gfm();
        assert!(constructs.attention, "should support `gfm` shortcut (1)");
        assert!(
            constructs.gfm_autolink_literal,
            "should support `gfm` shortcut (2)"
        );
        assert!(
            !constructs.mdx_jsx_flow,
            "should support `gfm` shortcut (3)"
        );
        assert!(!constructs.frontmatter, "should support `gfm` shortcut (4)");

        let constructs = Constructs::mdx();
        assert!(constructs.attention, "should support `gfm` shortcut (1)");
        assert!(
            !constructs.gfm_autolink_literal,
            "should support `mdx` shortcut (2)"
        );
        assert!(constructs.mdx_jsx_flow, "should support `mdx` shortcut (3)");
        assert!(!constructs.frontmatter, "should support `mdx` shortcut (4)");

        let constructs = Constructs::obsidian();
        assert!(
            constructs.obsidian_wikilink,
            "should support `obsidian` shortcut (1)"
        );
        assert!(
            constructs.obsidian_embed,
            "should support `obsidian` shortcut (2)"
        );
        assert!(
            constructs.obsidian_callout,
            "should support `obsidian` shortcut (3)"
        );
        assert!(
            !constructs.gfm_autolink_literal,
            "should support `obsidian` shortcut (4)"
        );

        let constructs = Constructs::code_hike();
        assert!(
            constructs.code_hike_blocks,
            "should support `code_hike` shortcut (1)"
        );
        assert!(
            constructs.attention,
            "should support `code_hike` shortcut (2)"
        );
        assert!(
            !constructs.gfm_autolink_literal,
            "should support `code_hike` shortcut (3)"
        );
    }

    #[test]
    fn test_parse_options() {
        ParseOptions::default();
        ParseOptions::gfm();
        ParseOptions::mdx();
        ParseOptions::obsidian();
        ParseOptions::code_hike();

        let options = ParseOptions::default();
        assert!(
            options.constructs.attention,
            "should default to `CommonMark` (1)"
        );
        assert!(
            !options.constructs.gfm_autolink_literal,
            "should default to `CommonMark` (2)"
        );
        assert!(
            !options.constructs.mdx_jsx_flow,
            "should default to `CommonMark` (3)"
        );
        assert!(
            !options.constructs.code_hike_blocks,
            "should default to `CommonMark` (4)"
        );

        let options = ParseOptions::gfm();
        assert!(
            options.constructs.attention,
            "should support `gfm` shortcut (1)"
        );
        assert!(
            options.constructs.gfm_autolink_literal,
            "should support `gfm` shortcut (2)"
        );
        assert!(
            !options.constructs.mdx_jsx_flow,
            "should support `gfm` shortcut (3)"
        );

        let options = ParseOptions::mdx();
        assert!(
            options.constructs.attention,
            "should support `mdx` shortcut (1)"
        );
        assert!(
            !options.constructs.gfm_autolink_literal,
            "should support `mdx` shortcut (2)"
        );
        assert!(
            options.constructs.mdx_jsx_flow,
            "should support `mdx` shortcut (3)"
        );

        let options = ParseOptions::obsidian();
        assert!(
            options.constructs.obsidian_wikilink,
            "should support `obsidian` shortcut (1)"
        );
        assert!(
            options.constructs.obsidian_callout,
            "should support `obsidian` shortcut (2)"
        );
        assert!(
            !options.constructs.gfm_autolink_literal,
            "should support `obsidian` shortcut (3)"
        );

        let options = ParseOptions::code_hike();
        assert!(
            options.constructs.code_hike_blocks,
            "should support `code_hike` shortcut (1)"
        );
        assert!(
            options.constructs.attention,
            "should support `code_hike` shortcut (2)"
        );

        assert_eq!(
            format!("{:?}", ParseOptions::default()),
            "ParseOptions { constructs: Constructs { attention: true, autolink: true, block_quote: true, character_escape: true, character_reference: true, code_indented: true, code_fenced: true, code_text: true, definition: true, frontmatter: false, gfm_autolink_literal: false, gfm_footnote_definition: false, gfm_label_start_footnote: false, gfm_strikethrough: false, gfm_table: false, gfm_task_list_item: false, hard_break_escape: true, hard_break_trailing: true, heading_atx: true, heading_setext: true, html_flow: true, html_text: true, label_start_image: true, label_start_link: true, label_end: true, list_item: true, math_flow: false, math_text: false, mdx_esm: false, mdx_expression_flow: false, mdx_expression_text: false, mdx_jsx_flow: false, mdx_jsx_text: false, thematic_break: true, obsidian_wikilink: false, obsidian_embed: false, obsidian_block_id: false, obsidian_comment: false, obsidian_highlight: false, obsidian_callout: false, code_hike_blocks: false }, gfm_strikethrough_single_tilde: true, math_text_single_dollar: true, mdx_expression_parse: None, mdx_esm_parse: None }",
            "should support `Debug` trait"
        );
        assert_eq!(
            format!("{:?}", ParseOptions {
                mdx_esm_parse: Some(Box::new(|_value| {
                    Signal::Ok
                })),
                mdx_expression_parse: Some(Box::new(|_value, _kind| {
                    Signal::Ok
                })),
                ..Default::default()
            }),
            "ParseOptions { constructs: Constructs { attention: true, autolink: true, block_quote: true, character_escape: true, character_reference: true, code_indented: true, code_fenced: true, code_text: true, definition: true, frontmatter: false, gfm_autolink_literal: false, gfm_footnote_definition: false, gfm_label_start_footnote: false, gfm_strikethrough: false, gfm_table: false, gfm_task_list_item: false, hard_break_escape: true, hard_break_trailing: true, heading_atx: true, heading_setext: true, html_flow: true, html_text: true, label_start_image: true, label_start_link: true, label_end: true, list_item: true, math_flow: false, math_text: false, mdx_esm: false, mdx_expression_flow: false, mdx_expression_text: false, mdx_jsx_flow: false, mdx_jsx_text: false, thematic_break: true, obsidian_wikilink: false, obsidian_embed: false, obsidian_block_id: false, obsidian_comment: false, obsidian_highlight: false, obsidian_callout: false, code_hike_blocks: false }, gfm_strikethrough_single_tilde: true, math_text_single_dollar: true, mdx_expression_parse: Some(\"[Function]\"), mdx_esm_parse: Some(\"[Function]\") }",
            "should support `Debug` trait on mdx functions"
        );
    }

    #[test]
    fn test_compile_options() {
        CompileOptions::default();
        CompileOptions::gfm();
        CompileOptions::obsidian();

        let options = CompileOptions::default();
        assert!(
            !options.allow_dangerous_html,
            "should default to safe `CommonMark` (1)"
        );
        assert!(
            !options.gfm_tagfilter,
            "should default to safe `CommonMark` (2)"
        );
        assert!(
            options.obsidian_link_resolver.is_none(),
            "should default to safe `CommonMark` (3)"
        );

        let options = CompileOptions::gfm();
        assert!(
            !options.allow_dangerous_html,
            "should support safe `gfm` shortcut (1)"
        );
        assert!(
            options.gfm_tagfilter,
            "should support safe `gfm` shortcut (1)"
        );

        let options = CompileOptions::obsidian();
        assert!(
            options.obsidian_link_resolver.is_none(),
            "should support `obsidian` shortcut (1)"
        );
        assert!(
            options.obsidian_embed_resolver.is_none(),
            "should support `obsidian` shortcut (2)"
        );
    }

    #[test]
    fn test_options() {
        Options::default();
        Options::code_hike();

        let options = Options::default();
        assert!(
            options.parse.constructs.attention,
            "should default to safe `CommonMark` (1)"
        );
        assert!(
            !options.parse.constructs.gfm_autolink_literal,
            "should default to safe `CommonMark` (2)"
        );
        assert!(
            !options.parse.constructs.mdx_jsx_flow,
            "should default to safe `CommonMark` (3)"
        );
        assert!(
            !options.compile.allow_dangerous_html,
            "should default to safe `CommonMark` (4)"
        );

        let options = Options::gfm();
        assert!(
            options.parse.constructs.attention,
            "should support safe `gfm` shortcut (1)"
        );
        assert!(
            options.parse.constructs.gfm_autolink_literal,
            "should support safe `gfm` shortcut (2)"
        );
        assert!(
            !options.parse.constructs.mdx_jsx_flow,
            "should support safe `gfm` shortcut (3)"
        );
        assert!(
            !options.compile.allow_dangerous_html,
            "should support safe `gfm` shortcut (4)"
        );

        let options = Options::obsidian();
        assert!(
            options.parse.constructs.obsidian_wikilink,
            "should support `obsidian` shortcut (1)"
        );
        assert!(
            options.parse.constructs.obsidian_callout,
            "should support `obsidian` shortcut (2)"
        );
        assert!(
            options.compile.obsidian_link_resolver.is_none(),
            "should support `obsidian` shortcut (3)"
        );

        let options = Options::code_hike();
        assert!(
            options.parse.constructs.code_hike_blocks,
            "should support `code_hike` shortcut (1)"
        );
        assert!(
            !options.compile.allow_dangerous_html,
            "should support `code_hike` shortcut (2)"
        );
    }
}
