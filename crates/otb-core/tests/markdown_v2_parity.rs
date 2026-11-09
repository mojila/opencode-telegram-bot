//! Parity port of `tests/bot/render/markdown-to-telegram-v2.test.ts`.
//!
//! Step 1 (RED): these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).
//!
//! TS `toContain` becomes `assert!(output.contains(...))`, `toBe` becomes
//! `assert_eq!`, and `not.toThrow()` is implicit: just calling the function
//! (a Rust panic on malformed input would fail the test).

use otb_core::render::markdown_v2::convert_to_telegram_markdown_v2;

#[test]
fn converts_common_inline_markdown_and_escapes_telegram_markdown_v2_text() {
    assert_eq!(
        convert_to_telegram_markdown_v2("Use **bold** and *italic* with a+b!"),
        "Use *bold* and _italic_ with a\\+b\\!"
    );
}

#[test]
fn escapes_code_and_link_delimiters_that_would_break_markdown_v2() {
    let output =
        convert_to_telegram_markdown_v2("Use `C:\\temp` and [docs](https://example.com/a_(b))");

    assert!(
        output.contains("`C:\\\\temp`"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("[docs](https://example.com/a_(b\\))"),
        "unexpected output: {output}"
    );
}

#[test]
fn renders_lists_and_tables_as_escaped_plain_telegram_markdown_v2_text() {
    let output = convert_to_telegram_markdown_v2(
        &["- item", "", "| A | B |", "| --- | --- |", "| C | D |"].join("\n"),
    );

    assert!(output.contains("\\- item"), "unexpected output: {output}");
    assert!(
        output.contains("\\| A \\| B \\|"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("\\| \\-\\-\\- \\| \\-\\-\\- \\|"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("\\| C \\| D \\|"),
        "unexpected output: {output}"
    );
}

#[test]
fn preserves_malformed_markdown_content_instead_of_throwing() {
    let input = [
        "*text: *value**",
        "<div>raw_html</div>",
        "[broken](https://example.com/foo",
    ]
    .join("\n\n");

    // Must not throw (panic): just calling it exercises this.
    let output = convert_to_telegram_markdown_v2(&input);

    assert!(output.contains("text"), "unexpected output: {output}");
    assert!(output.contains("value"), "unexpected output: {output}");
    assert!(output.contains("raw\\_html"), "unexpected output: {output}");
    assert!(output.contains("broken"), "unexpected output: {output}");
}
